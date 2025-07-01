//
//  LogsView.swift
//  librorum
//
//  Log viewing and monitoring
//

import SwiftUI
import SwiftData

struct LogsView: View {
    let coreManager: CoreManager
    @State private var logEntries: [LogEntry] = []
    @State private var isLoading = false
    @State private var selectedLogLevel: LogLevel = .all
    @State private var searchText = ""
    @State private var autoRefresh = true
    @State private var refreshTimer: Timer?
    @State private var showingExportSheet = false
    
    var filteredLogs: [LogEntry] {
        logEntries
            .filter { entry in
                if selectedLogLevel != .all && entry.level != selectedLogLevel {
                    return false
                }
                if !searchText.isEmpty && !entry.message.localizedCaseInsensitiveContains(searchText) {
                    return false
                }
                return true
            }
            .sorted { $0.timestamp > $1.timestamp }
    }
    
    var body: some View {
        VStack(spacing: 0) {
            // Controls
            LogControlsView(
                selectedLogLevel: $selectedLogLevel,
                searchText: $searchText,
                autoRefresh: $autoRefresh,
                onRefresh: {
                    await refreshLogs()
                },
                onExport: {
                    showingExportSheet = true
                },
                onClear: {
                    clearLogs()
                }
            )
            
            Divider()
            
            // Log List
            if isLoading {
                ProgressView("加载日志...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if filteredLogs.isEmpty {
                LogEmptyStateView(hasLogs: !logEntries.isEmpty)
            } else {
                LogListView(logs: filteredLogs)
            }
        }
        .navigationTitle("日志")
        .onAppear {
            Task {
                await refreshLogs()
            }
            startAutoRefresh()
        }
        .onDisappear {
            stopAutoRefresh()
        }
        .onChange(of: autoRefresh) { _, newValue in
            if newValue {
                startAutoRefresh()
            } else {
                stopAutoRefresh()
            }
        }
        .sheet(isPresented: $showingExportSheet) {
            LogExportSheet(logs: filteredLogs)
        }
    }
    
    private func refreshLogs() async {
        isLoading = true
        defer { isLoading = false }
        
        do {
            let newLogs = try await loadLogEntries()
            logEntries = newLogs
        } catch {
            print("Failed to load logs: \(error)")
        }
    }
    
    private func loadLogEntries() async throws -> [LogEntry] {
        // TODO: Implement actual log loading from backend
        // For now, return mock data
        return generateMockLogs()
    }
    
    private func generateMockLogs() -> [LogEntry] {
        let levels: [LogLevel] = [.info, .warn, .error, .debug, .trace]
        let messages = [
            "Backend service started successfully",
            "Node discovery completed, found 3 nodes",
            "File upload completed: example.txt",
            "Heartbeat received from node: local.librorum.local",
            "Storage usage: 25% (250MB/1GB)",
            "Network latency check: 15ms",
            "Configuration reloaded",
            "Connection established with node: remote.librorum.local",
            "Chunk replication completed",
            "Health check passed"
        ]
        
        return (0..<100).map { index in
            LogEntry(
                timestamp: Date().addingTimeInterval(-Double(index * 30)),
                level: levels.randomElement() ?? .info,
                module: ["core", "network", "storage", "grpc"].randomElement() ?? "core",
                message: messages.randomElement() ?? "Log message \(index)"
            )
        }
    }
    
    private func clearLogs() {
        logEntries.removeAll()
    }
    
    private func startAutoRefresh() {
        guard autoRefresh else { return }
        
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            Task {
                await refreshLogs()
            }
        }
    }
    
    private func stopAutoRefresh() {
        refreshTimer?.invalidate()
        refreshTimer = nil
    }
}

struct LogControlsView: View {
    @Binding var selectedLogLevel: LogLevel
    @Binding var searchText: String
    @Binding var autoRefresh: Bool
    let onRefresh: () async -> Void
    let onExport: () -> Void
    let onClear: () -> Void
    
    var body: some View {
        VStack(spacing: 12) {
            // Top row: Level picker and Auto refresh
            HStack {
                Picker("日志级别", selection: $selectedLogLevel) {
                    ForEach(LogLevel.allCases, id: \.self) { level in
                        Text(level.displayName).tag(level)
                    }
                }
                .pickerStyle(MenuPickerStyle())
                
                Spacer()
                
                Toggle("自动刷新", isOn: $autoRefresh)
                    .toggleStyle(SwitchToggleStyle())
            }
            
            // Bottom row: Search and actions
            HStack {
                SearchField(text: $searchText, placeholder: "搜索日志...")
                
                Button("刷新") {
                    Task { await onRefresh() }
                }
                .buttonStyle(BorderedButtonStyle())
                
                Menu {
                    Button("导出日志") {
                        onExport()
                    }
                    
                    Button("清空日志", role: .destructive) {
                        onClear()
                    }
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
                .buttonStyle(BorderedButtonStyle())
            }
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
    }
}

struct SearchField: View {
    @Binding var text: String
    let placeholder: String
    
    var body: some View {
        HStack {
            Image(systemName: "magnifyingglass")
                .foregroundColor(.secondary)
            
            TextField(placeholder, text: $text)
                .textFieldStyle(PlainTextFieldStyle())
            
            if !text.isEmpty {
                Button("清除") {
                    text = ""
                }
                .foregroundColor(.secondary)
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(Color.secondary.opacity(0.1))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

struct LogListView: View {
    let logs: [LogEntry]
    
    var body: some View {
        List(logs, id: \.id) { log in
            LogEntryRow(entry: log)
                .listRowSeparator(.hidden)
                .listRowInsets(EdgeInsets(top: 4, leading: 16, bottom: 4, trailing: 16))
        }
        .listStyle(PlainListStyle())
    }
}

struct LogEntryRow: View {
    let entry: LogEntry
    @State private var isExpanded = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                // Level indicator
                Circle()
                    .fill(entry.level.color)
                    .frame(width: 8, height: 8)
                
                // Timestamp
                Text(entry.timestamp.formatted(.dateTime.hour().minute().second()))
                    .font(.caption)
                    .fontDesign(.monospaced)
                    .foregroundColor(.secondary)
                
                // Module
                Text(entry.module.uppercased())
                    .font(.caption2)
                    .fontWeight(.medium)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Color.secondary.opacity(0.2))
                    .clipShape(Capsule())
                
                Spacer()
                
                // Level
                Text(entry.level.displayName)
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(entry.level.color)
            }
            
            // Message
            Text(entry.message)
                .font(.caption)
                .fontDesign(.monospaced)
                .lineLimit(isExpanded ? nil : 3)
                .animation(.easeInOut(duration: 0.2), value: isExpanded)
            
            if entry.message.count > 100 {
                Button(isExpanded ? "收起" : "展开") {
                    isExpanded.toggle()
                }
                .font(.caption2)
                .foregroundColor(.blue)
            }
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 12)
        .background(Color.secondary.opacity(0.05))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .onTapGesture {
            if entry.message.count > 100 {
                isExpanded.toggle()
            }
        }
    }
}

struct LogEmptyStateView: View {
    let hasLogs: Bool
    
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: hasLogs ? "line.horizontal.3.decrease.circle" : "doc.text")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            
            Text(hasLogs ? "没有符合条件的日志" : "暂无日志")
                .font(.title2)
                .fontWeight(.medium)
            
            Text(hasLogs ? "尝试调整筛选条件或搜索关键词" : "后端服务启动后将显示日志信息")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct LogExportSheet: View {
    let logs: [LogEntry]
    @Environment(\.dismiss) private var dismiss
    @State private var selectedFormat: ExportFormat = .text
    @State private var isExporting = false
    
    var body: some View {
        NavigationView {
            VStack(spacing: 24) {
                VStack(alignment: .leading, spacing: 16) {
                    Text("导出格式")
                        .font(.headline)
                    
                    Picker("格式", selection: $selectedFormat) {
                        ForEach(ExportFormat.allCases, id: \.self) { format in
                            Text(format.displayName).tag(format)
                        }
                    }
                    .pickerStyle(SegmentedPickerStyle())
                }
                
                VStack(alignment: .leading, spacing: 8) {
                    Text("导出信息")
                        .font(.headline)
                    
                    Text("将导出 \(logs.count) 条日志记录")
                        .foregroundColor(.secondary)
                    
                    Text("格式: \(selectedFormat.displayName)")
                        .foregroundColor(.secondary)
                }
                
                Button(action: {
                    Task { await exportLogs() }
                }) {
                    HStack {
                        if isExporting {
                            ProgressView()
                                .scaleEffect(0.8)
                        }
                        
                        Text(isExporting ? "导出中..." : "导出日志")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 8))
                }
                .disabled(isExporting)
                
                Spacer()
            }
            .padding()
            .navigationTitle("导出日志")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") {
                        dismiss()
                    }
                }
            }
        }
    }
    
    private func exportLogs() async {
        isExporting = true
        defer { isExporting = false }
        
        // TODO: Implement actual log export
        try? await Task.sleep(nanoseconds: 1_000_000_000)
        
        dismiss()
    }
}

// MARK: - Data Models

struct LogEntry: Identifiable {
    let id = UUID()
    let timestamp: Date
    let level: LogLevel
    let module: String
    let message: String
}

enum LogLevel: String, CaseIterable {
    case all = "all"
    case trace = "trace"
    case debug = "debug"
    case info = "info"
    case warn = "warn"
    case error = "error"
    
    var displayName: String {
        switch self {
        case .all: return "全部"
        case .trace: return "TRACE"
        case .debug: return "DEBUG"
        case .info: return "INFO"
        case .warn: return "WARN"
        case .error: return "ERROR"
        }
    }
    
    var color: Color {
        switch self {
        case .all: return .primary
        case .trace: return .gray
        case .debug: return .blue
        case .info: return .green
        case .warn: return .orange
        case .error: return .red
        }
    }
}

enum ExportFormat: String, CaseIterable {
    case text = "text"
    case json = "json"
    case csv = "csv"
    
    var displayName: String {
        switch self {
        case .text: return "文本文件"
        case .json: return "JSON"
        case .csv: return "CSV"
        }
    }
}

#Preview {
    LogsView(coreManager: CoreManager())
        .modelContainer(for: [NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self], inMemory: true)
}
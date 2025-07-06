//
//  LogsView.swift
//  librorum
//
//  Log viewing and monitoring
//

import SwiftUI
import SwiftData
#if os(macOS)
import AppKit
import UniformTypeIdentifiers
#elseif os(iOS)
import UIKit
#endif

struct LogsView: View {
    let coreManager: CoreManager
    @State private var logEntries: [LogEntryData] = []
    @State private var isLoading = false
    @State private var selectedLogLevel: LogLevel = .all
    @State private var searchText = ""
    @State private var autoRefresh = true
    @State private var refreshTimer: Timer?
    @State private var showingExportSheet = false
    @State private var isStreaming = false
    @State private var streamTask: Task<Void, Never>?
    
    var filteredLogs: [LogEntryData] {
        logEntries
            .filter { entry in
                if selectedLogLevel != .all && entry.level.rawValue != selectedLogLevel.rawValue {
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
                isStreaming: $isStreaming,
                onRefresh: {
                    await refreshLogs()
                },
                onExport: {
                    showingExportSheet = true
                },
                onClear: {
                    clearLogs()
                },
                onStartStreaming: {
                    startLogStreaming()
                },
                onStopStreaming: {
                    stopLogStreaming()
                },
                onCopyLogs: {
                    copyLogsToClipboard()
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
            LogExportSheet(logs: filteredLogs, coreManager: coreManager)
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
    
    private func loadLogEntries() async throws -> [LogEntryData] {
        guard let communicator = coreManager.grpcCommunicator else {
            throw LogError.notConnected
        }
        
        do {
            let result = try await communicator.getLogs(
                limit: 100,
                levelFilter: selectedLogLevel == .all ? "" : selectedLogLevel.rawValue,
                moduleFilter: "",
                searchText: searchText,
                reverse: true
            )
            return result.logs
        } catch {
            print("Failed to load logs from gRPC: \(error)")
            return generateMockLogs()
        }
    }
    
    private func generateMockLogs() -> [LogEntryData] {
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
        
        return (0..<20).map { index in
            LogEntryData(
                timestamp: Date().addingTimeInterval(-Double(index * 30)),
                level: CommunicatorLogLevel.allCases.randomElement() ?? .info,
                module: ["core", "network", "storage", "grpc"].randomElement() ?? "core",
                message: messages.randomElement() ?? "Log message \(index)",
                threadId: "ThreadId(1)",
                file: "main.rs",
                line: 42,
                fields: [:]
            )
        }
    }
    
    private func clearLogs() {
        Task {
            do {
                guard let communicator = coreManager.grpcCommunicator else { return }
                let _ = try await communicator.clearLogs(clearAll: true, beforeTimestamp: 0)
                await refreshLogs()
            } catch {
                print("Failed to clear logs: \(error)")
                // Fallback to local clear
                logEntries.removeAll()
            }
        }
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
    
    private func startLogStreaming() {
        guard let communicator = coreManager.grpcCommunicator else { return }
        
        // Stop existing stream
        stopLogStreaming()
        
        streamTask = Task {
            do {
                let logStream = try await communicator.streamLogs(
                    levelFilter: selectedLogLevel == .all ? "" : selectedLogLevel.rawValue,
                    moduleFilter: "",
                    follow: true,
                    tail: 10
                )
                
                for try await logEntry in logStream {
                    await MainActor.run {
                        logEntries.insert(logEntry, at: 0)
                        
                        // Keep only the most recent 500 entries to prevent memory issues
                        if logEntries.count > 500 {
                            logEntries = Array(logEntries.prefix(500))
                        }
                    }
                }
            } catch {
                await MainActor.run {
                    isStreaming = false
                    print("日志流失败: \(error)")
                }
            }
        }
    }
    
    private func stopLogStreaming() {
        streamTask?.cancel()
        streamTask = nil
    }
    
    private func copyLogsToClipboard() {
        let logText = formatLogsForClipboard(filteredLogs)
        
        #if os(macOS)
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(logText, forType: .string)
        #elseif os(iOS)
        UIPasteboard.general.string = logText
        #endif
        
        print("✅ LogsView: Copied \(filteredLogs.count) log entries to clipboard")
    }
    
    private func formatLogsForClipboard(_ logs: [LogEntryData]) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd HH:mm:ss.SSS"
        
        return logs.map { log in
            let timestamp = formatter.string(from: log.timestamp)
            let level = log.level.displayName.uppercased()
            let module = log.module.uppercased()
            return "[\(timestamp)] [\(level)] [\(module)] \(log.message)"
        }.joined(separator: "\n")
    }
}

struct LogControlsView: View {
    @Binding var selectedLogLevel: LogLevel
    @Binding var searchText: String
    @Binding var autoRefresh: Bool
    @Binding var isStreaming: Bool
    let onRefresh: () async -> Void
    let onExport: () -> Void
    let onClear: () -> Void
    let onStartStreaming: () -> Void
    let onStopStreaming: () -> Void
    let onCopyLogs: () -> Void
    
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
                
                Toggle("实时流", isOn: $isStreaming)
                    .toggleStyle(SwitchToggleStyle())
                    .onChange(of: isStreaming) { oldValue, newValue in
                        if newValue {
                            onStartStreaming()
                        } else {
                            onStopStreaming()
                        }
                    }
            }
            
            // Bottom row: Search and actions
            HStack {
                SearchField(text: $searchText, placeholder: "搜索日志...")
                
                Button("刷新") {
                    Task { await onRefresh() }
                }
                .buttonStyle(BorderedButtonStyle())
                
                Button("复制日志") {
                    onCopyLogs()
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
    let logs: [LogEntryData]
    
    var body: some View {
        List(logs.indices, id: \.self) { index in
            let log = logs[index]
            LogEntryRow(entry: log)
                .listRowSeparator(.hidden)
                .listRowInsets(EdgeInsets(top: 4, leading: 16, bottom: 4, trailing: 16))
        }
        .listStyle(PlainListStyle())
    }
}

struct LogEntryRow: View {
    let entry: LogEntryData
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
        .contextMenu {
            Button("复制此条日志") {
                copyLogEntryToClipboard(entry)
            }
            
            Button("复制消息内容") {
                copyMessageToClipboard(entry.message)
            }
        }
    }
    
    private func copyLogEntryToClipboard(_ entry: LogEntryData) {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd HH:mm:ss.SSS"
        
        let timestamp = formatter.string(from: entry.timestamp)
        let level = entry.level.displayName.uppercased()
        let module = entry.module.uppercased()
        let logText = "[\(timestamp)] [\(level)] [\(module)] \(entry.message)"
        
        #if os(macOS)
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(logText, forType: .string)
        #elseif os(iOS)
        UIPasteboard.general.string = logText
        #endif
        
        print("✅ LogEntryRow: Copied log entry to clipboard")
    }
    
    private func copyMessageToClipboard(_ message: String) {
        #if os(macOS)
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(message, forType: .string)
        #elseif os(iOS)
        UIPasteboard.general.string = message
        #endif
        
        print("✅ LogEntryRow: Copied message to clipboard")
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
    let logs: [LogEntryData]
    let coreManager: CoreManager
    @Environment(\.dismiss) private var dismiss
    @State private var selectedFormat: ExportFormat = .text
    @State private var isExporting = false
    @State private var exportError: String?
    
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
                
                VStack {
                    if let exportError = exportError {
                        Text("导出失败: \(exportError)")
                            .foregroundColor(.red)
                            .font(.caption)
                            .padding(.bottom, 8)
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
                }
                
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
        exportError = nil
        defer { isExporting = false }
        
        do {
            guard let communicator = coreManager.grpcCommunicator else {
                exportError = "未连接到后端服务"
                return
            }
            
            let grpcFormat: LogExportFormat
            switch selectedFormat {
            case .text:
                grpcFormat = .plain
            case .json:
                grpcFormat = .json
            case .csv:
                grpcFormat = .csv
            }
            
            let result = try await communicator.exportLogs(
                format: grpcFormat,
                levelFilter: "",
                moduleFilter: ""
            )
            
            if result.success {
                // Save the exported data
                await saveExportedData(result.data, filename: result.filename, mimeType: result.mimeType)
                dismiss()
            } else {
                exportError = "导出失败"
            }
        } catch {
            exportError = "导出失败: \(error.localizedDescription)"
        }
    }
    
    @MainActor
    private func saveExportedData(_ data: Data, filename: String, mimeType: String) async {
        #if os(macOS)
        let savePanel = NSSavePanel()
        savePanel.nameFieldStringValue = filename
        savePanel.allowedContentTypes = [.plainText, .json, .commaSeparatedText]
        
        if savePanel.runModal() == .OK, let url = savePanel.url {
            do {
                try data.write(to: url)
                print("✅ Log export saved to: \(url.path)")
            } catch {
                exportError = "保存文件失败: \(error.localizedDescription)"
            }
        }
        #else
        // iOS: Use share sheet instead of save panel
        let activityController = UIActivityViewController(activityItems: [data], applicationActivities: nil)
        
        if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
           let window = windowScene.windows.first,
           let rootViewController = window.rootViewController {
            rootViewController.present(activityController, animated: true)
        }
        #endif
    }
}

// MARK: - Data Models

enum LogError: Error {
    case notConnected
    case loadFailed(String)
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

// Extension to convert CommunicatorLogLevel to LogLevel for UI
extension CommunicatorLogLevel {
    var color: Color {
        switch self {
        case .unknown: return .gray
        case .trace: return .gray
        case .debug: return .blue
        case .info: return .green
        case .warn: return .orange
        case .error: return .red
        }
    }
    
    var displayName: String {
        switch self {
        case .unknown: return "UNKNOWN"
        case .trace: return "TRACE"
        case .debug: return "DEBUG"
        case .info: return "INFO"
        case .warn: return "WARN"
        case .error: return "ERROR"
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
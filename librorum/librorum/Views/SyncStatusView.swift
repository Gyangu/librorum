//
//  SyncStatusView.swift
//  librorum
//
//  Sync status and conflict resolution interface
//

import SwiftUI
import SwiftData

struct SyncStatusView: View {
    @Environment(\.modelContext) private var modelContext
    let coreManager: CoreManager
    @StateObject private var syncManager: SyncManager
    @State private var showingConflictResolver = false
    @State private var selectedConflictFile: FileItem?
    
    init(coreManager: CoreManager, modelContext: ModelContext) {
        self.coreManager = coreManager
        self._syncManager = StateObject(wrappedValue: SyncManager(
            modelContext: modelContext,
            grpcCommunicator: coreManager.grpcCommunicator ?? MockGRPCCommunicator()
        ))
    }
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                // Sync Status Header
                syncStatusHeader
                
                // Sync Progress
                if syncManager.syncStatus == .syncing {
                    syncProgressSection
                }
                
                // Conflict Files
                if !syncManager.conflictingFiles.isEmpty {
                    conflictFilesSection
                }
                
                // Sync Errors
                if !syncManager.syncErrors.isEmpty {
                    syncErrorsSection
                }
                
                Spacer()
                
                // Sync Controls
                syncControlsSection
            }
            .padding()
            .navigationTitle("同步状态")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.large)
            #endif
            .refreshable {
                await syncManager.detectAndResolveConflicts()
            }
        }
        .sheet(isPresented: $showingConflictResolver) {
            if let file = selectedConflictFile {
                ConflictResolverView(file: file, syncManager: syncManager)
            }
        }
        .onAppear {
            syncManager.startAutoSync()
        }
        .onDisappear {
            syncManager.stopAutoSync()
        }
    }
    
    // MARK: - View Components
    
    private var syncStatusHeader: some View {
        GroupBox {
            HStack {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("同步状态")
                            .font(.headline)
                        
                        Spacer()
                        
                        Text(syncManager.syncStatus.displayName)
                            .font(.subheadline)
                            .foregroundColor(statusColor)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 4)
                            .background(statusColor.opacity(0.1))
                            .cornerRadius(8)
                    }
                    
                    if let lastSync = syncManager.lastSyncDate {
                        Text("上次同步: \(lastSync, formatter: dateFormatter)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                
                Spacer()
                
                syncStatusIcon
            }
        }
    }
    
    private var syncProgressSection: some View {
        GroupBox("同步进度") {
            VStack(spacing: 12) {
                ProgressView(value: syncManager.syncProgress)
                    .progressViewStyle(LinearProgressViewStyle())
                
                Text("\(Int(syncManager.syncProgress * 100))% 完成")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding()
        }
    }
    
    private var conflictFilesSection: some View {
        GroupBox("冲突文件") {
            LazyVStack(spacing: 8) {
                ForEach(syncManager.conflictingFiles, id: \.path) { file in
                    ConflictFileRow(file: file) {
                        selectedConflictFile = file
                        showingConflictResolver = true
                    }
                }
            }
            .padding()
        }
    }
    
    private var syncErrorsSection: some View {
        GroupBox("同步错误") {
            LazyVStack(spacing: 8) {
                ForEach(syncManager.syncErrors) { error in
                    HStack {
                        Image(systemName: "exclamationmark.triangle")
                            .foregroundColor(.red)
                        
                        VStack(alignment: .leading) {
                            Text(error.message)
                                .font(.caption)
                                .foregroundColor(.primary)
                            
                            Text(error.timestamp, formatter: timeFormatter)
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                        
                        Spacer()
                    }
                    .padding(.vertical, 4)
                    .background(Color.red.opacity(0.05))
                    .cornerRadius(6)
                }
            }
            .padding()
        }
    }
    
    private var syncControlsSection: some View {
        VStack(spacing: 12) {
            HStack(spacing: 16) {
                Button("立即同步") {
                    Task {
                        await syncManager.performFullSync()
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(syncManager.syncStatus == .syncing)
                
                Button("检测冲突") {
                    Task {
                        await syncManager.detectAndResolveConflicts()
                    }
                }
                .buttonStyle(.bordered)
            }
            
            Toggle("自动同步", isOn: .constant(true))
                .toggleStyle(SwitchToggleStyle())
        }
    }
    
    private var statusColor: Color {
        switch syncManager.syncStatus {
        case .idle: return .blue
        case .syncing: return .orange
        case .completed: return .green
        case .error: return .red
        }
    }
    
    private var syncStatusIcon: some View {
        Group {
            switch syncManager.syncStatus {
            case .idle:
                Image(systemName: "checkmark.circle")
                    .foregroundColor(.blue)
            case .syncing:
                ProgressView()
                    .scaleEffect(0.8)
            case .completed:
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.green)
            case .error:
                Image(systemName: "exclamationmark.circle.fill")
                    .foregroundColor(.red)
            }
        }
        .font(.title2)
    }
    
    private var dateFormatter: DateFormatter {
        let formatter = DateFormatter()
        formatter.dateStyle = .short
        formatter.timeStyle = .short
        return formatter
    }
    
    private var timeFormatter: DateFormatter {
        let formatter = DateFormatter()
        formatter.timeStyle = .short
        return formatter
    }
}

// MARK: - Conflict File Row

struct ConflictFileRow: View {
    let file: FileItem
    let onTap: () -> Void
    
    var body: some View {
        Button(action: onTap) {
            HStack {
                Image(systemName: file.isDirectory ? "folder.fill" : "doc.fill")
                    .foregroundColor(.blue)
                
                VStack(alignment: .leading) {
                    Text(file.name)
                        .font(.body)
                        .foregroundColor(.primary)
                    
                    Text("版本冲突 - 需要解决")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                
                Spacer()
                
                Image(systemName: "chevron.right")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

// MARK: - Conflict Resolver View

struct ConflictResolverView: View {
    let file: FileItem
    let syncManager: SyncManager
    @Environment(\.dismiss) private var dismiss
    @State private var selectedResolution: ConflictResolution = .askUser
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                // File Info
                fileInfoSection
                
                // Resolution Options
                resolutionOptionsSection
                
                Spacer()
                
                // Action Buttons
                actionButtonsSection
            }
            .padding()
            .navigationTitle("解决冲突")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            .navigationBarItems(
                leading: Button("取消") { dismiss() }
            )
            #else
            .toolbar {
                ToolbarItem(placement: .navigation) {
                    Button("取消") { dismiss() }
                }
            }
            #endif
        }
    }
    
    private var fileInfoSection: some View {
        GroupBox("文件信息") {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Image(systemName: file.isDirectory ? "folder.fill" : "doc.fill")
                        .foregroundColor(.blue)
                    
                    Text(file.name)
                        .font(.headline)
                    
                    Spacer()
                }
                
                Text("路径: \(file.path)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Text("版本: \(file.version)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Text("修改时间: \(file.modificationDate, formatter: dateFormatter)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding()
        }
    }
    
    private var resolutionOptionsSection: some View {
        GroupBox("解决方案") {
            VStack(spacing: 12) {
                ForEach(ConflictResolution.allCases, id: \.self) { resolution in
                    HStack {
                        Button(action: {
                            selectedResolution = resolution
                        }) {
                            HStack {
                                Image(systemName: selectedResolution == resolution ? 
                                      "largecircle.fill.circle" : "circle")
                                
                                VStack(alignment: .leading) {
                                    Text(resolution.displayName)
                                        .font(.body)
                                    
                                    Text(resolutionDescription(for: resolution))
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                                
                                Spacer()
                            }
                        }
                        .buttonStyle(PlainButtonStyle())
                    }
                    .padding(.vertical, 4)
                }
            }
            .padding()
        }
    }
    
    private var actionButtonsSection: some View {
        HStack(spacing: 16) {
            Button("应用解决方案") {
                applyResolution()
            }
            .buttonStyle(.borderedProminent)
            .disabled(selectedResolution == .askUser)
            
            Button("稍后处理") {
                dismiss()
            }
            .buttonStyle(.bordered)
        }
    }
    
    private func resolutionDescription(for resolution: ConflictResolution) -> String {
        switch resolution {
        case .useLocal:
            return "使用本地文件版本，覆盖远程版本"
        case .useRemote:
            return "下载远程版本，覆盖本地文件"
        case .merge:
            return "尝试自动合并两个版本（仅限文本文件）"
        case .createBoth:
            return "保留两个版本，创建冲突副本"
        case .askUser:
            return "稍后手动处理冲突"
        }
    }
    
    private func applyResolution() {
        file.conflictResolution = selectedResolution
        
        Task {
            await syncManager.resolveConflict(for: file)
            await MainActor.run {
                dismiss()
            }
        }
    }
    
    private var dateFormatter: DateFormatter {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }
}

#Preview {
    SyncStatusView(coreManager: CoreManager(), modelContext: ModelContext(try! ModelContainer(for: FileItem.self)))
}
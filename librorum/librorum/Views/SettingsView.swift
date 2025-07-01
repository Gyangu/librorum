//
//  SettingsView.swift
//  librorum
//
//  Application settings and preferences
//

import SwiftUI
import SwiftData

struct SettingsView: View {
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss
    
    let coreManager: CoreManager
    @Query private var preferences: [UserPreferences]
    @State private var currentPreferences: UserPreferences?
    
    var body: some View {
        NavigationView {
            Form {
                // Backend Settings
                BackendSettingsSection(
                    preferences: currentPreferences,
                    coreManager: coreManager,
                    onUpdate: savePreferences
                )
                
                // Network Settings
                NetworkSettingsSection(
                    preferences: currentPreferences,
                    onUpdate: savePreferences
                )
                
                // Storage Settings
                StorageSettingsSection(
                    preferences: currentPreferences,
                    onUpdate: savePreferences
                )
                
                // Logging Settings
                LoggingSettingsSection(
                    preferences: currentPreferences,
                    onUpdate: savePreferences
                )
                
                // App Settings
                AppSettingsSection(
                    preferences: currentPreferences,
                    onUpdate: savePreferences
                )
                
                // Actions
                SettingsActionsSection(
                    preferences: currentPreferences,
                    coreManager: coreManager,
                    onUpdate: savePreferences
                )
            }
            .navigationTitle("设置")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("完成") {
                        dismiss()
                    }
                }
            }
        }
        .onAppear {
            loadPreferences()
        }
    }
    
    private func loadPreferences() {
        if let existing = preferences.first {
            currentPreferences = existing
        } else {
            let newPreferences = UserPreferences()
            modelContext.insert(newPreferences)
            try? modelContext.save()
            currentPreferences = newPreferences
        }
    }
    
    private func savePreferences() {
        try? modelContext.save()
    }
}

struct BackendSettingsSection: View {
    let preferences: UserPreferences?
    let coreManager: CoreManager
    let onUpdate: () -> Void
    
    var body: some View {
        Section(header: Text("后端设置")) {
            if let prefs = preferences {
                Toggle("自动启动后端", isOn: Binding(
                    get: { prefs.autoStartBackend },
                    set: { prefs.autoStartBackend = $0; onUpdate() }
                ))
                
                HStack {
                    Text("数据目录")
                    Spacer()
                    Text(prefs.dataDirectory)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                
                HStack {
                    Text("后端状态")
                    Spacer()
                    HStack(spacing: 4) {
                        Circle()
                            .fill(Color(coreManager.backendStatus.color))
                            .frame(width: 8, height: 8)
                        Text(coreManager.backendStatus.displayName)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
    }
}

struct NetworkSettingsSection: View {
    let preferences: UserPreferences?
    let onUpdate: () -> Void
    
    var body: some View {
        Section(header: Text("网络设置")) {
            if let prefs = preferences {
                HStack {
                    Text("绑定地址")
                    Spacer()
                    TextField("IP地址", text: Binding(
                        get: { prefs.bindHost },
                        set: { prefs.bindHost = $0; onUpdate() }
                    ))
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .frame(width: 120)
                }
                
                HStack {
                    Text("端口")
                    Spacer()
                    TextField("端口", value: Binding(
                        get: { prefs.bindPort },
                        set: { prefs.bindPort = $0; onUpdate() }
                    ), format: .number)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .frame(width: 80)
                }
                
                HStack {
                    Text("心跳间隔")
                    Spacer()
                    TextField("秒", value: Binding(
                        get: { prefs.heartbeatInterval },
                        set: { prefs.heartbeatInterval = $0; onUpdate() }
                    ), format: .number)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .frame(width: 60)
                    Text("秒")
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Text("发现间隔")
                    Spacer()
                    TextField("秒", value: Binding(
                        get: { prefs.discoveryInterval },
                        set: { prefs.discoveryInterval = $0; onUpdate() }
                    ), format: .number)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .frame(width: 60)
                    Text("秒")
                        .foregroundColor(.secondary)
                }
            }
        }
    }
}

struct StorageSettingsSection: View {
    let preferences: UserPreferences?
    let onUpdate: () -> Void
    
    var body: some View {
        Section(header: Text("存储设置")) {
            if let prefs = preferences {
                Toggle("启用压缩", isOn: Binding(
                    get: { prefs.enableCompression },
                    set: { prefs.enableCompression = $0; onUpdate() }
                ))
                
                HStack {
                    Text("默认副本因子")
                    Spacer()
                    Stepper(value: Binding(
                        get: { prefs.defaultReplicationFactor },
                        set: { prefs.defaultReplicationFactor = $0; onUpdate() }
                    ), in: 1...5) {
                        Text("\(prefs.defaultReplicationFactor)")
                    }
                }
                
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("数据块大小")
                        Spacer()
                        Text(prefs.formattedChunkSize)
                            .foregroundColor(.secondary)
                    }
                    
                    Slider(value: Binding(
                        get: { log2(Double(prefs.chunkSize)) - 10 }, // 2^10 = 1KB baseline
                        set: { prefs.chunkSize = Int(pow(2, $0 + 10)); onUpdate() }
                    ), in: 0...14, step: 1) // 1KB to 16MB
                }
            }
        }
    }
}

struct LoggingSettingsSection: View {
    let preferences: UserPreferences?
    let onUpdate: () -> Void
    
    var body: some View {
        Section(header: Text("日志设置")) {
            if let prefs = preferences {
                Picker("日志级别", selection: Binding(
                    get: { prefs.logLevel },
                    set: { prefs.logLevel = $0; onUpdate() }
                )) {
                    ForEach(prefs.logLevelOptions, id: \.self) { level in
                        Text(level.uppercased()).tag(level)
                    }
                }
                
                HStack {
                    Text("最大日志文件数")
                    Spacer()
                    Stepper(value: Binding(
                        get: { prefs.maxLogFiles },
                        set: { prefs.maxLogFiles = $0; onUpdate() }
                    ), in: 1...50) {
                        Text("\(prefs.maxLogFiles)")
                    }
                }
                
                HStack {
                    Text("日志保留天数")
                    Spacer()
                    Stepper(value: Binding(
                        get: { prefs.logRotationDays },
                        set: { prefs.logRotationDays = $0; onUpdate() }
                    ), in: 1...365) {
                        Text("\(prefs.logRotationDays)")
                    }
                }
            }
        }
    }
}

struct AppSettingsSection: View {
    let preferences: UserPreferences?
    let onUpdate: () -> Void
    
    var body: some View {
        Section(header: Text("应用设置")) {
            if let prefs = preferences {
                Toggle("启用通知", isOn: Binding(
                    get: { prefs.enableNotifications },
                    set: { prefs.enableNotifications = $0; onUpdate() }
                ))
                
                Picker("主题", selection: Binding(
                    get: { prefs.theme },
                    set: { prefs.theme = $0; onUpdate() }
                )) {
                    Text("自动").tag("auto")
                    Text("浅色").tag("light")
                    Text("深色").tag("dark")
                }
                
                Picker("语言", selection: Binding(
                    get: { prefs.language },
                    set: { prefs.language = $0; onUpdate() }
                )) {
                    Text("中文").tag("zh")
                    Text("English").tag("en")
                }
            }
        }
    }
}

struct SettingsActionsSection: View {
    let preferences: UserPreferences?
    let coreManager: CoreManager
    let onUpdate: () -> Void
    @State private var showingResetAlert = false
    @State private var showingRestartAlert = false
    
    var body: some View {
        Section(header: Text("操作")) {
            Button("重启后端服务") {
                showingRestartAlert = true
            }
            .foregroundColor(.blue)
            .alert("重启后端服务", isPresented: $showingRestartAlert) {
                Button("取消", role: .cancel) { }
                Button("重启", role: .destructive) {
                    Task {
                        try? await coreManager.restartBackend()
                    }
                }
            } message: {
                Text("这将重启后端服务，可能会中断正在进行的操作。")
            }
            
            Button("清理日志文件") {
                Task {
                    await cleanLogFiles()
                }
            }
            .foregroundColor(.orange)
            
            Button("重置所有设置") {
                showingResetAlert = true
            }
            .foregroundColor(.red)
            .alert("重置设置", isPresented: $showingResetAlert) {
                Button("取消", role: .cancel) { }
                Button("重置", role: .destructive) {
                    resetToDefaults()
                }
            } message: {
                Text("这将重置所有设置到默认值，此操作不可撤销。")
            }
        }
    }
    
    private func resetToDefaults() {
        guard let currentPreferences = preferences else { return }
        
        // Reset all properties to their default values
        currentPreferences.autoStartBackend = true
        currentPreferences.startupStrategy = "automatic"
        currentPreferences.logLevel = "info"
        currentPreferences.bindPort = 50051
        currentPreferences.bindHost = "0.0.0.0"
        currentPreferences.heartbeatInterval = 30
        currentPreferences.discoveryInterval = 60
        currentPreferences.enableCompression = true
        currentPreferences.defaultReplicationFactor = 3
        currentPreferences.chunkSize = 1048576 // 1MB
        currentPreferences.maxLogFiles = 10
        currentPreferences.logRotationDays = 7
        currentPreferences.enableNotifications = true
        currentPreferences.theme = "auto"
        currentPreferences.language = "zh"
        
        // Reset data directory to default
        #if os(macOS)
        currentPreferences.dataDirectory = NSHomeDirectory() + "/Library/Application Support/librorum"
        #else
        currentPreferences.dataDirectory = NSHomeDirectory() + "/Documents/librorum"
        #endif
        
        // Save changes
        onUpdate()
        
        print("✅ Settings reset to defaults")
    }
    
    private func cleanLogFiles() async {
        do {
            guard let communicator = coreManager.grpcCommunicator else {
                print("未连接到后端服务")
                return
            }
            
            // Clear all logs via gRPC
            let result = try await communicator.clearLogs(clearAll: true, beforeTimestamp: 0)
            
            if result.success {
                print("✅ Logs cleared successfully: \(result.clearedCount) entries removed")
                
                // Also clear any local log files if they exist
                let logDirectory = preferences?.dataDirectory.appending("/logs") ?? ""
                if !logDirectory.isEmpty {
                    await cleanLocalLogFiles(directory: logDirectory)
                }
            } else {
                print("❌ Failed to clear logs: \(result.message)")
            }
        } catch {
            print("❌ Error clearing logs: \(error)")
            // Fallback to local cleanup only
            let logDirectory = preferences?.dataDirectory.appending("/logs") ?? ""
            if !logDirectory.isEmpty {
                await cleanLocalLogFiles(directory: logDirectory)
            }
        }
    }
    
    private func cleanLocalLogFiles(directory: String) async {
        let fileManager = FileManager.default
        
        do {
            guard fileManager.fileExists(atPath: directory) else {
                print("📁 Log directory doesn't exist: \(directory)")
                return
            }
            
            let contents = try fileManager.contentsOfDirectory(atPath: directory)
            let logFiles = contents.filter { $0.hasSuffix(".log") || $0.hasSuffix(".txt") }
            
            var removedCount = 0
            for logFile in logFiles {
                let fullPath = directory + "/" + logFile
                try fileManager.removeItem(atPath: fullPath)
                removedCount += 1
            }
            
            print("✅ Cleaned \(removedCount) local log files from \(directory)")
        } catch {
            print("❌ Error cleaning local log files: \(error)")
        }
    }
}

#Preview {
    SettingsView(coreManager: CoreManager())
        .modelContainer(for: [NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self], inMemory: true)
}
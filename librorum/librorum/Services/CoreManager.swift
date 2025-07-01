//
//  CoreManager.swift
//  librorum
//
//  Core backend lifecycle management service
//

import Foundation
import SwiftUI
import SwiftData

@MainActor
@Observable
class CoreManager {
    
    // MARK: - Published Properties
    var backendStatus: BackendStatus = .stopped
    var connectedNodes: [NodeInfo] = []
    var systemHealth: SystemHealth?
    var lastError: String?
    var isInitialized: Bool = false
    
    // MARK: - Private Properties
    private var backendProcess: Process?
    private var healthTimer: Timer?
    private var nodeDiscoveryTimer: Timer?
    private var grpcClient: LibrorumClient?
    private let configFileName = "librorum.toml"
    
    // MARK: - Initialization
    init() {
        print("🎯 CoreManager: Initializing CoreManager...")
        setupDefaultConfiguration()
        print("✅ CoreManager: CoreManager initialized")
    }
    
    // MARK: - Backend Lifecycle Management
    
    func initializeBackend() async throws {
        print("🔧 CoreManager: initializeBackend called, isInitialized: \(isInitialized)")
        guard !isInitialized else { 
            print("🔧 CoreManager: Already initialized, skipping")
            return 
        }
        
        print("🔧 CoreManager: Setting up backend binary...")
        try await setupBackendBinary()
        
        print("🔧 CoreManager: Creating default configuration...")
        try await createDefaultConfiguration()
        
        isInitialized = true
        print("✅ CoreManager: Initialization completed")
    }
    
    func startBackend() async throws {
        print("🚀 CoreManager: startBackend called, current status: \(backendStatus)")
        
        if !isInitialized {
            print("🚀 CoreManager: Not initialized, initializing first...")
            try await initializeBackend()
        }
        
        guard backendStatus != .running else { 
            print("🚀 CoreManager: Backend already running, skipping")
            return 
        }
        
        print("🚀 CoreManager: Setting status to starting...")
        backendStatus = .starting
        lastError = nil
        
        do {
            print("🚀 CoreManager: Launching backend process...")
            try await launchBackendProcess()
            
            print("🚀 CoreManager: Waiting for backend ready...")
            try await waitForBackendReady()
            
            print("🚀 CoreManager: Establishing gRPC connection...")
            try await establishGRPCConnection()
            
            print("🚀 CoreManager: Setting status to running...")
            backendStatus = .running
            
            print("🚀 CoreManager: Starting monitoring...")
            startMonitoring()
            
            print("✅ CoreManager: Backend started successfully!")
            
        } catch {
            print("❌ CoreManager: Backend start failed - \(error)")
            backendStatus = .error
            lastError = error.localizedDescription
            throw error
        }
    }
    
    func stopBackend() async throws {
        guard backendStatus == .running else { return }
        
        backendStatus = .stopping
        stopMonitoring()
        
        do {
            try await sendStopCommand()
            await terminateBackendProcess()
            backendStatus = .stopped
            
        } catch {
            backendStatus = .error
            lastError = error.localizedDescription
            throw error
        }
    }
    
    func restartBackend() async throws {
        try await stopBackend()
        try await Task.sleep(nanoseconds: 1_000_000_000) // 1 second delay
        try await startBackend()
    }
    
    // MARK: - Health Monitoring
    
    func checkBackendHealth() async -> SystemHealth {
        let health = SystemHealth(
            timestamp: Date(),
            backendStatus: backendStatus,
            totalNodes: connectedNodes.count,
            onlineNodes: connectedNodes.filter { $0.isOnline }.count,
            offlineNodes: connectedNodes.filter { !$0.isOnline }.count
        )
        
        if let grpcClient = grpcClient {
            do {
                // Perform health check via gRPC
                let healthData = try await grpcClient.getSystemHealth()
                health.totalStorage = healthData.totalStorage
                health.usedStorage = healthData.usedStorage
                health.availableStorage = healthData.availableStorage
                health.totalFiles = healthData.totalFiles
                health.totalChunks = healthData.totalChunks
                health.networkLatency = healthData.networkLatency
                health.errorCount = healthData.errorCount
                health.uptime = healthData.uptime
                health.memoryUsage = healthData.memoryUsage
                health.cpuUsage = healthData.cpuUsage
            } catch {
                health.errorCount += 1
                health.lastError = error.localizedDescription
            }
        }
        
        self.systemHealth = health
        return health
    }
    
    // MARK: - Node Management
    
    func refreshNodes() async {
        guard let grpcClient = grpcClient else { return }
        
        do {
            let nodes = try await grpcClient.getConnectedNodes()
            self.connectedNodes = nodes
        } catch {
            lastError = "Failed to refresh nodes: \(error.localizedDescription)"
        }
    }
    
    func addNode(_ address: String) async throws {
        guard let grpcClient = grpcClient else {
            throw CoreManagerError.grpcNotConnected
        }
        
        try await grpcClient.addNode(address: address)
        await refreshNodes()
    }
    
    func removeNode(_ nodeId: String) async throws {
        guard let grpcClient = grpcClient else {
            throw CoreManagerError.grpcNotConnected
        }
        
        try await grpcClient.removeNode(nodeId: nodeId)
        await refreshNodes()
    }
    
    // MARK: - Private Implementation
    
    private func setupBackendBinary() async throws {
        let backendPath = getBackendBinaryPath()
        print("🔧 CoreManager: Backend binary path: \(backendPath)")
        
        guard FileManager.default.fileExists(atPath: backendPath) else {
            print("❌ CoreManager: Backend binary not found at: \(backendPath)")
            throw CoreManagerError.backendBinaryNotFound(backendPath)
        }
        
        print("✅ CoreManager: Backend binary found, setting permissions...")
        // Ensure binary is executable
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: backendPath
        )
        print("✅ CoreManager: Backend binary setup completed")
    }
    
    
    private func createDefaultConfiguration() async throws {
        let configPath = getConfigFilePath()
        
        guard !FileManager.default.fileExists(atPath: configPath) else { return }
        
        // Create default configuration
        let defaultConfig = """
        [node]
        bind_host = "0.0.0.0"
        bind_port = 50051
        node_prefix = "default"
        
        [logging]
        level = "info"
        
        [storage]
        data_dir = "\(getDataDirectory())"
        chunk_size = 1048576
        replication_factor = 3
        
        [network]
        heartbeat_interval = 30
        discovery_interval = 60
        """
        
        try defaultConfig.write(toFile: configPath, atomically: true, encoding: .utf8)
    }
    
    private func launchBackendProcess() async throws {
        let backendPath = getBackendBinaryPath()
        let configPath = getConfigFilePath()
        
        backendProcess = Process()
        print("🔧 CoreManager: Using real backend: \(backendPath)")
        backendProcess?.executableURL = URL(fileURLWithPath: backendPath)
        backendProcess?.arguments = ["start", "--config", configPath]
        
        // Setup logging
        let logPath = getLogFilePath()
        
        // Create log file if it doesn't exist
        if !FileManager.default.fileExists(atPath: logPath) {
            FileManager.default.createFile(atPath: logPath, contents: nil, attributes: nil)
        }
        
        let logURL = URL(fileURLWithPath: logPath)
        backendProcess?.standardOutput = try? FileHandle(forWritingTo: logURL)
        backendProcess?.standardError = try? FileHandle(forWritingTo: logURL)
        
        print("🔧 CoreManager: Starting process...")
        try backendProcess?.run()
        print("✅ CoreManager: Process started successfully")
    }
    
    private func waitForBackendReady() async throws {
        // Wait for backend to be ready (up to 10 seconds)
        print("🔍 CoreManager: Waiting for backend readiness...")
        for attempt in 1...20 {
            print("🔍 CoreManager: Readiness check attempt \(attempt)/20")
            if await isBackendReady() {
                print("✅ CoreManager: Backend is ready!")
                return
            }
            try await Task.sleep(nanoseconds: 500_000_000) // 0.5 seconds
        }
        print("❌ CoreManager: Backend startup timeout after 10 seconds")
        throw CoreManagerError.backendStartupTimeout
    }
    
    private func isBackendReady() async -> Bool {
        // Try to connect to real gRPC service
        do {
            print("🔍 CoreManager: Checking real backend readiness...")
            let client = LibrorumClient()
            try await client.connect(to: "localhost:50051")
            let isHealthy = await client.isHealthy()
            print("🔍 Real backend healthy: \(isHealthy)")
            return isHealthy
        } catch {
            print("🔍 Real backend not ready: \(error)")
            return false
        }
    }
    
    private func establishGRPCConnection() async throws {
        print("🔗 CoreManager: Establishing real gRPC connection...")
        grpcClient = LibrorumClient()
        try await grpcClient?.connect(to: "localhost:50051")
        print("✅ CoreManager: gRPC connection established")
    }
    
    private func sendStopCommand() async throws {
        let backendPath = getBackendBinaryPath()
        let configPath = getConfigFilePath()
        
        let stopProcess = Process()
        stopProcess.executableURL = URL(fileURLWithPath: backendPath)
        stopProcess.arguments = ["stop", "--config", configPath]
        
        try stopProcess.run()
        stopProcess.waitUntilExit()
    }
    
    private func terminateBackendProcess() async {
        backendProcess?.terminate()
        backendProcess?.waitUntilExit()
        backendProcess = nil
        grpcClient = nil
    }
    
    private func startMonitoring() {
        // Health monitoring timer
        healthTimer = Timer.scheduledTimer(withTimeInterval: 30.0, repeats: true) { [weak self] _ in
            Task {
                await self?.checkBackendHealth()
            }
        }
        
        // Node discovery timer
        nodeDiscoveryTimer = Timer.scheduledTimer(withTimeInterval: 60.0, repeats: true) { [weak self] _ in
            Task {
                await self?.refreshNodes()
            }
        }
    }
    
    private func stopMonitoring() {
        healthTimer?.invalidate()
        healthTimer = nil
        
        nodeDiscoveryTimer?.invalidate()
        nodeDiscoveryTimer = nil
    }
    
    private func setupDefaultConfiguration() {
        // Create necessary directories
        let dataDir = getDataDirectory()
        try? FileManager.default.createDirectory(
            atPath: dataDir,
            withIntermediateDirectories: true,
            attributes: nil
        )
        
        let logsDir = getLogsDirectory()
        try? FileManager.default.createDirectory(
            atPath: logsDir,
            withIntermediateDirectories: true,
            attributes: nil
        )
    }
    
    // MARK: - Path Helpers
    
    private func getBackendBinaryPath() -> String {
        // 尝试多个可能的后端二进制路径
        let possiblePaths = [
            // App bundle 中的路径
            Bundle.main.path(forResource: "librorum_backend", ofType: nil),
            Bundle.main.path(forResource: "librorum", ofType: nil),
            // 相对于bundle的路径
            (Bundle.main.resourcePath ?? "") + "/librorum_backend",
            (Bundle.main.resourcePath ?? "") + "/librorum",
            // 开发时的相对路径（相对于Swift项目）
            FileManager.default.currentDirectoryPath + "/../target/release/librorum",
            FileManager.default.currentDirectoryPath + "/../target/debug/librorum",
            // 绝对路径（当前目录向上查找）
            getProjectRootPath() + "/target/release/librorum",
            getProjectRootPath() + "/target/debug/librorum"
        ].compactMap { $0 }
        
        // 返回第一个存在的路径
        for path in possiblePaths {
            if FileManager.default.fileExists(atPath: path) {
                return path
            }
        }
        
        // 如果都不存在，返回默认路径（会在setupBackendBinary中报错）
        return (Bundle.main.resourcePath ?? "") + "/librorum_backend"
    }
    
    private func getProjectRootPath() -> String {
        // 从当前bundle路径向上查找，寻找包含Cargo.toml的目录
        var currentPath = Bundle.main.bundlePath
        
        for _ in 0..<10 { // 最多向上查找10级目录
            let parentPath = (currentPath as NSString).deletingLastPathComponent
            if parentPath == currentPath { break } // 已到根目录
            
            let cargoTomlPath = parentPath + "/Cargo.toml"
            if FileManager.default.fileExists(atPath: cargoTomlPath) {
                return parentPath
            }
            currentPath = parentPath
        }
        
        // 如果没找到，返回当前目录的上级目录
        return (FileManager.default.currentDirectoryPath as NSString).deletingLastPathComponent
    }
    
    private func getConfigFilePath() -> String {
        return getDataDirectory() + "/" + configFileName
    }
    
    private func getDataDirectory() -> String {
        #if os(macOS)
        return NSHomeDirectory() + "/Library/Application Support/librorum"
        #else
        return NSHomeDirectory() + "/Documents/librorum"
        #endif
    }
    
    private func getLogsDirectory() -> String {
        return getDataDirectory() + "/logs"
    }
    
    private func getLogFilePath() -> String {
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd"
        let dateString = dateFormatter.string(from: Date())
        return getLogsDirectory() + "/librorum-\(dateString).log"
    }
}

// MARK: - Error Types

enum CoreManagerError: LocalizedError {
    case backendBinaryNotFound(String)
    case backendStartupTimeout
    case grpcNotConnected
    case configurationError(String)
    
    var errorDescription: String? {
        switch self {
        case .backendBinaryNotFound(let path):
            return "Backend binary not found at path: \(path)"
        case .backendStartupTimeout:
            return "Backend startup timeout - failed to start within 10 seconds"
        case .grpcNotConnected:
            return "gRPC client is not connected"
        case .configurationError(let message):
            return "Configuration error: \(message)"
        }
    }
}

// MARK: - Extensions for SystemHealth

extension SystemHealth {
    convenience init(
        timestamp: Date,
        backendStatus: BackendStatus,
        totalNodes: Int,
        onlineNodes: Int,
        offlineNodes: Int
    ) {
        self.init(
            timestamp: timestamp,
            backendStatus: backendStatus,
            totalNodes: totalNodes,
            onlineNodes: onlineNodes,
            offlineNodes: offlineNodes,
            totalStorage: 0,
            usedStorage: 0,
            availableStorage: 0,
            totalFiles: 0,
            totalChunks: 0,
            networkLatency: 0,
            errorCount: 0,
            lastError: nil,
            uptime: 0,
            memoryUsage: 0,
            cpuUsage: 0
        )
    }
}
//
//  UtpTransport.swift
//  librorum
//
//  UTP (Data Portal Protocol) Swift实现
//  与Rust backend的hybrid架构集成
//

import Foundation
import Network
import SwiftUI
import OSLog

/// UTP传输模式
public enum UtpTransportMode: String, CaseIterable, Codable {
    case network = "network"
    case sharedMemory = "shared_memory"
    case auto = "auto"
}

/// UTP传输配置
public struct UtpConfig: Codable {
    public let mode: UtpTransportMode
    public let serverAddress: String?
    public let serverPort: Int?
    public let sharedMemorySize: Int
    public let sharedMemoryPath: String
    public let enableCompression: Bool
    public let enableEncryption: Bool
    public let chunkSize: Int
    public let timeoutSeconds: Int
    
    public init(
        mode: UtpTransportMode = .auto,
        serverAddress: String? = nil,
        serverPort: Int? = nil,
        sharedMemorySize: Int = 64 * 1024 * 1024,
        sharedMemoryPath: String = "/tmp/librorum_utp_swift",
        enableCompression: Bool = true,
        enableEncryption: Bool = false,
        chunkSize: Int = 8 * 1024 * 1024,
        timeoutSeconds: Int = 30
    ) {
        self.mode = mode
        self.serverAddress = serverAddress
        self.serverPort = serverPort
        self.sharedMemorySize = sharedMemorySize
        self.sharedMemoryPath = sharedMemoryPath
        self.enableCompression = enableCompression
        self.enableEncryption = enableEncryption
        self.chunkSize = chunkSize
        self.timeoutSeconds = timeoutSeconds
    }
}

/// UTP传输会话
@Observable
public class UtpSession: Identifiable {
    public let id = UUID()
    public let sessionId: String
    public let transferType: TransferType
    public let fileName: String
    public let totalSize: Int64
    
    @ObservationIgnored public private(set) var transferredBytes: Int64 = 0
    @ObservationIgnored public private(set) var transferRate: Double = 0.0
    @ObservationIgnored public private(set) var status: SessionStatus = .initializing
    @ObservationIgnored public private(set) var startTime: Date = Date()
    @ObservationIgnored public private(set) var error: String? = nil
    
    public enum TransferType: String, CaseIterable, Codable {
        case upload = "upload"
        case download = "download"
    }
    
    public enum SessionStatus: String, CaseIterable, Codable {
        case initializing = "initializing"
        case awaitingGrpcCoordination = "awaiting_grpc_coordination"
        case awaitingUtpConnection = "awaiting_utp_connection"
        case transferring = "transferring"
        case completed = "completed"
        case failed = "failed"
        case cancelled = "cancelled"
    }
    
    public init(sessionId: String, transferType: TransferType, fileName: String, totalSize: Int64) {
        self.sessionId = sessionId
        self.transferType = transferType
        self.fileName = fileName
        self.totalSize = totalSize
        self.startTime = Date()
    }
    
    /// 更新传输进度
    public func updateProgress(bytes: Int64, rate: Double) {
        transferredBytes = bytes
        transferRate = rate
    }
    
    /// 更新状态
    public func updateStatus(_ newStatus: SessionStatus, error: String? = nil) {
        status = newStatus
        self.error = error
    }
    
    /// 获取进度百分比
    public var progressPercent: Double {
        guard totalSize > 0 else { return 0.0 }
        return Double(transferredBytes) / Double(totalSize) * 100.0
    }
    
    /// 获取剩余时间估计 (秒)
    public var estimatedTimeRemaining: TimeInterval? {
        guard transferRate > 0, transferredBytes < totalSize else { return nil }
        let remainingBytes = totalSize - transferredBytes
        return Double(remainingBytes) / transferRate
    }
}

/// UTP传输事件
public enum UtpEvent {
    case sessionCreated(UtpSession)
    case grpcCoordinationComplete(sessionId: String, utpEndpoint: String)
    case utpConnectionEstablished(sessionId: String, mode: UtpTransportMode)
    case transferProgress(sessionId: String, bytes: Int64, totalBytes: Int64, rate: Double)
    case transferCompleted(sessionId: String, success: Bool, error: String?, elapsedSeconds: Double)
    case sessionStatusChanged(sessionId: String, oldStatus: UtpSession.SessionStatus, newStatus: UtpSession.SessionStatus)
}

/// UTP传输统计信息
public struct UtpStats: Codable {
    public let totalSessions: Int64
    public let successfulTransfers: Int64
    public let failedTransfers: Int64
    public let totalBytesTransferred: Int64
    public let averageTransferRate: Double
    public let maxTransferRate: Double
    public let networkModeUsage: Int64
    public let sharedMemoryModeUsage: Int64
    
    public init() {
        self.totalSessions = 0
        self.successfulTransfers = 0
        self.failedTransfers = 0
        self.totalBytesTransferred = 0
        self.averageTransferRate = 0.0
        self.maxTransferRate = 0.0
        self.networkModeUsage = 0
        self.sharedMemoryModeUsage = 0
    }
}

/// UTP传输客户端
@Observable
public class UtpTransportClient {
    public let config: UtpConfig
    @ObservationIgnored private let logger = Logger(subsystem: "com.librorum", category: "UtpTransport")
    
    @ObservationIgnored public private(set) var isConnected: Bool = false
    @ObservationIgnored public private(set) var activeSessions: [String: UtpSession] = [:]
    @ObservationIgnored public private(set) var stats: UtpStats = UtpStats()
    @ObservationIgnored public private(set) var connectionTime: Date? = nil
    
    /// 事件回调
    @ObservationIgnored private var eventHandlers: [(UtpEvent) -> Void] = []
    
    /// 网络连接 (用于网络模式)
    @ObservationIgnored private var networkConnection: NWConnection? = nil
    
    /// 共享内存管理 (用于共享内存模式)
    @ObservationIgnored private var sharedMemoryManager: SharedMemoryManager? = nil
    
    public init(config: UtpConfig) {
        self.config = config
        logger.info("🔧 UTP传输客户端初始化: mode=\\(config.mode.rawValue)")
    }
    
    /// 添加事件处理器
    public func addEventHandler(_ handler: @escaping (UtpEvent) -> Void) {
        eventHandlers.append(handler)
    }
    
    /// 触发事件
    private func triggerEvent(_ event: UtpEvent) {
        for handler in eventHandlers {
            handler(event)
        }
    }
    
    /// 连接到UTP服务器
    public func connect() async throws {
        guard !isConnected else {
            logger.info("ℹ️ UTP客户端已连接")
            return
        }
        
        logger.info("🔗 连接UTP服务器...")
        
        switch config.mode {
        case .network:
            try await connectNetwork()
        case .sharedMemory:
            try await connectSharedMemory()
        case .auto:
            // 自动选择模式：优先尝试共享内存，失败则使用网络
            do {
                try await connectSharedMemory()
            } catch {
                logger.warning("⚠️ 共享内存连接失败，回退到网络模式: \\(error)")
                try await connectNetwork()
            }
        }
        
        isConnected = true
        connectionTime = Date()
        logger.info("✅ UTP客户端连接成功")
    }
    
    /// 网络模式连接
    private func connectNetwork() async throws {
        guard let serverAddress = config.serverAddress,
              let serverPort = config.serverPort else {
            throw UtpError.configuration("网络模式需要服务器地址和端口")
        }
        
        let host = NWEndpoint.Host(serverAddress)
        let port = NWEndpoint.Port(integerLiteral: UInt16(serverPort))
        let endpoint = NWEndpoint.hostPort(host: host, port: port)
        
        let connection = NWConnection(to: endpoint, using: .tcp)
        
        return try await withCheckedThrowingContinuation { continuation in
            connection.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    self.logger.info("🌐 网络连接已建立: \\(serverAddress):\\(serverPort)")
                    self.networkConnection = connection
                    continuation.resume()
                case .failed(let error):
                    self.logger.error("❌ 网络连接失败: \\(error)")
                    continuation.resume(throwing: UtpError.network("连接失败: \\(error)"))
                case .cancelled:
                    continuation.resume(throwing: UtpError.network("连接被取消"))
                default:
                    break
                }
            }
            
            connection.start(queue: .global())
        }
    }
    
    /// 共享内存模式连接
    private func connectSharedMemory() async throws {
        logger.info("💾 初始化共享内存连接...")
        
        let manager = try SharedMemoryManager(
            path: config.sharedMemoryPath,
            size: config.sharedMemorySize
        )
        
        sharedMemoryManager = manager
        logger.info("✅ 共享内存连接已建立: \\(config.sharedMemoryPath)")
    }
    
    /// 断开连接
    public func disconnect() {
        guard isConnected else { return }
        
        logger.info("🔌 断开UTP连接...")
        
        // 关闭网络连接
        networkConnection?.cancel()
        networkConnection = nil
        
        // 清理共享内存
        sharedMemoryManager = nil
        
        // 取消所有活跃会话
        for session in activeSessions.values {
            session.updateStatus(.cancelled)
        }
        activeSessions.removeAll()
        
        isConnected = false
        connectionTime = nil
        logger.info("✅ UTP连接已断开")
    }
    
    /// 上传文件
    public func uploadFile(
        localPath: String,
        remotePath: String,
        fileId: String? = nil
    ) async throws -> UtpSession {
        guard isConnected else {
            throw UtpError.notConnected("UTP客户端未连接")
        }
        
        // 验证本地文件
        let fileURL = URL(fileURLWithPath: localPath)
        guard FileManager.default.fileExists(atPath: localPath) else {
            throw UtpError.fileNotFound("本地文件不存在: \\(localPath)")
        }
        
        let fileSize = try FileManager.default.attributesOfItem(atPath: localPath)[.size] as? Int64 ?? 0
        let fileName = fileURL.lastPathComponent
        
        logger.info("📤 开始上传文件: \\(fileName) (\\(formatFileSize(fileSize)))")
        
        let sessionId = UUID().uuidString
        let session = UtpSession(
            sessionId: sessionId,
            transferType: .upload,
            fileName: fileName,
            totalSize: fileSize
        )
        
        activeSessions[sessionId] = session
        triggerEvent(.sessionCreated(session))
        
        // 根据模式选择传输方式
        switch config.mode {
        case .network, .auto where networkConnection != nil:
            try await uploadFileNetwork(session: session, localPath: localPath, remotePath: remotePath)
        case .sharedMemory, .auto where sharedMemoryManager != nil:
            try await uploadFileSharedMemory(session: session, localPath: localPath, remotePath: remotePath)
        default:
            throw UtpError.configuration("无可用的传输模式")
        }
        
        return session
    }
    
    /// 网络模式上传文件
    private func uploadFileNetwork(session: UtpSession, localPath: String, remotePath: String) async throws {
        guard let connection = networkConnection else {
            throw UtpError.notConnected("网络连接不可用")
        }
        
        session.updateStatus(.transferring)
        
        let fileURL = URL(fileURLWithPath: localPath)
        let fileData = try Data(contentsOf: fileURL)
        
        let startTime = Date()
        var sentBytes: Int64 = 0
        let chunkSize = config.chunkSize
        
        // 分块发送文件数据
        let totalChunks = (fileData.count + chunkSize - 1) / chunkSize
        
        for chunkIndex in 0..<totalChunks {
            let chunkStart = chunkIndex * chunkSize
            let chunkEnd = min(chunkStart + chunkSize, fileData.count)
            let chunkData = fileData.subdata(in: chunkStart..<chunkEnd)
            
            // 创建UTP消息
            let message = UtpMessage.fileData(
                sequence: UInt64(chunkIndex),
                chunkIndex: UInt64(chunkIndex),
                data: chunkData,
                isLast: chunkIndex == totalChunks - 1
            )
            
            // 发送数据
            try await sendNetworkMessage(connection: connection, message: message)
            
            sentBytes += Int64(chunkData.count)
            let elapsed = Date().timeIntervalSince(startTime)
            let rate = elapsed > 0 ? Double(sentBytes) / elapsed : 0.0
            
            session.updateProgress(bytes: sentBytes, rate: rate)
            triggerEvent(.transferProgress(
                sessionId: session.sessionId,
                bytes: sentBytes,
                totalBytes: session.totalSize,
                rate: rate
            ))
            
            // 检查是否被取消
            if session.status == .cancelled {
                throw UtpError.cancelled("传输被取消")
            }
        }
        
        let elapsed = Date().timeIntervalSince(startTime)
        session.updateStatus(.completed)
        
        triggerEvent(.transferCompleted(
            sessionId: session.sessionId,
            success: true,
            error: nil,
            elapsedSeconds: elapsed
        ))
        
        logger.info("✅ 网络上传完成: \\(session.fileName) (\\(String(format: \"%.2f\", elapsed))s)")
    }
    
    /// 共享内存模式上传文件
    private func uploadFileSharedMemory(session: UtpSession, localPath: String, remotePath: String) async throws {
        guard let manager = sharedMemoryManager else {
            throw UtpError.notConnected("共享内存连接不可用")
        }
        
        session.updateStatus(.transferring)
        
        let fileURL = URL(fileURLWithPath: localPath)
        let fileData = try Data(contentsOf: fileURL)
        
        let startTime = Date()
        let chunkSize = config.chunkSize
        let totalChunks = (fileData.count + chunkSize - 1) / chunkSize
        
        // 发送文件头信息
        let fileInfo = FileTransferInfo(
            fileName: session.fileName,
            fileSize: session.totalSize,
            chunkCount: totalChunks,
            chunkSize: chunkSize
        )
        
        try manager.writeFileHeader(info: fileInfo)
        
        // 分块写入文件数据
        var sentBytes: Int64 = 0
        
        for chunkIndex in 0..<totalChunks {
            let chunkStart = chunkIndex * chunkSize
            let chunkEnd = min(chunkStart + chunkSize, fileData.count)
            let chunkData = fileData.subdata(in: chunkStart..<chunkEnd)
            
            try manager.writeFileChunk(
                chunkIndex: chunkIndex,
                data: chunkData,
                isLast: chunkIndex == totalChunks - 1
            )
            
            sentBytes += Int64(chunkData.count)
            let elapsed = Date().timeIntervalSince(startTime)
            let rate = elapsed > 0 ? Double(sentBytes) / elapsed : 0.0
            
            session.updateProgress(bytes: sentBytes, rate: rate)
            triggerEvent(.transferProgress(
                sessionId: session.sessionId,
                bytes: sentBytes,
                totalBytes: session.totalSize,
                rate: rate
            ))
            
            // 检查是否被取消
            if session.status == .cancelled {
                throw UtpError.cancelled("传输被取消")
            }
        }
        
        let elapsed = Date().timeIntervalSince(startTime)
        session.updateStatus(.completed)
        
        triggerEvent(.transferCompleted(
            sessionId: session.sessionId,
            success: true,
            error: nil,
            elapsedSeconds: elapsed
        ))
        
        logger.info("✅ 共享内存上传完成: \\(session.fileName) (\\(String(format: \"%.2f\", elapsed))s)")
    }
    
    /// 发送网络消息
    private func sendNetworkMessage(connection: NWConnection, message: UtpMessage) async throws {
        let messageData = message.toBytes()
        
        return try await withCheckedThrowingContinuation { continuation in
            connection.send(content: messageData, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: UtpError.network("发送失败: \\(error)"))
                } else {
                    continuation.resume()
                }
            })
        }
    }
    
    /// 取消会话
    public func cancelSession(_ sessionId: String) {
        guard let session = activeSessions[sessionId] else { return }
        
        logger.info("🛑 取消传输会话: \\(sessionId)")
        session.updateStatus(.cancelled)
        
        triggerEvent(.sessionStatusChanged(
            sessionId: sessionId,
            oldStatus: session.status,
            newStatus: .cancelled
        ))
    }
    
    /// 获取会话
    public func getSession(_ sessionId: String) -> UtpSession? {
        return activeSessions[sessionId]
    }
    
    /// 清理完成的会话
    public func cleanupCompletedSessions() {
        let completedSessionIds = activeSessions.compactMap { (key, session) in
            switch session.status {
            case .completed, .failed, .cancelled:
                return key
            default:
                return nil
            }
        }
        
        for sessionId in completedSessionIds {
            activeSessions.removeValue(forKey: sessionId)
        }
        
        if !completedSessionIds.isEmpty {
            logger.info("🧹 清理完成的会话: \\(completedSessionIds.count)个")
        }
    }
}

/// UTP错误类型
public enum UtpError: LocalizedError {
    case configuration(String)
    case network(String)
    case fileNotFound(String)
    case notConnected(String)
    case cancelled(String)
    case protocol(String)
    case timeout(String)
    case io(String)
    
    public var errorDescription: String? {
        switch self {
        case .configuration(let message):
            return "配置错误: \\(message)"
        case .network(let message):
            return "网络错误: \\(message)"
        case .fileNotFound(let message):
            return "文件未找到: \\(message)"
        case .notConnected(let message):
            return "未连接: \\(message)"
        case .cancelled(let message):
            return "已取消: \\(message)"
        case .protocol(let message):
            return "协议错误: \\(message)"
        case .timeout(let message):
            return "超时: \\(message)"
        case .io(let message):
            return "IO错误: \\(message)"
        }
    }
}

/// 工具函数
public func formatFileSize(_ bytes: Int64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB"]
    var size = Double(bytes)
    var unitIndex = 0
    
    while size >= 1024.0 && unitIndex < units.count - 1 {
        size /= 1024.0
        unitIndex += 1
    }
    
    if unitIndex == 0 {
        return "\\(Int(size)) \\(units[unitIndex])"
    } else {
        return String(format: "%.1f %@", size, units[unitIndex])
    }
}

public func formatTransferRate(_ bytesPerSecond: Double) -> String {
    return formatFileSize(Int64(bytesPerSecond)) + "/s"
}

public func formatDuration(_ seconds: TimeInterval) -> String {
    if seconds < 60 {
        return String(format: "%.1fs", seconds)
    } else if seconds < 3600 {
        let minutes = Int(seconds / 60)
        let secs = Int(seconds.truncatingRemainder(dividingBy: 60))
        return "\\(minutes)m\\(secs)s"
    } else {
        let hours = Int(seconds / 3600)
        let minutes = Int((seconds.truncatingRemainder(dividingBy: 3600)) / 60)
        return "\\(hours)h\\(minutes)m"
    }
}
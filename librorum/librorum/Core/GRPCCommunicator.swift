//
//  GRPCCommunicator.swift
//  librorum
//
//  Pure gRPC communication layer - NO UI dependencies
//

import Foundation
import GRPC
import NIO
import SwiftProtobuf

/// Pure communication protocol - no UI/SwiftUI dependencies
protocol GRPCCommunicatorProtocol {
    func connect(address: String) async throws
    func disconnect() async throws
    func isConnected() async -> Bool
    
    // Core gRPC operations
    func sendHeartbeat(nodeId: String) async throws -> HeartbeatResult
    func getNodeList() async throws -> [NodeData]
    func getSystemHealth() async throws -> CommunicatorSystemHealthData
    func addNode(address: String) async throws
    func removeNode(nodeId: String) async throws
    
    // File operations
    func listFiles(path: String, recursive: Bool, includeHidden: Bool) async throws -> FileListResult
    func uploadFile(metadata: FileUploadMetadata, data: Data) async throws -> FileUploadResult
    func downloadFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<FileDownloadChunk, Error>
    func deleteFile(fileId: String?, path: String?, recursive: Bool, force: Bool) async throws -> FileDeleteResult
    func createDirectory(path: String, createParents: Bool) async throws -> FileCreateDirectoryResult
    func getFileInfo(fileId: String?, path: String?, includeChunks: Bool) async throws -> FileInfoData
    func getSyncStatus(path: String?) async throws -> FileSyncStatusResult
}

/// Pure data structures - no SwiftData/SwiftUI dependencies
struct NodeData: Codable, Equatable {
    let nodeId: String
    let address: String
    let systemInfo: String
    let status: CommunicatorNodeStatus
    let lastHeartbeat: Date
    let connectionCount: Int
    let latency: TimeInterval
    let failureCount: Int
    let isOnline: Bool
    let discoveredAt: Date
}

// Using a different name to avoid conflict with existing NodeStatus
enum CommunicatorNodeStatus: String, Codable, CaseIterable {
    case online = "online"
    case offline = "offline"
    case connecting = "connecting"
    case error = "error"
}

struct HeartbeatResult: Codable, Equatable {
    let nodeId: String
    let address: String
    let systemInfo: String
    let timestamp: Date
    let status: Bool
    let latency: TimeInterval
}

struct CommunicatorSystemHealthData: Codable, Equatable {
    let totalStorage: Int64
    let usedStorage: Int64
    let availableStorage: Int64
    let totalFiles: Int
    let totalChunks: Int
    let networkLatency: TimeInterval
    let errorCount: Int
    let uptime: TimeInterval
    let memoryUsage: Int64
    let cpuUsage: Double
    let timestamp: Date
    
    init(
        totalStorage: Int64 = 0,
        usedStorage: Int64 = 0,
        availableStorage: Int64 = 0,
        totalFiles: Int = 0,
        totalChunks: Int = 0,
        networkLatency: TimeInterval = 0,
        errorCount: Int = 0,
        uptime: TimeInterval = 0,
        memoryUsage: Int64 = 0,
        cpuUsage: Double = 0,
        timestamp: Date = Date()
    ) {
        self.totalStorage = totalStorage
        self.usedStorage = usedStorage
        self.availableStorage = availableStorage
        self.totalFiles = totalFiles
        self.totalChunks = totalChunks
        self.networkLatency = networkLatency
        self.errorCount = errorCount
        self.uptime = uptime
        self.memoryUsage = memoryUsage
        self.cpuUsage = cpuUsage
        self.timestamp = timestamp
    }
}

/// Pure gRPC communication implementation
class GRPCCommunicator: GRPCCommunicatorProtocol {
    
    private var channel: GRPCChannel?
    private var client: Node_NodeServiceAsyncClient?
    private var eventLoopGroup: EventLoopGroup?
    private var isConnectedState: Bool = false
    private var serverAddress: String = ""
    private let connectionTimeout: TimeInterval = 10.0
    
    // MARK: - Connection Management
    
    func connect(address: String) async throws {
        guard !address.isEmpty else {
            throw GRPCError.invalidAddress
        }
        
        // Validate address format
        if !isValidAddress(address) {
            throw GRPCError.connectionFailed("Invalid address format")
        }
        
        let startTime = Date()
        
        // Parse address
        let components = address.components(separatedBy: ":")
        let host = components[0]
        let port = Int(components[1]) ?? 50051
        
        // Create event loop group
        self.eventLoopGroup = MultiThreadedEventLoopGroup(numberOfThreads: 1)
        
        // Create channel
        self.channel = try GRPCChannelPool.with(
            target: .host(host, port: port),
            transportSecurity: .plaintext,
            eventLoopGroup: eventLoopGroup!
        )
        
        // Create client
        self.client = Node_NodeServiceAsyncClient(
            channel: channel!,
            defaultCallOptions: CallOptions(
                timeLimit: .timeout(.seconds(30))
            )
        )
        
        // Test connection with a heartbeat
        do {
            let testRequest = Node_HeartbeatRequest.with {
                $0.nodeID = "test-connection"
                $0.address = address
                $0.systemInfo = "Swift Client"
                $0.timestamp = Int64(Date().timeIntervalSince1970)
            }
            
            _ = try await client!.heartbeat(testRequest)
            
            self.serverAddress = address
            self.isConnectedState = true
            
            let connectionTime = Date().timeIntervalSince(startTime)
            print("🔗 gRPC connected to \(address) in \(Int(connectionTime * 1000))ms")
        } catch {
            // Clean up on failure
            try? await channel?.close()
            try? await eventLoopGroup?.shutdownGracefully()
            self.channel = nil
            self.client = nil
            self.eventLoopGroup = nil
            
            throw GRPCError.connectionFailed("Failed to connect: \(error.localizedDescription)")
        }
    }
    
    func disconnect() async throws {
        guard isConnectedState else {
            throw GRPCError.notConnected
        }
        
        // Close channel and clean up
        do {
            try await channel?.close()
            try await eventLoopGroup?.shutdownGracefully()
        } catch {
            print("⚠️ Error during disconnect: \(error)")
        }
        
        self.channel = nil
        self.client = nil
        self.eventLoopGroup = nil
        self.isConnectedState = false
        self.serverAddress = ""
        
        print("🔌 gRPC disconnected")
    }
    
    func isConnected() async -> Bool {
        return isConnectedState
    }
    
    // MARK: - gRPC Operations
    
    func sendHeartbeat(nodeId: String) async throws -> HeartbeatResult {
        guard isConnectedState, let client = client else {
            throw GRPCError.notConnected
        }
        
        guard !nodeId.isEmpty else {
            throw GRPCError.invalidRequest("Node ID cannot be empty")
        }
        
        let startTime = Date()
        
        // Create request
        let request = Node_HeartbeatRequest.with {
            $0.nodeID = nodeId
            $0.address = serverAddress
            $0.systemInfo = getSystemInfo()
            $0.timestamp = Int64(Date().timeIntervalSince1970)
        }
        
        do {
            // Send heartbeat
            let response = try await client.heartbeat(request)
            let latency = Date().timeIntervalSince(startTime)
            
            return HeartbeatResult(
                nodeId: response.nodeID,
                address: response.address,
                systemInfo: response.systemInfo,
                timestamp: Date(timeIntervalSince1970: TimeInterval(response.timestamp)),
                status: response.status,
                latency: latency
            )
        } catch {
            throw GRPCError.serverError("Heartbeat failed: \(error.localizedDescription)")
        }
    }
    
    func getNodeList() async throws -> [NodeData] {
        guard isConnectedState, let client = client else {
            throw GRPCError.notConnected
        }
        
        // Create request
        let request = Node_NodeListRequest.with {
            $0.includeOffline = true // 包含离线节点
        }
        
        do {
            // Send gRPC request
            let response = try await client.getNodeList(request)
            
            // Convert proto NodeInfo to local NodeData
            let nodeList = response.nodes.map { protoNode in
                NodeData(
                    nodeId: protoNode.nodeID,
                    address: protoNode.address,
                    systemInfo: protoNode.systemInfo,
                    status: mapProtoStatus(protoNode.status),
                    lastHeartbeat: Date(timeIntervalSince1970: TimeInterval(protoNode.lastHeartbeat)),
                    connectionCount: Int(protoNode.connectionCount),
                    latency: protoNode.latencyMs / 1000.0, // 转换为秒
                    failureCount: Int(protoNode.failureCount),
                    isOnline: protoNode.isOnline,
                    discoveredAt: Date(timeIntervalSince1970: TimeInterval(protoNode.discoveredAt))
                )
            }
            
            print("📋 获取到 \(nodeList.count) 个节点 (\(response.onlineCount) 在线, \(response.offlineCount) 离线)")
            
            return nodeList
            
        } catch {
            throw GRPCError.serverError("获取节点列表失败: \(error.localizedDescription)")
        }
    }
    
    func getSystemHealth() async throws -> CommunicatorSystemHealthData {
        guard isConnectedState, let client = client else {
            throw GRPCError.notConnected
        }
        
        // Create request
        let request = Node_SystemHealthRequest()
        
        do {
            // Send gRPC request
            let response = try await client.getSystemHealth(request)
            
            // Convert proto response to local data structure
            let healthData = CommunicatorSystemHealthData(
                totalStorage: response.totalStorage,
                usedStorage: response.usedStorage,
                availableStorage: response.availableStorage,
                totalFiles: Int(response.totalFiles),
                totalChunks: Int(response.totalChunks),
                networkLatency: response.networkLatency,
                errorCount: Int(response.errorCount),
                uptime: TimeInterval(response.uptimeSeconds),
                memoryUsage: response.memoryUsage,
                cpuUsage: response.cpuUsage,
                timestamp: Date(timeIntervalSince1970: TimeInterval(response.timestamp))
            )
            
            print("💚 系统健康状态: \(healthData.memoryUsage / 1024 / 1024)MB 内存, \(healthData.cpuUsage)% CPU")
            
            return healthData
            
        } catch {
            throw GRPCError.serverError("获取系统健康状态失败: \(error.localizedDescription)")
        }
    }
    
    func addNode(address: String) async throws {
        guard isConnectedState, let client = client else {
            throw GRPCError.notConnected
        }
        
        guard isValidAddress(address) else {
            throw GRPCError.invalidRequest("Invalid node address format")
        }
        
        // Create request
        let request = Node_AddNodeRequest.with {
            $0.address = address
        }
        
        do {
            // Send gRPC request
            let response = try await client.addNode(request)
            
            if response.success {
                print("➕ 成功添加节点: \(address)")
                if let node = response.node {
                    print("   节点ID: \(node.nodeID)")
                    print("   状态: \(node.status)")
                }
            } else {
                throw GRPCError.serverError("添加节点失败: \(response.message)")
            }
            
        } catch {
            if error is GRPCError {
                throw error
            } else {
                throw GRPCError.serverError("添加节点失败: \(error.localizedDescription)")
            }
        }
    }
    
    func removeNode(nodeId: String) async throws {
        guard isConnectedState, let client = client else {
            throw GRPCError.notConnected
        }
        
        guard !nodeId.isEmpty else {
            throw GRPCError.invalidRequest("Node ID cannot be empty")
        }
        
        // Create request
        let request = Node_RemoveNodeRequest.with {
            $0.nodeID = nodeId
        }
        
        do {
            // Send gRPC request
            let response = try await client.removeNode(request)
            
            if response.success {
                print("➖ 成功移除节点: \(nodeId)")
            } else {
                throw GRPCError.serverError("移除节点失败: \(response.message)")
            }
            
        } catch {
            if error is GRPCError {
                throw error
            } else {
                throw GRPCError.serverError("移除节点失败: \(error.localizedDescription)")
            }
        }
    }
    
    // MARK: - Private Helpers
    
    private func isValidAddress(_ address: String) -> Bool {
        let components = address.components(separatedBy: ":")
        guard components.count == 2,
              let port = Int(components[1]),
              port > 0 && port <= 65535 else {
            return false
        }
        
        // Basic IP address validation
        let ipComponents = components[0].components(separatedBy: ".")
        if ipComponents.count == 4 {
            return ipComponents.allSatisfy { component in
                if let num = Int(component) {
                    return num >= 0 && num <= 255
                }
                return false
            }
        }
        
        // Allow hostnames
        return !components[0].isEmpty
    }
    
    private func getSystemInfo() -> String {
        #if os(macOS)
        let osVersion = ProcessInfo.processInfo.operatingSystemVersionString
        return "macOS \(osVersion)"
        #elseif os(iOS)
        let device = UIDevice.current
        return "iOS \(device.systemVersion) on \(device.model)"
        #else
        return "Unknown Platform"
        #endif
    }
    
    deinit {
        // Clean up resources
        Task {
            try? await disconnect()
        }
    }
    
    // MARK: - Helper Methods
    
    /// 映射proto状态字符串到本地枚举
    private func mapProtoStatus(_ status: String) -> CommunicatorNodeStatus {
        switch status.lowercased() {
        case "online":
            return .online
        case "offline":
            return .offline
        case "connecting":
            return .connecting
        case "error":
            return .error
        default:
            return .offline
        }
    }
    
    // MARK: - File Operations
    
    func listFiles(path: String, recursive: Bool, includeHidden: Bool) async throws -> FileListResult {
        guard isConnectedState, let channel = channel else {
            throw GRPCError.notConnected
        }
        
        let fileClient = File_FileServiceAsyncClient(channel: channel)
        let request = File_ListFilesRequest.with {
            $0.path = path
            $0.recursive = recursive
            $0.includeHidden = includeHidden
        }
        
        let response = try await fileClient.listFiles(request)
        
        let files = response.files.map { protoFile in
            FileInfoData(
                fileId: protoFile.fileID,
                name: protoFile.name,
                path: protoFile.path,
                parentPath: protoFile.parentPath,
                size: protoFile.size,
                createdAt: Date(timeIntervalSince1970: TimeInterval(protoFile.createdAt)),
                modifiedAt: Date(timeIntervalSince1970: TimeInterval(protoFile.modifiedAt)),
                accessedAt: Date(timeIntervalSince1970: TimeInterval(protoFile.accessedAt)),
                fileType: mapFileType(protoFile.fileType),
                mimeType: protoFile.mimeType,
                checksum: protoFile.checksum,
                isDirectory: protoFile.isDirectory,
                isSymlink: protoFile.isSymlink,
                chunkCount: Int(protoFile.chunkCount),
                chunkIds: protoFile.chunkIds,
                replicationFactor: Int(protoFile.replicationFactor),
                isCompressed: protoFile.isCompressed,
                isEncrypted: protoFile.isEncrypted,
                syncStatus: mapSyncStatus(protoFile.syncStatus)
            )
        }
        
        return FileListResult(
            files: files,
            currentPath: response.currentPath,
            totalCount: Int(response.totalCount),
            totalSize: response.totalSize
        )
    }
    
    func uploadFile(metadata: FileUploadMetadata, data: Data) async throws -> FileUploadResult {
        guard isConnectedState, let channel = channel else {
            throw GRPCError.notConnected
        }
        
        let fileClient = File_FileServiceAsyncClient(channel: channel)
        let uploadCall = fileClient.uploadFile()
        
        // Send metadata first
        let metadataRequest = File_UploadFileRequest.with {
            $0.metadata = File_UploadFileMetadata.with {
                $0.name = metadata.name
                $0.path = metadata.path
                $0.size = Int64(data.count)
                $0.mimeType = metadata.mimeType
                $0.checksum = metadata.checksum
                $0.overwrite = metadata.overwrite
                $0.compress = metadata.compress
                $0.encrypt = metadata.encrypt
            }
        }
        
        try await uploadCall.requestStream.send(metadataRequest)
        
        // Send data in chunks
        let chunkSize = 8192 // 8KB chunks
        var offset = 0
        
        while offset < data.count {
            let remainingBytes = data.count - offset
            let currentChunkSize = min(chunkSize, remainingBytes)
            let chunkData = data.subdata(in: offset..<(offset + currentChunkSize))
            
            let chunkRequest = File_UploadFileRequest.with {
                $0.chunk = chunkData
            }
            
            try await uploadCall.requestStream.send(chunkRequest)
            offset += currentChunkSize
        }
        
        try await uploadCall.requestStream.finish()
        let response = try await uploadCall.response
        
        return FileUploadResult(
            success: response.success,
            message: response.message,
            bytesUploaded: response.bytesUploaded,
            fileInfo: response.hasFileInfo ? mapFileInfo(response.fileInfo) : nil
        )
    }
    
    func downloadFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<FileDownloadChunk, Error> {
        guard isConnectedState, let channel = channel else {
            throw GRPCError.notConnected
        }
        
        let fileClient = File_FileServiceAsyncClient(channel: channel)
        let request = File_DownloadFileRequest.with {
            if let fileId = fileId {
                $0.fileID = fileId
            }
            if let path = path {
                $0.path = path
            }
            $0.offset = 0
            $0.length = 0 // Download entire file
        }
        
        let downloadCall = fileClient.downloadFile(request)
        
        return AsyncThrowingStream { continuation in
            Task {
                do {
                    for try await response in downloadCall.responseStream {
                        if response.hasFileInfo {
                            let chunk = FileDownloadChunk.fileInfo(mapFileInfo(response.fileInfo))
                            continuation.yield(chunk)
                        } else if response.data != nil {
                            let chunkData = response.chunk
                            let chunk = FileDownloadChunk.data(chunkData, offset: response.offset, totalSize: response.totalSize)
                            continuation.yield(chunk)
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
    }
    
    func deleteFile(fileId: String?, path: String?, recursive: Bool, force: Bool) async throws -> FileDeleteResult {
        guard isConnectedState, let channel = channel else {
            throw GRPCError.notConnected
        }
        
        let fileClient = File_FileServiceAsyncClient(channel: channel)
        let request = File_DeleteFileRequest.with {
            if let fileId = fileId {
                $0.fileID = fileId
            }
            if let path = path {
                $0.path = path
            }
            $0.recursive = recursive
            $0.force = force
        }
        
        let response = try await fileClient.deleteFile(request)
        
        return FileDeleteResult(
            success: response.success,
            message: response.message,
            deletedCount: Int(response.deletedCount)
        )
    }
    
    func createDirectory(path: String, createParents: Bool) async throws -> FileCreateDirectoryResult {
        guard isConnectedState, let channel = channel else {
            throw GRPCError.notConnected
        }
        
        let fileClient = File_FileServiceAsyncClient(channel: channel)
        let request = File_CreateDirectoryRequest.with {
            $0.path = path
            $0.createParents = createParents
        }
        
        let response = try await fileClient.createDirectory(request)
        
        return FileCreateDirectoryResult(
            success: response.success,
            message: response.message,
            directoryInfo: response.hasDirectoryInfo ? mapFileInfo(response.directoryInfo) : nil
        )
    }
    
    func getFileInfo(fileId: String?, path: String?, includeChunks: Bool) async throws -> FileInfoData {
        guard isConnectedState, let channel = channel else {
            throw GRPCError.notConnected
        }
        
        let fileClient = File_FileServiceAsyncClient(channel: channel)
        let request = File_GetFileInfoRequest.with {
            if let fileId = fileId {
                $0.fileID = fileId
            }
            if let path = path {
                $0.path = path
            }
            $0.includeChunks = includeChunks
        }
        
        let response = try await fileClient.getFileInfo(request)
        return mapFileInfo(response)
    }
    
    func getSyncStatus(path: String?) async throws -> FileSyncStatusResult {
        guard isConnectedState, let channel = channel else {
            throw GRPCError.notConnected
        }
        
        let fileClient = File_FileServiceAsyncClient(channel: channel)
        let request = File_GetSyncStatusRequest.with {
            if let path = path {
                $0.path = path
            }
        }
        
        let response = try await fileClient.getSyncStatus(request)
        
        return FileSyncStatusResult(
            overallStatus: mapSyncStatus(response.overallStatus),
            pendingUploads: Int(response.pendingUploads),
            pendingDownloads: Int(response.pendingDownloads),
            syncingFiles: Int(response.syncingFiles),
            errorFiles: Int(response.errorFiles),
            conflictFiles: Int(response.conflictFiles),
            bytesToUpload: response.bytesToUpload,
            bytesToDownload: response.bytesToDownload,
            pendingFiles: response.pendingFiles.map { mapFileInfo($0) }
        )
    }
    
    // MARK: - File Helper Methods
    
    private func mapFileInfo(_ protoFile: File_FileInfo) -> FileInfoData {
        return FileInfoData(
            fileId: protoFile.fileID,
            name: protoFile.name,
            path: protoFile.path,
            parentPath: protoFile.parentPath,
            size: protoFile.size,
            createdAt: Date(timeIntervalSince1970: TimeInterval(protoFile.createdAt)),
            modifiedAt: Date(timeIntervalSince1970: TimeInterval(protoFile.modifiedAt)),
            accessedAt: Date(timeIntervalSince1970: TimeInterval(protoFile.accessedAt)),
            fileType: mapFileType(protoFile.fileType),
            mimeType: protoFile.mimeType,
            checksum: protoFile.checksum,
            isDirectory: protoFile.isDirectory,
            isSymlink: protoFile.isSymlink,
            chunkCount: Int(protoFile.chunkCount),
            chunkIds: protoFile.chunkIds,
            replicationFactor: Int(protoFile.replicationFactor),
            isCompressed: protoFile.isCompressed,
            isEncrypted: protoFile.isEncrypted,
            syncStatus: mapSyncStatus(protoFile.syncStatus)
        )
    }
    
    private func mapFileType(_ protoType: File_FileType) -> FileTypeData {
        switch protoType {
        case .unknown:
            return .unknown
        case .regular:
            return .regular
        case .directory:
            return .directory
        case .symlink:
            return .symlink
        case .blockDevice:
            return .blockDevice
        case .charDevice:
            return .charDevice
        case .fifo:
            return .fifo
        case .socket:
            return .socket
        case .UNRECOGNIZED(_):
            return .unknown
        }
    }
    
    private func mapSyncStatus(_ protoStatus: File_SyncStatus) -> FileSyncStatusData {
        switch protoStatus {
        case .unknown:
            return .unknown
        case .synced:
            return .synced
        case .pending:
            return .pending
        case .syncing:
            return .syncing
        case .error:
            return .error
        case .conflict:
            return .conflict
        case .UNRECOGNIZED(_):
            return .unknown
        }
    }
}

// MARK: - Error Types

enum GRPCError: Error, LocalizedError, Equatable {
    case notConnected
    case connectionFailed(String)
    case invalidAddress
    case invalidRequest(String)
    case timeout
    case serverError(String)
    case unknownError
    
    var errorDescription: String? {
        switch self {
        case .notConnected:
            return "Not connected to gRPC server"
        case .connectionFailed(let message):
            return "Connection failed: \(message)"
        case .invalidAddress:
            return "Invalid server address"
        case .invalidRequest(let message):
            return "Invalid request: \(message)"
        case .timeout:
            return "Request timeout"
        case .serverError(let message):
            return "Server error: \(message)"
        case .unknownError:
            return "Unknown error occurred"
        }
    }
}

// MARK: - File Operation Data Structures

struct FileInfoData: Codable, Equatable {
    let fileId: String
    let name: String
    let path: String
    let parentPath: String
    let size: Int64
    let createdAt: Date
    let modifiedAt: Date
    let accessedAt: Date
    let fileType: FileTypeData
    let mimeType: String
    let checksum: String
    let isDirectory: Bool
    let isSymlink: Bool
    let chunkCount: Int
    let chunkIds: [String]
    let replicationFactor: Int
    let isCompressed: Bool
    let isEncrypted: Bool
    let syncStatus: FileSyncStatusData
}

enum FileTypeData: String, Codable, CaseIterable {
    case unknown = "unknown"
    case regular = "regular"
    case directory = "directory"
    case symlink = "symlink"
    case blockDevice = "blockDevice"
    case charDevice = "charDevice"
    case fifo = "fifo"
    case socket = "socket"
}

enum FileSyncStatusData: String, Codable, CaseIterable {
    case unknown = "unknown"
    case synced = "synced"
    case pending = "pending"
    case syncing = "syncing"
    case error = "error"
    case conflict = "conflict"
}

struct FileUploadMetadata: Codable, Equatable {
    let name: String
    let path: String
    let mimeType: String
    let checksum: String
    let overwrite: Bool
    let compress: Bool
    let encrypt: Bool
}

struct FileListResult: Codable, Equatable {
    let files: [FileInfoData]
    let currentPath: String
    let totalCount: Int
    let totalSize: Int64
}

struct FileUploadResult: Codable, Equatable {
    let success: Bool
    let message: String
    let bytesUploaded: Int64
    let fileInfo: FileInfoData?
}

enum FileDownloadChunk: Equatable {
    case fileInfo(FileInfoData)
    case data(Data, offset: Int64, totalSize: Int64)
}

struct FileDeleteResult: Codable, Equatable {
    let success: Bool
    let message: String
    let deletedCount: Int
}

struct FileCreateDirectoryResult: Codable, Equatable {
    let success: Bool
    let message: String
    let directoryInfo: FileInfoData?
}

struct FileSyncStatusResult: Codable, Equatable {
    let overallStatus: FileSyncStatusData
    let pendingUploads: Int
    let pendingDownloads: Int
    let syncingFiles: Int
    let errorFiles: Int
    let conflictFiles: Int
    let bytesToUpload: Int64
    let bytesToDownload: Int64
    let pendingFiles: [FileInfoData]
}
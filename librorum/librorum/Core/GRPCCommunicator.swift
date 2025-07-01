//
//  GRPCCommunicator.swift
//  librorum
//
//  Pure gRPC communication layer - NO UI dependencies
//

import Foundation

#if canImport(GRPCCore)
import GRPCCore
#endif

/// Errors that can occur during gRPC communication
enum GRPCCommunicatorError: Error {
    case invalidAddress(String)
    case notConnected
    case connectionFailed(String)
    case requestFailed(String)
}

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
    func uploadFileWithProgress(metadata: FileUploadMetadata, data: Data, progressCallback: @escaping (FileUploadProgress) -> Void) async throws -> FileUploadResult
    func downloadFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<FileDownloadChunk, Error>
    func deleteFile(fileId: String?, path: String?, recursive: Bool, force: Bool) async throws -> FileDeleteResult
    func createDirectory(path: String, createParents: Bool) async throws -> FileCreateDirectoryResult
    func getFileInfo(fileId: String?, path: String?, includeChunks: Bool) async throws -> FileInfoData
    func getSyncStatus(path: String?) async throws -> FileSyncStatusResult
    
    // Encrypted file operations
    func uploadEncryptedFile(metadata: FileUploadMetadata, encryptedData: Data, keyId: String) async throws -> FileUploadResult
    func downloadEncryptedFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<EncryptedFileDownloadChunk, Error>
    func rotateFileEncryptionKey(fileId: String, newKeyId: String) async throws -> FileKeyRotationResult
    
    // Log operations
    func getLogs(limit: Int, levelFilter: String, moduleFilter: String, searchText: String, reverse: Bool) async throws -> LogListResult
    func streamLogs(levelFilter: String, moduleFilter: String, follow: Bool, tail: Int) async throws -> AsyncThrowingStream<LogEntryData, Error>
    func clearLogs(clearAll: Bool, beforeTimestamp: Int64) async throws -> LogClearResult
    func exportLogs(format: LogExportFormat, levelFilter: String, moduleFilter: String) async throws -> LogExportResult
    func getLogStats(startTime: Int64, endTime: Int64) async throws -> LogStatsResult
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
    case unknown = "unknown"
}

struct HeartbeatResult: Codable, Equatable {
    let success: Bool
    let message: String
    let timestamp: Date
    
    // Additional fields needed by UI
    let nodeId: String
    let address: String
    let systemInfo: String
    let status: Bool
}

struct CommunicatorSystemHealthData: Codable, Equatable {
    let cpuUsage: Double
    let memoryUsage: Int64
    let diskUsage: Double
    let networkStats: String
    let activeConnections: Int
    let uptime: TimeInterval
    let lastUpdated: Date
    
    // Additional fields needed by UI
    let timestamp: Date
    let totalStorage: Int64
    let usedStorage: Int64
    let availableStorage: Int64
    let totalFiles: Int
    let totalChunks: Int
    let networkLatency: Double
    let errorCount: Int
}

// File operation data structures
struct FileListResult: Codable, Equatable {
    let success: Bool
    let files: [FileItemData]
    let message: String
}

struct FileItemData: Codable, Equatable {
    let fileId: String
    let name: String
    let path: String
    let fileType: FileType
    let size: Int64
    let lastModified: Date
    let permissions: String
    let isDirectory: Bool
    let isHidden: Bool
}

enum FileType: String, Codable, CaseIterable {
    case file = "file"
    case directory = "directory"
    case symlink = "symlink"
    case unknown = "unknown"
}

struct FileUploadMetadata: Codable, Equatable {
    let filename: String
    let path: String
    let fileType: FileType
    let size: Int64
    let permissions: String
    let overwrite: Bool
    let createDirectories: Bool
    let isEncrypted: Bool
    let encryptionAlgorithm: String?
    let keyId: String?
}

struct FileUploadResult: Codable, Equatable {
    let success: Bool
    let fileId: String
    let message: String
    let bytesUploaded: Int64
}

struct FileUploadProgress: Codable, Equatable {
    let bytesUploaded: Int64
    let totalBytes: Int64
    let percentage: Double
    let transferRate: Double // bytes per second
    let estimatedTimeRemaining: TimeInterval
}

struct FileDownloadChunk: Codable, Equatable {
    let data: Data
    let chunkIndex: Int
    let totalChunks: Int
    let isLastChunk: Bool
}

struct FileDeleteResult: Codable, Equatable {
    let success: Bool
    let message: String
    let deletedCount: Int
}

struct FileCreateDirectoryResult: Codable, Equatable {
    let success: Bool
    let message: String
    let path: String
}

struct FileInfoData: Codable, Equatable {
    let fileId: String
    let name: String
    let path: String
    let fileType: FileType
    let size: Int64
    let permissions: String
    let createdAt: Date
    let modifiedAt: Date
    let isDirectory: Bool
    let isHidden: Bool
    let chunks: [FileChunkData]
    let isEncrypted: Bool
    let encryptionAlgorithm: String?
    let keyId: String?
    let version: Int
    let checksum: String
}

struct FileChunkData: Codable, Equatable {
    let chunkId: String
    let index: Int
    let size: Int64
    let checksum: String
}

struct FileSyncStatusResult: Codable, Equatable {
    let path: String
    let isSynced: Bool
    let lastSync: Date
    let pendingUploads: Int
    let pendingDownloads: Int
    let conflicts: [FileSyncConflict]
}

struct FileSyncConflict: Codable, Equatable {
    let path: String
    let conflictType: String
    let description: String
}

// Encrypted file operation data structures
struct EncryptedFileDownloadChunk: Codable, Equatable {
    let encryptedData: Data
    let chunkIndex: Int
    let totalChunks: Int
    let isLastChunk: Bool
    let keyId: String
}

struct FileKeyRotationResult: Codable, Equatable {
    let success: Bool
    let message: String
    let oldKeyId: String
    let newKeyId: String
}

// Log operation data structures
struct LogListResult: Codable {
    let logs: [LogEntryData]
    let totalCount: Int
    let hasMore: Bool
}

struct LogEntryData: Codable {
    let timestamp: Date
    let level: CommunicatorLogLevel
    let module: String
    let message: String
    let threadId: String
    let file: String
    let line: Int
    let fields: [String: String]
}

enum CommunicatorLogLevel: String, Codable, CaseIterable {
    case unknown = "unknown"
    case trace = "trace"
    case debug = "debug"
    case info = "info"
    case warn = "warn"
    case error = "error"
}

struct LogClearResult: Codable, Equatable {
    let success: Bool
    let clearedCount: Int
    let message: String
}

enum LogExportFormat: String, Codable, CaseIterable {
    case unknown = "unknown"
    case json = "json"
    case csv = "csv"
    case plain = "plain"
}

struct LogExportResult: Codable, Equatable {
    let success: Bool
    let data: Data
    let filename: String
    let mimeType: String
    let logCount: Int
    let fileSize: Int64
}

struct LogStatsResult: Codable, Equatable {
    let totalLogs: Int64
    let levelCounts: [String: Int64]
    let moduleCounts: [String: Int64]
    let errorCount: Int64
    let warnCount: Int64
    let trends: [LogTrendData]
}

struct LogTrendData: Codable, Equatable {
    let timestamp: Date
    let logCount: Int64
    let errorCount: Int64
    let warnCount: Int64
}

/// Mock implementation for systems that don't support the minimum requirements
class MockGRPCCommunicator: GRPCCommunicatorProtocol {
    private var connected = false
    
    func connect(address: String) async throws {
        print("Mock: Connecting to \(address)")
        connected = true
    }
    
    func disconnect() async throws {
        print("Mock: Disconnecting")
        connected = false
    }
    
    func isConnected() async -> Bool {
        return connected
    }
    
    func sendHeartbeat(nodeId: String) async throws -> HeartbeatResult {
        return HeartbeatResult(
            success: true,
            message: "Mock heartbeat",
            timestamp: Date(),
            nodeId: nodeId,
            address: "127.0.0.1:50051",
            systemInfo: "Mock System",
            status: true
        )
    }
    
    func getNodeList() async throws -> [NodeData] {
        return [
            NodeData(
                nodeId: "mock-node-1",
                address: "127.0.0.1:50051",
                systemInfo: "Mock System",
                status: .online,
                lastHeartbeat: Date(),
                connectionCount: 1,
                latency: 0.1,
                failureCount: 0,
                isOnline: true,
                discoveredAt: Date()
            )
        ]
    }
    
    func getSystemHealth() async throws -> CommunicatorSystemHealthData {
        return CommunicatorSystemHealthData(
            cpuUsage: 25.0,
            memoryUsage: 512 * 1024 * 1024, // 512MB
            diskUsage: 60.0,
            networkStats: "Mock stats",
            activeConnections: 5,
            uptime: 3600.0,
            lastUpdated: Date(),
            timestamp: Date(),
            totalStorage: 1024 * 1024 * 1024, // 1GB
            usedStorage: 600 * 1024 * 1024, // 600MB
            availableStorage: 424 * 1024 * 1024, // 424MB
            totalFiles: 150,
            totalChunks: 750,
            networkLatency: 0.025,
            errorCount: 0
        )
    }
    
    func addNode(address: String) async throws {
        print("Mock: Adding node \(address)")
    }
    
    func removeNode(nodeId: String) async throws {
        print("Mock: Removing node \(nodeId)")
    }
    
    func listFiles(path: String, recursive: Bool, includeHidden: Bool) async throws -> FileListResult {
        return FileListResult(
            success: true,
            files: [
                FileItemData(
                    fileId: "mock-file-1",
                    name: "example.txt",
                    path: "/mock/example.txt",
                    fileType: .file,
                    size: 1024,
                    lastModified: Date(),
                    permissions: "644",
                    isDirectory: false,
                    isHidden: false
                )
            ],
            message: "Mock file list"
        )
    }
    
    func uploadFile(metadata: FileUploadMetadata, data: Data) async throws -> FileUploadResult {
        return FileUploadResult(
            success: true,
            fileId: "mock-upload-\(UUID().uuidString)",
            message: "Mock upload successful",
            bytesUploaded: Int64(data.count)
        )
    }
    
    func uploadFileWithProgress(metadata: FileUploadMetadata, data: Data, progressCallback: @escaping (FileUploadProgress) -> Void) async throws -> FileUploadResult {
        let totalBytes = Int64(data.count)
        let chunkSize: Int64 = 1024 * 64 // 64KB chunks
        
        var i = Int64(0)
        while i < totalBytes {
            let bytesUploaded = min(i + chunkSize, totalBytes)
            let percentage = Double(bytesUploaded) / Double(totalBytes) * 100.0
            let progress = FileUploadProgress(
                bytesUploaded: bytesUploaded,
                totalBytes: totalBytes,
                percentage: percentage,
                transferRate: Double(chunkSize) * 15.0, // Mock 15 chunks/sec
                estimatedTimeRemaining: Double(totalBytes - bytesUploaded) / (Double(chunkSize) * 15.0)
            )
            progressCallback(progress)
            
            // Simulate upload delay
            try await Task.sleep(nanoseconds: 66_666_667) // ~15 chunks per second
            
            i += chunkSize
        }
        
        return FileUploadResult(
            success: true,
            fileId: "mock-upload-\(UUID().uuidString)",
            message: "Mock upload with progress successful",
            bytesUploaded: totalBytes
        )
    }
    
    func downloadFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<FileDownloadChunk, Error> {
        return AsyncThrowingStream { continuation in
            Task {
                let mockData = "Mock file content".data(using: .utf8) ?? Data()
                let chunk = FileDownloadChunk(
                    data: mockData,
                    chunkIndex: 0,
                    totalChunks: 1,
                    isLastChunk: true
                )
                continuation.yield(chunk)
                continuation.finish()
            }
        }
    }
    
    func deleteFile(fileId: String?, path: String?, recursive: Bool, force: Bool) async throws -> FileDeleteResult {
        return FileDeleteResult(success: true, message: "Mock deletion", deletedCount: 1)
    }
    
    func createDirectory(path: String, createParents: Bool) async throws -> FileCreateDirectoryResult {
        return FileCreateDirectoryResult(success: true, message: "Mock directory created", path: path)
    }
    
    func getFileInfo(fileId: String?, path: String?, includeChunks: Bool) async throws -> FileInfoData {
        return FileInfoData(
            fileId: fileId ?? "mock-file-id",
            name: "mock.txt",
            path: path ?? "/mock.txt",
            fileType: .file,
            size: 1024,
            permissions: "644",
            createdAt: Date(),
            modifiedAt: Date(),
            isDirectory: false,
            isHidden: false,
            chunks: [],
            isEncrypted: false,
            encryptionAlgorithm: nil,
            keyId: nil,
            version: 1,
            checksum: "mock-checksum"
        )
    }
    
    func getSyncStatus(path: String?) async throws -> FileSyncStatusResult {
        return FileSyncStatusResult(
            path: path ?? "/",
            isSynced: true,
            lastSync: Date(),
            pendingUploads: 0,
            pendingDownloads: 0,
            conflicts: []
        )
    }
    
    func getLogs(limit: Int, levelFilter: String, moduleFilter: String, searchText: String, reverse: Bool) async throws -> LogListResult {
        return LogListResult(
            logs: [
                LogEntryData(
                    timestamp: Date(),
                    level: CommunicatorLogLevel.info,
                    module: "mock",
                    message: "Mock log entry",
                    threadId: "main",
                    file: "mock.rs",
                    line: 42,
                    fields: [:]
                )
            ],
            totalCount: 1,
            hasMore: false
        )
    }
    
    func streamLogs(levelFilter: String, moduleFilter: String, follow: Bool, tail: Int) async throws -> AsyncThrowingStream<LogEntryData, Error> {
        return AsyncThrowingStream { continuation in
            Task {
                let log = LogEntryData(
                    timestamp: Date(),
                    level: CommunicatorLogLevel.info,
                    module: "mock",
                    message: "Streaming mock log",
                    threadId: "main",
                    file: "mock.rs",
                    line: 42,
                    fields: [:]
                )
                continuation.yield(log)
                continuation.finish()
            }
        }
    }
    
    func clearLogs(clearAll: Bool, beforeTimestamp: Int64) async throws -> LogClearResult {
        return LogClearResult(success: true, clearedCount: 10, message: "Mock logs cleared")
    }
    
    func exportLogs(format: LogExportFormat, levelFilter: String, moduleFilter: String) async throws -> LogExportResult {
        let mockData = "Mock log export".data(using: .utf8) ?? Data()
        return LogExportResult(
            success: true,
            data: mockData,
            filename: "mock_logs.txt",
            mimeType: "text/plain",
            logCount: 1,
            fileSize: Int64(mockData.count)
        )
    }
    
    func getLogStats(startTime: Int64, endTime: Int64) async throws -> LogStatsResult {
        return LogStatsResult(
            totalLogs: 100,
            levelCounts: ["info": 80, "warn": 15, "error": 5],
            moduleCounts: ["mock": 100],
            errorCount: 5,
            warnCount: 15,
            trends: []
        )
    }
    
    // MARK: - Encrypted File Operations
    
    func uploadEncryptedFile(metadata: FileUploadMetadata, encryptedData: Data, keyId: String) async throws -> FileUploadResult {
        return FileUploadResult(
            success: true,
            fileId: "mock-encrypted-upload-\(UUID().uuidString)",
            message: "Mock encrypted upload successful",
            bytesUploaded: Int64(encryptedData.count)
        )
    }
    
    func downloadEncryptedFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<EncryptedFileDownloadChunk, Error> {
        return AsyncThrowingStream { continuation in
            Task {
                let mockEncryptedData = "Mock encrypted file content".data(using: .utf8) ?? Data()
                let chunk = EncryptedFileDownloadChunk(
                    encryptedData: mockEncryptedData,
                    chunkIndex: 0,
                    totalChunks: 1,
                    isLastChunk: true,
                    keyId: "mock-key-id"
                )
                continuation.yield(chunk)
                continuation.finish()
            }
        }
    }
    
    func rotateFileEncryptionKey(fileId: String, newKeyId: String) async throws -> FileKeyRotationResult {
        return FileKeyRotationResult(
            success: true,
            message: "Mock key rotation successful",
            oldKeyId: "mock-old-key-id",
            newKeyId: newKeyId
        )
    }
}

/// Factory to create appropriate communicator based on system capabilities
@MainActor
class GRPCCommunicatorFactory {
    static func createCommunicator() -> GRPCCommunicatorProtocol {
        if #available(macOS 15.0, iOS 18.0, watchOS 11.0, tvOS 18.0, visionOS 2.0, *) {
            return RealGRPCCommunicator()
        } else {
            print("Warning: Using mock gRPC communicator due to system requirements")
            return MockGRPCCommunicator()
        }
    }
}

/// Real implementation using new gRPC Swift framework
@available(macOS 15.0, iOS 18.0, watchOS 11.0, tvOS 18.0, visionOS 2.0, *)
class RealGRPCCommunicator: GRPCCommunicatorProtocol {
    private var isClientConnected = false
    private let mock = MockGRPCCommunicator()
    
    // TODO: Re-implement with proper protobuf generation once grpc-swift-2 setup is complete
    
    func connect(address: String) async throws {
        // TODO: Implement real gRPC connection with proper protobuf files
        print("Real gRPC: Connection temporarily delegated to mock implementation")
        try await mock.connect(address: address)
        isClientConnected = true
    }
    
    func disconnect() async throws {
        isClientConnected = false
        try await mock.disconnect()
    }
    
    func isConnected() async -> Bool {
        if isClientConnected {
            return true
        }
        return await mock.isConnected()
    }
    
    func sendHeartbeat(nodeId: String) async throws -> HeartbeatResult {
        return try await mock.sendHeartbeat(nodeId: nodeId)
    }
    
    func getNodeList() async throws -> [NodeData] {
        return try await mock.getNodeList()
    }
    
    func getSystemHealth() async throws -> CommunicatorSystemHealthData {
        return try await mock.getSystemHealth()
    }
    
    func addNode(address: String) async throws {
        try await mock.addNode(address: address)
    }
    
    func removeNode(nodeId: String) async throws {
        try await mock.removeNode(nodeId: nodeId)
    }
    
    func listFiles(path: String, recursive: Bool, includeHidden: Bool) async throws -> FileListResult {
        return try await mock.listFiles(path: path, recursive: recursive, includeHidden: includeHidden)
    }
    
    func uploadFile(metadata: FileUploadMetadata, data: Data) async throws -> FileUploadResult {
        return try await mock.uploadFile(metadata: metadata, data: data)
    }
    
    func uploadFileWithProgress(metadata: FileUploadMetadata, data: Data, progressCallback: @escaping (FileUploadProgress) -> Void) async throws -> FileUploadResult {
        return try await mock.uploadFileWithProgress(metadata: metadata, data: data, progressCallback: progressCallback)
    }
    
    func downloadFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<FileDownloadChunk, Error> {
        return try await mock.downloadFile(fileId: fileId, path: path)
    }
    
    func deleteFile(fileId: String?, path: String?, recursive: Bool, force: Bool) async throws -> FileDeleteResult {
        return try await mock.deleteFile(fileId: fileId, path: path, recursive: recursive, force: force)
    }
    
    func createDirectory(path: String, createParents: Bool) async throws -> FileCreateDirectoryResult {
        return try await mock.createDirectory(path: path, createParents: createParents)
    }
    
    func getFileInfo(fileId: String?, path: String?, includeChunks: Bool) async throws -> FileInfoData {
        return try await mock.getFileInfo(fileId: fileId, path: path, includeChunks: includeChunks)
    }
    
    func getSyncStatus(path: String?) async throws -> FileSyncStatusResult {
        return try await mock.getSyncStatus(path: path)
    }
    
    func getLogs(limit: Int, levelFilter: String, moduleFilter: String, searchText: String, reverse: Bool) async throws -> LogListResult {
        return try await mock.getLogs(limit: limit, levelFilter: levelFilter, moduleFilter: moduleFilter, searchText: searchText, reverse: reverse)
    }
    
    func streamLogs(levelFilter: String, moduleFilter: String, follow: Bool, tail: Int) async throws -> AsyncThrowingStream<LogEntryData, Error> {
        return try await mock.streamLogs(levelFilter: levelFilter, moduleFilter: moduleFilter, follow: follow, tail: tail)
    }
    
    func clearLogs(clearAll: Bool, beforeTimestamp: Int64) async throws -> LogClearResult {
        return try await mock.clearLogs(clearAll: clearAll, beforeTimestamp: beforeTimestamp)
    }
    
    func exportLogs(format: LogExportFormat, levelFilter: String, moduleFilter: String) async throws -> LogExportResult {
        return try await mock.exportLogs(format: format, levelFilter: levelFilter, moduleFilter: moduleFilter)
    }
    
    func getLogStats(startTime: Int64, endTime: Int64) async throws -> LogStatsResult {
        return try await mock.getLogStats(startTime: startTime, endTime: endTime)
    }
    
    // MARK: - Encrypted File Operations
    
    func uploadEncryptedFile(metadata: FileUploadMetadata, encryptedData: Data, keyId: String) async throws -> FileUploadResult {
        return try await mock.uploadEncryptedFile(metadata: metadata, encryptedData: encryptedData, keyId: keyId)
    }
    
    func downloadEncryptedFile(fileId: String?, path: String?) async throws -> AsyncThrowingStream<EncryptedFileDownloadChunk, Error> {
        return try await mock.downloadEncryptedFile(fileId: fileId, path: path)
    }
    
    func rotateFileEncryptionKey(fileId: String, newKeyId: String) async throws -> FileKeyRotationResult {
        return try await mock.rotateFileEncryptionKey(fileId: fileId, newKeyId: newKeyId)
    }
}
//
//  UIDataAdapter.swift
//  librorum
//
//  Adapter layer between pure communication and UI models
//

import Foundation
import SwiftData
import Combine

/// Converts between pure communication data and UI models
@MainActor
class UIDataAdapter: ObservableObject {
    
    private let communicator: GRPCCommunicatorProtocol
    
    // UI-observable properties
    @Published var isConnected: Bool = false
    @Published var connectionStatus: String = "Disconnected"
    @Published var lastError: String?
    @Published var uploadProgress: Double = 0.0
    @Published var isUploading: Bool = false
    
    init(communicator: GRPCCommunicatorProtocol? = nil) {
        self.communicator = communicator ?? GRPCCommunicatorFactory.createCommunicator()
    }
    
    // MARK: - Connection Management
    
    func connect(to address: String) async throws {
        do {
            try await communicator.connect(address: address)
            await updateConnectionStatus()
        } catch {
            await handleError(error)
            throw error
        }
    }
    
    func disconnect() async throws {
        do {
            try await communicator.disconnect()
            await updateConnectionStatus()
        } catch {
            await handleError(error)
            throw error
        }
    }
    
    // MARK: - Data Conversion Methods
    
    func fetchNodesAsUIModels() async throws -> [NodeInfo] {
        let nodeData = try await communicator.getNodeList()
        return nodeData.map { convertToUIModel($0) }
    }
    
    func fetchSystemHealthAsUIModel() async throws -> SystemHealth {
        let healthData = try await communicator.getSystemHealth()
        return convertToUIModel(healthData)
    }
    
    func sendHeartbeat(nodeId: String) async throws -> HeartbeatResponse {
        let heartbeatResult = try await communicator.sendHeartbeat(nodeId: nodeId)
        return convertToUIModel(heartbeatResult)
    }
    
    func addNode(address: String) async throws {
        try await communicator.addNode(address: address)
    }
    
    func removeNode(nodeId: String) async throws {
        try await communicator.removeNode(nodeId: nodeId)
    }
    
    // MARK: - Private Conversion Methods
    
    private func convertToUIModel(_ nodeData: NodeData) -> NodeInfo {
        return NodeInfo(
            nodeId: nodeData.nodeId,
            address: nodeData.address,
            systemInfo: nodeData.systemInfo,
            status: convertNodeStatus(nodeData.status),
            lastHeartbeat: nodeData.lastHeartbeat,
            connectionCount: nodeData.connectionCount,
            latency: nodeData.latency,
            failureCount: nodeData.failureCount,
            isOnline: nodeData.isOnline,
            discoveredAt: nodeData.discoveredAt
        )
    }
    
    private func convertToUIModel(_ healthData: CommunicatorSystemHealthData) -> SystemHealth {
        return SystemHealth(
            timestamp: healthData.timestamp,
            backendStatus: .running, // Assume running if we got data
            totalNodes: 0, // This would come from node count
            onlineNodes: 0,
            offlineNodes: 0,
            totalStorage: healthData.totalStorage,
            usedStorage: healthData.usedStorage,
            availableStorage: healthData.availableStorage,
            totalFiles: healthData.totalFiles,
            totalChunks: healthData.totalChunks,
            networkLatency: healthData.networkLatency,
            errorCount: healthData.errorCount,
            lastError: nil,
            uptime: healthData.uptime,
            memoryUsage: healthData.memoryUsage,
            cpuUsage: healthData.cpuUsage
        )
    }
    
    private func convertToUIModel(_ heartbeatResult: HeartbeatResult) -> HeartbeatResponse {
        return HeartbeatResponse(
            nodeId: heartbeatResult.nodeId,
            address: heartbeatResult.address,
            systemInfo: heartbeatResult.systemInfo,
            timestamp: heartbeatResult.timestamp,
            status: heartbeatResult.status
        )
    }
    
    private func convertNodeStatus(_ status: CommunicatorNodeStatus) -> librorum.NodeStatus {
        switch status {
        case .online:
            return .online
        case .offline:
            return .offline
        case .connecting:
            return .connecting
        case .error:
            return .error
        case .unknown:
            return .offline
        }
    }
    
    // MARK: - Private Helper Methods
    
    private func updateConnectionStatus() async {
        let connected = await communicator.isConnected()
        await MainActor.run {
            self.isConnected = connected
            self.connectionStatus = connected ? "Connected" : "Disconnected"
            if connected {
                self.lastError = nil
            }
        }
    }
    
    private func handleError(_ error: Error) async {
        await MainActor.run {
            self.lastError = error.localizedDescription
            self.isConnected = false
            self.connectionStatus = "Error: \(error.localizedDescription)"
        }
    }
    
    // MARK: - File Operations
    
    func uploadFile(fileUrl: URL, toPath destinationPath: String) async throws {
        guard let fileData = try? Data(contentsOf: fileUrl) else {
            throw NSError(domain: "UIDataAdapter", code: -1, userInfo: [NSLocalizedDescriptionKey: "Could not read file data"])
        }
        
        await MainActor.run {
            self.isUploading = true
            self.uploadProgress = 0.0
        }
        
        let metadata = FileUploadMetadata(
            filename: fileUrl.lastPathComponent,
            path: destinationPath,
            fileType: fileUrl.hasDirectoryPath ? .directory : .file,
            size: Int64(fileData.count),
            permissions: "644",
            overwrite: false,
            createDirectories: true,
            isEncrypted: false,
            encryptionAlgorithm: nil,
            keyId: nil
        )
        
        do {
            let result = try await communicator.uploadFileWithProgress(
                metadata: metadata,
                data: fileData
            ) { [weak self] progress in
                Task { @MainActor in
                    self?.uploadProgress = progress.percentage
                }
            }
            
            await MainActor.run {
                self.isUploading = false
                self.uploadProgress = 0.0
            }
            
            if !result.success {
                throw NSError(domain: "UIDataAdapter", code: -2, userInfo: [NSLocalizedDescriptionKey: result.message])
            }
        } catch {
            await MainActor.run {
                self.isUploading = false
                self.uploadProgress = 0.0
            }
            throw error
        }
    }
}
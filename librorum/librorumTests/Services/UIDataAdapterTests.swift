//
//  UIDataAdapterTests.swift
//  librorumTests
//
//  Tests for UI data adapter layer
//

import Testing
import Foundation
@testable import librorum

@MainActor
struct UIDataAdapterTests {
    
    // MARK: - Mock Communicator for Testing
    
    class MockCommunicator: GRPCCommunicatorProtocol {
        var isConnectedState = false
        var shouldThrowError = false
        var mockNodes: [NodeData] = []
        var mockHealth: CommunicatorSystemHealthData = CommunicatorSystemHealthData()
        
        func connect(address: String) async throws {
            if shouldThrowError {
                throw GRPCError.connectionFailed("Mock error")
            }
            isConnectedState = true
        }
        
        func disconnect() async throws {
            if shouldThrowError {
                throw GRPCError.serverError("Mock disconnect error")
            }
            isConnectedState = false
        }
        
        func isConnected() async -> Bool {
            return isConnectedState
        }
        
        func sendHeartbeat(nodeId: String) async throws -> HeartbeatResult {
            if shouldThrowError {
                throw GRPCError.timeout
            }
            return HeartbeatResult(
                nodeId: nodeId,
                address: "127.0.0.1:50051",
                systemInfo: "Mock System",
                timestamp: Date(),
                status: true,
                latency: 0.025
            )
        }
        
        func getNodeList() async throws -> [NodeData] {
            if shouldThrowError {
                throw GRPCError.serverError("Mock server error")
            }
            return mockNodes
        }
        
        func getSystemHealth() async throws -> CommunicatorSystemHealthData {
            if shouldThrowError {
                throw GRPCError.notConnected
            }
            return mockHealth
        }
        
        func addNode(address: String) async throws {
            if shouldThrowError {
                throw GRPCError.invalidRequest("Mock validation error")
            }
            // Add to mock nodes
        }
        
        func removeNode(nodeId: String) async throws {
            if shouldThrowError {
                throw GRPCError.invalidRequest("Mock removal error")
            }
            // Remove from mock nodes
        }
    }
    
    // MARK: - Test Helper Methods
    
    private func createAdapter(shouldThrowError: Bool = false) -> UIDataAdapter {
        let mockCommunicator = MockCommunicator()
        mockCommunicator.shouldThrowError = shouldThrowError
        return UIDataAdapter(communicator: mockCommunicator)
    }
    
    // MARK: - Connection Tests
    
    @Test("UI adapter connection management")
    func testUIAdapterConnectionManagement() async throws {
        let adapter = createAdapter()
        
        // Initial state
        #expect(adapter.isConnected == false)
        #expect(adapter.connectionStatus == "Disconnected")
        
        // Connect
        try await adapter.connect(to: "127.0.0.1:50051")
        #expect(adapter.isConnected == true)
        #expect(adapter.connectionStatus == "Connected")
        #expect(adapter.lastError == nil)
        
        // Disconnect
        try await adapter.disconnect()
        #expect(adapter.isConnected == false)
        #expect(adapter.connectionStatus == "Disconnected")
    }
    
    @Test("UI adapter connection error handling")
    func testUIAdapterConnectionErrorHandling() async throws {
        let adapter = createAdapter(shouldThrowError: true)
        
        // Test connection error
        do {
            try await adapter.connect(to: "invalid:address")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(adapter.isConnected == false)
            #expect(adapter.lastError != nil)
            #expect(adapter.connectionStatus.contains("Error"))
        }
    }
    
    // MARK: - Data Conversion Tests
    
    @Test("UI adapter node data conversion")
    func testUIAdapterNodeDataConversion() async throws {
        let mockCommunicator = MockCommunicator()
        mockCommunicator.mockNodes = [
            NodeData(
                nodeId: "test.node.local",
                address: "192.168.1.100:50051",
                systemInfo: "Test System",
                status: .online,
                lastHeartbeat: Date(),
                connectionCount: 5,
                latency: 0.025,
                failureCount: 0,
                isOnline: true,
                discoveredAt: Date()
            )
        ]
        
        let adapter = UIDataAdapter(communicator: mockCommunicator)
        try await adapter.connect(to: "test:address")
        
        let uiNodes = try await adapter.fetchNodesAsUIModels()
        
        #expect(uiNodes.count == 1)
        let node = uiNodes[0]
        #expect(node.nodeId == "test.node.local")
        #expect(node.address == "192.168.1.100:50051")
        #expect(node.systemInfo == "Test System")
        #expect(node.status == .online)
        #expect(node.connectionCount == 5)
        #expect(node.latency == 0.025)
        #expect(node.isOnline == true)
    }
    
    @Test("UI adapter system health conversion")
    func testUIAdapterSystemHealthConversion() async throws {
        let mockCommunicator = MockCommunicator()
        mockCommunicator.mockHealth = CommunicatorSystemHealthData(
            totalStorage: 1000000000,
            usedStorage: 250000000,
            availableStorage: 750000000,
            totalFiles: 100,
            totalChunks: 500,
            networkLatency: 0.025,
            errorCount: 2,
            uptime: 3600,
            memoryUsage: 50000000,
            cpuUsage: 15.5
        )
        
        let adapter = UIDataAdapter(communicator: mockCommunicator)
        try await adapter.connect(to: "test:address")
        
        let uiHealth = try await adapter.fetchSystemHealthAsUIModel()
        
        #expect(uiHealth.totalStorage == 1000000000)
        #expect(uiHealth.usedStorage == 250000000)
        #expect(uiHealth.availableStorage == 750000000)
        #expect(uiHealth.totalFiles == 100)
        #expect(uiHealth.totalChunks == 500)
        #expect(uiHealth.networkLatency == 0.025)
        #expect(uiHealth.errorCount == 2)
        #expect(uiHealth.uptime == 3600)
        #expect(uiHealth.memoryUsage == 50000000)
        #expect(uiHealth.cpuUsage == 15.5)
        #expect(uiHealth.backendStatus == .running)
    }
    
    @Test("UI adapter heartbeat conversion")
    func testUIAdapterHeartbeatConversion() async throws {
        let adapter = createAdapter()
        try await adapter.connect(to: "test:address")
        
        let uiHeartbeat = try await adapter.sendHeartbeat(nodeId: "test.node")
        
        #expect(uiHeartbeat.nodeId == "test.node")
        #expect(uiHeartbeat.address == "127.0.0.1:50051")
        #expect(uiHeartbeat.systemInfo == "Mock System")
        #expect(uiHeartbeat.status == true)
    }
    
    // MARK: - Node Management Tests
    
    @Test("UI adapter node management operations")
    func testUIAdapterNodeManagementOperations() async throws {
        let adapter = createAdapter()
        try await adapter.connect(to: "test:address")
        
        // Test add node
        try await adapter.addNode(address: "192.168.1.200:50051")
        
        // Test remove node
        try await adapter.removeNode(nodeId: "test.node.id")
        
        // Should complete without errors
        #expect(true)
    }
    
    @Test("UI adapter node management errors")
    func testUIAdapterNodeManagementErrors() async throws {
        let adapter = createAdapter(shouldThrowError: true)
        try await adapter.connect(to: "test:address")
        
        // Test add node error
        do {
            try await adapter.addNode(address: "invalid")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        // Test remove node error
        do {
            try await adapter.removeNode(nodeId: "invalid")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
    }
    
    // MARK: - Error Propagation Tests
    
    @Test("UI adapter error propagation")
    func testUIAdapterErrorPropagation() async throws {
        let adapter = createAdapter(shouldThrowError: true)
        try await adapter.connect(to: "test:address")
        
        // Test that errors are properly propagated from communicator
        do {
            _ = try await adapter.sendHeartbeat(nodeId: "test")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        do {
            _ = try await adapter.fetchNodesAsUIModels()
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        do {
            _ = try await adapter.fetchSystemHealthAsUIModel()
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        // Verify error state is updated
        #expect(adapter.lastError != nil)
    }
    
    // MARK: - Observable Properties Tests
    
    @Test("UI adapter observable properties")
    func testUIAdapterObservableProperties() async throws {
        let adapter = createAdapter()
        
        // Test initial observable state
        #expect(adapter.isConnected == false)
        #expect(adapter.connectionStatus == "Disconnected")
        #expect(adapter.lastError == nil)
        
        // Test state changes during connection
        try await adapter.connect(to: "test:address")
        
        #expect(adapter.isConnected == true)
        #expect(adapter.connectionStatus == "Connected")
        #expect(adapter.lastError == nil)
        
        // Test state changes during disconnection
        try await adapter.disconnect()
        
        #expect(adapter.isConnected == false)
        #expect(adapter.connectionStatus == "Disconnected")
    }
    
    @Test("UI adapter observable error states")
    func testUIAdapterObservableErrorStates() async throws {
        let adapter = createAdapter(shouldThrowError: true)
        
        // Trigger an error
        do {
            try await adapter.connect(to: "test:address")
        } catch {
            // Expected to throw
        }
        
        // Verify error state is observable
        #expect(adapter.isConnected == false)
        #expect(adapter.connectionStatus.contains("Error"))
        #expect(adapter.lastError != nil)
        #expect(adapter.lastError!.contains("Mock error"))
    }
    
    // MARK: - Data Type Conversion Tests
    
    @Test("UI adapter node status conversion")
    func testUIAdapterNodeStatusConversion() async throws {
        let mockCommunicator = MockCommunicator()
        
        // Test all node status conversions
        let testStatuses: [(CommunicatorNodeStatus, librorum.NodeStatus)] = [
            (.online, .online),
            (.offline, .offline),
            (.connecting, .connecting),
            (.error, .error)
        ]
        
        for (communicatorStatus, expectedUIStatus) in testStatuses {
            mockCommunicator.mockNodes = [
                NodeData(
                    nodeId: "test",
                    address: "test:123",
                    systemInfo: "test",
                    status: communicatorStatus,
                    lastHeartbeat: Date(),
                    connectionCount: 0,
                    latency: 0,
                    failureCount: 0,
                    isOnline: communicatorStatus == .online,
                    discoveredAt: Date()
                )
            ]
            
            let adapter = UIDataAdapter(communicator: mockCommunicator)
            try await adapter.connect(to: "test:address")
            
            let uiNodes = try await adapter.fetchNodesAsUIModels()
            #expect(uiNodes.first?.status == expectedUIStatus)
        }
    }
    
    // MARK: - Concurrent Operations Tests
    
    @Test("UI adapter concurrent operations")
    func testUIAdapterConcurrentOperations() async throws {
        let adapter = createAdapter()
        try await adapter.connect(to: "test:address")
        
        await withTaskGroup(of: Bool.self) { group in
            // Multiple concurrent heartbeats
            for i in 0..<5 {
                group.addTask {
                    do {
                        let heartbeat = try await adapter.sendHeartbeat(nodeId: "concurrent.\(i)")
                        return await MainActor.run { heartbeat.status }
                    } catch {
                        return false
                    }
                }
            }
            
            // Concurrent health checks
            for _ in 0..<3 {
                group.addTask {
                    do {
                        let health = try await adapter.fetchSystemHealthAsUIModel()
                        return await MainActor.run { health.totalStorage >= 0 }
                    } catch {
                        return false
                    }
                }
            }
            
            var successCount = 0
            for await success in group {
                if success {
                    successCount += 1
                }
            }
            
            // All operations should succeed
            #expect(successCount == 8)
        }
    }
}
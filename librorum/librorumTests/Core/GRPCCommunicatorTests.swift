//
//  GRPCCommunicatorTests.swift
//  librorumTests
//
//  Pure gRPC communication tests - NO UI dependencies
//

import Testing
import Foundation
@testable import librorum

/// Pure communication layer tests - completely independent of UI/SwiftUI
struct GRPCCommunicatorTests {
    
    // MARK: - Test Configuration
    
    static let testAddress = "127.0.0.1:50051"
    static let invalidAddress = "invalid:99999"
    static let testTimeout: TimeInterval = 5.0
    
    // MARK: - Helper Methods
    
    private func createCommunicator() -> GRPCCommunicator {
        return GRPCCommunicator()
    }
    
    // MARK: - Connection Tests
    
    @Test("Pure gRPC connection establishment")
    func testConnectionEstablishment() async throws {
        let communicator = createCommunicator()
        
        // Initial state
        let initiallyConnected = await communicator.isConnected()
        #expect(initiallyConnected == false)
        
        // Connect
        try await communicator.connect(address: Self.testAddress)
        let connectedState = await communicator.isConnected()
        #expect(connectedState == true)
        
        // Disconnect
        try await communicator.disconnect()
        let disconnectedState = await communicator.isConnected()
        #expect(disconnectedState == false)
    }
    
    @Test("Pure gRPC connection failure handling")
    func testConnectionFailure() async throws {
        let communicator = createCommunicator()
        
        // Test invalid address
        do {
            try await communicator.connect(address: "")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        // Test malformed address
        do {
            try await communicator.connect(address: Self.invalidAddress)
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        // Should remain disconnected
        let isConnected = await communicator.isConnected()
        #expect(isConnected == false)
    }
    
    @Test("Pure gRPC concurrent connection handling")
    func testConcurrentConnections() async throws {
        await withTaskGroup(of: Bool.self) { group in
            // Create multiple communicators and connect concurrently
            for i in 0..<5 {
                group.addTask {
                    do {
                        let communicator = GRPCCommunicator()
                        let address = "127.0.0.1:5005\(i)" // Different ports
                        try await communicator.connect(address: address)
                        let isConnected = await communicator.isConnected()
                        try await communicator.disconnect()
                        return isConnected
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
            
            // All concurrent connections should succeed
            #expect(successCount == 5)
        }
    }
    
    // MARK: - Heartbeat Tests
    
    @Test("Pure gRPC heartbeat operation")
    func testHeartbeatOperation() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        let heartbeat = try await communicator.sendHeartbeat(nodeId: "test.node.local")
        
        #expect(heartbeat.nodeId == "test.node.local")
        #expect(heartbeat.address == Self.testAddress)
        #expect(!heartbeat.systemInfo.isEmpty)
        #expect(heartbeat.status == true)
        #expect(heartbeat.latency > 0)
        #expect(heartbeat.timestamp <= Date())
        
        try await communicator.disconnect()
    }
    
    @Test("Pure gRPC heartbeat without connection")
    func testHeartbeatWithoutConnection() async throws {
        let communicator = createCommunicator()
        
        do {
            _ = try await communicator.sendHeartbeat(nodeId: "test.node")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
    }
    
    @Test("Pure gRPC heartbeat with invalid node ID")
    func testHeartbeatWithInvalidNodeId() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        do {
            _ = try await communicator.sendHeartbeat(nodeId: "")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        try await communicator.disconnect()
    }
    
    @Test("Pure gRPC heartbeat performance measurement")
    func testHeartbeatPerformance() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        var latencies: [TimeInterval] = []
        
        // Measure latency over multiple heartbeats
        for i in 0..<10 {
            let startTime = Date()
            let heartbeat = try await communicator.sendHeartbeat(nodeId: "perf.test.\(i)")
            let totalLatency = Date().timeIntervalSince(startTime)
            
            latencies.append(totalLatency)
            
            // Verify heartbeat contains measured latency
            #expect(heartbeat.latency > 0)
            #expect(heartbeat.latency <= totalLatency)
        }
        
        let averageLatency = latencies.reduce(0, +) / Double(latencies.count)
        let maxLatency = latencies.max() ?? 0
        
        print("📊 Heartbeat Performance:")
        print("   Average: \(Int(averageLatency * 1000))ms")
        print("   Max: \(Int(maxLatency * 1000))ms")
        
        // Performance assertions
        #expect(averageLatency < 1.0) // Under 1 second
        #expect(maxLatency < 2.0)     // Max under 2 seconds
        
        try await communicator.disconnect()
    }
    
    // MARK: - Node Management Tests
    
    @Test("Pure gRPC node list retrieval")
    func testNodeListRetrieval() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        let nodes = try await communicator.getNodeList()
        
        // Validate node list structure
        #expect(nodes.count >= 0)
        
        for node in nodes {
            #expect(!node.nodeId.isEmpty)
            #expect(!node.address.isEmpty)
            #expect(!node.systemInfo.isEmpty)
            #expect(node.latency >= 0)
            #expect(node.connectionCount >= 0)
            #expect(node.failureCount >= 0)
        }
        
        try await communicator.disconnect()
    }
    
    @Test("Pure gRPC node management operations")
    func testNodeManagementOperations() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        let testNodeAddress = "192.168.1.200:50051"
        
        // Add node
        try await communicator.addNode(address: testNodeAddress)
        
        // Get updated node list
        let nodesAfterAdd = try await communicator.getNodeList()
        let addedNode = nodesAfterAdd.first { $0.address == testNodeAddress }
        
        if let addedNode = addedNode {
            #expect(addedNode.address == testNodeAddress)
            
            // Remove node
            try await communicator.removeNode(nodeId: addedNode.nodeId)
            
            // Verify removal (in real implementation)
            // let nodesAfterRemove = try await communicator.getNodeList()
            // let removedNode = nodesAfterRemove.first { $0.nodeId == addedNode.nodeId }
            // #expect(removedNode == nil)
        }
        
        try await communicator.disconnect()
    }
    
    @Test("Pure gRPC invalid node operations")
    func testInvalidNodeOperations() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        // Test invalid add node
        do {
            try await communicator.addNode(address: "invalid.address")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        // Test invalid remove node
        do {
            try await communicator.removeNode(nodeId: "")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
        
        try await communicator.disconnect()
    }
    
    // MARK: - System Health Tests
    
    @Test("Pure gRPC system health retrieval")
    func testSystemHealthRetrieval() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        let health = try await communicator.getSystemHealth()
        
        // Validate health data structure
        #expect(health.totalStorage >= 0)
        #expect(health.usedStorage >= 0)
        #expect(health.availableStorage >= 0)
        #expect(health.totalFiles >= 0)
        #expect(health.totalChunks >= 0)
        #expect(health.networkLatency >= 0)
        #expect(health.errorCount >= 0)
        #expect(health.uptime >= 0)
        #expect(health.memoryUsage >= 0)
        #expect(health.cpuUsage >= 0)
        #expect(health.timestamp <= Date())
        
        // Validate storage consistency
        #expect(health.usedStorage + health.availableStorage <= health.totalStorage)
        
        try await communicator.disconnect()
    }
    
    @Test("Pure gRPC system health without connection")
    func testSystemHealthWithoutConnection() async throws {
        let communicator = createCommunicator()
        
        do {
            _ = try await communicator.getSystemHealth()
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is GRPCError)
        }
    }
    
    // MARK: - Data Structure Tests
    
    @Test("Pure data structure encoding/decoding")
    func testDataStructureEncodingDecoding() async throws {
        let originalNode = NodeData(
            nodeId: "test.node.local",
            address: "127.0.0.1:50051",
            systemInfo: "Test System",
            status: .online,
            lastHeartbeat: Date(),
            connectionCount: 5,
            latency: 0.025,
            failureCount: 0,
            isOnline: true,
            discoveredAt: Date()
        )
        
        // Test JSON encoding/decoding
        let encoder = JSONEncoder()
        let decoder = JSONDecoder()
        
        let encoded = try encoder.encode(originalNode)
        let decoded = try decoder.decode(NodeData.self, from: encoded)
        
        #expect(decoded == originalNode)
    }
    
    @Test("Pure heartbeat result encoding/decoding")
    func testHeartbeatResultEncodingDecoding() async throws {
        let originalHeartbeat = HeartbeatResult(
            nodeId: "test.node",
            address: "127.0.0.1:50051",
            systemInfo: "Test",
            timestamp: Date(),
            status: true,
            latency: 0.05
        )
        
        let encoder = JSONEncoder()
        let decoder = JSONDecoder()
        
        let encoded = try encoder.encode(originalHeartbeat)
        let decoded = try decoder.decode(HeartbeatResult.self, from: encoded)
        
        #expect(decoded == originalHeartbeat)
    }
    
    @Test("Pure system health data encoding/decoding")
    @MainActor
    func testSystemHealthDataEncodingDecoding() async throws {
        let originalHealth = CommunicatorSystemHealthData(
            totalStorage: 1000000000,
            usedStorage: 250000000,
            availableStorage: 750000000,
            totalFiles: 100,
            totalChunks: 500,
            networkLatency: 0.025,
            errorCount: 2,
            uptime: 3600,
            memoryUsage: 50000000,
            cpuUsage: 15.5,
            timestamp: Date()
        )
        
        let encoder = JSONEncoder()
        let decoder = JSONDecoder()
        
        let encoded = try encoder.encode(originalHealth)
        let decoded = try decoder.decode(CommunicatorSystemHealthData.self, from: encoded)
        
        #expect(decoded == originalHealth)
    }
    
    // MARK: - Error Handling Tests
    
    @Test("Pure gRPC error handling")
    func testErrorHandling() async throws {
        let communicator = createCommunicator()
        
        // Test all operations without connection
        let operationsToTest: [() async throws -> Void] = [
            { _ = try await communicator.sendHeartbeat(nodeId: "test") },
            { _ = try await communicator.getNodeList() },
            { _ = try await communicator.getSystemHealth() },
            { try await communicator.addNode(address: "test:123") },
            { try await communicator.removeNode(nodeId: "test") },
            { try await communicator.disconnect() }
        ]
        
        for operation in operationsToTest {
            do {
                try await operation()
                // Some operations might not throw (like disconnect)
            } catch let error as GRPCError {
                // Verify we get proper GRPCError types
                #expect(error == .notConnected)
            } catch {
                #expect(Bool(false), "Unexpected error type: \(error)")
            }
        }
    }
    
    // MARK: - Concurrency and Thread Safety Tests
    
    @Test("Pure gRPC concurrent operations")
    func testConcurrentOperations() async throws {
        let communicator = createCommunicator()
        try await communicator.connect(address: Self.testAddress)
        
        await withTaskGroup(of: Bool.self) { group in
            // Multiple concurrent heartbeats
            for i in 0..<10 {
                group.addTask {
                    do {
                        let heartbeat = try await communicator.sendHeartbeat(nodeId: "concurrent.\(i)")
                        return heartbeat.status
                    } catch {
                        return false
                    }
                }
            }
            
            // Concurrent health checks
            for _ in 0..<5 {
                group.addTask {
                    do {
                        let health = try await communicator.getSystemHealth()
                        return health.totalStorage >= 0
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
            
            // Most operations should succeed
            #expect(successCount >= 12) // At least 80% success rate
        }
        
        try await communicator.disconnect()
    }
    
    // MARK: - Address Validation Tests
    
    @Test("Pure gRPC address validation")
    func testAddressValidation() async throws {
        let communicator = createCommunicator()
        
        // Valid addresses
        let validAddresses = [
            "127.0.0.1:50051",
            "192.168.1.1:8080",
            "localhost:3000",
            "example.com:443"
        ]
        
        for address in validAddresses {
            try await communicator.connect(address: address)
            let isConnected = await communicator.isConnected()
            #expect(isConnected == true)
            try await communicator.disconnect()
        }
        
        // Invalid addresses
        let invalidAddresses = [
            "",
            "invalid",
            "127.0.0.1",
            "127.0.0.1:",
            ":50051",
            "256.256.256.256:50051",
            "127.0.0.1:99999"
        ]
        
        for address in invalidAddresses {
            do {
                try await communicator.connect(address: address)
                #expect(Bool(false), "Should have thrown an error for address: \(address)")
            } catch {
                #expect(error is GRPCError)
            }
        }
    }
}
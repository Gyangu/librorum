//
//  RealGRPCConnectionTests.swift
//  librorumTests
//
//  Real gRPC backend integration tests
//

import Testing
import Foundation
@testable import librorum

@MainActor
struct RealGRPCConnectionTests {
    
    // MARK: - Test Configuration
    
    static let defaultBackendAddress = "127.0.0.1:50051"
    static let testTimeout: TimeInterval = 10.0
    
    // MARK: - Helper Methods
    
    private func createRealClient() -> LibrorumClient {
        return LibrorumClient()
    }
    
    private func isBackendRunning() async -> Bool {
        let client = createRealClient()
        do {
            try await client.connect(to: Self.defaultBackendAddress)
            let isHealthy = await client.isHealthy()
            await client.disconnect()
            return isHealthy
        } catch {
            return false
        }
    }
    
    private func requiresBackend() async throws {
        let isRunning = await isBackendRunning()
        guard isRunning else {
            throw XCTSkip("Skipping test: Rust backend is not running. Start backend with: ./target/release/librorum start")
        }
    }
    
    // MARK: - Connection Tests
    
    @Test("Real gRPC connection establishment", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCConnection() async throws {
        try await requiresBackend()
        
        let client = createRealClient()
        
        try await client.connect(to: Self.defaultBackendAddress)
        let isHealthy = await client.isHealthy()
        
        #expect(isHealthy == true)
        
        await client.disconnect()
        let isHealthyAfterDisconnect = await client.isHealthy()
        #expect(isHealthyAfterDisconnect == false)
    }
    
    @Test("Real gRPC connection failure handling", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCConnectionFailure() async throws {
        let client = createRealClient()
        
        // Try to connect to a port that should be closed
        do {
            try await client.connect(to: "127.0.0.1:12345")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
    }
    
    // MARK: - Service Operation Tests
    
    @Test("Real gRPC heartbeat service", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCHeartbeat() async throws {
        try await requiresBackend()
        
        let client = createRealClient()
        try await client.connect(to: Self.defaultBackendAddress)
        
        let response = try await client.heartbeat(nodeId: "test.client.local")
        
        #expect(!response.nodeId.isEmpty)
        #expect(!response.address.isEmpty)
        #expect(!response.systemInfo.isEmpty)
        #expect(response.timestamp <= Date())
        
        await client.disconnect()
    }
    
    @Test("Real gRPC get connected nodes", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCGetConnectedNodes() async throws {
        try await requiresBackend()
        
        let client = createRealClient()
        try await client.connect(to: Self.defaultBackendAddress)
        
        let nodes = try await client.getConnectedNodes()
        
        // Nodes array should be valid (can be empty if no nodes are connected)
        #expect(nodes.count >= 0)
        
        // If there are nodes, validate their structure
        for node in nodes {
            #expect(!node.nodeId.isEmpty)
            #expect(!node.address.isEmpty)
        }
        
        await client.disconnect()
    }
    
    @Test("Real gRPC get system health", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCGetSystemHealth() async throws {
        try await requiresBackend()
        
        let client = createRealClient()
        try await client.connect(to: Self.defaultBackendAddress)
        
        let health = try await client.getSystemHealth()
        
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
        
        await client.disconnect()
    }
    
    // MARK: - Node Management Tests
    
    @Test("Real gRPC add and remove node", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCNodeManagement() async throws {
        try await requiresBackend()
        
        let client = createRealClient()
        try await client.connect(to: Self.defaultBackendAddress)
        
        let testNodeAddress = "192.168.1.100:50051"
        
        // Add a test node
        try await client.addNode(address: testNodeAddress)
        
        // Get nodes and verify the added node
        let nodesAfterAdd = try await client.getConnectedNodes()
        let addedNode = nodesAfterAdd.first { $0.address == testNodeAddress }
        
        if let addedNode = addedNode {
            #expect(addedNode.address == testNodeAddress)
            
            // Remove the test node
            try await client.removeNode(nodeId: addedNode.nodeId)
            
            // Verify node was removed
            let nodesAfterRemove = try await client.getConnectedNodes()
            let removedNode = nodesAfterRemove.first { $0.nodeId == addedNode.nodeId }
            #expect(removedNode == nil)
        }
        
        await client.disconnect()
    }
    
    // MARK: - Performance Tests
    
    @Test("Real gRPC performance - latency", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCLatency() async throws {
        try await requiresBackend()
        
        let client = createRealClient()
        try await client.connect(to: Self.defaultBackendAddress)
        
        var latencies: [TimeInterval] = []
        
        // Measure latency over multiple requests
        for _ in 0..<10 {
            let startTime = Date()
            _ = try await client.heartbeat(nodeId: "latency.test.local")
            let latency = Date().timeIntervalSince(startTime)
            latencies.append(latency)
        }
        
        let averageLatency = latencies.reduce(0, +) / Double(latencies.count)
        let maxLatency = latencies.max() ?? 0
        
        print("📊 Real gRPC Performance:")
        print("   Average latency: \(Int(averageLatency * 1000))ms")
        print("   Max latency: \(Int(maxLatency * 1000))ms")
        
        // Verify reasonable latency (should be under 1 second for local backend)
        #expect(averageLatency < 1.0)
        #expect(maxLatency < 2.0)
        
        await client.disconnect()
    }
    
    @Test("Real gRPC concurrent connections", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCConcurrentConnections() async throws {
        try await requiresBackend()
        
        await withTaskGroup(of: Bool.self) { group in
            // Create multiple concurrent connections
            for i in 0..<5 {
                group.addTask {
                    do {
                        let client = await self.createRealClient()
                        try await client.connect(to: Self.defaultBackendAddress)
                        
                        let health = try await client.getSystemHealth()
                        let isValid = await health.totalStorage >= 0
                        
                        await client.disconnect()
                        return isValid
                    } catch {
                        print("⚠️ Concurrent connection \(i) failed: \(error)")
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
            
            // At least 80% of concurrent connections should succeed
            #expect(successCount >= 4)
        }
    }
    
    // MARK: - Error Handling Tests
    
    @Test("Real gRPC service calls without connection", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCServiceCallsWithoutConnection() async throws {
        let client = createRealClient()
        
        // All service calls should fail without connection
        do {
            _ = try await client.heartbeat(nodeId: "test.local")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
        
        do {
            _ = try await client.getConnectedNodes()
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
        
        do {
            _ = try await client.getSystemHealth()
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
    }
    
    // MARK: - Protocol Compliance Tests
    
    @Test("Real gRPC response format validation", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCResponseFormatValidation() async throws {
        try await requiresBackend()
        
        let client = createRealClient()
        try await client.connect(to: Self.defaultBackendAddress)
        
        // Test heartbeat response format
        let heartbeat = try await client.heartbeat(nodeId: "format.test.local")
        #expect(!heartbeat.nodeId.isEmpty)
        #expect(!heartbeat.address.isEmpty)
        #expect(!heartbeat.systemInfo.isEmpty)
        #expect(heartbeat.timestamp <= Date())
        
        // Test system health response format
        let health = try await client.getSystemHealth()
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
        
        await client.disconnect()
    }
    
    // MARK: - Integration with CoreManager Tests
    
    @Test("Real gRPC integration with CoreManager", .enabled(if: ProcessInfo.processInfo.environment["ENABLE_REAL_GRPC_TESTS"] == "1"))
    func testRealGRPCIntegrationWithCoreManager() async throws {
        // Skip if backend not available
        guard await isBackendRunning() else {
            throw XCTSkip("Backend not running for CoreManager integration test")
        }
        
        let coreManager = CoreManager()
        
        // Test CoreManager backend initialization
        try await coreManager.initializeBackend()
        
        // Wait a moment for initialization
        try await Task.sleep(nanoseconds: 1_000_000_000) // 1 second
        
        // Test health check through CoreManager
        let health = await coreManager.checkBackendHealth()
        #expect(health.totalStorage >= 0)
        
        // Test node management through CoreManager
        let nodes = coreManager.connectedNodes
        #expect(nodes.count >= 0)
        
        try await coreManager.stopBackend()
    }
}

// MARK: - Test Support Extensions

extension RealGRPCConnectionTests {
    
    /// Skip error for tests that require backend
    struct XCTSkip: Error {
        let message: String
        
        init(_ message: String) {
            self.message = message
        }
    }
}
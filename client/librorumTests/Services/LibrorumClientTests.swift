//
//  LibrorumClientTests.swift
//  librorumTests
//
//  Service layer tests for LibrorumClient
//

import Testing
import Foundation
@testable import librorum

@MainActor
struct LibrorumClientTests {
    
    // MARK: - Mock Client for Testing
    
    @MainActor
    class MockLibrorumClient: LibrorumClient {
        var shouldFailConnection = false
        var shouldFailHealthCheck = false
        var mockSystemHealth: SystemHealthData?
        var mockNodes: [NodeInfo] = []
        var connectionCallCount = 0
        var healthCheckCallCount = 0
        
        override func connect(to address: String) async throws {
            connectionCallCount += 1
            if shouldFailConnection {
                throw LibrorumClientError.connectionFailed("Mock connection failure")
            }
            // Simulate connection delay
            try await Task.sleep(nanoseconds: 10_000_000) // 10ms
        }
        
        override func isHealthy() async -> Bool {
            healthCheckCallCount += 1
            return !shouldFailHealthCheck
        }
        
        override func getSystemHealth() async throws -> SystemHealthData {
            if let mockHealth = mockSystemHealth {
                return mockHealth
            }
            return SystemHealthData(
                totalStorage: 1000000000,
                usedStorage: 250000000,
                availableStorage: 750000000,
                totalFiles: 100,
                totalChunks: 500,
                networkLatency: 0.05,
                errorCount: 0,
                uptime: 3600,
                memoryUsage: 50000000,
                cpuUsage: 15.5
            )
        }
        
        override func getConnectedNodes() async throws -> [NodeInfo] {
            return mockNodes
        }
    }
    
    // MARK: - Basic Connection Tests
    
    @Test("LibrorumClient successful connection")
    func testLibrorumClientSuccessfulConnection() async throws {
        let client = MockLibrorumClient()
        
        try await client.connect(to: "localhost:50051")
        
        #expect(client.connectionCallCount == 1)
        #expect(await client.isHealthy() == true)
    }
    
    @Test("LibrorumClient connection failure")
    func testLibrorumClientConnectionFailure() async throws {
        let client = MockLibrorumClient()
        client.shouldFailConnection = true
        
        do {
            try await client.connect(to: "invalid:9999")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
        
        #expect(client.connectionCallCount == 1)
    }
    
    @Test("LibrorumClient health check")
    func testLibrorumClientHealthCheck() async throws {
        let client = MockLibrorumClient()
        
        let isHealthy = await client.isHealthy()
        
        #expect(isHealthy == true)
        #expect(client.healthCheckCallCount == 1)
    }
    
    @Test("LibrorumClient unhealthy state")
    func testLibrorumClientUnhealthyState() async throws {
        let client = MockLibrorumClient()
        client.shouldFailHealthCheck = true
        
        let isHealthy = await client.isHealthy()
        
        #expect(isHealthy == false)
        #expect(client.healthCheckCallCount == 1)
    }
    
    // MARK: - System Health Tests
    
    @Test("LibrorumClient get system health")
    func testLibrorumClientGetSystemHealth() async throws {
        let client = MockLibrorumClient()
        let mockHealth = SystemHealthData(
            totalStorage: 2000000000,
            usedStorage: 800000000,
            availableStorage: 1200000000,
            totalFiles: 200,
            totalChunks: 1000,
            networkLatency: 0.025,
            errorCount: 2,
            uptime: 7200,
            memoryUsage: 100000000,
            cpuUsage: 25.0
        )
        client.mockSystemHealth = mockHealth
        
        let health = try await client.getSystemHealth()
        
        #expect(health.totalStorage == 2000000000)
        #expect(health.usedStorage == 800000000)
        #expect(health.availableStorage == 1200000000)
        #expect(health.totalFiles == 200)
        #expect(health.totalChunks == 1000)
        #expect(health.networkLatency == 0.025)
        #expect(health.errorCount == 2)
        #expect(health.uptime == 7200)
        #expect(health.memoryUsage == 100000000)
        #expect(health.cpuUsage == 25.0)
    }
    
    @Test("LibrorumClient system health without connection")
    func testLibrorumClientSystemHealthWithoutConnection() async throws {
        let client = LibrorumClient() // Real client, not mock
        
        do {
            _ = try await client.getSystemHealth()
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
    }
    
    // MARK: - Node Management Tests
    
    @Test("LibrorumClient get connected nodes")
    func testLibrorumClientGetConnectedNodes() async throws {
        let client = MockLibrorumClient()
        let mockNodes = [
            NodeInfo(
                nodeId: "node1.librorum.local",
                address: "192.168.1.100:50051",
                systemInfo: "macOS 14.0",
                status: .online,
                isOnline: true
            ),
            NodeInfo(
                nodeId: "node2.librorum.local",
                address: "192.168.1.101:50051",
                systemInfo: "Ubuntu 22.04",
                status: .offline,
                isOnline: false
            )
        ]
        client.mockNodes = mockNodes
        
        let nodes = try await client.getConnectedNodes()
        
        #expect(nodes.count == 2)
        #expect(nodes[0].nodeId == "node1.librorum.local")
        #expect(nodes[0].status == .online)
        #expect(nodes[1].nodeId == "node2.librorum.local")
        #expect(nodes[1].status == .offline)
    }
    
    @Test("LibrorumClient add node")
    func testLibrorumClientAddNode() async throws {
        let client = MockLibrorumClient()
        
        // Should not throw
        try await client.addNode(address: "192.168.1.102:50051")
    }
    
    @Test("LibrorumClient remove node")
    func testLibrorumClientRemoveNode() async throws {
        let client = MockLibrorumClient()
        
        // Should not throw
        try await client.removeNode(nodeId: "test.node.local")
    }
    
    @Test("LibrorumClient heartbeat")
    func testLibrorumClientHeartbeat() async throws {
        let client = MockLibrorumClient()
        
        let response = try await client.heartbeat(nodeId: "test.local")
        
        #expect(response.nodeId == "test.local")
        #expect(response.status == true)
        #expect(!response.systemInfo.isEmpty)
    }
    
    // MARK: - Error Handling Tests
    
    @Test("LibrorumClient error types")
    func testLibrorumClientErrorTypes() async throws {
        let notConnectedError = LibrorumClientError.notConnected
        let connectionFailedError = LibrorumClientError.connectionFailed("Test failure")
        let requestFailedError = LibrorumClientError.requestFailed("Request failed")
        let invalidResponseError = LibrorumClientError.invalidResponse
        
        #expect(notConnectedError.errorDescription == "gRPC client is not connected to server")
        #expect(connectionFailedError.errorDescription == "Connection failed: Test failure")
        #expect(requestFailedError.errorDescription == "Request failed: Request failed")
        #expect(invalidResponseError.errorDescription == "Invalid response from server")
    }
    
    // MARK: - Connection State Tests
    
    @Test("LibrorumClient connection state management")
    func testLibrorumClientConnectionStateManagement() async throws {
        let client = LibrorumClient()
        
        // Initially not connected
        do {
            _ = try await client.getSystemHealth()
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
        
        // After connection attempt (will fail in real client without backend)
        do {
            try await client.connect(to: "localhost:50051")
            #expect(Bool(false), "Should have thrown an error")
        } catch {
            #expect(error is LibrorumClientError)
        }
    }
    
    // MARK: - Concurrent Operations Tests
    
    @Test("LibrorumClient concurrent health checks")
    func testLibrorumClientConcurrentHealthChecks() async throws {
        let client = MockLibrorumClient()
        
        // Perform multiple concurrent health checks
        await withTaskGroup(of: Bool.self) { group in
            for _ in 0..<10 {
                group.addTask {
                    await client.isHealthy()
                }
            }
            
            var results: [Bool] = []
            for await result in group {
                results.append(result)
            }
            
            #expect(results.count == 10)
            #expect(results.allSatisfy { $0 == true })
        }
        
        #expect(client.healthCheckCallCount == 10)
    }
    
    @Test("LibrorumClient concurrent connections")
    func testLibrorumClientConcurrentConnections() async throws {
        let client = MockLibrorumClient()
        
        // Perform multiple concurrent connections
        await withTaskGroup(of: Void.self) { group in
            for i in 0..<5 {
                group.addTask {
                    try? await client.connect(to: "localhost:5005\(i)")
                }
            }
            
            await group.waitForAll()
        }
        
        #expect(client.connectionCallCount == 5)
    }
    
    // MARK: - Data Structure Tests
    
    @Test("SystemHealthData structure")
    func testSystemHealthDataStructure() async throws {
        let healthData = SystemHealthData(
            totalStorage: 1000,
            usedStorage: 500,
            availableStorage: 500,
            totalFiles: 10,
            totalChunks: 50,
            networkLatency: 0.1,
            errorCount: 1,
            uptime: 1000,
            memoryUsage: 256000000,
            cpuUsage: 50.0
        )
        
        #expect(healthData.totalStorage == 1000)
        #expect(healthData.usedStorage == 500)
        #expect(healthData.availableStorage == 500)
        #expect(healthData.totalFiles == 10)
        #expect(healthData.totalChunks == 50)
        #expect(healthData.networkLatency == 0.1)
        #expect(healthData.errorCount == 1)
        #expect(healthData.uptime == 1000)
        #expect(healthData.memoryUsage == 256000000)
        #expect(healthData.cpuUsage == 50.0)
    }
    
    @Test("HeartbeatResponse structure")
    func testHeartbeatResponseStructure() async throws {
        let now = Date()
        let response = HeartbeatResponse(
            nodeId: "test.response.local",
            address: "127.0.0.1:8080",
            systemInfo: "Test System",
            timestamp: now,
            status: true
        )
        
        #expect(response.nodeId == "test.response.local")
        #expect(response.address == "127.0.0.1:8080")
        #expect(response.systemInfo == "Test System")
        #expect(response.timestamp == now)
        #expect(response.status == true)
    }
    
    // MARK: - Performance Tests
    
    @Test("LibrorumClient connection performance")
    func testLibrorumClientConnectionPerformance() async throws {
        let client = MockLibrorumClient()
        
        let startTime = Date()
        try await client.connect(to: "localhost:50051")
        let endTime = Date()
        
        let duration = endTime.timeIntervalSince(startTime)
        #expect(duration < 1.0) // Should complete within 1 second
    }
    
    @Test("LibrorumClient health check performance")
    func testLibrorumClientHealthCheckPerformance() async throws {
        let client = MockLibrorumClient()
        
        let startTime = Date()
        _ = await client.isHealthy()
        let endTime = Date()
        
        let duration = endTime.timeIntervalSince(startTime)
        #expect(duration < 0.1) // Should complete within 100ms
    }
}
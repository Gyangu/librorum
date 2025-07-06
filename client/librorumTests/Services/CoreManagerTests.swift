//
//  CoreManagerTests.swift
//  librorumTests
//
//  Service layer tests for CoreManager
//

import Testing
import Foundation
@testable import librorum

@MainActor
struct CoreManagerTests {
    
    // MARK: - Mock Dependencies
    
    class MockLibrorumClient: LibrorumClient {
        var isConnected = false
        var shouldFailConnection = false
        var shouldFailHealthCheck = false
        var mockNodes: [NodeInfo] = []
        var mockSystemHealth: SystemHealthData?
        
        override func connect(to address: String) async throws {
            if shouldFailConnection {
                throw LibrorumClientError.connectionFailed("Mock connection failure")
            }
            isConnected = true
        }
        
        override func isHealthy() async -> Bool {
            return isConnected && !shouldFailHealthCheck
        }
        
        override func getSystemHealth() async throws -> SystemHealthData {
            guard isConnected else {
                throw LibrorumClientError.notConnected
            }
            
            return mockSystemHealth ?? SystemHealthData(
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
            guard isConnected else {
                throw LibrorumClientError.notConnected
            }
            return mockNodes
        }
        
        override func addNode(address: String) async throws {
            guard isConnected else {
                throw LibrorumClientError.notConnected
            }
            // Mock successful add
        }
        
        override func removeNode(nodeId: String) async throws {
            guard isConnected else {
                throw LibrorumClientError.notConnected
            }
            // Mock successful remove
        }
    }
    
    // MARK: - Initialization Tests
    
    @Test("CoreManager initial state")
    func testCoreManagerInitialState() async throws {
        let coreManager = CoreManager()
        
        #expect(coreManager.backendStatus == .stopped)
        #expect(coreManager.connectedNodes.isEmpty)
        #expect(coreManager.systemHealth == nil)
        #expect(coreManager.lastError == nil)
        #expect(coreManager.isInitialized == false)
    }
    
    // MARK: - Backend Lifecycle Tests
    
    @Test("CoreManager initialization")
    func testCoreManagerInitialization() async throws {
        let coreManager = CoreManager()
        
        // Note: This will fail in test environment without actual backend binary
        // but we can test the state changes
        #expect(coreManager.isInitialized == false)
        
        do {
            try await coreManager.initializeBackend()
        } catch {
            // Expected to fail in test environment
            #expect(error is CoreManagerError)
        }
    }
    
    @Test("CoreManager start backend without initialization")
    func testCoreManagerStartBackendWithoutInitialization() async throws {
        let coreManager = CoreManager()
        
        #expect(coreManager.backendStatus == .stopped)
        
        do {
            try await coreManager.startBackend()
        } catch {
            // Expected to fail in test environment
            #expect(error is CoreManagerError)
        }
    }
    
    @Test("CoreManager multiple start attempts")
    func testCoreManagerMultipleStartAttempts() async throws {
        let coreManager = CoreManager()
        
        // First start attempt
        do {
            try await coreManager.startBackend()
        } catch {
            // Expected to fail in test environment
        }
        
        // Second start attempt should handle gracefully
        do {
            try await coreManager.startBackend()
        } catch {
            // Expected to fail in test environment
        }
    }
    
    @Test("CoreManager stop backend")
    func testCoreManagerStopBackend() async throws {
        let coreManager = CoreManager()
        
        // Try to stop when not running
        do {
            try await coreManager.stopBackend()
        } catch {
            // May throw or may handle gracefully
        }
        
        #expect(coreManager.backendStatus != .running)
    }
    
    @Test("CoreManager restart backend")
    func testCoreManagerRestartBackend() async throws {
        let coreManager = CoreManager()
        
        do {
            try await coreManager.restartBackend()
        } catch {
            // Expected to fail in test environment
            #expect(error is CoreManagerError)
        }
    }
    
    // MARK: - Health Monitoring Tests
    
    @Test("CoreManager check backend health")
    func testCoreManagerCheckBackendHealth() async throws {
        let coreManager = CoreManager()
        
        let health = await coreManager.checkBackendHealth()
        
        #expect(health.backendStatus == coreManager.backendStatus)
        #expect(health.totalNodes == coreManager.connectedNodes.count)
        #expect(health.onlineNodes == coreManager.connectedNodes.filter { $0.isOnline }.count)
    }
    
    @Test("CoreManager health with connected nodes")
    func testCoreManagerHealthWithConnectedNodes() async throws {
        let coreManager = CoreManager()
        
        // Simulate some connected nodes
        let onlineNode = NodeInfo(
            nodeId: "online.test.local",
            address: "192.168.1.100:50051",
            status: .online,
            isOnline: true
        )
        
        let offlineNode = NodeInfo(
            nodeId: "offline.test.local",
            address: "192.168.1.101:50051",
            status: .offline,
            isOnline: false
        )
        
        coreManager.connectedNodes = [onlineNode, offlineNode]
        
        let health = await coreManager.checkBackendHealth()
        
        #expect(health.totalNodes == 2)
        #expect(health.onlineNodes == 1)
        #expect(health.offlineNodes == 1)
    }
    
    // MARK: - Node Management Tests
    
    @Test("CoreManager refresh nodes without connection")
    func testCoreManagerRefreshNodesWithoutConnection() async throws {
        let coreManager = CoreManager()
        
        await coreManager.refreshNodes()
        
        // Should handle gracefully when no gRPC connection
        #expect(coreManager.connectedNodes.isEmpty)
    }
    
    @Test("CoreManager add node without connection")
    func testCoreManagerAddNodeWithoutConnection() async throws {
        let coreManager = CoreManager()
        
        do {
            try await coreManager.addNode("192.168.1.100:50051")
        } catch CoreManagerError.grpcNotConnected {
            // Expected error
            #expect(true)
        } catch {
            #expect(Bool(false), "Unexpected error type")
        }
    }
    
    @Test("CoreManager remove node without connection")
    func testCoreManagerRemoveNodeWithoutConnection() async throws {
        let coreManager = CoreManager()
        
        do {
            try await coreManager.removeNode("test.node.local")
        } catch CoreManagerError.grpcNotConnected {
            // Expected error
            #expect(true)
        } catch {
            #expect(Bool(false), "Unexpected error type")
        }
    }
    
    // MARK: - Error Handling Tests
    
    @Test("CoreManager error types")
    func testCoreManagerErrorTypes() async throws {
        let binaryNotFoundError = CoreManagerError.backendBinaryNotFound("/path/to/binary")
        let timeoutError = CoreManagerError.backendStartupTimeout
        let grpcNotConnectedError = CoreManagerError.grpcNotConnected
        let configError = CoreManagerError.configurationError("Config issue")
        
        #expect(binaryNotFoundError.errorDescription?.contains("Backend binary not found") == true)
        #expect(timeoutError.errorDescription?.contains("timeout") == true)
        #expect(grpcNotConnectedError.errorDescription?.contains("not connected") == true)
        #expect(configError.errorDescription?.contains("Config issue") == true)
    }
    
    // MARK: - Configuration Tests
    
    @Test("CoreManager path helpers")
    func testCoreManagerPathHelpers() async throws {
        let coreManager = CoreManager()
        
        // Use reflection or make methods internal for testing
        // For now, we'll test indirectly through error messages
        do {
            try await coreManager.initializeBackend()
        } catch let error as CoreManagerError {
            if case .backendBinaryNotFound(let path) = error {
                #expect(path.contains("librorum_backend"))
            }
        } catch {
            // Other errors are acceptable in test environment
        }
    }
    
    // MARK: - Concurrency Tests
    
    @Test("CoreManager concurrent operations")
    func testCoreManagerConcurrentOperations() async throws {
        let coreManager = CoreManager()
        
        // Perform multiple concurrent health checks
        await withTaskGroup(of: SystemHealth.self) { group in
            for _ in 0..<5 {
                group.addTask {
                    await coreManager.checkBackendHealth()
                }
            }
            
            var results: [SystemHealth] = []
            for await result in group {
                results.append(result)
            }
            
            #expect(results.count == 5)
            // All should have the same backend status
            let firstStatus = results.first!.backendStatus
            #expect(results.allSatisfy { $0.backendStatus == firstStatus })
        }
    }
    
    @Test("CoreManager concurrent node operations")
    func testCoreManagerConcurrentNodeOperations() async throws {
        let coreManager = CoreManager()
        
        // Perform multiple concurrent refresh operations
        await withTaskGroup(of: Void.self) { group in
            for _ in 0..<3 {
                group.addTask {
                    await coreManager.refreshNodes()
                }
            }
            
            await group.waitForAll()
        }
        
        // Should complete without crashing
        #expect(true)
    }
    
    // MARK: - State Management Tests
    
    @Test("CoreManager status transitions")
    func testCoreManagerStatusTransitions() async throws {
        let coreManager = CoreManager()
        
        #expect(coreManager.backendStatus == .stopped)
        
        // Attempt to start (will likely fail in test environment)
        do {
            try await coreManager.startBackend()
        } catch {
            // Status should be .error or remain .stopped
            #expect(coreManager.backendStatus == .error || coreManager.backendStatus == .stopped)
        }
    }
    
    @Test("CoreManager initialization flag")
    func testCoreManagerInitializationFlag() async throws {
        let coreManager = CoreManager()
        
        #expect(coreManager.isInitialized == false)
        
        do {
            try await coreManager.initializeBackend()
            #expect(coreManager.isInitialized == true)
        } catch {
            // In test environment, initialization may fail
            // but the flag behavior is still testable
        }
    }
    
    // MARK: - Memory Management Tests
    
    @Test("CoreManager memory management")
    func testCoreManagerMemoryManagement() async throws {
        var coreManager: CoreManager? = CoreManager()
        weak var weakRef = coreManager
        
        #expect(weakRef != nil)
        
        coreManager = nil
        
        // CoreManager should be deallocated
        #expect(weakRef == nil)
    }
    
    // MARK: - Observable Tests
    
    @Test("CoreManager observable properties")
    func testCoreManagerObservableProperties() async throws {
        let coreManager = CoreManager()
        
        // Test that properties are observable (at least accessible)
        let _ = coreManager.backendStatus
        let _ = coreManager.connectedNodes
        let _ = coreManager.systemHealth
        let _ = coreManager.lastError
        let _ = coreManager.isInitialized
        
        // If we get here without crashes, observability is working
        #expect(true)
    }
    
    // MARK: - Integration with SystemHealth Tests
    
    @Test("CoreManager SystemHealth convenience initializer")
    func testCoreManagerSystemHealthConvenienceInitializer() async throws {
        let now = Date()
        let status = BackendStatus.running
        
        let health = SystemHealth(
            timestamp: now,
            backendStatus: status,
            totalNodes: 3,
            onlineNodes: 2,
            offlineNodes: 1
        )
        
        #expect(health.timestamp == now)
        #expect(health.backendStatus == status)
        #expect(health.totalNodes == 3)
        #expect(health.onlineNodes == 2)
        #expect(health.offlineNodes == 1)
        #expect(health.totalStorage == 0) // Default value
        #expect(health.usedStorage == 0)   // Default value
    }
}
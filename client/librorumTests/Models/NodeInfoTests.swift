//
//  NodeInfoTests.swift
//  librorumTests
//
//  Data model tests for NodeInfo
//

import Testing
import SwiftData
import Foundation
@testable import librorum

@MainActor
struct NodeInfoTests {
    
    // MARK: - Initialization Tests
    
    @Test("NodeInfo default initialization")
    func testNodeInfoDefaultInitialization() async throws {
        let nodeInfo = NodeInfo(
            nodeId: "test.librorum.local",
            address: "192.168.1.100:50051"
        )
        
        #expect(nodeInfo.nodeId == "test.librorum.local")
        #expect(nodeInfo.address == "192.168.1.100:50051")
        #expect(nodeInfo.systemInfo == "")
        #expect(nodeInfo.status == .unknown)
        #expect(nodeInfo.connectionCount == 0)
        #expect(nodeInfo.latency == 0)
        #expect(nodeInfo.failureCount == 0)
        #expect(nodeInfo.isOnline == false)
    }
    
    @Test("NodeInfo full initialization")
    func testNodeInfoFullInitialization() async throws {
        let now = Date()
        let nodeInfo = NodeInfo(
            nodeId: "full.test.local",
            address: "10.0.0.1:8080",
            systemInfo: "macOS 14.0",
            status: .online,
            lastHeartbeat: now,
            connectionCount: 5,
            latency: 0.025,
            failureCount: 1,
            isOnline: true,
            discoveredAt: now
        )
        
        #expect(nodeInfo.nodeId == "full.test.local")
        #expect(nodeInfo.address == "10.0.0.1:8080")
        #expect(nodeInfo.systemInfo == "macOS 14.0")
        #expect(nodeInfo.status == .online)
        #expect(nodeInfo.lastHeartbeat == now)
        #expect(nodeInfo.connectionCount == 5)
        #expect(nodeInfo.latency == 0.025)
        #expect(nodeInfo.failureCount == 1)
        #expect(nodeInfo.isOnline == true)
        #expect(nodeInfo.discoveredAt == now)
    }
    
    // MARK: - NodeStatus Tests
    
    @Test("NodeStatus display names")
    func testNodeStatusDisplayNames() async throws {
        #expect(NodeStatus.online.displayName == "在线")
        #expect(NodeStatus.offline.displayName == "离线")
        #expect(NodeStatus.unknown.displayName == "未知")
        #expect(NodeStatus.error.displayName == "错误")
    }
    
    @Test("NodeStatus colors")
    func testNodeStatusColors() async throws {
        #expect(NodeStatus.online.color == "green")
        #expect(NodeStatus.offline.color == "gray")
        #expect(NodeStatus.unknown.color == "yellow")
        #expect(NodeStatus.error.color == "red")
    }
    
    @Test("NodeStatus case iteration")
    func testNodeStatusCaseIteration() async throws {
        let allCases = NodeStatus.allCases
        #expect(allCases.count == 4)
        #expect(allCases.contains(.online))
        #expect(allCases.contains(.offline))
        #expect(allCases.contains(.unknown))
        #expect(allCases.contains(.error))
    }
    
    // MARK: - SwiftData Integration Tests
    
    @Test("NodeInfo SwiftData persistence")
    func testNodeInfoSwiftDataPersistence() async throws {
        let container = try ModelContainer(
            for: NodeInfo.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let context = ModelContext(container)
        
        let nodeInfo = NodeInfo(
            nodeId: "persist.test.local",
            address: "127.0.0.1:9090",
            systemInfo: "Test OS",
            status: .online,
            connectionCount: 3,
            latency: 0.01,
            isOnline: true
        )
        
        context.insert(nodeInfo)
        try context.save()
        
        let fetchDescriptor = FetchDescriptor<NodeInfo>(
            predicate: #Predicate<NodeInfo> { $0.nodeId == "persist.test.local" }
        )
        let fetchedNodes = try context.fetch(fetchDescriptor)
        
        #expect(fetchedNodes.count == 1)
        let fetchedNode = fetchedNodes.first!
        #expect(fetchedNode.nodeId == "persist.test.local")
        #expect(fetchedNode.address == "127.0.0.1:9090")
        #expect(fetchedNode.systemInfo == "Test OS")
        #expect(fetchedNode.status == .online)
        #expect(fetchedNode.connectionCount == 3)
        #expect(fetchedNode.latency == 0.01)
        #expect(fetchedNode.isOnline == true)
    }
    
    // MARK: - Edge Cases and Validation
    
    @Test("NodeInfo with empty strings")
    func testNodeInfoWithEmptyStrings() async throws {
        let nodeInfo = NodeInfo(
            nodeId: "",
            address: "",
            systemInfo: ""
        )
        
        #expect(nodeInfo.nodeId == "")
        #expect(nodeInfo.address == "")
        #expect(nodeInfo.systemInfo == "")
    }
    
    @Test("NodeInfo with extreme values")
    func testNodeInfoWithExtremeValues() async throws {
        let nodeInfo = NodeInfo(
            nodeId: "test.node.local",
            address: "255.255.255.255:65535",
            connectionCount: Int.max,
            latency: Double.greatestFiniteMagnitude,
            failureCount: Int.max
        )
        
        #expect(nodeInfo.connectionCount == Int.max)
        #expect(nodeInfo.latency == Double.greatestFiniteMagnitude)
        #expect(nodeInfo.failureCount == Int.max)
    }
    
    @Test("NodeInfo latency calculations")
    func testNodeInfoLatencyCalculations() async throws {
        let nodeInfo = NodeInfo(
            nodeId: "latency.test.local",
            address: "192.168.1.1:50051",
            latency: 0.025 // 25ms
        )
        
        let latencyMs = nodeInfo.latency * 1000
        #expect(latencyMs == 25.0)
    }
}
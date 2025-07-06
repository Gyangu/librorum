//
//  TransportManager.swift
//  Universal Transport Protocol
//
//  Transport management and coordination
//

import Foundation
import Logging
import UniversalTransportSharedMemory
// import UniversalTransportNetwork

/// Main transport manager coordinating all transport types
@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
public actor TransportManager {
    
    // MARK: - Properties
    
    private let logger = Logger(label: "transport-manager")
    private let configuration: TransportConfiguration
    
    // Transport implementations
    private let sharedMemoryTransport: SharedMemoryTransport
    // private let networkTransport: NetworkTransportManager
    
    // Node registry
    private var knownNodes: [String: NodeInfo] = [:]
    private var nodeConnections: [String: TransportStrategy] = [:]
    
    // MARK: - Initialization
    
    public init(configuration: TransportConfiguration) async throws {
        self.configuration = configuration
        
        // Initialize transport implementations
        if configuration.enableSharedMemory {
            self.sharedMemoryTransport = SharedMemoryTransport(
                configuration: SharedMemoryConfiguration(
                    defaultRegionSize: configuration.maxMessageSize,
                    maxRegions: 32,
                    enableMetrics: configuration.performanceMonitoringEnabled,
                    defaultTimeout: configuration.defaultTimeout
                )
            )
        } else {
            self.sharedMemoryTransport = SharedMemoryTransport()
        }
        
        // self.networkTransport = NetworkTransportManager(configuration: configuration)
        
        logger.info("Transport manager initialized with configuration: \(configuration)")
    }
    
    // MARK: - High-Level Interface
    
    /// Send structured data to a destination node
    public func send<T: Codable>(_ data: T, to destination: NodeInfo, using strategy: TransportStrategy) async throws {
        logger.debug("Sending data to \(destination.id) using strategy \(strategy)")
        
        switch strategy {
        case .sharedMemory(let region):
            guard destination.isLocalMachine else {
                throw TransportError.transportNotAvailable(.sharedMemory(region: region))
            }
            
            // Ensure region exists
            try await sharedMemoryTransport.getOrCreateRegion(
                name: region,
                size: configuration.maxMessageSize
            )
            
            try await sharedMemoryTransport.send(data, to: region, timeout: configuration.defaultTimeout)
            
        case .swiftOptimized:
            throw TransportError.transportNotAvailable(.swiftOptimized)
            
        case .universal:
            throw TransportError.transportNotAvailable(.universal)
        }
        
        // Update connection registry
        nodeConnections[destination.id] = strategy
    }
    
    /// Receive structured data from a source node
    public func receive<T: Codable>(
        _ type: T.Type,
        from source: NodeInfo,
        using strategy: TransportStrategy,
        timeout: TimeInterval
    ) async throws -> T {
        logger.debug("Receiving data from \(source.id) using strategy \(strategy)")
        
        switch strategy {
        case .sharedMemory(let region):
            guard source.isLocalMachine else {
                throw TransportError.transportNotAvailable(.sharedMemory(region: region))
            }
            
            return try await sharedMemoryTransport.receive(type, from: region, timeout: timeout)
            
        case .swiftOptimized:
            throw TransportError.transportNotAvailable(.swiftOptimized)
            
        case .universal:
            throw TransportError.transportNotAvailable(.universal)
        }
    }
    
    // MARK: - Node Management
    
    /// Register a node for communication
    public func registerNode(_ node: NodeInfo) async {
        knownNodes[node.id] = node
        logger.debug("Registered node: \(node)")
    }
    
    /// Unregister a node
    public func unregisterNode(_ nodeId: String) async {
        knownNodes.removeValue(forKey: nodeId)
        nodeConnections.removeValue(forKey: nodeId)
        logger.debug("Unregistered node: \(nodeId)")
    }
    
    /// Get information about a known node
    public func getNode(_ nodeId: String) async -> NodeInfo? {
        return knownNodes[nodeId]
    }
    
    /// List all known nodes
    public func listKnownNodes() async -> [NodeInfo] {
        return Array(knownNodes.values)
    }
    
    /// Check if we can communicate with a node
    public func canCommunicateWith(_ node: NodeInfo) async -> Bool {
        // Check shared memory for local nodes
        if node.isLocalMachine {
            if let regionName = node.sharedMemoryName {
                return await sharedMemoryTransport.isRegionAvailable(regionName)
            }
        }
        
        // Check network connectivity for remote nodes
        if node.endpoint != nil {
            // TODO: Implement network connectivity check
            return false
        }
        
        return false
    }
    
    // MARK: - Transport Information
    
    /// Get available transports
    public func availableTransports() async -> [TransportInfo] {
        var transports: [TransportInfo] = []
        
        // Shared memory transport
        if configuration.enableSharedMemory {
            transports.append(
                TransportInfo(
                    transportType: .sharedMemory,
                    isAvailable: true,
                    supportedPlatforms: ["macOS", "iOS", "Linux"],
                    performanceTier: .extreme,
                    description: "POSIX shared memory with ring buffers"
                )
            )
        }
        
        // Network transports (placeholder)
        transports.append(
            TransportInfo(
                transportType: .swiftNetwork,
                isAvailable: false,
                supportedPlatforms: ["macOS", "iOS"],
                performanceTier: .high,
                description: "Swift-optimized network protocol (not implemented)"
            )
        )
        
        transports.append(
            TransportInfo(
                transportType: .universal,
                isAvailable: false,
                supportedPlatforms: ["all"],
                performanceTier: .compatibility,
                description: "Universal compatibility protocol (not implemented)"
            )
        )
        
        return transports
    }
    
    /// Get transport statistics
    public func getTransportStats() async -> TransportStats {
        let _ = await sharedMemoryTransport.getPerformanceMetrics()
        
        return TransportStats(
            totalNodes: knownNodes.count,
            activeConnections: nodeConnections.count,
            sharedMemoryRegions: await sharedMemoryTransport.listRegions().count,
            networkConnections: 0, // TODO: Implement when network transport is ready
            totalMessagesSent: 0, // TODO: Implement
            totalMessagesReceived: 0, // TODO: Implement
            averageLatency: 0.0, // TODO: Calculate from metrics
            totalThroughput: 0.0 // TODO: Calculate from metrics
        )
    }
}

// MARK: - Transport Statistics

public struct TransportStats: Codable {
    public let totalNodes: Int
    public let activeConnections: Int
    public let sharedMemoryRegions: Int
    public let networkConnections: Int
    public let totalMessagesSent: Int
    public let totalMessagesReceived: Int
    public let averageLatency: TimeInterval
    public let totalThroughput: Double // bytes per second
    
    public init(
        totalNodes: Int,
        activeConnections: Int,
        sharedMemoryRegions: Int,
        networkConnections: Int,
        totalMessagesSent: Int,
        totalMessagesReceived: Int,
        averageLatency: TimeInterval,
        totalThroughput: Double
    ) {
        self.totalNodes = totalNodes
        self.activeConnections = activeConnections
        self.sharedMemoryRegions = sharedMemoryRegions
        self.networkConnections = networkConnections
        self.totalMessagesSent = totalMessagesSent
        self.totalMessagesReceived = totalMessagesReceived
        self.averageLatency = averageLatency
        self.totalThroughput = totalThroughput
    }
}

// MARK: - Performance Data

public struct PerformanceData {
    public let averageLatency: TimeInterval
    public let throughput: Double
    public let successRate: Double
    public let recommendedStrategy: TransportStrategy?
    
    public init(
        averageLatency: TimeInterval,
        throughput: Double,
        successRate: Double,
        recommendedStrategy: TransportStrategy?
    ) {
        self.averageLatency = averageLatency
        self.throughput = throughput
        self.successRate = successRate
        self.recommendedStrategy = recommendedStrategy
    }
}
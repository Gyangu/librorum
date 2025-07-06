//
//  UniversalTransport.swift
//  Universal Transport Protocol
//
//  High-performance cross-platform communication for Swift and Rust
//

import Foundation
import Logging
import UniversalTransportSharedMemory
import UniversalTransportNetwork

/// Universal Transport Protocol main interface
@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
public actor UniversalTransport {
    
    // MARK: - Properties
    
    private let logger = Logger(label: "universal-transport")
    private let transportManager: TransportManager
    private let performanceMonitor: PerformanceMonitor
    
    // MARK: - Initialization
    
    public init(configuration: TransportConfiguration = .default) async throws {
        self.transportManager = try await TransportManager(configuration: configuration)
        self.performanceMonitor = PerformanceMonitor()
        
        logger.info("Universal Transport initialized with configuration: \(configuration)")
    }
    
    // MARK: - High-Level Interface
    
    /// Send structured data to a destination node
    /// - Parameters:
    ///   - data: The data to send (must be Codable)
    ///   - destination: Target node information
    /// - Returns: Void on success
    /// - Throws: TransportError on failure
    public func send<T: Codable>(_ data: T, to destination: NodeInfo) async throws {
        let startTime = Date()
        
        do {
            let strategy = await selectOptimalStrategy(for: destination, dataSize: MemoryLayout<T>.size)
            logger.debug("Selected transport strategy: \(strategy) for destination: \(destination.id)")
            
            try await transportManager.send(data, to: destination, using: strategy)
            
            // Record performance metrics
            let duration = Date().timeIntervalSince(startTime)
            await performanceMonitor.recordSend(
                strategy: strategy,
                dataSize: MemoryLayout<T>.size,
                duration: duration,
                success: true
            )
            
        } catch {
            let duration = Date().timeIntervalSince(startTime)
            await performanceMonitor.recordSend(
                strategy: .universal,
                dataSize: MemoryLayout<T>.size,
                duration: duration,
                success: false
            )
            throw error
        }
    }
    
    /// Receive structured data from a source node
    /// - Parameters:
    ///   - type: The expected data type
    ///   - source: Source node information
    ///   - timeout: Timeout in milliseconds
    /// - Returns: Decoded data of type T
    /// - Throws: TransportError on failure
    public func receive<T: Codable>(_ type: T.Type, from source: NodeInfo, timeout: TimeInterval = 30.0) async throws -> T {
        let startTime = Date()
        
        do {
            let strategy = await selectOptimalStrategy(for: source, dataSize: MemoryLayout<T>.size)
            let result: T = try await transportManager.receive(type, from: source, using: strategy, timeout: timeout)
            
            // Record performance metrics
            let duration = Date().timeIntervalSince(startTime)
            await performanceMonitor.recordReceive(
                strategy: strategy,
                dataSize: MemoryLayout<T>.size,
                duration: duration,
                success: true
            )
            
            return result
            
        } catch {
            let duration = Date().timeIntervalSince(startTime)
            await performanceMonitor.recordReceive(
                strategy: .universal,
                dataSize: MemoryLayout<T>.size,
                duration: duration,
                success: false
            )
            throw error
        }
    }
    
    /// Broadcast data to multiple destinations
    /// - Parameters:
    ///   - data: The data to broadcast
    ///   - destinations: Array of destination nodes
    /// - Returns: Array of results (success/failure for each destination)
    public func broadcast<T: Codable>(_ data: T, to destinations: [NodeInfo]) async -> [Result<Void, Error>] {
        await withTaskGroup(of: (Int, Result<Void, Error>).self) { group in
            for (index, destination) in destinations.enumerated() {
                group.addTask {
                    let result: Result<Void, Error>
                    do {
                        try await self.send(data, to: destination)
                        result = .success(())
                    } catch {
                        result = .failure(error)
                    }
                    return (index, result)
                }
            }
            
            var results = Array<Result<Void, Error>?>(repeating: nil, count: destinations.count)
            for await (index, result) in group {
                results[index] = result
            }
            
            return results.compactMap { $0 }
        }
    }
    
    // MARK: - Strategy Selection
    
    /// Select the optimal transport strategy for communication
    private func selectOptimalStrategy(for destination: NodeInfo, dataSize: Int) async -> TransportStrategy {
        // 1. Check if same machine - use shared memory
        if destination.isLocalMachine {
            logger.debug("Using shared memory for local communication")
            return .sharedMemory(region: destination.getSharedMemoryName())
        }
        
        // 2. Get historical performance data
        let performanceData = await performanceMonitor.getPerformanceData(for: destination)
        
        // 3. Consider data size and network conditions
        if dataSize > 1024 * 1024 { // > 1MB
            // Large data - prefer high-throughput transports
            if destination.language == .swift {
                return .swiftOptimized
            } else {
                return .universal // Cross-language for large data
            }
        }
        
        // 4. Small data - prefer low-latency transports
        if let bestStrategy = performanceData?.recommendedStrategy {
            return bestStrategy
        }
        
        // 5. Default selection based on destination language
        switch destination.language {
        case .swift:
            return .swiftOptimized
        case .rust:
            return .universal // Cross-language
        }
    }
    
    // MARK: - Information and Monitoring
    
    /// Get available transport information
    public func availableTransports() async -> [TransportInfo] {
        await transportManager.availableTransports()
    }
    
    /// Get performance metrics
    public func performanceMetrics() async -> PerformanceMetrics {
        await performanceMonitor.getOverallMetrics()
    }
    
    /// Check connectivity to a node
    public func checkConnectivity(to node: NodeInfo) async -> Bool {
        await transportManager.canCommunicateWith(node)
    }
}

// MARK: - Supporting Types

/// Transport strategy enumeration
public enum TransportStrategy: Hashable, CustomStringConvertible {
    case sharedMemory(region: String)
    case swiftOptimized
    case universal
    
    public var description: String {
        switch self {
        case .sharedMemory(let region):
            return "SharedMemory(\(region))"
        case .swiftOptimized:
            return "SwiftOptimized"
        case .universal:
            return "Universal"
        }
    }
}

/// Transport configuration
public struct TransportConfiguration: Codable {
    public let enableSharedMemory: Bool
    public let enableSwiftOptimization: Bool
    public let enableCompression: Bool
    public let enableEncryption: Bool
    public let maxMessageSize: Int
    public let defaultTimeout: TimeInterval
    public let performanceMonitoringEnabled: Bool
    
    public static let `default` = TransportConfiguration(
        enableSharedMemory: true,
        enableSwiftOptimization: true,
        enableCompression: false,
        enableEncryption: false,
        maxMessageSize: 64 * 1024 * 1024, // 64MB
        defaultTimeout: 30.0,
        performanceMonitoringEnabled: true
    )
    
    public init(
        enableSharedMemory: Bool = true,
        enableSwiftOptimization: Bool = true,
        enableCompression: Bool = false,
        enableEncryption: Bool = false,
        maxMessageSize: Int = 64 * 1024 * 1024,
        defaultTimeout: TimeInterval = 30.0,
        performanceMonitoringEnabled: Bool = true
    ) {
        self.enableSharedMemory = enableSharedMemory
        self.enableSwiftOptimization = enableSwiftOptimization
        self.enableCompression = enableCompression
        self.enableEncryption = enableEncryption
        self.maxMessageSize = maxMessageSize
        self.defaultTimeout = defaultTimeout
        self.performanceMonitoringEnabled = performanceMonitoringEnabled
    }
}

/// Transport error types
public enum TransportError: Error, LocalizedError {
    case nodeNotFound(String)
    case transportNotAvailable(TransportStrategy)
    case timeout(TimeInterval)
    case serialization(String)
    case network(String)
    case sharedMemory(String)
    case configuration(String)
    case permissionDenied(String)
    case invalidData(String)
    case `internal`(String)
    
    public var errorDescription: String? {
        switch self {
        case .nodeNotFound(let nodeId):
            return "Node not found: \(nodeId)"
        case .transportNotAvailable(let strategy):
            return "Transport not available: \(strategy)"
        case .timeout(let duration):
            return "Operation timed out after \(duration) seconds"
        case .serialization(let message):
            return "Serialization error: \(message)"
        case .network(let message):
            return "Network error: \(message)"
        case .sharedMemory(let message):
            return "Shared memory error: \(message)"
        case .configuration(let message):
            return "Configuration error: \(message)"
        case .permissionDenied(let message):
            return "Permission denied: \(message)"
        case .invalidData(let message):
            return "Invalid data: \(message)"
        case .internal(let message):
            return "Internal error: \(message)"
        }
    }
}
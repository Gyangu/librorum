//
//  UniversalTransportSharedMemory.swift
//  Universal Transport Shared Memory
//
//  Main module exports for shared memory transport
//

import Foundation

// MARK: - Module Exports

@_exported import struct Foundation.Data
@_exported import struct Foundation.Date
@_exported import class Foundation.Bundle

// Export all public types from this module
public typealias SharedMemoryTransportType = SharedMemoryTransport
public typealias SharedMemoryRegionType = SharedMemoryRegion
public typealias SharedMemoryMessageType = SharedMemoryMessage
public typealias SharedMemoryConfigurationType = SharedMemoryConfiguration

// MARK: - Module Information

/// Module version information
public enum UniversalTransportSharedMemoryVersion {
    public static let major = 0
    public static let minor = 1
    public static let patch = 0
    
    public static var string: String {
        return "\(major).\(minor).\(patch)"
    }
}

/// Module capabilities
public struct ModuleCapabilities {
    public static let supportsPOSIXSharedMemory = true
    public static let supportsRingBuffers = true
    public static let supportsCrossLanguageSerialization = true
    public static let supportsPerformanceMetrics = true
    
    public static var summary: String {
        return """
        Universal Transport Shared Memory v\(UniversalTransportSharedMemoryVersion.string)
        - POSIX Shared Memory: \(supportsPOSIXSharedMemory ? "✓" : "✗")
        - Ring Buffers: \(supportsRingBuffers ? "✓" : "✗")
        - Cross-Language Serialization: \(supportsCrossLanguageSerialization ? "✓" : "✗")
        - Performance Metrics: \(supportsPerformanceMetrics ? "✓" : "✗")
        """
    }
}

// MARK: - Convenience APIs

/// Convenience factory for creating shared memory transport
@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
public func createSharedMemoryTransport(
    configuration: SharedMemoryConfiguration = .default
) async -> SharedMemoryTransport {
    return SharedMemoryTransport(configuration: configuration)
}

/// Convenience function for quick shared memory communication setup
@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
public func setupSharedMemoryRegion(
    name: String,
    size: Int = 64 * 1024 * 1024
) async throws -> SharedMemoryTransport {
    let transport = SharedMemoryTransport()
    try await transport.getOrCreateRegion(name: name, size: size)
    return transport
}

// MARK: - Common Use Cases

/// Example usage patterns
public enum UsageExamples {
    
    /// Example: Basic send/receive pattern
    public static let basicUsage = """
    // Create transport
    let transport = await createSharedMemoryTransport()
    
    // Setup region
    try await transport.getOrCreateRegion(name: "my-app-data", size: 1024 * 1024)
    
    // Send data
    struct MyData: Codable {
        let message: String
        let timestamp: Date
    }
    
    let data = MyData(message: "Hello from Swift!", timestamp: Date())
    try await transport.send(data, to: "my-app-data")
    
    // Receive data (from another process/thread)
    let received = try await transport.receive(MyData.self, from: "my-app-data")
    print("Received: \\(received.message)")
    """
    
    /// Example: Performance monitoring
    public static let performanceMonitoring = """
    // Get performance metrics
    let metrics = await transport.getPerformanceMetrics()
    let allRegionMetrics = await metrics.getAllMetrics()
    
    for regionMetric in allRegionMetrics {
        print("Region: \\(regionMetric.regionName)")
        print("Average send duration: \\(regionMetric.averageSendDuration)s")
        print("Average receive duration: \\(regionMetric.averageReceiveDuration)s")
        print("Throughput: \\(regionMetric.totalThroughput) bytes/sec")
    }
    """
    
    /// Example: Cross-language communication
    public static let crossLanguage = """
    // This Swift code can communicate with Rust processes
    // using the same shared memory region and protocol
    
    // Create region with name that Rust process will also use
    try await transport.getOrCreateRegion(name: "rust-swift-bridge", size: 2 * 1024 * 1024)
    
    // Send structured data that Rust can deserialize
    struct SwiftToRustMessage: Codable {
        let operation: String
        let parameters: [String: String]
        let timestamp: Double
    }
    
    let message = SwiftToRustMessage(
        operation: "process_data",
        parameters: ["input": "user_data.json", "output": "processed_data.json"],
        timestamp: Date().timeIntervalSince1970
    )
    
    try await transport.send(message, to: "rust-swift-bridge")
    
    // Rust process can receive this message using the same protocol
    """
}

// MARK: - Error Handling Utilities

/// Convenient error handling helpers
public extension SharedMemoryError {
    
    /// Check if error is recoverable
    var isRecoverable: Bool {
        switch self {
        case .timeout, .bufferFull, .bufferEmpty:
            return true
        case .regionNotFound, .protocolError, .dataCorruption, .permissionDenied:
            return false
        case .regionCreationFailed, .mappingFailed, .messageTooLarge, .platformError:
            return false
        }
    }
    
    /// Get retry delay for recoverable errors
    var suggestedRetryDelay: TimeInterval? {
        guard isRecoverable else { return nil }
        
        switch self {
        case .timeout:
            return 1.0
        case .bufferFull, .bufferEmpty:
            return 0.01 // 10ms
        default:
            return nil
        }
    }
}

// MARK: - Debug Utilities

#if DEBUG
/// Debug utilities for development
public enum DebugUtilities {
    
    /// Print module information
    public static func printModuleInfo() {
        print(ModuleCapabilities.summary)
    }
    
    /// Validate shared memory region
    public static func validateRegion(_ region: SharedMemoryRegion) -> Bool {
        do {
            // Try to read/write a test pattern
            let testData = Data("test".utf8)
            try region.write(testData, at: 0)
            let readData = try region.read(offset: 0, length: testData.count)
            return readData == testData
        } catch {
            print("Region validation failed: \(error)")
            return false
        }
    }
}
#endif
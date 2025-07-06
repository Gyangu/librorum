//
//  SharedMemoryTransport.swift
//  Universal Transport Shared Memory
//
//  High-performance shared memory transport implementation
//

import Foundation
import Logging

// MARK: - Shared Memory Transport

/// High-performance shared memory transport for same-machine communication
@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
public actor SharedMemoryTransport {
    
    // MARK: - Properties
    
    private let logger = Logger(label: "shared-memory-transport")
    private var regions: [String: SharedMemoryRegion] = [:]
    private let configuration: SharedMemoryConfiguration
    private let performanceMetrics: PerformanceMetrics
    
    // MARK: - Initialization
    
    public init(configuration: SharedMemoryConfiguration = .default) {
        self.configuration = configuration
        self.performanceMetrics = PerformanceMetrics()
        
        logger.info("Shared memory transport initialized with configuration: \(configuration)")
    }
    
    deinit {
        Task { @Sendable in
            // Note: regions will be cleaned up automatically when they go out of scope
        }
    }
    
    // MARK: - High-Level Interface
    
    /// Send structured data through shared memory
    /// - Parameters:
    ///   - data: Data to send (must be Codable)
    ///   - regionName: Shared memory region identifier
    ///   - timeout: Send timeout in seconds
    /// - Throws: SharedMemoryError on failure
    public func send<T: Codable>(_ data: T, to regionName: String, timeout: TimeInterval = 30.0) async throws {
        let startTime = Date()
        
        do {
            // Serialize data using MessagePack for cross-language compatibility
            let serializedData = try MessagePackSerializer.serialize(data)
            let message = try SharedMemoryMessage.data(serializedData)
            
            try await sendMessage(message, to: regionName, timeout: timeout)
            
            // Record performance metrics
            let duration = Date().timeIntervalSince(startTime)
            await performanceMetrics.recordSend(
                regionName: regionName,
                dataSize: serializedData.count,
                duration: duration,
                success: true
            )
            
            logger.debug("Sent \(serializedData.count) bytes to region '\(regionName)' in \(duration)s")
            
        } catch {
            let duration = Date().timeIntervalSince(startTime)
            await performanceMetrics.recordSend(
                regionName: regionName,
                dataSize: 0,
                duration: duration,
                success: false
            )
            logger.error("Failed to send to region '\(regionName)': \(error)")
            throw error
        }
    }
    
    /// Receive structured data from shared memory
    /// - Parameters:
    ///   - type: Expected data type
    ///   - regionName: Shared memory region identifier
    ///   - timeout: Receive timeout in seconds
    /// - Returns: Deserialized data of type T
    /// - Throws: SharedMemoryError on failure
    public func receive<T: Codable>(_ type: T.Type, from regionName: String, timeout: TimeInterval = 30.0) async throws -> T {
        let startTime = Date()
        
        do {
            let message = try await receiveMessage(from: regionName, timeout: timeout)
            
            // Deserialize data using MessagePack
            let deserializedData = try MessagePackSerializer.deserialize(message.payload, as: type)
            
            // Record performance metrics
            let duration = Date().timeIntervalSince(startTime)
            await performanceMetrics.recordReceive(
                regionName: regionName,
                dataSize: message.payload.count,
                duration: duration,
                success: true
            )
            
            logger.debug("Received \(message.payload.count) bytes from region '\(regionName)' in \(duration)s")
            
            return deserializedData
            
        } catch {
            let duration = Date().timeIntervalSince(startTime)
            await performanceMetrics.recordReceive(
                regionName: regionName,
                dataSize: 0,
                duration: duration,
                success: false
            )
            logger.error("Failed to receive from region '\(regionName)': \(error)")
            throw error
        }
    }
    
    // MARK: - Region Management
    
    /// Create or get a shared memory region
    /// - Parameters:
    ///   - name: Region identifier
    ///   - size: Region size in bytes
    /// - Returns: True if region was created, false if opened existing
    /// - Throws: SharedMemoryError on failure
    @discardableResult
    public func getOrCreateRegion(name: String, size: Int) async throws -> Bool {
        if regions[name] != nil {
            logger.debug("Using existing region: \(name)")
            return false
        }
        
        // Try to open existing region first
        do {
            let region = try SharedMemoryRegion.open(name: name)
            regions[name] = region
            logger.debug("Opened existing shared memory region: \(name)")
            return false
        } catch {
            // Region doesn't exist, create it
            let region = try SharedMemoryRegion.create(name: name, size: size)
            try region.initializeRingBuffer(capacity: UInt64(size - MemoryLayout<RingBufferHeader>.size))
            regions[name] = region
            logger.info("Created new shared memory region: \(name), size: \(size)")
            return true
        }
    }
    
    /// Remove a region from management
    /// - Parameter name: Region identifier
    public func removeRegion(name: String) async {
        if regions.removeValue(forKey: name) != nil {
            // Region cleanup happens in deinit
            logger.debug("Removed region from management: \(name)")
        }
    }
    
    /// List all managed regions
    public func listRegions() async -> [String] {
        return Array(regions.keys)
    }
    
    // MARK: - Low-Level Message Operations
    
    /// Send a message to shared memory region
    private func sendMessage(_ message: SharedMemoryMessage, to regionName: String, timeout: TimeInterval) async throws {
        guard let region = regions[regionName] else {
            throw SharedMemoryError.regionNotFound(regionName)
        }
        
        let timeoutDate = Date().addingTimeInterval(timeout)
        var sequence: UInt64 = 0
        
        // Get next sequence number (simplified - should be atomic)
        sequence = await getNextSequenceNumber(for: regionName)
        
        var messageWithSequence = message
        messageWithSequence.setSequence(sequence)
        
        // Write message to ring buffer with timeout
        while Date() < timeoutDate {
            if try await writeMessageToRingBuffer(messageWithSequence, region: region) {
                return // Success
            }
            
            // Buffer full, wait a bit and retry
            try await Task.sleep(nanoseconds: 1_000_000) // 1ms
        }
        
        throw SharedMemoryError.timeout(timeout)
    }
    
    /// Receive a message from shared memory region
    private func receiveMessage(from regionName: String, timeout: TimeInterval) async throws -> SharedMemoryMessage {
        guard let region = regions[regionName] else {
            throw SharedMemoryError.regionNotFound(regionName)
        }
        
        let timeoutDate = Date().addingTimeInterval(timeout)
        
        // Read message from ring buffer with timeout
        while Date() < timeoutDate {
            if let message = try await readMessageFromRingBuffer(region: region) {
                try message.validate()
                return message
            }
            
            // Buffer empty, wait a bit and retry
            try await Task.sleep(nanoseconds: 1_000_000) // 1ms
        }
        
        throw SharedMemoryError.timeout(timeout)
    }
    
    /// Write message to ring buffer
    private func writeMessageToRingBuffer(_ message: SharedMemoryMessage, region: SharedMemoryRegion) async throws -> Bool {
        let ringBufferPtr = try region.getRingBuffer()
        let ringBuffer = ringBufferPtr.pointee
        
        // Check if there's enough space
        let messageSize = UInt64(message.totalSize)
        let availableSpace = ringBuffer.capacity - ringBuffer.availableBytes
        
        guard messageSize <= availableSpace else {
            return false // Buffer full
        }
        
        // Calculate write position in data area
        let dataAreaOffset = MemoryLayout<RingBufferHeader>.size
        let dataAreaCapacity = ringBuffer.capacity
        let writePos = ringBuffer.writePosition
        
        // Serialize message (header + payload)
        var headerData = Data(count: MESSAGE_HEADER_SIZE)
        headerData.withUnsafeMutableBytes { bytes in
            bytes.storeBytes(of: message.header, as: MessageHeader.self)
        }
        
        let totalMessage = headerData + message.payload
        
        // Handle wrap-around
        let endPos = (writePos + messageSize) % dataAreaCapacity
        
        if writePos + messageSize <= dataAreaCapacity {
            // No wrap-around needed
            try region.write(totalMessage, at: dataAreaOffset + Int(writePos))
        } else {
            // Handle wrap-around
            let firstChunkSize = Int(dataAreaCapacity - writePos)
            let secondChunkSize = totalMessage.count - firstChunkSize
            
            let firstChunk = totalMessage.prefix(firstChunkSize)
            let secondChunk = totalMessage.suffix(secondChunkSize)
            
            try region.write(Data(firstChunk), at: dataAreaOffset + Int(writePos))
            try region.write(Data(secondChunk), at: dataAreaOffset)
        }
        
        // Update ring buffer pointers atomically (simplified)
        ringBufferPtr.pointee.writePosition = endPos
        ringBufferPtr.pointee.availableBytes += messageSize
        
        return true
    }
    
    /// Read message from ring buffer
    private func readMessageFromRingBuffer(region: SharedMemoryRegion) async throws -> SharedMemoryMessage? {
        let ringBufferPtr = try region.getRingBuffer()
        let ringBuffer = ringBufferPtr.pointee
        
        // Check if there's data available
        guard ringBuffer.availableBytes >= UInt64(MESSAGE_HEADER_SIZE) else {
            return nil // Buffer empty
        }
        
        let dataAreaOffset = MemoryLayout<RingBufferHeader>.size
        let readPos = ringBuffer.readPosition
        
        // Read message header first
        let headerData = try region.read(
            offset: dataAreaOffset + Int(readPos),
            length: MESSAGE_HEADER_SIZE
        )
        
        guard headerData.count == MESSAGE_HEADER_SIZE else {
            throw SharedMemoryError.dataCorruption("Incomplete header read")
        }
        
        let header = headerData.withUnsafeBytes { bytes in
            bytes.load(as: MessageHeader.self)
        }
        
        try header.validate()
        
        let totalMessageSize = UInt64(MESSAGE_HEADER_SIZE) + UInt64(header.size)
        
        // Check if complete message is available
        guard ringBuffer.availableBytes >= totalMessageSize else {
            return nil // Incomplete message
        }
        
        // Read payload
        let payloadSize = Int(header.size)
        var payload = Data()
        
        if payloadSize > 0 {
            let payloadStartPos = (readPos + UInt64(MESSAGE_HEADER_SIZE)) % ringBuffer.capacity
            
            if payloadStartPos + UInt64(payloadSize) <= ringBuffer.capacity {
                // No wrap-around
                payload = try region.read(
                    offset: dataAreaOffset + Int(payloadStartPos),
                    length: payloadSize
                )
            } else {
                // Handle wrap-around
                let firstChunkSize = Int(ringBuffer.capacity - payloadStartPos)
                let secondChunkSize = payloadSize - firstChunkSize
                
                let firstChunk = try region.read(
                    offset: dataAreaOffset + Int(payloadStartPos),
                    length: firstChunkSize
                )
                
                let secondChunk = try region.read(
                    offset: dataAreaOffset,
                    length: secondChunkSize
                )
                
                payload = firstChunk + secondChunk
            }
        }
        
        // Create message
        let messageType = MessageType(rawValue: header.messageType)!
        var message = try SharedMemoryMessage(type: messageType, payload: payload)
        message.setSequence(header.sequence)
        
        // Update ring buffer pointers atomically (simplified)
        let newReadPos = (readPos + totalMessageSize) % ringBuffer.capacity
        ringBufferPtr.pointee.readPosition = newReadPos
        ringBufferPtr.pointee.availableBytes -= totalMessageSize
        
        return message
    }
    
    // MARK: - Utilities
    
    /// Get next sequence number for a region
    private func getNextSequenceNumber(for regionName: String) async -> UInt64 {
        // Simplified implementation - should use atomic operations
        return UInt64(Date().timeIntervalSince1970 * 1000000) // microseconds
    }
    
    /// Get performance metrics
    public func getPerformanceMetrics() async -> PerformanceMetrics {
        return performanceMetrics
    }
    
    /// Check if region is available
    public func isRegionAvailable(_ name: String) async -> Bool {
        return regions[name] != nil
    }
    
    /// Cleanup all regions
    private func cleanup() async {
        logger.debug("Cleaning up \(regions.count) shared memory regions")
        regions.removeAll()
    }
}

// MARK: - Configuration

/// Shared memory transport configuration
public struct SharedMemoryConfiguration: Codable {
    public let defaultRegionSize: Int
    public let maxRegions: Int
    public let enableMetrics: Bool
    public let defaultTimeout: TimeInterval
    
    public static let `default` = SharedMemoryConfiguration(
        defaultRegionSize: 64 * 1024 * 1024, // 64MB
        maxRegions: 32,
        enableMetrics: true,
        defaultTimeout: 30.0
    )
    
    public init(
        defaultRegionSize: Int = 64 * 1024 * 1024,
        maxRegions: Int = 32,
        enableMetrics: Bool = true,
        defaultTimeout: TimeInterval = 30.0
    ) {
        self.defaultRegionSize = defaultRegionSize
        self.maxRegions = maxRegions
        self.enableMetrics = enableMetrics
        self.defaultTimeout = defaultTimeout
    }
}

// MARK: - Performance Metrics

/// Performance metrics for shared memory transport
public actor PerformanceMetrics {
    private var sendMetrics: [String: [OperationMetric]] = [:]
    private var receiveMetrics: [String: [OperationMetric]] = [:]
    
    public func recordSend(regionName: String, dataSize: Int, duration: TimeInterval, success: Bool) {
        let metric = OperationMetric(
            timestamp: Date(),
            dataSize: dataSize,
            duration: duration,
            success: success
        )
        
        if sendMetrics[regionName] == nil {
            sendMetrics[regionName] = []
        }
        sendMetrics[regionName]?.append(metric)
        
        // Keep only recent metrics (last 1000 operations)
        if sendMetrics[regionName]!.count > 1000 {
            sendMetrics[regionName]?.removeFirst()
        }
    }
    
    public func recordReceive(regionName: String, dataSize: Int, duration: TimeInterval, success: Bool) {
        let metric = OperationMetric(
            timestamp: Date(),
            dataSize: dataSize,
            duration: duration,
            success: success
        )
        
        if receiveMetrics[regionName] == nil {
            receiveMetrics[regionName] = []
        }
        receiveMetrics[regionName]?.append(metric)
        
        // Keep only recent metrics (last 1000 operations)
        if receiveMetrics[regionName]!.count > 1000 {
            receiveMetrics[regionName]?.removeFirst()
        }
    }
    
    public func getMetrics(for regionName: String) -> RegionMetrics? {
        guard let sends = sendMetrics[regionName], let receives = receiveMetrics[regionName] else {
            return nil
        }
        
        return RegionMetrics(regionName: regionName, sends: sends, receives: receives)
    }
    
    public func getAllMetrics() -> [RegionMetrics] {
        let allRegions = Set(sendMetrics.keys).union(Set(receiveMetrics.keys))
        return allRegions.compactMap { getMetrics(for: $0) }
    }
}

public struct OperationMetric {
    public let timestamp: Date
    public let dataSize: Int
    public let duration: TimeInterval
    public let success: Bool
}

public struct RegionMetrics {
    public let regionName: String
    public let sends: [OperationMetric]
    public let receives: [OperationMetric]
    
    public var averageSendDuration: TimeInterval {
        let successful = sends.filter { $0.success }
        guard !successful.isEmpty else { return 0 }
        return successful.map { $0.duration }.reduce(0, +) / Double(successful.count)
    }
    
    public var averageReceiveDuration: TimeInterval {
        let successful = receives.filter { $0.success }
        guard !successful.isEmpty else { return 0 }
        return successful.map { $0.duration }.reduce(0, +) / Double(successful.count)
    }
    
    public var totalThroughput: Double {
        let totalBytes = sends.map { $0.dataSize }.reduce(0, +) + receives.map { $0.dataSize }.reduce(0, +)
        let totalDuration = sends.map { $0.duration }.reduce(0, +) + receives.map { $0.duration }.reduce(0, +)
        guard totalDuration > 0 else { return 0 }
        return Double(totalBytes) / totalDuration // bytes per second
    }
}

// MARK: - MessagePack Serialization

/// MessagePack serialization for cross-language compatibility
public enum MessagePackSerializer {
    
    public static func serialize<T: Codable>(_ data: T) throws -> Data {
        // For now, use JSON as a fallback
        // In a real implementation, use MessagePack library
        let encoder = JSONEncoder()
        return try encoder.encode(data)
    }
    
    public static func deserialize<T: Codable>(_ data: Data, as type: T.Type) throws -> T {
        // For now, use JSON as a fallback
        // In a real implementation, use MessagePack library
        let decoder = JSONDecoder()
        return try decoder.decode(type, from: data)
    }
}
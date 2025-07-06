//
//  SharedMemoryProtocol.swift
//  Universal Transport Shared Memory
//
//  Cross-platform shared memory protocol for Swift-Rust interoperability
//

import Foundation
import Logging

#if canImport(zlib)
import zlib
#endif

// MARK: - Protocol Constants

/// Shared memory protocol magic number (matches Rust implementation)
public let SHARED_MEMORY_MAGIC: UInt32 = 0x55545054 // "UTPT"

/// Protocol version
public let SHARED_MEMORY_VERSION: UInt8 = 1

/// Maximum message size (64MB)
public let MAX_MESSAGE_SIZE: UInt32 = 64 * 1024 * 1024

/// Message header size (32 bytes, cache-line aligned)
public let MESSAGE_HEADER_SIZE = 32

// MARK: - Message Types

/// Message type enumeration (matches Rust MessageType)
public enum MessageType: UInt8, CaseIterable {
    case data = 0x01
    case heartbeat = 0x02
    case acknowledgment = 0x03
    case error = 0x04
}

// MARK: - Message Header

/// Shared memory message header (32 bytes, cache-line aligned)
/// This structure must match the Rust MessageHeader exactly
public struct MessageHeader {
    /// Protocol magic number
    public var magic: UInt32
    /// Protocol version
    public var version: UInt8
    /// Message type
    public var messageType: UInt8
    /// Flags
    public var flags: UInt16
    /// Message size (excluding header)
    public var size: UInt32
    /// Sequence number
    public var sequence: UInt64
    /// Timestamp (milliseconds since epoch)
    public var timestamp: UInt64
    /// CRC32 checksum of the payload
    public var checksum: UInt32
    /// Reserved for future use (4 bytes padding)
    private var _reserved: UInt32
    
    public init(messageType: MessageType, payload: Data) {
        self.magic = SHARED_MEMORY_MAGIC
        self.version = SHARED_MEMORY_VERSION
        self.messageType = messageType.rawValue
        self.flags = 0
        self.size = UInt32(payload.count)
        self.sequence = 0 // Will be set by sender
        self.timestamp = UInt64(Date().timeIntervalSince1970 * 1000) // milliseconds
        self.checksum = Self.calculateCRC32(for: payload)
        self._reserved = 0
    }
    
    /// Validate the header
    public func validate() throws {
        guard magic == SHARED_MEMORY_MAGIC else {
            throw SharedMemoryError.protocolError("Invalid magic number: 0x\(String(magic, radix: 16))")
        }
        
        guard version == SHARED_MEMORY_VERSION else {
            throw SharedMemoryError.protocolError("Unsupported version: \(version)")
        }
        
        guard let _ = MessageType(rawValue: messageType) else {
            throw SharedMemoryError.protocolError("Invalid message type: \(messageType)")
        }
        
        guard size <= MAX_MESSAGE_SIZE else {
            throw SharedMemoryError.protocolError("Message size too large: \(size)")
        }
    }
    
    /// Verify payload checksum
    public func verifyChecksum(for payload: Data) -> Bool {
        return checksum == Self.calculateCRC32(for: payload)
    }
    
    /// Calculate CRC32 checksum
    private static func calculateCRC32(for data: Data) -> UInt32 {
        #if canImport(zlib)
        return data.withUnsafeBytes { (bytes: UnsafeRawBufferPointer) in
            let pointer = bytes.bindMemory(to: UInt8.self).baseAddress
            return UInt32(crc32(0, pointer, UInt32(bytes.count)))
        }
        #else
        // Fallback: simple hash for platforms without zlib
        return UInt32(data.hashValue & 0xFFFFFFFF)
        #endif
    }
}

// MARK: - Message

/// Complete shared memory message
public struct SharedMemoryMessage {
    public var header: MessageHeader
    public let payload: Data
    
    public init(type: MessageType, payload: Data) throws {
        guard payload.count <= MAX_MESSAGE_SIZE else {
            throw SharedMemoryError.messageTooLarge(payload.count)
        }
        
        self.header = MessageHeader(messageType: type, payload: payload)
        self.payload = payload
    }
    
    /// Create a data message
    public static func data(_ payload: Data) throws -> SharedMemoryMessage {
        return try SharedMemoryMessage(type: .data, payload: payload)
    }
    
    /// Create a heartbeat message
    public static func heartbeat() throws -> SharedMemoryMessage {
        return try SharedMemoryMessage(type: .heartbeat, payload: Data())
    }
    
    /// Create an acknowledgment message
    public static func acknowledgment(sequence: UInt64) throws -> SharedMemoryMessage {
        var sequenceData = Data(count: 8)
        sequenceData.withUnsafeMutableBytes { bytes in
            bytes.storeBytes(of: sequence.littleEndian, as: UInt64.self)
        }
        return try SharedMemoryMessage(type: .acknowledgment, payload: sequenceData)
    }
    
    /// Get total message size (header + payload)
    public var totalSize: Int {
        return MESSAGE_HEADER_SIZE + payload.count
    }
    
    /// Validate the complete message
    public func validate() throws {
        try header.validate()
        
        guard header.verifyChecksum(for: payload) else {
            throw SharedMemoryError.dataCorruption("Checksum mismatch")
        }
    }
    
    /// Set sequence number
    public mutating func setSequence(_ sequence: UInt64) {
        self.header.sequence = sequence
    }
}

// MARK: - Ring Buffer

/// Ring buffer for shared memory communication
public struct RingBuffer {
    /// Buffer capacity
    public let capacity: UInt64
    /// Write position (atomic)
    public private(set) var writePosition: UInt64
    /// Read position (atomic)
    public private(set) var readPosition: UInt64
    /// Number of available bytes (atomic)
    public private(set) var availableBytes: UInt64
    
    public init(capacity: UInt64) {
        self.capacity = capacity
        self.writePosition = 0
        self.readPosition = 0
        self.availableBytes = 0
    }
    
    /// Get available space for writing
    public var availableWriteSpace: UInt64 {
        return capacity - availableBytes
    }
    
    /// Get available data for reading
    public var availableReadData: UInt64 {
        return availableBytes
    }
    
    /// Check if buffer is empty
    public var isEmpty: Bool {
        return availableBytes == 0
    }
    
    /// Check if buffer is full
    public var isFull: Bool {
        return availableBytes == capacity
    }
    
    /// Calculate next position with wrap-around
    public func nextPosition(_ position: UInt64, offset: UInt64) -> UInt64 {
        return (position + offset) % capacity
    }
}

// MARK: - Cross-Language Serialization

/// Serializable message for cross-language communication
public struct SerializableMessage: Codable {
    public let messageType: UInt8
    public let sequence: UInt64
    public let timestamp: UInt64
    public let payload: Data
    
    public init(from message: SharedMemoryMessage) {
        self.messageType = message.header.messageType
        self.sequence = message.header.sequence
        self.timestamp = message.header.timestamp
        self.payload = message.payload
    }
    
    public func toSharedMemoryMessage() throws -> SharedMemoryMessage {
        guard let type = MessageType(rawValue: messageType) else {
            throw SharedMemoryError.protocolError("Invalid message type: \(messageType)")
        }
        
        var message = try SharedMemoryMessage(type: type, payload: payload)
        message.setSequence(sequence)
        // Note: timestamp will be reset in header init, could preserve if needed
        
        return message
    }
}

// MARK: - Error Types

/// Shared memory specific errors
public enum SharedMemoryError: Error, LocalizedError {
    case regionNotFound(String)
    case regionCreationFailed(String)
    case mappingFailed(String)
    case protocolError(String)
    case dataCorruption(String)
    case messageTooLarge(Int)
    case bufferFull
    case bufferEmpty
    case timeout(TimeInterval)
    case permissionDenied(String)
    case platformError(String)
    
    public var errorDescription: String? {
        switch self {
        case .regionNotFound(let name):
            return "Shared memory region not found: \(name)"
        case .regionCreationFailed(let message):
            return "Failed to create shared memory region: \(message)"
        case .mappingFailed(let message):
            return "Failed to map shared memory: \(message)"
        case .protocolError(let message):
            return "Protocol error: \(message)"
        case .dataCorruption(let message):
            return "Data corruption detected: \(message)"
        case .messageTooLarge(let size):
            return "Message too large: \(size) bytes (max: \(MAX_MESSAGE_SIZE))"
        case .bufferFull:
            return "Ring buffer is full"
        case .bufferEmpty:
            return "Ring buffer is empty"
        case .timeout(let duration):
            return "Shared memory operation timed out after \(duration) seconds"
        case .permissionDenied(let message):
            return "Permission denied: \(message)"
        case .platformError(let message):
            return "Platform-specific error: \(message)"
        }
    }
}
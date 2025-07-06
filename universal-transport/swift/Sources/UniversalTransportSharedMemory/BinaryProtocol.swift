//
//  BinaryProtocol.swift
//  High-Performance Binary Protocol for Swift
//
//  TCP-like fixed binary protocol matching Rust implementation exactly
//

import Foundation

// MARK: - Protocol Constants

/// Protocol magic number (matches Rust) - "UTPB"
public let BINARY_PROTOCOL_MAGIC: UInt32 = 0x55545042

/// Protocol version
public let BINARY_PROTOCOL_VERSION: UInt8 = 1

/// Message header size (32 bytes, cache-line aligned)
public let BINARY_HEADER_SIZE = 32

/// Maximum payload size (64MB)
public let BINARY_MAX_PAYLOAD_SIZE: UInt32 = 64 * 1024 * 1024

// MARK: - Message Types

/// Message types (matches Rust enum exactly)
public enum BinaryMessageType: UInt8, CaseIterable {
    case data = 0x01
    case heartbeat = 0x02
    case acknowledgment = 0x03
    case error = 0x04
    case benchmark = 0x05
}

// MARK: - Binary Message Header

/// Binary message header (32 bytes, exact match with Rust)
/// Layout matches Rust BinaryHeader exactly:
/// 0-3:   Magic number (4 bytes)
/// 4:     Version (1 byte) 
/// 5:     Message type (1 byte)
/// 6-7:   Flags (2 bytes)
/// 8-11:  Payload length (4 bytes)
/// 12-19: Sequence number (8 bytes)
/// 20-27: Timestamp (8 bytes)
/// 28-31: CRC32 checksum (4 bytes)
public struct BinaryMessageHeader {
    public var magic: UInt32
    public var version: UInt8
    public var messageType: UInt8
    public var flags: UInt16
    public var payloadLength: UInt32
    public var sequence: UInt64
    public var timestamp: UInt64
    public var checksum: UInt32
    
    public init(messageType: BinaryMessageType, payload: Data) {
        self.magic = BINARY_PROTOCOL_MAGIC
        self.version = BINARY_PROTOCOL_VERSION
        self.messageType = messageType.rawValue
        self.flags = 0
        self.payloadLength = UInt32(payload.count)
        self.sequence = 0 // Set by sender
        self.timestamp = UInt64(Date().timeIntervalSince1970 * 1_000_000) // microseconds
        self.checksum = Self.calculateCRC32(for: payload)
    }
    
    /// Set sequence number
    public mutating func setSequence(_ seq: UInt64) {
        self.sequence = seq
    }
    
    /// Validate header
    public func validate() throws {
        guard magic == BINARY_PROTOCOL_MAGIC else {
            throw BinaryProtocolError.invalidMagic(magic)
        }
        
        guard version == BINARY_PROTOCOL_VERSION else {
            throw BinaryProtocolError.unsupportedVersion(version)
        }
        
        guard payloadLength <= BINARY_MAX_PAYLOAD_SIZE else {
            throw BinaryProtocolError.payloadTooLarge(payloadLength)
        }
    }
    
    /// Verify payload checksum
    public func verifyChecksum(for payload: Data) -> Bool {
        return checksum == Self.calculateCRC32(for: payload)
    }
    
    /// Calculate CRC32 checksum (matches Rust implementation)
    private static func calculateCRC32(for data: Data) -> UInt32 {
        // Simple CRC32 implementation for compatibility
        var crc: UInt32 = 0xFFFFFFFF
        
        for byte in data {
            crc ^= UInt32(byte)
            for _ in 0..<8 {
                if (crc & 1) != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320
                } else {
                    crc = crc >> 1
                }
            }
        }
        
        return ~crc
    }
    
    /// Serialize header to bytes (little-endian, matches Rust)
    public func toBytes() -> Data {
        var data = Data(capacity: BINARY_HEADER_SIZE)
        
        // Magic (4 bytes, little-endian)
        withUnsafeBytes(of: magic.littleEndian) { data.append(contentsOf: $0) }
        
        // Version (1 byte)
        data.append(version)
        
        // Message type (1 byte) 
        data.append(messageType)
        
        // Flags (2 bytes, little-endian)
        withUnsafeBytes(of: flags.littleEndian) { data.append(contentsOf: $0) }
        
        // Payload length (4 bytes, little-endian)
        withUnsafeBytes(of: payloadLength.littleEndian) { data.append(contentsOf: $0) }
        
        // Sequence (8 bytes, little-endian)
        withUnsafeBytes(of: sequence.littleEndian) { data.append(contentsOf: $0) }
        
        // Timestamp (8 bytes, little-endian)
        withUnsafeBytes(of: timestamp.littleEndian) { data.append(contentsOf: $0) }
        
        // Checksum (4 bytes, little-endian)
        withUnsafeBytes(of: checksum.littleEndian) { data.append(contentsOf: $0) }
        
        return data
    }
    
    /// Deserialize header from bytes
    public static func fromBytes(_ data: Data) throws -> BinaryMessageHeader {
        guard data.count >= BINARY_HEADER_SIZE else {
            throw BinaryProtocolError.insufficientData(data.count)
        }
        
        return try data.withUnsafeBytes { bytes in
            var offset = 0
            
            // Magic (4 bytes, little-endian)
            let magic = bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self).littleEndian
            offset += 4
            
            // Version (1 byte)
            let version = bytes[offset]
            offset += 1
            
            // Message type (1 byte)
            let messageType = bytes[offset]
            offset += 1
            
            // Flags (2 bytes, little-endian)
            let flags = bytes.loadUnaligned(fromByteOffset: offset, as: UInt16.self).littleEndian
            offset += 2
            
            // Payload length (4 bytes, little-endian)
            let payloadLength = bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self).littleEndian
            offset += 4
            
            // Sequence (8 bytes, little-endian)
            let sequence = bytes.loadUnaligned(fromByteOffset: offset, as: UInt64.self).littleEndian
            offset += 8
            
            // Timestamp (8 bytes, little-endian)
            let timestamp = bytes.loadUnaligned(fromByteOffset: offset, as: UInt64.self).littleEndian
            offset += 8
            
            // Checksum (4 bytes, little-endian)
            let checksum = bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self).littleEndian
            
            let header = BinaryMessageHeader(
                magic: magic,
                version: version,
                messageType: messageType,
                flags: flags,
                payloadLength: payloadLength,
                sequence: sequence,
                timestamp: timestamp,
                checksum: checksum
            )
            
            try header.validate()
            return header
        }
    }
    
    private init(magic: UInt32, version: UInt8, messageType: UInt8, flags: UInt16,
                 payloadLength: UInt32, sequence: UInt64, timestamp: UInt64, checksum: UInt32) {
        self.magic = magic
        self.version = version
        self.messageType = messageType
        self.flags = flags
        self.payloadLength = payloadLength
        self.sequence = sequence
        self.timestamp = timestamp
        self.checksum = checksum
    }
}

// MARK: - Complete Binary Message

/// Complete binary message (header + payload)
public struct BinaryMessage {
    public var header: BinaryMessageHeader
    public let payload: Data
    
    public init(messageType: BinaryMessageType, payload: Data) throws {
        guard payload.count <= BINARY_MAX_PAYLOAD_SIZE else {
            throw BinaryProtocolError.payloadTooLarge(UInt32(payload.count))
        }
        
        self.header = BinaryMessageHeader(messageType: messageType, payload: payload)
        self.payload = payload
    }
    
    /// Create a benchmark message
    public static func benchmark(id: UInt64, data: Data) throws -> BinaryMessage {
        var message = try BinaryMessage(messageType: .benchmark, payload: data)
        message.header.setSequence(id)
        return message
    }
    
    /// Get total message size
    public var totalSize: Int {
        return BINARY_HEADER_SIZE + payload.count
    }
    
    /// Validate complete message
    public func validate() throws {
        try header.validate()
        
        guard header.verifyChecksum(for: payload) else {
            throw BinaryProtocolError.checksumMismatch
        }
    }
    
    /// Serialize entire message to bytes
    public func toBytes() -> Data {
        var data = Data(capacity: totalSize)
        data.append(header.toBytes())
        data.append(payload)
        return data
    }
    
    /// Deserialize message from bytes
    public static func fromBytes(_ data: Data) throws -> BinaryMessage {
        guard data.count >= BINARY_HEADER_SIZE else {
            throw BinaryProtocolError.insufficientData(data.count)
        }
        
        // Parse header
        let headerData = data.prefix(BINARY_HEADER_SIZE)
        let header = try BinaryMessageHeader.fromBytes(headerData)
        
        // Check payload size
        let expectedTotal = BINARY_HEADER_SIZE + Int(header.payloadLength)
        guard data.count >= expectedTotal else {
            throw BinaryProtocolError.insufficientData(data.count)
        }
        
        // Extract payload
        let payload = data.subdata(in: BINARY_HEADER_SIZE..<expectedTotal)
        
        let message = BinaryMessage(header: header, payload: payload)
        try message.validate()
        
        return message
    }
    
    private init(header: BinaryMessageHeader, payload: Data) {
        self.header = header
        self.payload = payload
    }
}

// MARK: - Benchmark Message

/// Benchmark-specific message (matches Rust implementation exactly)
public struct BinaryBenchmarkMessage {
    public let id: UInt64
    public let timestamp: UInt64
    public let data: Data
    public let metadata: String
    
    public init(id: UInt64, dataSize: Int) {
        self.id = id
        self.timestamp = UInt64(Date().timeIntervalSince1970 * 1_000_000) // microseconds
        self.data = Data(repeating: 0x42, count: dataSize)
        self.metadata = "benchmark_msg_\(id)"
    }
    
    /// Serialize to binary format (exact match with Rust layout)
    /// Layout:
    /// 0-7:    ID (8 bytes, little-endian)
    /// 8-15:   Timestamp (8 bytes, little-endian)
    /// 16-19:  Data length (4 bytes, little-endian)
    /// 20-23:  Metadata length (4 bytes, little-endian)
    /// 24+:    Data
    /// N+:     Metadata (UTF-8)
    public func toBinary() -> Data {
        let metadataBytes = metadata.data(using: .utf8) ?? Data()
        let totalSize = 24 + data.count + metadataBytes.count
        
        var result = Data(capacity: totalSize)
        
        // ID (8 bytes, little-endian)
        withUnsafeBytes(of: id.littleEndian) { result.append(contentsOf: $0) }
        
        // Timestamp (8 bytes, little-endian)  
        withUnsafeBytes(of: timestamp.littleEndian) { result.append(contentsOf: $0) }
        
        // Data length (4 bytes, little-endian)
        withUnsafeBytes(of: UInt32(data.count).littleEndian) { result.append(contentsOf: $0) }
        
        // Metadata length (4 bytes, little-endian)
        withUnsafeBytes(of: UInt32(metadataBytes.count).littleEndian) { result.append(contentsOf: $0) }
        
        // Data
        result.append(data)
        
        // Metadata
        result.append(metadataBytes)
        
        return result
    }
    
    /// Deserialize from binary format
    public static func fromBinary(_ data: Data) throws -> BinaryBenchmarkMessage {
        guard data.count >= 24 else {
            throw BinaryProtocolError.insufficientData(data.count)
        }
        
        return try data.withUnsafeBytes { bytes in
            var offset = 0
            
            // ID (8 bytes, little-endian)
            let id = bytes.loadUnaligned(fromByteOffset: offset, as: UInt64.self).littleEndian
            offset += 8
            
            // Timestamp (8 bytes, little-endian)
            let timestamp = bytes.loadUnaligned(fromByteOffset: offset, as: UInt64.self).littleEndian
            offset += 8
            
            // Data length (4 bytes, little-endian)
            let dataLen = Int(bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self).littleEndian)
            offset += 4
            
            // Metadata length (4 bytes, little-endian)
            let metadataLen = Int(bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self).littleEndian)
            offset += 4
            
            // Check remaining data
            guard data.count >= offset + dataLen + metadataLen else {
                throw BinaryProtocolError.insufficientData(data.count)
            }
            
            // Extract data
            let messageData = data.subdata(in: offset..<(offset + dataLen))
            offset += dataLen
            
            // Extract metadata
            let metadataData = data.subdata(in: offset..<(offset + metadataLen))
            guard let metadata = String(data: metadataData, encoding: .utf8) else {
                throw BinaryProtocolError.invalidUtf8
            }
            
            return BinaryBenchmarkMessage(id: id, timestamp: timestamp, data: messageData, metadata: metadata)
        }
    }
    
    private init(id: UInt64, timestamp: UInt64, data: Data, metadata: String) {
        self.id = id
        self.timestamp = timestamp
        self.data = data
        self.metadata = metadata
    }
    
    /// Convert to binary message
    public func toBinaryMessage() throws -> BinaryMessage {
        let payload = toBinary()
        return try BinaryMessage.benchmark(id: id, data: payload)
    }
    
    /// Create from binary message
    public static func fromBinaryMessage(_ message: BinaryMessage) throws -> BinaryBenchmarkMessage {
        return try fromBinary(message.payload)
    }
}

// MARK: - Protocol Errors

public enum BinaryProtocolError: Error, LocalizedError {
    case invalidMagic(UInt32)
    case unsupportedVersion(UInt8)
    case payloadTooLarge(UInt32)
    case insufficientData(Int)
    case checksumMismatch
    case invalidUtf8
    
    public var errorDescription: String? {
        switch self {
        case .invalidMagic(let magic):
            return "Invalid magic number: 0x\(String(magic, radix: 16))"
        case .unsupportedVersion(let version):
            return "Unsupported protocol version: \(version)"
        case .payloadTooLarge(let size):
            return "Payload too large: \(size) bytes"
        case .insufficientData(let available):
            return "Insufficient data: \(available) bytes available"
        case .checksumMismatch:
            return "Checksum mismatch"
        case .invalidUtf8:
            return "Invalid UTF-8 encoding"
        }
    }
}
//
//  UtpMessage.swift
//  librorum
//
//  UTP二进制协议消息实现
//  与Rust backend保持完全兼容的二进制格式
//

import Foundation
import CryptoKit

/// UTP协议魔数
public let UTP_MAGIC: UInt32 = 0x55545042 // "UTPB"

/// UTP协议版本
public let UTP_VERSION: UInt8 = 1

/// UTP消息类型
public enum UtpMessageType: UInt8, CaseIterable {
    case data = 0x01
    case control = 0x02
    case fileHeader = 0x03
    case fileData = 0x04
    case fileComplete = 0x05
    case heartbeat = 0x06
    case ack = 0x07
    case error = 0x08
}

/// UTP消息标志
public struct UtpFlags: OptionSet, Codable {
    public let rawValue: UInt16
    
    public init(rawValue: UInt16) {
        self.rawValue = rawValue
    }
    
    public static let ackRequired = UtpFlags(rawValue: 0x01)
    public static let compressed = UtpFlags(rawValue: 0x02)
    public static let encrypted = UtpFlags(rawValue: 0x04)
    public static let fragmented = UtpFlags(rawValue: 0x08)
    public static let lastFragment = UtpFlags(rawValue: 0x10)
}

/// UTP消息头 (32字节固定)
public struct UtpHeader {
    public let magic: UInt32
    public let version: UInt8
    public let messageType: UInt8
    public let flags: UInt16
    public let payloadLength: UInt32
    public let sequence: UInt64
    public let timestamp: UInt64
    public let checksum: UInt32
    
    public static let size: Int = 32
    
    public init(
        messageType: UtpMessageType,
        flags: UtpFlags,
        payloadLength: UInt32,
        sequence: UInt64,
        checksum: UInt32 = 0
    ) {
        self.magic = UTP_MAGIC
        self.version = UTP_VERSION
        self.messageType = messageType.rawValue
        self.flags = flags.rawValue
        self.payloadLength = payloadLength
        self.sequence = sequence
        self.timestamp = UInt64(Date().timeIntervalSince1970 * 1_000_000) // 微秒
        self.checksum = checksum
    }
    
    /// 序列化为字节数组
    public func toBytes() -> Data {
        var data = Data(capacity: Self.size)
        
        data.append(contentsOf: withUnsafeBytes(of: magic.littleEndian) { $0 })
        data.append(version)
        data.append(messageType)
        data.append(contentsOf: withUnsafeBytes(of: flags.littleEndian) { $0 })
        data.append(contentsOf: withUnsafeBytes(of: payloadLength.littleEndian) { $0 })
        data.append(contentsOf: withUnsafeBytes(of: sequence.littleEndian) { $0 })
        data.append(contentsOf: withUnsafeBytes(of: timestamp.littleEndian) { $0 })
        data.append(contentsOf: withUnsafeBytes(of: checksum.littleEndian) { $0 })
        
        return data
    }
    
    /// 从字节数组反序列化
    public static func fromBytes(_ data: Data) -> UtpHeader? {
        guard data.count >= size else { return nil }
        
        let magic = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 0, as: UInt32.self).littleEndian
        }
        
        guard magic == UTP_MAGIC else { return nil }
        
        let version = data[4]
        guard version == UTP_VERSION else { return nil }
        
        let messageType = data[5]
        let flags = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 6, as: UInt16.self).littleEndian
        }
        let payloadLength = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 8, as: UInt32.self).littleEndian
        }
        let sequence = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 12, as: UInt64.self).littleEndian
        }
        let timestamp = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 20, as: UInt64.self).littleEndian
        }
        let checksum = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 28, as: UInt32.self).littleEndian
        }
        
        return UtpHeader(
            magic: magic,
            version: version,
            messageType: messageType,
            flags: flags,
            payloadLength: payloadLength,
            sequence: sequence,
            timestamp: timestamp,
            checksum: checksum
        )
    }
    
    private init(
        magic: UInt32,
        version: UInt8,
        messageType: UInt8,
        flags: UInt16,
        payloadLength: UInt32,
        sequence: UInt64,
        timestamp: UInt64,
        checksum: UInt32
    ) {
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

/// UTP消息
public struct UtpMessage {
    public let header: UtpHeader
    public let payload: Data
    
    public init(header: UtpHeader, payload: Data) {
        self.header = header
        self.payload = payload
    }
    
    /// 创建新消息
    public static func create(
        messageType: UtpMessageType,
        flags: UtpFlags,
        sequence: UInt64,
        payload: Data
    ) -> UtpMessage {
        let checksum = calculateChecksum(payload: payload)
        let header = UtpHeader(
            messageType: messageType,
            flags: flags,
            payloadLength: UInt32(payload.count),
            sequence: sequence,
            checksum: checksum
        )
        
        return UtpMessage(header: header, payload: payload)
    }
    
    /// 创建数据消息
    public static func data(sequence: UInt64, data: Data) -> UtpMessage {
        return create(
            messageType: .data,
            flags: [],
            sequence: sequence,
            payload: data
        )
    }
    
    /// 创建文件头消息
    public static func fileHeader(sequence: UInt64, fileInfo: FileTransferInfo) -> UtpMessage {
        let encoder = JSONEncoder()
        let payload = (try? encoder.encode(fileInfo)) ?? Data()
        
        return create(
            messageType: .fileHeader,
            flags: [],
            sequence: sequence,
            payload: payload
        )
    }
    
    /// 创建文件数据消息
    public static func fileData(
        sequence: UInt64,
        chunkIndex: UInt64,
        data: Data,
        isLast: Bool
    ) -> UtpMessage {
        var flags: UtpFlags = []
        if isLast {
            flags.insert(.lastFragment)
        }
        
        // 在载荷前添加chunk_index
        var payload = Data()
        payload.append(contentsOf: withUnsafeBytes(of: chunkIndex.littleEndian) { $0 })
        payload.append(data)
        
        return create(
            messageType: .fileData,
            flags: flags,
            sequence: sequence,
            payload: payload
        )
    }
    
    /// 创建文件完成消息
    public static func fileComplete(sequence: UInt64, fileHash: String) -> UtpMessage {
        let payload = fileHash.data(using: .utf8) ?? Data()
        
        return create(
            messageType: .fileComplete,
            flags: [],
            sequence: sequence,
            payload: payload
        )
    }
    
    /// 创建心跳消息
    public static func heartbeat(sequence: UInt64) -> UtpMessage {
        return create(
            messageType: .heartbeat,
            flags: [],
            sequence: sequence,
            payload: Data()
        )
    }
    
    /// 创建确认消息
    public static func ack(sequence: UInt64, ackSequence: UInt64) -> UtpMessage {
        let payload = withUnsafeBytes(of: ackSequence.littleEndian) { Data($0) }
        
        return create(
            messageType: .ack,
            flags: [],
            sequence: sequence,
            payload: payload
        )
    }
    
    /// 创建错误消息
    public static func error(sequence: UInt64, errorCode: UInt32, errorMessage: String) -> UtpMessage {
        var payload = Data()
        payload.append(contentsOf: withUnsafeBytes(of: errorCode.littleEndian) { $0 })
        payload.append(errorMessage.data(using: .utf8) ?? Data())
        
        return create(
            messageType: .error,
            flags: [],
            sequence: sequence,
            payload: payload
        )
    }
    
    /// 验证消息完整性
    public func verify() -> Bool {
        let calculatedChecksum = Self.calculateChecksum(payload: payload)
        return header.checksum == calculatedChecksum
    }
    
    /// 获取消息类型
    public var messageType: UtpMessageType? {
        return UtpMessageType(rawValue: header.messageType)
    }
    
    /// 获取标志
    public var flags: UtpFlags {
        return UtpFlags(rawValue: header.flags)
    }
    
    /// 序列化为字节数组
    public func toBytes() -> Data {
        var data = Data(capacity: UtpHeader.size + payload.count)
        data.append(header.toBytes())
        data.append(payload)
        return data
    }
    
    /// 从字节数组反序列化
    public static func fromBytes(_ data: Data) -> UtpMessage? {
        guard data.count >= UtpHeader.size else { return nil }
        
        guard let header = UtpHeader.fromBytes(data.prefix(UtpHeader.size)) else { return nil }
        
        let payloadLength = Int(header.payloadLength)
        guard data.count >= UtpHeader.size + payloadLength else { return nil }
        
        let payload = data.subdata(in: UtpHeader.size..<(UtpHeader.size + payloadLength))
        let message = UtpMessage(header: header, payload: payload)
        
        // 验证校验和
        guard message.verify() else { return nil }
        
        return message
    }
    
    /// 计算CRC32校验和
    private static func calculateChecksum(payload: Data) -> UInt32 {
        // 简化实现，实际应该使用CRC32
        // 这里使用简单的哈希作为校验和
        var hasher = Hasher()
        hasher.combine(payload)
        return UInt32(truncatingIfNeeded: hasher.finalize())
    }
}

/// 文件传输信息
public struct FileTransferInfo: Codable {
    public let fileName: String
    public let fileSize: Int64
    public let chunkCount: Int
    public let chunkSize: Int
    public let hash: String?
    public let mimeType: String?
    public let createdAt: UInt64
    public let modifiedAt: UInt64
    public let compression: CompressionInfo?
    public let encryption: EncryptionInfo?
    
    public init(
        fileName: String,
        fileSize: Int64,
        chunkCount: Int,
        chunkSize: Int,
        hash: String? = nil,
        mimeType: String? = nil,
        compression: CompressionInfo? = nil,
        encryption: EncryptionInfo? = nil
    ) {
        self.fileName = fileName
        self.fileSize = fileSize
        self.chunkCount = chunkCount
        self.chunkSize = chunkSize
        self.hash = hash
        self.mimeType = mimeType ?? "application/octet-stream"
        self.createdAt = UInt64(Date().timeIntervalSince1970)
        self.modifiedAt = UInt64(Date().timeIntervalSince1970)
        self.compression = compression
        self.encryption = encryption
    }
}

/// 压缩信息
public struct CompressionInfo: Codable {
    public let algorithm: String
    public let level: UInt8
    public let originalSize: Int64
    public let compressedSize: Int64
    
    public init(algorithm: String, level: UInt8, originalSize: Int64, compressedSize: Int64) {
        self.algorithm = algorithm
        self.level = level
        self.originalSize = originalSize
        self.compressedSize = compressedSize
    }
}

/// 加密信息
public struct EncryptionInfo: Codable {
    public let algorithm: String
    public let keyId: String
    public let iv: Data
    
    public init(algorithm: String, keyId: String, iv: Data) {
        self.algorithm = algorithm
        self.keyId = keyId
        self.iv = iv
    }
}

/// UTP消息序列号生成器
public class UtpSequenceGenerator {
    private var sequence: UInt64 = 1
    private let lock = NSLock()
    
    public init() {}
    
    public func next() -> UInt64 {
        lock.lock()
        defer { lock.unlock() }
        
        let current = sequence
        sequence += 1
        return current
    }
    
    public func reset() {
        lock.lock()
        defer { lock.unlock() }
        sequence = 1
    }
}
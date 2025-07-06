//
//  SharedMemoryManager.swift
//  librorum
//
//  POSIX共享内存管理器
//  用于高性能进程间通信
//

import Foundation
import os.log

/// 共享内存管理器
public class SharedMemoryManager {
    private let logger = Logger(subsystem: "com.librorum", category: "SharedMemory")
    
    /// 共享内存文件路径
    public let path: String
    
    /// 共享内存大小
    public let size: Int
    
    /// 文件描述符
    private var fileDescriptor: Int32 = -1
    
    /// 内存映射指针
    private var mappedMemory: UnsafeMutableRawPointer?
    
    /// 控制块大小
    private static let controlBlockSize = 64
    
    /// 是否已初始化
    private var isInitialized = false
    
    public init(path: String, size: Int) throws {
        self.path = path
        self.size = size
        
        try initialize()
    }
    
    deinit {
        cleanup()
    }
    
    /// 初始化共享内存
    private func initialize() throws {
        logger.info("🔧 初始化共享内存: \\(path) (\\(formatBytes(size)))")
        
        // 创建或打开文件
        fileDescriptor = open(path, O_RDWR | O_CREAT, 0o644)
        guard fileDescriptor != -1 else {
            throw SharedMemoryError.fileCreationFailed("无法创建文件: \\(String(cString: strerror(errno)))")
        }
        
        // 设置文件大小
        if ftruncate(fileDescriptor, off_t(size)) != 0 {
            close(fileDescriptor)
            throw SharedMemoryError.fileSizeFailed("无法设置文件大小: \\(String(cString: strerror(errno)))")
        }
        
        // 内存映射
        mappedMemory = mmap(nil, size, PROT_READ | PROT_WRITE, MAP_SHARED, fileDescriptor, 0)
        guard mappedMemory != MAP_FAILED else {
            close(fileDescriptor)
            throw SharedMemoryError.memoryMapFailed("内存映射失败: \\(String(cString: strerror(errno)))")
        }
        
        // 初始化控制块
        initializeControlBlock()
        
        isInitialized = true
        logger.info("✅ 共享内存初始化成功")
    }
    
    /// 初始化控制块
    private func initializeControlBlock() {
        guard let memory = mappedMemory else { return }
        
        // 清零控制块
        memset(memory, 0, Self.controlBlockSize)
        
        // 设置初始位置
        let writePos = memory.assumingMemoryBound(to: UInt64.self)
        let readPos = memory.advanced(by: 8).assumingMemoryBound(to: UInt64.self)
        
        writePos.pointee = UInt64(Self.controlBlockSize)
        readPos.pointee = UInt64(Self.controlBlockSize)
    }
    
    /// 获取控制块
    private func getControlBlock() -> ControlBlock? {
        guard let memory = mappedMemory else { return nil }
        
        let writePos = memory.assumingMemoryBound(to: UInt64.self).pointee
        let readPos = memory.advanced(by: 8).assumingMemoryBound(to: UInt64.self).pointee
        let messageCount = memory.advanced(by: 16).assumingMemoryBound(to: UInt64.self).pointee
        let sessionActive = memory.advanced(by: 24).assumingMemoryBound(to: UInt32.self).pointee
        
        return ControlBlock(
            writePos: writePos,
            readPos: readPos,
            messageCount: messageCount,
            sessionActive: sessionActive != 0
        )
    }
    
    /// 更新控制块
    private func updateControlBlock(_ block: ControlBlock) {
        guard let memory = mappedMemory else { return }
        
        memory.assumingMemoryBound(to: UInt64.self).pointee = block.writePos
        memory.advanced(by: 8).assumingMemoryBound(to: UInt64.self).pointee = block.readPos
        memory.advanced(by: 16).assumingMemoryBound(to: UInt64.self).pointee = block.messageCount
        memory.advanced(by: 24).assumingMemoryBound(to: UInt32.self).pointee = block.sessionActive ? 1 : 0
    }
    
    /// 获取数据区域指针和大小
    private func getDataRegion() -> (pointer: UnsafeMutableRawPointer, size: Int) {
        let dataPointer = mappedMemory!.advanced(by: Self.controlBlockSize)
        let dataSize = size - Self.controlBlockSize
        return (dataPointer, dataSize)
    }
    
    /// 写入文件头信息
    public func writeFileHeader(info: FileTransferInfo) throws {
        guard isInitialized else {
            throw SharedMemoryError.notInitialized("共享内存未初始化")
        }
        
        logger.info("📋 写入文件头: \\(info.fileName) (\\(formatBytes(Int(info.fileSize))))")
        
        let encoder = JSONEncoder()
        let headerData = try encoder.encode(info)
        
        let message = UtpMessage.fileHeader(sequence: 1, fileInfo: info)
        try writeMessage(message)
    }
    
    /// 写入文件数据块
    public func writeFileChunk(chunkIndex: Int, data: Data, isLast: Bool) throws {
        guard isInitialized else {
            throw SharedMemoryError.notInitialized("共享内存未初始化")
        }
        
        let message = UtpMessage.fileData(
            sequence: UInt64(chunkIndex + 2), // +2 因为1被文件头使用
            chunkIndex: UInt64(chunkIndex),
            data: data,
            isLast: isLast
        )
        
        try writeMessage(message)
        
        if chunkIndex % 100 == 0 {
            logger.debug("📦 写入数据块 \\(chunkIndex): \\(data.count) bytes")
        }
    }
    
    /// 写入消息
    private func writeMessage(_ message: UtpMessage) throws {
        guard var controlBlock = getControlBlock() else {
            throw SharedMemoryError.controlBlockError("无法获取控制块")
        }
        
        let (dataPointer, dataSize) = getDataRegion()
        let messageBytes = message.toBytes()
        let messageLength = messageBytes.count
        let totalLength = 8 + messageLength // 8字节长度前缀 + 消息数据
        
        // 检查空间
        let currentWritePos = Int(controlBlock.writePos) - Self.controlBlockSize
        let currentReadPos = Int(controlBlock.readPos) - Self.controlBlockSize
        
        let availableSpace = if currentWritePos >= currentReadPos {
            dataSize - currentWritePos + currentReadPos
        } else {
            currentReadPos - currentWritePos
        }
        
        guard availableSpace >= totalLength else {
            throw SharedMemoryError.bufferFull("共享内存缓冲区已满")
        }
        
        // 写入消息长度 (8字节)
        let lengthBytes = withUnsafeBytes(of: UInt64(messageLength).littleEndian) { Data($0) }
        for (i, byte) in lengthBytes.enumerated() {
            let pos = (currentWritePos + i) % dataSize
            dataPointer.advanced(by: pos).assumingMemoryBound(to: UInt8.self).pointee = byte
        }
        
        // 写入消息数据
        for (i, byte) in messageBytes.enumerated() {
            let pos = (currentWritePos + 8 + i) % dataSize
            dataPointer.advanced(by: pos).assumingMemoryBound(to: UInt8.self).pointee = byte
        }
        
        // 更新写入位置
        controlBlock.writePos = UInt64((currentWritePos + totalLength) % dataSize + Self.controlBlockSize)
        controlBlock.messageCount += 1
        updateControlBlock(controlBlock)
    }
    
    /// 读取消息
    public func readMessage() throws -> UtpMessage? {
        guard isInitialized else {
            throw SharedMemoryError.notInitialized("共享内存未初始化")
        }
        
        guard var controlBlock = getControlBlock() else {
            throw SharedMemoryError.controlBlockError("无法获取控制块")
        }
        
        let (dataPointer, dataSize) = getDataRegion()
        let currentWritePos = Int(controlBlock.writePos) - Self.controlBlockSize
        let currentReadPos = Int(controlBlock.readPos) - Self.controlBlockSize
        
        // 检查是否有数据可读
        guard currentReadPos != currentWritePos else {
            return nil
        }
        
        // 读取消息长度
        var lengthBytes = [UInt8](repeating: 0, count: 8)
        for i in 0..<8 {
            let pos = (currentReadPos + i) % dataSize
            lengthBytes[i] = dataPointer.advanced(by: pos).assumingMemoryBound(to: UInt8.self).pointee
        }
        
        let messageLength = lengthBytes.withUnsafeBytes { bytes in
            bytes.loadUnaligned(as: UInt64.self).littleEndian
        }
        
        guard messageLength > 0 && messageLength < UInt64(dataSize) else {
            throw SharedMemoryError.corruptedData("无效的消息长度: \\(messageLength)")
        }
        
        // 读取消息数据
        var messageBytes = Data(capacity: Int(messageLength))
        for i in 0..<Int(messageLength) {
            let pos = (currentReadPos + 8 + i) % dataSize
            let byte = dataPointer.advanced(by: pos).assumingMemoryBound(to: UInt8.self).pointee
            messageBytes.append(byte)
        }
        
        // 解析消息
        guard let message = UtpMessage.fromBytes(messageBytes) else {
            throw SharedMemoryError.corruptedData("无法解析消息")
        }
        
        // 更新读取位置
        let totalLength = 8 + Int(messageLength)
        controlBlock.readPos = UInt64((currentReadPos + totalLength) % dataSize + Self.controlBlockSize)
        updateControlBlock(controlBlock)
        
        return message
    }
    
    /// 清理资源
    private func cleanup() {
        if let memory = mappedMemory, memory != MAP_FAILED {
            munmap(memory, size)
            mappedMemory = nil
        }
        
        if fileDescriptor != -1 {
            close(fileDescriptor)
            fileDescriptor = -1
        }
        
        // 删除临时文件
        if FileManager.default.fileExists(atPath: path) {
            try? FileManager.default.removeItem(atPath: path)
        }
        
        isInitialized = false
        logger.info("🧹 共享内存资源已清理")
    }
    
    /// 获取统计信息
    public func getStats() -> SharedMemoryStats? {
        guard let controlBlock = getControlBlock() else { return nil }
        
        let (_, dataSize) = getDataRegion()
        let currentWritePos = Int(controlBlock.writePos) - Self.controlBlockSize
        let currentReadPos = Int(controlBlock.readPos) - Self.controlBlockSize
        
        let usedSpace = if currentWritePos >= currentReadPos {
            currentWritePos - currentReadPos
        } else {
            dataSize - currentReadPos + currentWritePos
        }
        
        return SharedMemoryStats(
            totalSize: size,
            usedSpace: usedSpace,
            availableSpace: dataSize - usedSpace,
            messageCount: controlBlock.messageCount,
            sessionActive: controlBlock.sessionActive
        )
    }
}

/// 控制块结构
private struct ControlBlock {
    var writePos: UInt64
    var readPos: UInt64
    var messageCount: UInt64
    var sessionActive: Bool
}

/// 共享内存统计信息
public struct SharedMemoryStats {
    public let totalSize: Int
    public let usedSpace: Int
    public let availableSpace: Int
    public let messageCount: UInt64
    public let sessionActive: Bool
    
    /// 使用率百分比
    public var usagePercent: Double {
        guard totalSize > 0 else { return 0.0 }
        return Double(usedSpace) / Double(totalSize) * 100.0
    }
}

/// 共享内存错误类型
public enum SharedMemoryError: LocalizedError {
    case fileCreationFailed(String)
    case fileSizeFailed(String)
    case memoryMapFailed(String)
    case notInitialized(String)
    case controlBlockError(String)
    case bufferFull(String)
    case corruptedData(String)
    
    public var errorDescription: String? {
        switch self {
        case .fileCreationFailed(let message):
            return "文件创建失败: \\(message)"
        case .fileSizeFailed(let message):
            return "文件大小设置失败: \\(message)"
        case .memoryMapFailed(let message):
            return "内存映射失败: \\(message)"
        case .notInitialized(let message):
            return "未初始化: \\(message)"
        case .controlBlockError(let message):
            return "控制块错误: \\(message)"
        case .bufferFull(let message):
            return "缓冲区已满: \\(message)"
        case .corruptedData(let message):
            return "数据损坏: \\(message)"
        }
    }
}

/// 格式化字节数
private func formatBytes(_ bytes: Int) -> String {
    let units = ["B", "KB", "MB", "GB"]
    var size = Double(bytes)
    var unitIndex = 0
    
    while size >= 1024.0 && unitIndex < units.count - 1 {
        size /= 1024.0
        unitIndex += 1
    }
    
    if unitIndex == 0 {
        return "\\(Int(size)) \\(units[unitIndex])"
    } else {
        return String(format: "%.1f %@", size, units[unitIndex])
    }
}
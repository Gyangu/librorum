#!/usr/bin/env swift

//! Swift POSIX共享内存客户端
//! 与Rust服务器进行真正的进程间共享内存通信

import Foundation
import Darwin

/// UTP消息头（与Rust完全兼容）
struct UtpMessageHeader {
    let magic: UInt32 = 0x55545042
    let version: UInt8 = 1
    let messageType: UInt8
    let flags: UInt16 = 0
    let payloadLength: UInt32
    let sequence: UInt64
    let timestamp: UInt64
    let checksum: UInt32 = 0
    
    static let size = 32
    
    init(messageType: UInt8, payloadLength: UInt32, sequence: UInt64) {
        self.messageType = messageType
        self.payloadLength = payloadLength
        self.sequence = sequence
        self.timestamp = UInt64(Date().timeIntervalSince1970 * 1_000_000) // 微秒
    }
    
    func toBytes() -> Data {
        var data = Data()
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
    
    static func fromBytes(_ data: Data, offset: Int = 0) -> UtpMessageHeader? {
        guard data.count >= offset + size else { return nil }
        
        let magic = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: offset, as: UInt32.self).littleEndian
        }
        guard magic == 0x55545042 else { return nil }
        
        let messageType = data[offset + 5]
        let payloadLength = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: offset + 8, as: UInt32.self).littleEndian
        }
        let sequence = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: offset + 12, as: UInt64.self).littleEndian
        }
        let timestamp = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: offset + 20, as: UInt64.self).littleEndian
        }
        
        return UtpMessageHeader(
            messageType: messageType,
            payloadLength: payloadLength,
            sequence: sequence,
            timestamp: timestamp
        )
    }
    
    private init(messageType: UInt8, payloadLength: UInt32, sequence: UInt64, timestamp: UInt64) {
        self.messageType = messageType
        self.payloadLength = payloadLength
        self.sequence = sequence
        self.timestamp = timestamp
    }
}

/// 共享内存控制块（与Rust兼容）
class SharedControl {
    private let ptr: UnsafeMutableRawPointer
    static let size = 64
    
    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
    
    var writePos: UInt64 {
        get { ptr.load(fromByteOffset: 0, as: UInt64.self) }
        set { ptr.storeBytes(of: newValue, toByteOffset: 0, as: UInt64.self) }
    }
    
    var readPos: UInt64 {
        get { ptr.load(fromByteOffset: 8, as: UInt64.self) }
        set { ptr.storeBytes(of: newValue, toByteOffset: 8, as: UInt64.self) }
    }
    
    var messageCount: UInt64 {
        get { ptr.load(fromByteOffset: 16, as: UInt64.self) }
        set { ptr.storeBytes(of: newValue, toByteOffset: 16, as: UInt64.self) }
    }
    
    var serverStatus: UInt32 {
        get { ptr.load(fromByteOffset: 24, as: UInt32.self) }
        set { ptr.storeBytes(of: newValue, toByteOffset: 24, as: UInt32.self) }
    }
    
    var clientStatus: UInt32 {
        get { ptr.load(fromByteOffset: 28, as: UInt32.self) }
        set { ptr.storeBytes(of: newValue, toByteOffset: 28, as: UInt32.self) }
    }
}

/// Swift POSIX共享内存客户端
class PosixSharedMemoryClient {
    private let name: String
    private let size: Int
    private var fd: Int32 = -1
    private var ptr: UnsafeMutableRawPointer?
    private var control: SharedControl?
    
    init(name: String, size: Int) {
        self.name = name
        self.size = size
    }
    
    func connect() -> Bool {
        // 打开共享内存
        fd = shm_open(name, O_RDWR, 0)
        if fd == -1 {
            print("❌ 无法打开共享内存 \(name)")
            print("   请确保Rust服务器已启动")
            return false
        }
        
        // 映射内存
        ptr = mmap(nil, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)
        if ptr == MAP_FAILED {
            print("❌ 内存映射失败")
            close(fd)
            return false
        }
        
        control = SharedControl(ptr: ptr!)
        
        // 设置客户端状态为已连接
        control!.clientStatus = 1
        
        print("✅ 成功连接到共享内存")
        print("   地址: \(ptr!)")
        print("   大小: \(size) bytes")
        return true
    }
    
    func getDataPtr() -> UnsafeMutableRawPointer {
        return ptr!.advanced(by: SharedControl.size)
    }
    
    func getDataSize() -> Int {
        return size - SharedControl.size
    }
    
    func writeMessage(messageType: UInt8, payload: Data) -> Bool {
        guard let control = control else { return false }
        
        let dataPtr = getDataPtr()
        let dataSize = getDataSize()
        let totalSize = UtpMessageHeader.size + payload.count
        
        if totalSize > dataSize {
            print("❌ 消息太大")
            return false
        }
        
        let writePos = control.writePos
        let readPos = control.readPos
        
        let available = writePos >= readPos ? 
            dataSize - Int(writePos - readPos) : 
            Int(readPos - writePos)
        
        if totalSize > available {
            print("❌ 缓冲区已满")
            return false
        }
        
        let sequence = control.messageCount
        control.messageCount = sequence + 1
        
        let header = UtpMessageHeader(
            messageType: messageType,
            payloadLength: UInt32(payload.count),
            sequence: sequence
        )
        
        let writeOffset = Int(writePos % UInt64(dataSize))
        
        // 写入消息头
        let headerData = header.toBytes()
        headerData.withUnsafeBytes { bytes in
            dataPtr.advanced(by: writeOffset).copyMemory(from: bytes.baseAddress!, byteCount: UtpMessageHeader.size)
        }
        
        // 写入载荷
        if !payload.isEmpty {
            payload.withUnsafeBytes { bytes in
                dataPtr.advanced(by: writeOffset + UtpMessageHeader.size).copyMemory(from: bytes.baseAddress!, byteCount: payload.count)
            }
        }
        
        control.writePos = writePos + UInt64(totalSize)
        return true
    }
    
    func readMessage() -> (UtpMessageHeader, Data)? {
        guard let control = control else { return nil }
        
        let dataPtr = getDataPtr()
        let dataSize = getDataSize()
        
        let readPos = control.readPos
        let writePos = control.writePos
        
        if readPos >= writePos { return nil }
        
        let readOffset = Int(readPos % UInt64(dataSize))
        
        // 读取消息头
        let headerData = Data(bytes: dataPtr.advanced(by: readOffset), count: UtpMessageHeader.size)
        guard let header = UtpMessageHeader.fromBytes(headerData) else {
            print("❌ 无效消息头")
            return nil
        }
        
        // 读取载荷
        let payload = header.payloadLength > 0 ? 
            Data(bytes: dataPtr.advanced(by: readOffset + UtpMessageHeader.size), count: Int(header.payloadLength)) :
            Data()
        
        let totalSize = UtpMessageHeader.size + Int(header.payloadLength)
        control.readPos = readPos + UInt64(totalSize)
        
        return (header, payload)
    }
    
    func run() {
        print("🚀 Swift POSIX共享内存客户端启动")
        print("=================================")
        print("")
        
        guard connect() else {
            print("❌ 连接失败")
            return
        }
        
        print("📡 开始与Rust服务器通信...")
        print("")
        
        var messageId: UInt64 = 0
        
        // 运行通信循环
        while control?.serverStatus == 1 {
            // 读取Rust服务器消息
            while let (header, payload) = readMessage() {
                let content = String(data: payload, encoding: .utf8) ?? "<binary>"
                if header.messageType == 0x02 {
                    print("💓 收到Rust心跳")
                } else {
                    print("📨 收到Rust消息: type=0x\(String(format: "%02X", header.messageType)), seq=\(header.sequence), \"\(content)\"")
                }
            }
            
            // 发送回复消息
            let messages = [
                (0x01, "Swift→Rust 响应消息 #\(messageId)"),
                (0x03, "Swift确认 #\(messageId)"),
            ]
            
            for (msgType, content) in messages {
                let payloadData = content.data(using: .utf8) ?? Data()
                if writeMessage(messageType: UInt8(msgType), payload: payloadData) {
                    print("📤 发送: type=0x\(String(format: "%02X", msgType)), \"\(content)\"")
                } else {
                    print("❌ 发送失败: \(content)")
                }
            }
            
            messageId += 1
            Thread.sleep(forTimeInterval: 1.5)
        }
        
        print("🏁 服务器已停止，断开连接")
        disconnect()
    }
    
    func disconnect() {
        control?.clientStatus = 0
        
        if let ptr = ptr {
            munmap(ptr, size)
        }
        
        if fd != -1 {
            close(fd)
        }
        
        print("🔌 已断开连接")
    }
    
    deinit {
        disconnect()
    }
}

// 主程序
func main() {
    print("🌟 Swift端 POSIX共享内存客户端")
    print("==============================")
    print("")
    
    let client = PosixSharedMemoryClient(name: "/rust_swift_shm", size: 1024 * 1024)
    
    // 设置信号处理
    signal(SIGINT) { _ in
        print("\n🛑 收到中断信号，正在断开...")
        exit(0)
    }
    
    client.run()
}

main()
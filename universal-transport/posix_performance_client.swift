#!/usr/bin/env swift

//! Swift POSIX共享内存性能测试客户端
//! 与Rust服务器进行高频通信测试

import Foundation
import Darwin

struct PerfTestHeader {
    let magic: UInt32
    let messageId: UInt64
    let timestamp: UInt64
    let payloadSize: UInt32
    
    static let size = 20
    static let magic: UInt32 = 0x50455246 // "PERF"
    
    init(messageId: UInt64, payloadSize: UInt32) {
        self.magic = Self.magic
        self.messageId = messageId
        self.timestamp = UInt64(Date().timeIntervalSince1970 * 1_000_000_000) // 纳秒
        self.payloadSize = payloadSize
    }
    
    static func fromMemory(data: Data) -> PerfTestHeader? {
        guard data.count >= size else { return nil }
        
        return data.withUnsafeBytes { bytes -> PerfTestHeader? in
            let magic = bytes.loadUnaligned(fromByteOffset: 0, as: UInt32.self)
            guard magic == Self.magic else { return nil }
            
            let messageId = bytes.loadUnaligned(fromByteOffset: 4, as: UInt64.self)
            let timestamp = bytes.loadUnaligned(fromByteOffset: 12, as: UInt64.self)
            let payloadSize = bytes.loadUnaligned(fromByteOffset: 20, as: UInt32.self)
            
            return PerfTestHeader(
                magic: magic,
                messageId: messageId,
                timestamp: timestamp,
                payloadSize: payloadSize
            )
        }
    }
    
    private init(magic: UInt32, messageId: UInt64, timestamp: UInt64, payloadSize: UInt32) {
        self.magic = magic
        self.messageId = messageId
        self.timestamp = timestamp
        self.payloadSize = payloadSize
    }
    
    func toBytes() -> Data {
        var data = Data(capacity: Self.size)
        data.append(contentsOf: withUnsafeBytes(of: magic.littleEndian) { $0 })
        data.append(contentsOf: withUnsafeBytes(of: messageId.littleEndian) { $0 })
        data.append(contentsOf: withUnsafeBytes(of: timestamp.littleEndian) { $0 })
        data.append(contentsOf: withUnsafeBytes(of: payloadSize.littleEndian) { $0 })
        return data
    }
}

class PosixPerfClient {
    private let filePath: String
    private let size: Int
    private var fd: Int32 = -1
    private var ptr: UnsafeMutableRawPointer?
    
    private let controlSize = 64
    private var messageCounter: UInt64 = 0
    
    init(filePath: String, size: Int) {
        self.filePath = filePath
        self.size = size
    }
    
    func connect() -> Bool {
        print("🔗 Swift性能测试客户端连接中...")
        
        // 等待文件创建
        var attempts = 0
        while attempts < 30 {
            if access(filePath, F_OK) == 0 {
                break
            }
            print("   等待Rust服务器创建文件... (\(attempts + 1)/30)")
            Thread.sleep(forTimeInterval: 1.0)
            attempts += 1
        }
        
        if attempts >= 30 {
            print("❌ 超时等待文件: \(filePath)")
            return false
        }
        
        fd = open(filePath, O_RDWR)
        if fd == -1 {
            print("❌ 无法打开文件: \(String(cString: strerror(errno)))")
            return false
        }
        
        ptr = mmap(nil, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)
        if ptr == MAP_FAILED {
            print("❌ 内存映射失败: \(String(cString: strerror(errno)))")
            close(fd)
            return false
        }
        
        // 通知Rust客户端已连接
        ptr!.storeBytes(of: UInt32(1), toByteOffset: 28, as: UInt32.self) // swift_connected
        
        print("✅ Swift客户端连接成功")
        return true
    }
    
    private func getAreaSize() -> Int {
        return (size - controlSize) / 2
    }
    
    private func getRustDataPtr() -> UnsafeMutableRawPointer {
        return ptr!.advanced(by: controlSize)
    }
    
    private func getSwiftDataPtr() -> UnsafeMutableRawPointer {
        let areaSize = getAreaSize()
        return ptr!.advanced(by: controlSize + areaSize)
    }
    
    private func readRustMessage() -> (UInt64, Data)? {
        guard let ptr = ptr else { return nil }
        
        let rustReadPos = ptr.load(fromByteOffset: 8, as: UInt64.self)
        let rustWritePos = ptr.load(fromByteOffset: 0, as: UInt64.self)
        
        if rustReadPos == rustWritePos { return nil }
        
        let areaSize = getAreaSize()
        let readOffset = Int(rustReadPos % UInt64(areaSize))
        let rustDataPtr = getRustDataPtr()
        
        // 读取消息头
        let headerData = Data(bytes: rustDataPtr.advanced(by: readOffset), count: PerfTestHeader.size)
        guard let header = PerfTestHeader.fromMemory(data: headerData) else { return nil }
        
        // 读取载荷
        let payload: Data
        if header.payloadSize > 0 {
            payload = Data(bytes: rustDataPtr.advanced(by: readOffset + PerfTestHeader.size), 
                          count: Int(header.payloadSize))
        } else {
            payload = Data()
        }
        
        // 更新读取位置
        let totalSize = PerfTestHeader.size + Int(header.payloadSize)
        let newReadPos = (rustReadPos + UInt64(totalSize)) % UInt64(areaSize)
        ptr.storeBytes(of: newReadPos, toByteOffset: 8, as: UInt64.self)
        
        return (header.messageId, payload)
    }
    
    private func writeSwiftMessage(payload: Data) -> Bool {
        guard let ptr = ptr else { return false }
        
        let areaSize = getAreaSize()
        let totalSize = PerfTestHeader.size + payload.count
        
        if totalSize > areaSize { return false }
        
        let swiftWritePos = ptr.load(fromByteOffset: 16, as: UInt64.self)
        let writeOffset = Int(swiftWritePos % UInt64(areaSize))
        
        messageCounter += 1
        let header = PerfTestHeader(messageId: messageCounter, payloadSize: UInt32(payload.count))
        
        let swiftDataPtr = getSwiftDataPtr()
        
        // 写入消息头
        let headerData = header.toBytes()
        headerData.withUnsafeBytes { bytes in
            swiftDataPtr.advanced(by: writeOffset).copyMemory(from: bytes.baseAddress!, byteCount: PerfTestHeader.size)
        }
        
        // 写入载荷
        if !payload.isEmpty {
            payload.withUnsafeBytes { bytes in
                swiftDataPtr.advanced(by: writeOffset + PerfTestHeader.size)
                          .copyMemory(from: bytes.baseAddress!, byteCount: payload.count)
            }
        }
        
        // 更新写入位置
        let newWritePos = (swiftWritePos + UInt64(totalSize)) % UInt64(areaSize)
        ptr.storeBytes(of: newWritePos, toByteOffset: 16, as: UInt64.self)
        
        return true
    }
    
    func runPerformanceTest(messageSize: Int) {
        print("\n🚀 Swift性能测试开始")
        print("===================")
        print("消息大小: \(messageSize)字节")
        
        guard connect() else {
            print("❌ 连接失败")
            return
        }
        
        let testPayload = Data(repeating: 0x42, count: messageSize)
        var swiftSent = 0
        var rustReceived = 0
        var lastReport = Date()
        let startTime = Date()
        
        print("📊 开始高频双向通信测试...")
        
        // 检查测试是否还在运行
        while ptr!.load(fromByteOffset: 24, as: UInt32.self) == 1 { // test_running
            
            // 高频发送消息
            for _ in 0..<100 {
                if writeSwiftMessage(payload: testPayload) {
                    swiftSent += 1
                }
            }
            
            // 读取Rust消息
            var readCount = 0
            while let (messageId, _) = readRustMessage() {
                rustReceived += 1
                readCount += 1
                if readCount > 100 { break } // 避免阻塞发送
            }
            
            // 每秒报告
            if Date().timeIntervalSince(lastReport) >= 1.0 {
                let elapsed = Date().timeIntervalSince(startTime)
                let swiftRate = Double(swiftSent) / elapsed
                let rustRate = Double(rustReceived) / elapsed
                let swiftBandwidth = (Double(swiftSent) * Double(messageSize)) / elapsed / 1024.0 / 1024.0
                let rustBandwidth = (Double(rustReceived) * Double(messageSize)) / elapsed / 1024.0 / 1024.0
                
                print("📊 [\(String(format: "%.1f", elapsed))s] Swift发送: \(swiftSent) msg (\(String(format: "%.0f", swiftRate)) msg/s, \(String(format: "%.1f", swiftBandwidth)) MB/s), Rust接收: \(rustReceived) msg (\(String(format: "%.0f", rustRate)) msg/s, \(String(format: "%.1f", rustBandwidth)) MB/s)")
                
                lastReport = Date()
            }
            
            // 微小延迟
            Thread.sleep(forTimeInterval: 0.000001) // 1微秒
        }
        
        let finalElapsed = Date().timeIntervalSince(startTime)
        
        print("\n🎯 Swift端最终结果:")
        print("==================")
        print("测试时长: \(String(format: "%.2f", finalElapsed))秒")
        print("消息大小: \(messageSize)字节")
        print("")
        
        print("Swift→Rust通信:")
        print("  发送消息: \(swiftSent)")
        print("  消息速率: \(String(format: "%.0f", Double(swiftSent) / finalElapsed)) msg/s")
        print("  数据速率: \(String(format: "%.2f", (Double(swiftSent) * Double(messageSize)) / finalElapsed / 1024.0 / 1024.0)) MB/s")
        
        print("Rust→Swift通信:")
        print("  接收消息: \(rustReceived)")
        print("  消息速率: \(String(format: "%.0f", Double(rustReceived) / finalElapsed)) msg/s")
        print("  数据速率: \(String(format: "%.2f", (Double(rustReceived) * Double(messageSize)) / finalElapsed / 1024.0 / 1024.0)) MB/s")
        
        let totalMessages = swiftSent + rustReceived
        let totalBytes = Double(totalMessages) * Double(messageSize)
        print("双向总计:")
        print("  总消息数: \(totalMessages)")
        print("  总数据量: \(String(format: "%.2f", totalBytes / 1024.0 / 1024.0)) MB")
        print("  平均速率: \(String(format: "%.0f", Double(totalMessages) / finalElapsed)) msg/s")
        print("  平均带宽: \(String(format: "%.2f", totalBytes / finalElapsed / 1024.0 / 1024.0)) MB/s")
        
        disconnect()
    }
    
    func disconnect() {
        if let ptr = ptr {
            ptr.storeBytes(of: UInt32(0), toByteOffset: 28, as: UInt32.self) // swift_connected = 0
            munmap(ptr, size)
        }
        
        if fd != -1 {
            close(fd)
        }
        
        print("🔌 Swift客户端已断开")
    }
    
    deinit {
        disconnect()
    }
}

// 主程序
func main() {
    print("🌟 Swift POSIX共享内存性能测试客户端")
    print("===================================")
    print("")
    
    let sharedFile = "/tmp/posix_perf_test.dat"
    let sharedSize = 16 * 1024 * 1024 // 16MB
    let messageSize = 1024 // 1KB
    
    let client = PosixPerfClient(filePath: sharedFile, size: sharedSize)
    
    // 信号处理
    signal(SIGINT) { _ in
        print("\n🛑 收到中断信号")
        exit(0)
    }
    
    client.runPerformanceTest(messageSize: messageSize)
    
    print("\n✅ 性能测试完成")
}

main()
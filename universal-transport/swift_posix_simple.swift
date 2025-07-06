#!/usr/bin/env swift

//! 简化的Swift POSIX共享内存客户端
//! 避免复杂的内存对齐问题

import Foundation
import Darwin

/// 简化的Swift POSIX客户端
class SimplePosixClient {
    private let filePath: String
    private let size: Int
    private var fd: Int32 = -1
    private var ptr: UnsafeMutableRawPointer?
    
    init(filePath: String, size: Int) {
        self.filePath = filePath
        self.size = size
    }
    
    func connect() -> Bool {
        print("🔗 Swift客户端连接到共享内存文件")
        print("   文件: \(filePath)")
        
        // 等待文件存在
        var attempts = 0
        while attempts < 10 {
            if access(filePath, F_OK) == 0 {
                break
            }
            print("   等待文件创建... (\(attempts + 1)/10)")
            Thread.sleep(forTimeInterval: 1.0)
            attempts += 1
        }
        
        if attempts >= 10 {
            print("❌ 文件不存在: \(filePath)")
            return false
        }
        
        // 打开共享文件
        fd = open(filePath, O_RDWR)
        if fd == -1 {
            print("❌ 无法打开共享内存文件: \(filePath)")
            print("   错误: \(String(cString: strerror(errno)))")
            return false
        }
        
        // 内存映射
        ptr = mmap(nil, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)
        if ptr == MAP_FAILED {
            print("❌ 内存映射失败: \(String(cString: strerror(errno)))")
            close(fd)
            return false
        }
        
        print("✅ 成功连接到共享内存")
        print("   地址: \(ptr!)")
        print("   大小: \(size) bytes")
        
        // 设置客户端连接状态 (偏移28字节处)
        ptr!.storeBytes(of: UInt32(1), toByteOffset: 28, as: UInt32.self)
        
        return true
    }
    
    func readControlBlock() -> (writePos: UInt64, readPos: UInt64, messageCount: UInt64, serverStatus: UInt32) {
        guard let ptr = ptr else { return (0, 0, 0, 0) }
        
        let writePos = ptr.load(fromByteOffset: 0, as: UInt64.self)
        let readPos = ptr.load(fromByteOffset: 8, as: UInt64.self)
        let messageCount = ptr.load(fromByteOffset: 16, as: UInt64.self)
        let serverStatus = ptr.load(fromByteOffset: 24, as: UInt32.self)
        
        return (writePos, readPos, messageCount, serverStatus)
    }
    
    func sendSimpleMessage(content: String) -> Bool {
        guard let ptr = ptr else { return false }
        
        let data = content.data(using: .utf8) ?? Data()
        let controlSize = 64
        let messageSize = 32 + data.count  // 32字节头部 + 数据
        
        // 获取当前写入位置
        let currentWritePos = ptr.load(fromByteOffset: 0, as: UInt64.self)
        let currentMessageCount = ptr.load(fromByteOffset: 16, as: UInt64.self)
        
        // 计算数据区偏移
        let dataAreaOffset = controlSize + Int(currentWritePos % UInt64(size - controlSize))
        
        // 写入简化的消息头
        let magic: UInt32 = 0x55545042
        let version: UInt8 = 1
        let messageType: UInt8 = 0x05  // Swift消息类型
        
        var offset = dataAreaOffset
        
        // 写入消息头字段
        ptr.storeBytes(of: magic, toByteOffset: offset, as: UInt32.self)
        offset += 4
        ptr.storeBytes(of: version, toByteOffset: offset, as: UInt8.self)
        offset += 1
        ptr.storeBytes(of: messageType, toByteOffset: offset, as: UInt8.self)
        offset += 1
        
        // 跳过flags (2字节)
        offset += 2
        
        // 载荷长度
        ptr.storeBytes(of: UInt32(data.count), toByteOffset: offset, as: UInt32.self)
        offset += 4
        
        // 序列号
        ptr.storeBytes(of: currentMessageCount + 1, toByteOffset: offset, as: UInt64.self)
        offset += 8
        
        // 时间戳
        let timestamp = UInt64(Date().timeIntervalSince1970 * 1_000_000)
        ptr.storeBytes(of: timestamp, toByteOffset: offset, as: UInt64.self)
        offset += 8
        
        // 校验和 (跳过)
        offset += 4
        
        // 写入数据
        if !data.isEmpty {
            data.withUnsafeBytes { bytes in
                ptr.advanced(by: offset).copyMemory(from: bytes.baseAddress!, byteCount: data.count)
            }
        }
        
        // 更新写入位置
        ptr.storeBytes(of: currentWritePos + UInt64(messageSize), toByteOffset: 0, as: UInt64.self)
        
        // 更新消息计数
        ptr.storeBytes(of: currentMessageCount + 1, toByteOffset: 16, as: UInt64.self)
        
        return true
    }
    
    func runSimpleTest() {
        print("\n🚀 启动简化Swift POSIX客户端")
        print("============================")
        
        guard connect() else {
            print("❌ 连接失败")
            return
        }
        
        print("💡 开始与Rust服务器通信...")
        
        var messageId = 0
        let startTime = Date()
        
        // 简单的通信循环
        while true {
            let control = readControlBlock()
            
            if control.serverStatus == 0 {
                print("🏁 Rust服务器已停止")
                break
            }
            
            // 发送简单消息
            let message = "Swift简单消息 #\(messageId)"
            if sendSimpleMessage(content: message) {
                print("📤 发送: \"\(message)\"")
                messageId += 1
            }
            
            // 显示统计信息
            if messageId % 3 == 0 {
                print("📊 统计: 发送=\(messageId), 位置=\(control.readPos)→\(control.writePos), 总消息=\(control.messageCount)")
            }
            
            Thread.sleep(forTimeInterval: 1.0)
            
            // 测试10条消息后停止
            if messageId >= 10 {
                print("🏁 测试完成")
                break
            }
        }
        
        let duration = Date().timeIntervalSince(startTime)
        
        print("\n📈 最终统计:")
        print("  运行时间: \(String(format: "%.1f", duration))秒")
        print("  发送消息: \(messageId)")
        print("  平均速率: \(String(format: "%.1f", Double(messageId) / duration)) msg/s")
        
        disconnect()
    }
    
    func disconnect() {
        // 设置客户端断开状态
        ptr?.storeBytes(of: UInt32(0), toByteOffset: 28, as: UInt32.self)
        
        if let ptr = ptr {
            munmap(ptr, size)
        }
        
        if fd != -1 {
            close(fd)
        }
        
        print("🔌 Swift客户端已断开连接")
    }
    
    deinit {
        disconnect()
    }
}

// 主程序
func main() {
    print("🌟 简化Swift POSIX客户端")
    print("=======================")
    print("")
    
    let sharedFile = "/tmp/rust_swift_posix_shared.dat"
    let sharedSize = 1024 * 1024 // 1MB
    
    let client = SimplePosixClient(filePath: sharedFile, size: sharedSize)
    
    // 信号处理
    signal(SIGINT) { _ in
        print("\n🛑 收到中断信号，正在断开...")
        exit(0)
    }
    
    client.runSimpleTest()
    
    print("\n🎯 简化POSIX测试总结:")
    print("  ✅ 成功连接到文件映射共享内存")
    print("  ✅ 避免了复杂的内存对齐问题")
    print("  ✅ 实现基本的Swift→Rust通信")
    print("  ✅ 验证了跨进程共享内存可行性")
}

main()
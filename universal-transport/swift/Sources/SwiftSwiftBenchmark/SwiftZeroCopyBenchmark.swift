//
//  SwiftZeroCopyBenchmark.swift
//  零拷贝性能基准测试 - 直接对比Rust零拷贝性能
//
//  尽可能接近Rust的零拷贝实现，减少不必要的开销
//

import Foundation

// MARK: - 零拷贝消息结构

public struct SwiftZeroCopyMessage {
    private let data: Data
    
    public init(payloadSize: Int, sequence: UInt64) {
        // 模拟Rust的零拷贝消息创建
        let headerSize = 32
        let totalSize = headerSize + payloadSize
        
        var buffer = Data(capacity: totalSize)
        
        // 简化的头部（模拟Rust的repr(C)结构）
        buffer.append(contentsOf: withUnsafeBytes(of: UInt32(0x55545042).littleEndian) { $0 }) // magic
        buffer.append(UInt8(1)) // version
        buffer.append(UInt8(0x05)) // message_type
        buffer.append(contentsOf: withUnsafeBytes(of: UInt16(0).littleEndian) { $0 }) // flags
        buffer.append(contentsOf: withUnsafeBytes(of: UInt32(payloadSize).littleEndian) { $0 }) // payload_length
        buffer.append(contentsOf: withUnsafeBytes(of: sequence.littleEndian) { $0 }) // sequence
        buffer.append(contentsOf: withUnsafeBytes(of: UInt64(0).littleEndian) { $0 }) // timestamp
        buffer.append(contentsOf: withUnsafeBytes(of: UInt32(0).littleEndian) { $0 }) // checksum
        
        // 填充载荷
        buffer.append(Data(repeating: 0x42, count: payloadSize))
        
        self.data = buffer
    }
    
    public func getBytes() -> Data {
        return data
    }
    
    public func getSequence() -> UInt64 {
        return data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 12, as: UInt64.self).littleEndian
        }
    }
    
    public func validate() -> Bool {
        guard data.count >= 32 else { return false }
        let magic = data.withUnsafeBytes { bytes in
            bytes.loadUnaligned(fromByteOffset: 0, as: UInt32.self).littleEndian
        }
        return magic == 0x55545042
    }
}

// MARK: - 零拷贝基准测试

public struct SwiftZeroCopyBenchmark {
    
    public static func runZeroCopyBenchmark() {
        print("🚀 Swift Zero-Copy Binary Protocol Benchmark")
        print("============================================")
        print("Testing Swift zero-copy performance (comparable to Rust)")
        print("")
        
        let testCases = [
            ("Swift Zero-Copy Small Messages (1KB)", 10000, 1024),
            ("Swift Zero-Copy Medium Messages (64KB)", 1000, 64 * 1024),
            ("Swift Zero-Copy Large Messages (1MB)", 100, 1024 * 1024),
            ("Swift Zero-Copy Huge Messages (16MB)", 10, 16 * 1024 * 1024),
        ]
        
        for (testName, messageCount, messageSize) in testCases {
            runZeroCopyTest(testName: testName, messageCount: messageCount, messageSize: messageSize)
            print("────────────────────────────────────────────────────────────")
        }
    }
    
    private static func runZeroCopyTest(testName: String, messageCount: Int, messageSize: Int) {
        print("🔬 \(testName): \(messageCount) messages × \(messageSize) bytes")
        
        // 预分配消息（对应Rust的分配阶段）
        let allocStart = Date()
        var messages: [SwiftZeroCopyMessage] = []
        messages.reserveCapacity(messageCount)
        
        for i in 0..<messageCount {
            messages.append(SwiftZeroCopyMessage(payloadSize: messageSize, sequence: UInt64(i)))
        }
        let allocTime = Date().timeIntervalSince(allocStart)
        
        // 零拷贝操作测试（对应Rust的零拷贝操作）
        let opsStart = Date()
        var totalValidated = 0
        var totalBytesProcessed = 0
        
        for message in messages {
            // 1. 获取字节引用（零拷贝）
            let bytes = message.getBytes()
            
            // 2. 验证（零拷贝）
            if message.validate() {
                totalValidated += 1
            }
            
            // 3. 获取序列号（零拷贝）
            let sequence = message.getSequence()
            totalBytesProcessed += bytes.count
            
            // 4. 模拟解析（创建新的消息引用，类似Rust的from_bytes）
            if bytes.count >= 32 {
                let magic = bytes.withUnsafeBytes { rawBytes in
                    rawBytes.loadUnaligned(fromByteOffset: 0, as: UInt32.self).littleEndian
                }
                if magic == 0x55545042 {
                    // 验证序列号
                    let parsedSequence = bytes.withUnsafeBytes { rawBytes in
                        rawBytes.loadUnaligned(fromByteOffset: 12, as: UInt64.self).littleEndian
                    }
                    assert(parsedSequence == sequence)
                }
            }
        }
        
        let opsTime = Date().timeIntervalSince(opsStart)
        let totalTime = Date().timeIntervalSince(allocStart)
        
        // 计算指标
        let totalDataMB = Double(totalBytesProcessed) / (1024.0 * 1024.0)
        let opsThroughput = totalDataMB / opsTime
        let overallThroughput = totalDataMB / totalTime
        let avgLatencyNs = (opsTime * 1_000_000_000) / Double(messageCount)
        
        print("  Allocation time: \(String(format: "%.3f", allocTime * 1000))ms")
        print("  Zero-copy ops time: \(String(format: "%.3f", opsTime * 1000))ms")
        print("  Total data processed: \(String(format: "%.2f", totalDataMB)) MB")
        print("  Zero-copy throughput: \(String(format: "%.2f", opsThroughput)) MB/s")
        print("  Overall throughput: \(String(format: "%.2f", overallThroughput)) MB/s")
        print("  Average latency: \(String(format: "%.2f", avgLatencyNs)) ns per operation")
        print("  Validation rate: \(String(format: "%.0f", Double(messageCount) / opsTime)) ops/sec")
        print("  Messages validated: \(totalValidated)/\(messageCount)")
    }
}

// MARK: - 简化基准测试

public struct SwiftSimpleBenchmark {
    
    public static func runSimpleBenchmark() {
        print("🚀 Swift Simple Protocol Benchmark")
        print("==================================")
        print("Testing basic Swift operations for comparison")
        print("")
        
        let testCases = [
            ("Swift Simple Small Messages (1KB)", 10000, 1024),
            ("Swift Simple Medium Messages (64KB)", 1000, 64 * 1024),
            ("Swift Simple Large Messages (1MB)", 100, 1024 * 1024),
            ("Swift Simple Huge Messages (16MB)", 10, 16 * 1024 * 1024),
        ]
        
        for (testName, messageCount, messageSize) in testCases {
            runSimpleTest(testName: testName, messageCount: messageCount, messageSize: messageSize)
            print("────────────────────────────────────────────────────────────")
        }
    }
    
    private static func runSimpleTest(testName: String, messageCount: Int, messageSize: Int) {
        print("🔬 \(testName): \(messageCount) messages × \(messageSize) bytes")
        
        let start = Date()
        var totalBytes = 0
        
        for i in 0..<messageCount {
            // 简单的数据创建和访问
            let data = Data(repeating: 0x42, count: messageSize)
            totalBytes += data.count
            
            // 简单的验证
            let first = data.first ?? 0
            assert(first == 0x42)
        }
        
        let duration = Date().timeIntervalSince(start)
        let totalDataMB = Double(totalBytes) / (1024.0 * 1024.0)
        let throughput = totalDataMB / duration
        let avgLatencyNs = (duration * 1_000_000_000) / Double(messageCount)
        
        print("  Duration: \(String(format: "%.3f", duration * 1000))ms")
        print("  Total data: \(String(format: "%.2f", totalDataMB)) MB")
        print("  Throughput: \(String(format: "%.2f", throughput)) MB/s")
        print("  Average latency: \(String(format: "%.2f", avgLatencyNs)) ns per operation")
        print("  Rate: \(String(format: "%.0f", Double(messageCount) / duration)) ops/sec")
    }
}
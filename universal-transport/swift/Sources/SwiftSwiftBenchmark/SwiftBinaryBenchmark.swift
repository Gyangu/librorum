//
//  SwiftBinaryBenchmark.swift
//  High-Performance Binary Protocol Benchmark for Swift
//
//  Direct equivalent to Rust binary protocol benchmark
//

import Foundation
import Logging
import UniversalTransportSharedMemory

// MARK: - Binary Benchmark Results

public struct SwiftBinaryBenchmarkResults {
    public let testName: String
    public let messageCount: Int
    public let messageSize: Int
    public let duration: TimeInterval
    public let throughputMBps: Double
    public let messagesPerSecond: Double
    public let averageLatencyMicros: Double
    public let serializationOverhead: Double
    
    public init(
        testName: String,
        messageCount: Int,
        messageSize: Int,
        duration: TimeInterval,
        totalBytes: Int,
        rawBytes: Int
    ) {
        self.testName = testName
        self.messageCount = messageCount
        self.messageSize = messageSize
        self.duration = duration
        
        self.throughputMBps = (Double(totalBytes) / (1024.0 * 1024.0)) / duration
        self.messagesPerSecond = Double(messageCount) / duration
        self.averageLatencyMicros = (duration * 1_000_000) / Double(messageCount)
        self.serializationOverhead = ((Double(totalBytes - rawBytes) / Double(rawBytes)) * 100.0)
    }
    
    public func printSummary() {
        print("")
        print("=== \(testName) ===")
        print("Messages: \(messageCount) × \(messageSize) bytes")
        print("Duration: \(String(format: "%.3f", duration))s")
        print("Throughput: \(String(format: "%.2f", throughputMBps)) MB/s")
        print("Rate: \(String(format: "%.0f", messagesPerSecond)) messages/sec")
        print("Avg Latency: \(String(format: "%.2f", averageLatencyMicros)) μs")
        print("Serialization overhead: \(String(format: "%.2f", serializationOverhead))%")
    }
}

// MARK: - Swift Binary Protocol Benchmark

@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
public class SwiftBinaryProtocolBenchmark {
    private let logger = Logger(label: "swift-binary-benchmark")
    
    public init() {
        logger.info("Swift Binary Protocol benchmark initialized")
    }
    
    /// Run complete benchmark suite (matches Rust implementation)
    public func runBenchmarkSuite() async -> [SwiftBinaryBenchmarkResults] {
        logger.info("Starting Swift Binary Protocol benchmark suite")
        
        let testCases: [(String, Int, Int)] = [
            ("Swift Binary Small Messages (1KB)", 1000, 1024),
            ("Swift Binary Medium Messages (64KB)", 200, 64 * 1024),
            ("Swift Binary Large Messages (1MB)", 50, 1024 * 1024),
            ("Swift Binary Huge Messages (16MB)", 10, 16 * 1024 * 1024),
        ]
        
        var results: [SwiftBinaryBenchmarkResults] = []
        
        // Run throughput tests
        for (testName, messageCount, messageSize) in testCases {
            logger.info("Running: \(testName)")
            
            let result = await runBinarySerializationTest(
                testName: testName,
                messageCount: messageCount,
                messageSize: messageSize
            )
            
            result.printSummary()
            results.append(result)
            
            print("────────────────────────────────────────────────────────────")
        }
        
        // Run latency test
        logger.info("Running Swift binary protocol latency test")
        let latencyResult = await runLatencyTest()
        latencyResult.printSummary()
        results.append(latencyResult)
        
        return results
    }
    
    /// Test binary protocol serialization performance
    private func runBinarySerializationTest(
        testName: String,
        messageCount: Int,
        messageSize: Int
    ) async -> SwiftBinaryBenchmarkResults {
        
        // Generate benchmark messages
        let messages = (0..<messageCount).map { i in
            BinaryBenchmarkMessage(id: UInt64(i), dataSize: messageSize)
        }
        
        let start = Date()
        var totalSerializedBytes = 0
        var totalRawBytes = 0
        
        // Test serialization and deserialization
        for message in messages {
            do {
                // Convert to binary message
                let binaryMsg = try message.toBinaryMessage()
                
                // Serialize to bytes
                let serialized = binaryMsg.toBytes()
                totalSerializedBytes += serialized.count
                totalRawBytes += message.data.count
                
                // Deserialize back
                let deserialized = try BinaryMessage.fromBytes(serialized)
                
                // Convert back to benchmark message
                let recovered = try BinaryBenchmarkMessage.fromBinaryMessage(deserialized)
                
                // Verify data integrity
                assert(message.id == recovered.id, "ID mismatch")
                assert(message.data.count == recovered.data.count, "Data size mismatch")
                
            } catch {
                logger.error("Serialization failed: \(error)")
            }
        }
        
        let duration = Date().timeIntervalSince(start)
        
        return SwiftBinaryBenchmarkResults(
            testName: testName,
            messageCount: messageCount,
            messageSize: messageSize,
            duration: duration,
            totalBytes: totalSerializedBytes,
            rawBytes: totalRawBytes
        )
    }
    
    /// Test latency with small messages
    private func runLatencyTest() async -> SwiftBinaryBenchmarkResults {
        let testName = "Swift Binary Protocol Latency Test"
        let iterations = 10000
        let messageSize = 64 // Small message for latency
        
        let start = Date()
        var totalBytes = 0
        
        for i in 0..<iterations {
            do {
                let message = BinaryBenchmarkMessage(id: UInt64(i), dataSize: messageSize)
                let binaryMsg = try message.toBinaryMessage()
                let serialized = binaryMsg.toBytes()
                totalBytes += serialized.count
                
                // Immediate deserialization (simulating round-trip)
                let _ = try BinaryMessage.fromBytes(serialized)
                
            } catch {
                logger.error("Latency test failed: \(error)")
            }
        }
        
        let duration = Date().timeIntervalSince(start)
        
        return SwiftBinaryBenchmarkResults(
            testName: testName,
            messageCount: iterations,
            messageSize: messageSize,
            duration: duration,
            totalBytes: totalBytes,
            rawBytes: iterations * messageSize
        )
    }
    
    /// Compare with previous JSON performance
    public func compareBinaryVsJSON() {
        print("")
        print("📊 Binary vs JSON Comparison (Swift)")
        print("===================================")
        
        let testMessage = BinaryBenchmarkMessage(id: 1, dataSize: 1024)
        
        // Binary serialization
        let binaryStart = Date()
        do {
            let binaryMsg = try testMessage.toBinaryMessage()
            let binaryBytes = binaryMsg.toBytes()
            let binaryDuration = Date().timeIntervalSince(binaryStart)
            
            // Simulate previous JSON approach
            let jsonStart = Date()
            let jsonDict: [String: Any] = [
                "id": testMessage.id,
                "timestamp": testMessage.timestamp,
                "data_len": testMessage.data.count,
                "metadata": testMessage.metadata
            ]
            let jsonData = try JSONSerialization.data(withJSONObject: jsonDict)
            let jsonTotalSize = jsonData.count + testMessage.data.count
            let jsonDuration = Date().timeIntervalSince(jsonStart)
            
            print("Binary protocol:")
            print("  Size: \(binaryBytes.count) bytes")
            print("  Time: \(String(format: "%.3f", binaryDuration * 1000))ms")
            print("  Header overhead: \(BINARY_HEADER_SIZE) bytes")
            
            print("JSON equivalent:")
            print("  Size: \(jsonTotalSize) bytes")
            print("  Time: \(String(format: "%.3f", jsonDuration * 1000))ms")
            print("  Size overhead: \(String(format: "%.1f", (Double(jsonTotalSize - binaryBytes.count) / Double(binaryBytes.count)) * 100))%")
            
            print("Binary advantage:")
            print("  Size reduction: \(String(format: "%.1f", Double(jsonTotalSize) / Double(binaryBytes.count)))x smaller")
            print("  Speed improvement: \(String(format: "%.1f", jsonDuration / binaryDuration))x faster")
            
        } catch {
            logger.error("Comparison failed: \(error)")
        }
    }
}
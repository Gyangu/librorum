//
//  SwiftNetworkBenchmark.swift
//  Swift TCP网络通信性能基准测试
//
//  对比Swift和Rust的网络通信性能
//

import Foundation
import Network

// MARK: - 网络基准测试结果

public struct SwiftNetworkBenchmarkResult {
    public let testName: String
    public let messageCount: Int
    public let messageSize: Int
    public let duration: TimeInterval
    public let throughputMBps: Double
    public let latencyMicros: Double
    public let messagesPerSecond: Double
    public let successfulMessages: Int
    
    public init(
        testName: String,
        messageCount: Int,
        messageSize: Int,
        duration: TimeInterval,
        successfulMessages: Int
    ) {
        self.testName = testName
        self.messageCount = messageCount
        self.messageSize = messageSize
        self.duration = duration
        self.successfulMessages = successfulMessages
        
        let totalBytes = Double(successfulMessages * messageSize)
        self.throughputMBps = (totalBytes / (1024.0 * 1024.0)) / duration
        self.latencyMicros = (duration * 1_000_000) / Double(successfulMessages)
        self.messagesPerSecond = Double(successfulMessages) / duration
    }
    
    public func printSummary() {
        print("📊 \(testName)")
        print("   Messages: \(successfulMessages)/\(messageCount) successful")
        print("   Total data: \(String(format: "%.2f", Double(successfulMessages * messageSize) / (1024.0 * 1024.0))) MB")
        print("   Duration: \(String(format: "%.3f", duration))s")
        print("   Throughput: \(String(format: "%.2f", throughputMBps)) MB/s")
        print("   Latency: \(String(format: "%.2f", latencyMicros)) μs")
        print("   Rate: \(String(format: "%.0f", messagesPerSecond)) msg/s")
        print("   Success: \(String(format: "%.1f", Double(successfulMessages)/Double(messageCount)*100))%")
        print("   ──────────────────────────────────────")
    }
}

// MARK: - Swift TCP服务器

@available(macOS 10.14, iOS 12.0, watchOS 5.0, tvOS 12.0, *)
public class SwiftTCPServer {
    private let port: UInt16
    private var listener: NWListener?
    private var expectedMessages: Int = 0
    private var receivedMessages: Int = 0
    
    public init(port: UInt16) {
        self.port = port
    }
    
    public func start(expectedMessages: Int) async throws {
        self.expectedMessages = expectedMessages
        self.receivedMessages = 0
        
        let tcpOptions = NWProtocolTCP.Options()
        tcpOptions.noDelay = true // 禁用Nagle算法以减少延迟
        
        let parameters = NWParameters(tls: nil, tcp: tcpOptions)
        parameters.acceptLocalOnly = true
        parameters.allowLocalEndpointReuse = true
        
        listener = try NWListener(using: parameters, on: NWEndpoint.Port(integerLiteral: port))
        
        listener?.newConnectionHandler = { [weak self] connection in
            Task {
                await self?.handleConnection(connection)
            }
        }
        
        listener?.start(queue: .global())
        
        // 等待服务器启动
        try await Task.sleep(nanoseconds: 100_000_000) // 100ms
    }
    
    public func stop() {
        listener?.cancel()
        listener = nil
    }
    
    private func handleConnection(_ connection: NWConnection) async {
        connection.start(queue: .global())
        
        while receivedMessages < expectedMessages {
            do {
                // 读取消息大小 (4字节)
                let sizeData = try await receiveData(connection: connection, length: 4)
                let messageSize = sizeData.withUnsafeBytes { bytes in
                    bytes.loadUnaligned(as: UInt32.self).littleEndian
                }
                
                // 读取消息数据
                let messageData = try await receiveData(connection: connection, length: Int(messageSize))
                
                // 回显相同消息
                try await sendData(connection: connection, data: sizeData)
                try await sendData(connection: connection, data: messageData)
                
                receivedMessages += 1
                
            } catch {
                print("Server connection error: \(error)")
                break
            }
        }
        
        connection.cancel()
    }
    
    private func receiveData(connection: NWConnection, length: Int) async throws -> Data {
        return try await withCheckedThrowingContinuation { continuation in
            connection.receive(minimumIncompleteLength: length, maximumLength: length) { data, _, isComplete, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data, data.count == length {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: URLError(.badServerResponse))
                }
            }
        }
    }
    
    private func sendData(connection: NWConnection, data: Data) async throws {
        return try await withCheckedThrowingContinuation { continuation in
            connection.send(content: data, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
    }
}

// MARK: - Swift TCP客户端

@available(macOS 10.14, iOS 12.0, watchOS 5.0, tvOS 12.0, *)
public class SwiftTCPClient {
    private let serverHost: String
    private let serverPort: UInt16
    private var connection: NWConnection?
    
    public init(serverHost: String = "127.0.0.1", serverPort: UInt16) {
        self.serverHost = serverHost
        self.serverPort = serverPort
    }
    
    public func connect() async throws {
        let endpoint = NWEndpoint.hostPort(
            host: NWEndpoint.Host(serverHost),
            port: NWEndpoint.Port(integerLiteral: serverPort)
        )
        
        let tcpOptions = NWProtocolTCP.Options()
        tcpOptions.noDelay = true
        
        let parameters = NWParameters(tls: nil, tcp: tcpOptions)
        parameters.allowLocalEndpointReuse = true
        
        connection = NWConnection(to: endpoint, using: parameters)
        
        return try await withCheckedThrowingContinuation { continuation in
            connection?.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    continuation.resume()
                case .failed(let error):
                    continuation.resume(throwing: error)
                case .cancelled:
                    continuation.resume(throwing: URLError(.cancelled))
                default:
                    break
                }
            }
            
            connection?.start(queue: .global())
        }
    }
    
    public func runBenchmark(messageCount: Int, messageSize: Int) async throws -> SwiftNetworkBenchmarkResult {
        guard let connection = self.connection else {
            throw URLError(.notConnectedToInternet)
        }
        
        let start = Date()
        var successfulMessages = 0
        
        for i in 0..<messageCount {
            do {
                // 创建消息数据
                let messageData = Data(repeating: 0x42, count: messageSize)
                let sizeData = withUnsafeBytes(of: UInt32(messageSize).littleEndian) { Data($0) }
                
                // 发送消息大小和数据
                try await sendData(connection: connection, data: sizeData)
                try await sendData(connection: connection, data: messageData)
                
                // 接收响应大小
                let responseSizeData = try await receiveData(connection: connection, length: 4)
                let responseSize = responseSizeData.withUnsafeBytes { bytes in
                    bytes.loadUnaligned(as: UInt32.self).littleEndian
                }
                
                // 接收响应数据
                let responseData = try await receiveData(connection: connection, length: Int(responseSize))
                
                // 验证响应
                if responseData.count == messageSize {
                    successfulMessages += 1
                }
                
                // 进度指示
                if i % 100 == 0 && i > 0 {
                    print("📈 Progress: \(i)/\(messageCount) messages")
                }
                
            } catch {
                print("❌ Message \(i) failed: \(error)")
            }
        }
        
        let duration = Date().timeIntervalSince(start)
        
        return SwiftNetworkBenchmarkResult(
            testName: "Swift TCP Socket (\(messageSize) bytes)",
            messageCount: messageCount,
            messageSize: messageSize,
            duration: duration,
            successfulMessages: successfulMessages
        )
    }
    
    public func disconnect() {
        connection?.cancel()
        connection = nil
    }
    
    private func sendData(connection: NWConnection, data: Data) async throws {
        return try await withCheckedThrowingContinuation { continuation in
            connection.send(content: data, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
    }
    
    private func receiveData(connection: NWConnection, length: Int) async throws -> Data {
        return try await withCheckedThrowingContinuation { continuation in
            connection.receive(minimumIncompleteLength: length, maximumLength: length) { data, _, isComplete, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data, data.count == length {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: URLError(.badServerResponse))
                }
            }
        }
    }
}

// MARK: - Swift网络基准测试套件

@available(macOS 10.14, iOS 12.0, watchOS 5.0, tvOS 12.0, *)
public class SwiftNetworkBenchmark {
    
    public static func runNetworkBenchmark() async {
        print("🌐 Swift TCP Network Communication Benchmark")
        print("============================================")
        print("Testing Swift TCP performance vs Rust TCP performance")
        print("")
        
        let testCases = [
            ("Swift Small Messages", 1000, 1024),        // 1KB
            ("Swift Medium Messages", 200, 64 * 1024),   // 64KB
            ("Swift Large Messages", 50, 1024 * 1024),   // 1MB
            ("Swift Huge Messages", 10, 4 * 1024 * 1024), // 4MB
        ]
        
        var results: [SwiftNetworkBenchmarkResult] = []
        
        for (testName, messageCount, messageSize) in testCases {
            print("🔬 Testing \(testName) (\(messageCount) × \(messageSize) bytes)")
            
            do {
                let result = try await runSingleNetworkTest(
                    testName: testName,
                    messageCount: messageCount,
                    messageSize: messageSize
                )
                result.printSummary()
                results.append(result)
                
            } catch {
                print("❌ Test \(testName) failed: \(error)")
            }
            
            // 测试间等待
            try? await Task.sleep(nanoseconds: 1_000_000_000) // 1秒
        }
        
        // 结果汇总
        print("")
        print("🎯 SWIFT vs RUST TCP PERFORMANCE COMPARISON")
        print("==========================================")
        
        print("📊 Swift TCP Performance:")
        for result in results {
            print("  \(result.testName): \(String(format: "%.1f", result.throughputMBps)) MB/s, \(String(format: "%.1f", result.latencyMicros)) μs latency")
        }
        
        print("")
        print("📊 Rust TCP Performance (for comparison):")
        print("  Rust Small Messages (1KB): 13.2 MB/s, 73.9 μs latency")
        print("  Rust Medium Messages (64KB): 383.0 MB/s, 163.2 μs latency")
        print("  Rust Large Messages (1MB): 1,188.3 MB/s, 841.5 μs latency")
        print("  Rust Huge Messages (4MB): 644.9 MB/s, 6,202.8 μs latency")
        
        print("")
        print("🔍 Performance Analysis:")
        if !results.isEmpty {
            let avgSwiftThroughput = results.reduce(0.0) { $0 + $1.throughputMBps } / Double(results.count)
            print("  Swift Average: \(String(format: "%.1f", avgSwiftThroughput)) MB/s")
            print("  Rust Average: ~557 MB/s")
            print("  Performance Ratio: \(String(format: "%.2f", avgSwiftThroughput / 557))x")
        }
    }
    
    private static func runSingleNetworkTest(
        testName: String,
        messageCount: Int,
        messageSize: Int
    ) async throws -> SwiftNetworkBenchmarkResult {
        
        let port: UInt16 = 9082
        
        // 启动服务器
        let server = SwiftTCPServer(port: port)
        try await server.start(expectedMessages: messageCount)
        
        // 等待服务器完全启动
        try await Task.sleep(nanoseconds: 200_000_000) // 200ms
        
        // 启动客户端
        let client = SwiftTCPClient(serverPort: port)
        try await client.connect()
        
        // 运行基准测试
        let result = try await client.runBenchmark(messageCount: messageCount, messageSize: messageSize)
        
        // 清理
        client.disconnect()
        server.stop()
        
        // 等待端口释放
        try await Task.sleep(nanoseconds: 500_000_000) // 500ms
        
        return result
    }
}
//
//  CrossLanguageClient.swift
//  真正的跨语言通信客户端
//
//  连接到Rust服务器进行实际的IPC性能测试
//

import Foundation
import Network
import Logging
import UniversalTransportSharedMemory

// MARK: - Client Configuration

/// Client information sent to server
public struct ClientInfo: Codable {
    public let clientId: String
    public let language: String
    public let version: String
    public let capabilities: [String]
    
    public init(clientId: String) {
        self.clientId = clientId
        self.language = "swift"
        self.version = "1.0.0"
        self.capabilities = ["zero-copy", "binary-protocol", "tcp-socket"]
    }
}

/// Server statistics response
public struct ServerStats: Codable {
    public let totalMessages: UInt64
    public let totalBytes: UInt64
    public let rustClients: UInt32
    public let swiftClients: UInt32
    public let uptimeSeconds: UInt64
    public let averageLatencyMicros: Double
    
    private enum CodingKeys: String, CodingKey {
        case totalMessages = "total_messages"
        case totalBytes = "total_bytes"
        case rustClients = "rust_clients"
        case swiftClients = "swift_clients"
        case uptimeSeconds = "uptime_seconds"
        case averageLatencyMicros = "average_latency_micros"
    }
}

// MARK: - Cross-Language Message Types

/// Message types for cross-language communication
public enum CrossLanguageMessageType: UInt8 {
    case benchmarkRequest = 0x10
    case benchmarkResponse = 0x11
    case clientInfo = 0x12
    case serverStats = 0x13
}

/// Cross-language zero-copy message
public struct CrossLanguageMessage {
    public let header: BinaryMessageHeader
    public let payload: Data
    
    public init(messageType: CrossLanguageMessageType, payload: Data, sequence: UInt64) {
        var header = BinaryMessageHeader(messageType: BinaryMessageType.benchmark, payload: payload)
        header.messageType = messageType.rawValue
        header.setSequence(sequence)
        
        self.header = header
        self.payload = payload
    }
    
    /// Convert to bytes for network transmission
    public func toBytes() -> Data {
        var data = Data()
        data.append(header.toBytes())
        data.append(payload)
        return data
    }
    
    /// Parse from bytes
    public static func fromBytes(_ data: Data) throws -> CrossLanguageMessage {
        guard data.count >= BINARY_HEADER_SIZE else {
            throw BinaryProtocolError.insufficientData(data.count)
        }
        
        let headerData = data.prefix(BINARY_HEADER_SIZE)
        let header = try BinaryMessageHeader.fromBytes(headerData)
        
        let expectedTotal = BINARY_HEADER_SIZE + Int(header.payloadLength)
        guard data.count >= expectedTotal else {
            throw BinaryProtocolError.insufficientData(data.count)
        }
        
        let payload = data.subdata(in: BINARY_HEADER_SIZE..<expectedTotal)
        
        return CrossLanguageMessage(
            header: header,
            payload: payload
        )
    }
    
    private init(header: BinaryMessageHeader, payload: Data) {
        self.header = header
        self.payload = payload
    }
}

// MARK: - TCP Socket Client

/// TCP socket client for cross-language communication
public class CrossLanguageSocketClient {
    private let logger = Logger(label: "cross-language-client")
    private let serverHost: String
    private let serverPort: UInt16
    private let clientInfo: ClientInfo
    private var connection: NWConnection?
    
    public init(serverHost: String = "127.0.0.1", serverPort: UInt16 = 9080, clientId: String) {
        self.serverHost = serverHost
        self.serverPort = serverPort
        self.clientInfo = ClientInfo(clientId: clientId)
    }
    
    /// Connect to the server
    public func connect() async throws {
        let endpoint = NWEndpoint.hostPort(host: NWEndpoint.Host(serverHost), port: NWEndpoint.Port(integerLiteral: serverPort))
        let connection = NWConnection(to: endpoint, using: .tcp)
        
        self.connection = connection
        
        return try await withCheckedThrowingContinuation { continuation in
            connection.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    self.logger.info("✅ Connected to server at \(self.serverHost):\(self.serverPort)")
                    continuation.resume()
                case .failed(let error):
                    self.logger.error("❌ Connection failed: \(error)")
                    continuation.resume(throwing: error)
                case .cancelled:
                    self.logger.info("🔌 Connection cancelled")
                    continuation.resume(throwing: URLError(.cancelled))
                default:
                    break
                }
            }
            
            connection.start(queue: .global())
        }
    }
    
    /// Send client info to server
    public func sendClientInfo() async throws {
        guard let connection = self.connection else {
            throw URLError(.notConnectedToInternet)
        }
        
        let infoData = try JSONEncoder().encode(clientInfo)
        let message = CrossLanguageMessage(
            messageType: .clientInfo,
            payload: infoData,
            sequence: 0
        )
        
        try await sendMessage(message, connection: connection)
        logger.info("📤 Sent client info to server")
    }
    
    /// Run benchmark with the server
    public func runBenchmark(messageCount: Int, messageSize: Int) async throws -> CrossLanguageBenchmarkResults {
        guard let connection = self.connection else {
            throw URLError(.notConnectedToInternet)
        }
        
        logger.info("🚀 Starting benchmark: \(messageCount) messages × \(messageSize) bytes")
        
        let benchmarkStart = Date()
        var totalLatency: TimeInterval = 0
        var successfulMessages = 0
        
        // Send benchmark messages
        for i in 0..<messageCount {
            let messageStart = Date()
            
            // Create benchmark payload
            let payload = Data(repeating: 0x42, count: messageSize)
            let message = CrossLanguageMessage(
                messageType: .benchmarkRequest,
                payload: payload,
                sequence: UInt64(i)
            )
            
            do {
                // Send message and wait for response
                try await sendMessage(message, connection: connection)
                let response = try await receiveMessage(connection: connection)
                
                // Validate response
                if response.header.sequence == UInt64(i) {
                    successfulMessages += 1
                    totalLatency += Date().timeIntervalSince(messageStart)
                }
                
                // Progress indicator
                if i % 100 == 0 && i > 0 {
                    logger.info("📈 Progress: \(i)/\(messageCount) messages")
                }
                
            } catch {
                logger.error("❌ Message \(i) failed: \(error)")
            }
        }
        
        let benchmarkDuration = Date().timeIntervalSince(benchmarkStart)
        
        logger.info("🎯 Benchmark completed: \(successfulMessages)/\(messageCount) messages successful")
        
        return CrossLanguageBenchmarkResults(
            testName: "Swift→Rust Cross-Language (\(messageSize) bytes)",
            messageCount: messageCount,
            messageSize: messageSize,
            swiftToRustDuration: benchmarkDuration,
            rustToSwiftDuration: 0, // Not applicable for single direction
            totalDuration: benchmarkDuration,
            successfulSwiftToRust: successfulMessages,
            successfulRustToSwift: 0,
            serializationOverhead: calculateSerializationOverhead(messageSize: messageSize)
        )
    }
    
    /// Get server statistics
    public func getServerStats() async throws -> ServerStats {
        guard let connection = self.connection else {
            throw URLError(.notConnectedToInternet)
        }
        
        let message = CrossLanguageMessage(
            messageType: .serverStats,
            payload: Data(),
            sequence: 999
        )
        
        try await sendMessage(message, connection: connection)
        let response = try await receiveMessage(connection: connection)
        
        return try JSONDecoder().decode(ServerStats.self, from: response.payload)
    }
    
    /// Disconnect from server
    public func disconnect() {
        connection?.cancel()
        connection = nil
        logger.info("🔌 Disconnected from server")
    }
    
    // MARK: - Private Methods
    
    /// Send a message to the server
    private func sendMessage(_ message: CrossLanguageMessage, connection: NWConnection) async throws {
        let messageData = message.toBytes()
        let sizeData = withUnsafeBytes(of: UInt32(messageData.count).littleEndian) { Data($0) }
        
        // Send message size first
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            connection.send(content: sizeData, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
        
        // Send message data
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            connection.send(content: messageData, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
    }
    
    /// Receive a message from the server
    private func receiveMessage(connection: NWConnection) async throws -> CrossLanguageMessage {
        // Read message size first (4 bytes)
        let sizeData = try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Data, Error>) in
            connection.receive(minimumIncompleteLength: 4, maximumLength: 4) { data, _, isComplete, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: URLError(.badServerResponse))
                }
            }
        }
        
        let messageSize = sizeData.withUnsafeBytes { $0.loadUnaligned(as: UInt32.self).littleEndian }
        
        // Read message data
        let messageData = try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Data, Error>) in
            connection.receive(minimumIncompleteLength: Int(messageSize), maximumLength: Int(messageSize)) { data, _, isComplete, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: URLError(.badServerResponse))
                }
            }
        }
        
        return try CrossLanguageMessage.fromBytes(messageData)
    }
    
    /// Calculate serialization overhead
    private func calculateSerializationOverhead(messageSize: Int) -> Double {
        let headerSize = BINARY_HEADER_SIZE
        let totalSize = headerSize + messageSize
        return Double(headerSize) / Double(totalSize)
    }
}

// MARK: - Benchmark Runner

/// Cross-language benchmark runner
public class RealCrossLanguageBenchmark {
    private let logger = Logger(label: "real-cross-language-benchmark")
    private let client: CrossLanguageSocketClient
    
    public init(clientId: String = "swift-client-\(UUID().uuidString.prefix(8))") {
        self.client = CrossLanguageSocketClient(clientId: clientId)
    }
    
    /// Run the complete benchmark suite
    public func runBenchmarkSuite() async throws -> [CrossLanguageBenchmarkResults] {
        logger.info("🚀 Starting real cross-language benchmark suite")
        
        // Connect to server
        try await client.connect()
        try await client.sendClientInfo()
        
        var results: [CrossLanguageBenchmarkResults] = []
        
        // Define test cases
        let testCases: [(String, Int, Int)] = [
            ("Real Cross-Language Small Messages (1KB)", 500, 1024),
            ("Real Cross-Language Medium Messages (64KB)", 100, 64 * 1024),
            ("Real Cross-Language Large Messages (1MB)", 50, 1024 * 1024),
            ("Real Cross-Language Huge Messages (4MB)", 20, 4 * 1024 * 1024),
        ]
        
        for (testName, messageCount, messageSize) in testCases {
            logger.info("🔬 Running test: \(testName)")
            
            do {
                let result = try await client.runBenchmark(messageCount: messageCount, messageSize: messageSize)
                result.printSummary()
                results.append(result)
                
                // Wait between tests
                try await Task.sleep(nanoseconds: 2_000_000_000) // 2 seconds
                
            } catch {
                logger.error("❌ Test \(testName) failed: \(error)")
            }
        }
        
        // Get server statistics
        do {
            let stats = try await client.getServerStats()
            logger.info("📊 Server Stats: \(stats.totalMessages) messages, \(stats.totalBytes) bytes, \(stats.rustClients) Rust clients, \(stats.swiftClients) Swift clients")
        } catch {
            logger.warning("⚠️ Could not get server stats: \(error)")
        }
        
        // Disconnect
        client.disconnect()
        
        return results
    }
}

// MARK: - Benchmark Results (Reuse from RustSwiftBenchmark.swift)

/// Results from real cross-language tests
public struct RealCrossLanguageBenchmarkResults {
    public let testName: String
    public let messageCount: Int
    public let messageSize: Int
    public let duration: TimeInterval
    public let successfulMessages: Int
    public let throughputMBps: Double
    public let averageLatencyMicros: Double
    public let serializationOverhead: Double
    
    public init(
        testName: String,
        messageCount: Int,
        messageSize: Int,
        duration: TimeInterval,
        successfulMessages: Int,
        serializationOverhead: Double
    ) {
        self.testName = testName
        self.messageCount = messageCount
        self.messageSize = messageSize
        self.duration = duration
        self.successfulMessages = successfulMessages
        self.serializationOverhead = serializationOverhead
        
        // Calculate metrics
        let totalBytes = Double(successfulMessages * messageSize)
        self.throughputMBps = (totalBytes / (1024.0 * 1024.0)) / duration
        self.averageLatencyMicros = (duration * 1_000_000) / Double(successfulMessages)
    }
    
    public func printSummary() {
        print("")
        print("=== \(testName) ===")
        print("Messages: \(successfulMessages)/\(messageCount) successful")
        print("Total data: \(String(format: "%.2f", Double(successfulMessages * messageSize) / (1024.0 * 1024.0))) MB")
        print("Duration: \(String(format: "%.3f", duration))s")
        print("Throughput: \(String(format: "%.2f", throughputMBps)) MB/s")
        print("Average latency: \(String(format: "%.2f", averageLatencyMicros)) μs")
        print("Success rate: \(String(format: "%.1f", Double(successfulMessages) / Double(messageCount) * 100))%")
        print("Serialization overhead: \(String(format: "%.2f", serializationOverhead * 100))%")
    }
}
//
//  main.swift
//  Universal Transport Example
//
//  Swift-Rust interoperability demonstration
//

import Foundation
import UniversalTransport
import UniversalTransportSharedMemory
import Logging

// MARK: - Example Data Structures

/// Example message structure for Swift-Rust communication
struct DataProcessingRequest: Codable {
    let operation: String
    let inputData: [Double]
    let parameters: [String: String]
    let timestamp: Double
    let requestId: String
    
    init(operation: String, inputData: [Double], parameters: [String: String] = [:]) {
        self.operation = operation
        self.inputData = inputData
        self.parameters = parameters
        self.timestamp = Date().timeIntervalSince1970
        self.requestId = UUID().uuidString
    }
}

struct DataProcessingResponse: Codable {
    let requestId: String
    let result: [Double]
    let processingTime: Double
    let status: String
    let metadata: [String: String]
    
    init(requestId: String, result: [Double], processingTime: Double, status: String = "success") {
        self.requestId = requestId
        self.result = result
        self.processingTime = processingTime
        self.status = status
        self.metadata = [
            "swift_version": "5.9",
            "processed_at": ISO8601DateFormatter().string(from: Date())
        ]
    }
}

// MARK: - Swift Service Implementation

@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
actor SwiftDataProcessor {
    private let transport: UniversalTransport
    private let logger = Logger(label: "swift-processor")
    
    init() async throws {
        // Initialize transport with optimized configuration
        let config = TransportConfiguration(
            enableSharedMemory: true,
            enableSwiftOptimization: true,
            enableCompression: false,
            enableEncryption: false,
            maxMessageSize: 64 * 1024 * 1024,
            defaultTimeout: 30.0,
            performanceMonitoringEnabled: true
        )
        
        self.transport = try await UniversalTransport(configuration: config)
        logger.info("Swift data processor initialized")
    }
    
    /// Process data and send response back to Rust
    func processDataRequest(_ request: DataProcessingRequest) async throws -> DataProcessingResponse {
        let startTime = Date()
        logger.info("Processing request: \(request.operation) with \(request.inputData.count) data points")
        
        // Simulate data processing based on operation type
        let result: [Double]
        switch request.operation {
        case "sum":
            result = [request.inputData.reduce(0, +)]
        case "multiply":
            let factor = Double(request.parameters["factor"] ?? "2.0") ?? 2.0
            result = request.inputData.map { $0 * factor }
        case "filter":
            let threshold = Double(request.parameters["threshold"] ?? "0.5") ?? 0.5
            result = request.inputData.filter { $0 > threshold }
        case "normalize":
            let max = request.inputData.max() ?? 1.0
            result = max > 0 ? request.inputData.map { $0 / max } : request.inputData
        case "fft_simulation":
            // Simulate FFT processing
            result = Array(0..<request.inputData.count).map { i in
                sin(Double(i) * 0.1) * request.inputData[i]
            }
        default:
            result = request.inputData // Echo back
        }
        
        let processingTime = Date().timeIntervalSince(startTime)
        logger.info("Processed \(request.operation) in \(processingTime)s, result size: \(result.count)")
        
        return DataProcessingResponse(
            requestId: request.requestId,
            result: result,
            processingTime: processingTime
        )
    }
    
    /// Start listening for requests from Rust
    func startListening(regionName: String = "swift-rust-bridge") async throws {
        logger.info("Starting to listen for requests on region: \(regionName)")
        
        while true {
            do {
                // Receive request from Rust
                let request = try await transport.receive(
                    DataProcessingRequest.self,
                    from: NodeInfo.local(id: "rust-service", language: .rust),
                    timeout: 30.0
                )
                
                logger.info("Received request: \(request.requestId)")
                
                // Process the request
                let response = try await processDataRequest(request)
                
                // Send response back to Rust
                try await transport.send(
                    response,
                    to: NodeInfo.local(id: "rust-service", language: .rust)
                )
                
                logger.info("Sent response for request: \(request.requestId)")
                
            } catch {
                logger.error("Error processing request: \(error)")
                // Continue listening despite errors
                try await Task.sleep(nanoseconds: 100_000_000) // 100ms
            }
        }
    }
    
    /// Send a test request to Rust and wait for response
    func sendTestRequest() async throws {
        let testData = Array(0..<1000).map { Double($0) * 0.01 }
        let request = DataProcessingRequest(
            operation: "rust_fft",
            inputData: testData,
            parameters: ["window": "hamming", "size": "1024"]
        )
        
        logger.info("Sending test request to Rust: \(request.requestId)")
        
        let rustNode = NodeInfo.local(id: "rust-service", language: .rust)
        
        // Send request to Rust
        try await transport.send(request, to: rustNode)
        
        // Wait for response
        let response = try await transport.receive(
            DataProcessingResponse.self,
            from: rustNode,
            timeout: 60.0
        )
        
        logger.info("Received response from Rust: \(response.requestId), processing time: \(response.processingTime)s")
        print("✅ Swift → Rust communication successful!")
        print("   Request ID: \(response.requestId)")
        print("   Result size: \(response.result.count)")
        print("   Processing time: \(String(format: "%.3f", response.processingTime))s")
        print("   Status: \(response.status)")
    }
    
    /// Get performance metrics
    func getPerformanceMetrics() async {
        let metrics = await transport.performanceMetrics()
        logger.info("Performance metrics: \(metrics)")
        
        print("\n📊 Performance Metrics:")
        print("   Total operations: \(metrics.totalOperations)")
        print("   Success rate: \(String(format: "%.1f%%", metrics.successRate * 100))")
        print("   Average latency: \(String(format: "%.3f", metrics.averageLatency))s")
        print("   Throughput: \(String(format: "%.1f", metrics.overallThroughput / 1024 / 1024)) MB/s")
        if !metrics.recommendedStrategies.isEmpty {
            print("   Recommended strategies: \(metrics.recommendedStrategies.joined(separator: ", "))")
        }
    }
}

// MARK: - Example Runner

@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
@main
struct UniversalTransportExample {
    static func main() async {
        print("🚀 Universal Transport Swift-Rust Interoperability Example")
        print("============================================================")
        
        let logger = Logger(label: "example-main")
        
        do {
            let processor = try await SwiftDataProcessor()
            
            // Check command line arguments for mode
            let arguments = CommandLine.arguments
            let mode = arguments.count > 1 ? arguments[1] : "interactive"
            
            switch mode {
            case "listen":
                print("🎧 Starting Swift service in listening mode...")
                try await processor.startListening()
                
            case "send":
                print("📤 Sending test request to Rust service...")
                try await processor.sendTestRequest()
                await processor.getPerformanceMetrics()
                
            case "benchmark":
                print("⚡ Running performance benchmark...")
                try await runBenchmark(processor: processor)
                
            default:
                print("🔄 Running interactive demo...")
                try await runInteractiveDemo(processor: processor)
            }
            
        } catch {
            logger.error("Example failed: \(error)")
            print("❌ Error: \(error.localizedDescription)")
            exit(1)
        }
    }
    
    /// Run interactive demonstration
    static func runInteractiveDemo(processor: SwiftDataProcessor) async throws {
        print("\n📋 Available commands:")
        print("   1. Test local Swift processing")
        print("   2. Send request to Rust (if running)")
        print("   3. Show performance metrics")
        print("   4. Run mini benchmark")
        print("   5. Exit")
        
        while true {
            print("\nEnter command (1-5): ", terminator: "")
            if let input = readLine(), let choice = Int(input) {
                switch choice {
                case 1:
                    await testLocalProcessing(processor: processor)
                case 2:
                    await testRustCommunication(processor: processor)
                case 3:
                    await processor.getPerformanceMetrics()
                case 4:
                    try await runMiniBenchmark(processor: processor)
                case 5:
                    print("👋 Goodbye!")
                    return
                default:
                    print("❌ Invalid choice. Please enter 1-5.")
                }
            } else {
                print("❌ Invalid input. Please enter a number.")
            }
        }
    }
    
    /// Test local Swift processing
    static func testLocalProcessing(processor: SwiftDataProcessor) async {
        print("\n🧮 Testing local Swift processing...")
        
        let testData = Array(0..<100).map { Double($0) }
        let operations = ["sum", "multiply", "filter", "normalize", "fft_simulation"]
        
        for operation in operations {
            let request = DataProcessingRequest(
                operation: operation,
                inputData: testData,
                parameters: operation == "multiply" ? ["factor": "3.0"] : 
                           operation == "filter" ? ["threshold": "50.0"] : [:]
            )
            
            do {
                let response = try await processor.processDataRequest(request)
                print("   ✅ \(operation): \(response.result.count) results in \(String(format: "%.3f", response.processingTime))s")
            } catch {
                print("   ❌ \(operation): \(error.localizedDescription)")
            }
        }
    }
    
    /// Test communication with Rust
    static func testRustCommunication(processor: SwiftDataProcessor) async {
        print("\n🦀 Testing Swift → Rust communication...")
        
        do {
            try await processor.sendTestRequest()
        } catch {
            print("❌ Rust communication failed: \(error.localizedDescription)")
            print("💡 Make sure the Rust service is running with: cargo run --bin rust-service")
        }
    }
    
    /// Run mini benchmark
    static func runMiniBenchmark(processor: SwiftDataProcessor) async throws {
        print("\n⚡ Running mini benchmark...")
        
        let dataSizes = [100, 1000, 10000]
        let operations = ["sum", "multiply", "normalize"]
        
        for dataSize in dataSizes {
            for operation in operations {
                let testData = Array(0..<dataSize).map { Double($0) }
                let request = DataProcessingRequest(
                    operation: operation,
                    inputData: testData,
                    parameters: ["factor": "2.0"]
                )
                
                let startTime = Date()
                let _ = try await processor.processDataRequest(request)
                let duration = Date().timeIntervalSince(startTime)
                
                let throughput = Double(dataSize) / duration
                print("   📊 \(operation) (\(dataSize) items): \(String(format: "%.3f", duration))s, \(String(format: "%.0f", throughput)) items/s")
            }
        }
    }
    
    /// Run comprehensive benchmark
    static func runBenchmark(processor: SwiftDataProcessor) async throws {
        print("\n⚡ Running comprehensive benchmark...")
        
        let iterations = 100
        let dataSize = 10000
        let testData = Array(0..<dataSize).map { _ in Double.random(in: 0...1) }
        
        print("   Testing with \(iterations) iterations of \(dataSize) data points each...")
        
        var totalTime: TimeInterval = 0
        var successCount = 0
        
        for i in 0..<iterations {
            let request = DataProcessingRequest(
                operation: "fft_simulation",
                inputData: testData
            )
            
            do {
                let response = try await processor.processDataRequest(request)
                totalTime += response.processingTime
                successCount += 1
                
                if (i + 1) % 20 == 0 {
                    print("   Progress: \(i + 1)/\(iterations) (\(String(format: "%.1f", Double(i + 1) / Double(iterations) * 100))%)")
                }
            } catch {
                print("   ❌ Iteration \(i + 1) failed: \(error)")
            }
        }
        
        let avgTime = totalTime / Double(successCount)
        let throughput = Double(dataSize) / avgTime
        let successRate = Double(successCount) / Double(iterations) * 100
        
        print("\n📊 Benchmark Results:")
        print("   Total iterations: \(iterations)")
        print("   Successful: \(successCount) (\(String(format: "%.1f", successRate))%)")
        print("   Average processing time: \(String(format: "%.3f", avgTime))s")
        print("   Throughput: \(String(format: "%.0f", throughput)) items/s")
        print("   Total data processed: \(String(format: "%.1f", Double(successCount * dataSize) / 1_000_000)) million items")
        
        await processor.getPerformanceMetrics()
    }
}
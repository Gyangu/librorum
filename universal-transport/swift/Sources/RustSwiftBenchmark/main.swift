import Foundation
import Logging

@main
struct RustSwiftBenchmarkApp {
    static func main() async {
        LoggingSystem.bootstrap(StreamLogHandler.standardOutput)
        
        print("🚀 Real Rust ↔ Swift Cross-Language Benchmark")
        print("===========================================")
        print("Testing actual IPC performance with TCP sockets")
        print("")
        
        let benchmark = RealCrossLanguageBenchmark()
        
        do {
            let results = try await benchmark.runBenchmarkSuite()
            
            print("")
            print("🎯 CROSS-LANGUAGE BENCHMARK SUMMARY")
            print("===================================")
            
            var totalThroughput: Double = 0
            var totalLatency: Double = 0
            var totalSuccessRate: Double = 0
            
            for result in results {
                let successRate = Double(result.successfulMessages) / Double(result.messageCount) * 100
                totalThroughput += result.throughputMBps
                totalLatency += result.averageLatencyMicros
                totalSuccessRate += successRate
                
                print("  \(result.testName):")
                print("    Throughput: \(String(format: "%.2f", result.throughputMBps)) MB/s")
                print("    Latency: \(String(format: "%.2f", result.averageLatencyMicros)) μs")
                print("    Success: \(String(format: "%.1f", successRate))%")
            }
            
            let avgThroughput = totalThroughput / Double(results.count)
            let avgLatency = totalLatency / Double(results.count)
            let avgSuccessRate = totalSuccessRate / Double(results.count)
            
            print("")
            print("📊 Overall Performance:")
            print("  Average throughput: \(String(format: "%.2f", avgThroughput)) MB/s")
            print("  Average latency: \(String(format: "%.2f", avgLatency)) μs")
            print("  Average success rate: \(String(format: "%.1f", avgSuccessRate))%")
            
            print("")
            print("🔍 Protocol Features:")
            print("  ✓ Zero-copy binary protocol")
            print("  ✓ TCP socket communication")
            print("  ✓ CRC32 checksums")
            print("  ✓ Little-endian byte order")
            print("  ✓ 32-byte fixed headers")
            print("  ✓ Cross-language compatibility")
            
        } catch {
            print("❌ Benchmark failed: \(error)")
            
            // If server is not running, provide instructions
            if error.localizedDescription.contains("Connection refused") || 
               error.localizedDescription.contains("not connected") {
                print("")
                print("💡 To run this benchmark:")
                print("  1. Start the Rust server:")
                print("     cargo run --example cross_language_server server")
                print("  2. Run this Swift client in another terminal:")
                print("     swift run RustSwiftBenchmark")
                print("")
                print("  The server will handle both Rust and Swift clients simultaneously.")
            }
        }
        
        print("")
        print("✅ Cross-language benchmark completed!")
    }
}
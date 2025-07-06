//! Binary Protocol Performance Benchmark
//! 
//! High-performance benchmark using TCP-like fixed binary protocol
//! instead of JSON serialization

use universal_transport_core::binary_protocol::*;
use std::time::{Duration, Instant};
use bytes::Bytes;

/// Performance test results
#[derive(Debug)]
pub struct BinaryBenchmarkResults {
    pub test_name: String,
    pub message_count: usize,
    pub message_size: usize,
    pub duration: Duration,
    pub throughput_mbps: f64,
    pub messages_per_second: f64,
    pub avg_latency_us: f64,
    pub serialization_overhead: f64,
}

impl BinaryBenchmarkResults {
    pub fn print_summary(&self) {
        println!("\n=== {} ===", self.test_name);
        println!("Messages: {} × {} bytes", self.message_count, self.message_size);
        println!("Duration: {:.3}s", self.duration.as_secs_f64());
        println!("Throughput: {:.2} MB/s", self.throughput_mbps);
        println!("Rate: {:.0} messages/sec", self.messages_per_second);
        println!("Avg Latency: {:.2} μs", self.avg_latency_us);
        println!("Serialization overhead: {:.2}%", self.serialization_overhead);
    }
}

/// Binary protocol benchmark runner
pub struct BinaryProtocolBenchmark;

impl BinaryProtocolBenchmark {
    /// Run complete benchmark suite
    pub fn run_benchmark_suite() -> Vec<BinaryBenchmarkResults> {
        println!("🚀 Binary Protocol Performance Benchmark");
        println!("========================================");
        println!("Testing TCP-like fixed binary protocol (no JSON overhead)");
        println!();
        
        let test_cases = vec![
            ("Binary Small Messages (1KB)", 1000, 1024),
            ("Binary Medium Messages (64KB)", 200, 64 * 1024),
            ("Binary Large Messages (1MB)", 50, 1024 * 1024),
            ("Binary Huge Messages (16MB)", 10, 16 * 1024 * 1024),
        ];
        
        let mut results = Vec::new();
        
        for (test_name, message_count, message_size) in test_cases {
            println!("🔬 {}: {} messages × {} bytes", test_name, message_count, message_size);
            
            let result = Self::run_binary_serialization_test(test_name, message_count, message_size);
            result.print_summary();
            results.push(result);
            
            println!("{}", "─".repeat(60));
        }
        
        // Run latency test
        println!("🔬 Running binary protocol latency test...");
        let latency_result = Self::run_latency_test();
        latency_result.print_summary();
        results.push(latency_result);
        
        results
    }
    
    /// Test binary protocol serialization performance
    fn run_binary_serialization_test(
        test_name: &str,
        message_count: usize,
        message_size: usize,
    ) -> BinaryBenchmarkResults {
        // Generate benchmark messages
        let messages: Vec<BenchmarkMessage> = (0..message_count)
            .map(|i| BenchmarkMessage::new(i as u64, message_size))
            .collect();
        
        let start = Instant::now();
        let mut total_serialized_bytes = 0;
        let mut total_raw_bytes = 0;
        
        // Test serialization and deserialization
        for message in &messages {
            // Convert to binary message
            let binary_msg = message.to_binary_message().expect("Failed to create binary message");
            
            // Serialize to bytes
            let serialized = binary_msg.to_bytes();
            total_serialized_bytes += serialized.len();
            total_raw_bytes += message.data.len();
            
            // Deserialize back
            let deserialized = BinaryMessage::from_bytes(&serialized)
                .expect("Failed to deserialize");
            
            // Convert back to benchmark message
            let recovered = BenchmarkMessage::from_binary_message(&deserialized)
                .expect("Failed to recover benchmark message");
            
            // Verify data integrity
            assert_eq!(message.id, recovered.id);
            assert_eq!(message.data.len(), recovered.data.len());
        }
        
        let duration = start.elapsed();
        
        // Calculate metrics
        let throughput_mbps = (total_serialized_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
        let messages_per_second = message_count as f64 / duration.as_secs_f64();
        let avg_latency_us = duration.as_micros() as f64 / message_count as f64;
        let serialization_overhead = ((total_serialized_bytes as f64 - total_raw_bytes as f64) / total_raw_bytes as f64) * 100.0;
        
        BinaryBenchmarkResults {
            test_name: test_name.to_string(),
            message_count,
            message_size,
            duration,
            throughput_mbps,
            messages_per_second,
            avg_latency_us,
            serialization_overhead,
        }
    }
    
    /// Test latency with small messages
    fn run_latency_test() -> BinaryBenchmarkResults {
        let test_name = "Binary Protocol Latency Test";
        let iterations = 10000;
        let message_size = 64; // Small message for latency
        
        let start = Instant::now();
        let mut total_bytes = 0;
        
        for i in 0..iterations {
            let message = BenchmarkMessage::new(i as u64, message_size);
            let binary_msg = message.to_binary_message().unwrap();
            let serialized = binary_msg.to_bytes();
            total_bytes += serialized.len();
            
            // Immediate deserialization (simulating round-trip)
            let _deserialized = BinaryMessage::from_bytes(&serialized).unwrap();
        }
        
        let duration = start.elapsed();
        
        let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
        let messages_per_second = iterations as f64 / duration.as_secs_f64();
        let avg_latency_us = duration.as_micros() as f64 / iterations as f64;
        
        BinaryBenchmarkResults {
            test_name: test_name.to_string(),
            message_count: iterations,
            message_size,
            duration,
            throughput_mbps,
            messages_per_second,
            avg_latency_us,
            serialization_overhead: 0.0, // Calculate separately if needed
        }
    }
    
    /// Compare with JSON serialization
    pub fn compare_with_json() {
        println!("\n📊 Binary vs JSON Comparison");
        println!("=============================");
        
        let test_message = BenchmarkMessage::new(1, 1024);
        
        // Binary serialization
        let binary_start = Instant::now();
        let binary_msg = test_message.to_binary_message().unwrap();
        let binary_bytes = binary_msg.to_bytes();
        let binary_duration = binary_start.elapsed();
        
        // Simulate JSON serialization overhead
        let json_start = Instant::now();
        let json_string = format!(
            r#"{{"id":{},"timestamp":{},"data_len":{},"metadata":"{}"}}"#,
            test_message.id,
            test_message.timestamp,
            test_message.data.len(),
            test_message.metadata
        );
        let json_bytes = json_string.as_bytes().len() + test_message.data.len();
        let json_duration = json_start.elapsed();
        
        println!("Binary protocol:");
        println!("  Size: {} bytes", binary_bytes.len());
        println!("  Time: {:?}", binary_duration);
        println!("  Header overhead: {} bytes", HEADER_SIZE);
        
        println!("JSON equivalent:");
        println!("  Size: {} bytes", json_bytes);
        println!("  Time: {:?}", json_duration);
        println!("  Overhead: ~{}% size increase", 
                 ((json_bytes as f64 - binary_bytes.len() as f64) / binary_bytes.len() as f64) * 100.0);
        
        println!("Binary advantage:");
        println!("  Size reduction: {:.1}x smaller", json_bytes as f64 / binary_bytes.len() as f64);
        println!("  Speed improvement: {:.1}x faster", json_duration.as_nanos() as f64 / binary_duration.as_nanos() as f64);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run binary protocol benchmarks
    let results = BinaryProtocolBenchmark::run_benchmark_suite();
    
    // Display summary
    println!("\n🎯 BINARY PROTOCOL BENCHMARK SUMMARY");
    println!("====================================");
    
    for result in &results {
        println!("{}: {:.2} MB/s, {:.2} μs latency", 
                 result.test_name, result.throughput_mbps, result.avg_latency_us);
    }
    
    // Compare with JSON
    BinaryProtocolBenchmark::compare_with_json();
    
    // Calculate overall statistics
    let avg_throughput = results.iter()
        .map(|r| r.throughput_mbps)
        .sum::<f64>() / results.len() as f64;
    
    let avg_overhead = results.iter()
        .filter(|r| r.serialization_overhead > 0.0)
        .map(|r| r.serialization_overhead)
        .sum::<f64>() / results.iter().filter(|r| r.serialization_overhead > 0.0).count() as f64;
    
    println!("\n📈 Performance Summary:");
    println!("  Average throughput: {:.2} MB/s", avg_throughput);
    println!("  Average overhead: {:.2}%", avg_overhead);
    println!("  Protocol efficiency: Binary headers + CRC32 validation");
    
    println!("\n✅ Binary protocol benchmark completed!");
    
    Ok(())
}
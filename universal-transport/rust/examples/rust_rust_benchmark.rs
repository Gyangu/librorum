//! Rust ↔ Rust Performance Benchmark
//! 
//! This benchmark tests the actual performance of Universal Transport Protocol
//! for same-language communication using shared memory.

use universal_transport_core::prelude::*;
use universal_transport_shared_memory::{SharedMemoryTransportAdapter, SharedMemoryConfig};
use bincode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{info, warn, error};

/// Test message structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkMessage {
    pub id: u64,
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub metadata: String,
}

impl BenchmarkMessage {
    pub fn new(id: u64, data_size: usize) -> Self {
        Self {
            id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            data: vec![0x42; data_size],
            metadata: format!("benchmark_message_{}", id),
        }
    }
    
    pub fn size(&self) -> usize {
        bincode::serialize(self).unwrap_or_default().len()
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub test_name: String,
    pub message_count: usize,
    pub message_size: usize,
    pub total_duration: Duration,
    pub send_duration: Duration,
    pub receive_duration: Duration,
    pub successful_sends: usize,
    pub successful_receives: usize,
    pub send_throughput_mbps: f64,
    pub receive_throughput_mbps: f64,
    pub overall_throughput_mbps: f64,
    pub average_latency_us: f64,
}

impl BenchmarkResults {
    pub fn print_summary(&self) {
        println!("\n=== {} ===", self.test_name);
        println!("Messages: {} × {} bytes each", self.message_count, self.message_size);
        println!("Total data: {:.2} MB", (self.message_count * self.message_size) as f64 / (1024.0 * 1024.0));
        println!("Success rate: {}/{} sends, {}/{} receives", 
                 self.successful_sends, self.message_count,
                 self.successful_receives, self.message_count);
        println!("Duration: {:.3}s total ({:.3}s send, {:.3}s receive)", 
                 self.total_duration.as_secs_f64(),
                 self.send_duration.as_secs_f64(),
                 self.receive_duration.as_secs_f64());
        println!("Throughput: {:.2} MB/s send, {:.2} MB/s receive, {:.2} MB/s overall",
                 self.send_throughput_mbps,
                 self.receive_throughput_mbps,
                 self.overall_throughput_mbps);
        println!("Average latency: {:.2} μs", self.average_latency_us);
    }
}

/// Rust-to-Rust benchmark runner
pub struct RustRustBenchmark {
    transport: Arc<SharedMemoryTransportAdapter>,
    sender_node: NodeInfo,
    receiver_node: NodeInfo,
}

impl RustRustBenchmark {
    pub fn new() -> Self {
        let config = SharedMemoryConfig {
            default_region_size: 128 * 1024 * 1024, // 128MB
            message_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(5),
            max_retries: 3,
            enable_optimizations: true,
        };
        
        let transport = Arc::new(SharedMemoryTransportAdapter::new(config));
        
        let sender_node = NodeInfo::new("rust-sender", Language::Rust);
        let mut receiver_node = NodeInfo::new("rust-receiver", Language::Rust);
        receiver_node.shared_memory_name = Some("benchmark_region".to_string());
        
        Self {
            transport,
            sender_node,
            receiver_node,
        }
    }
    
    /// Run a complete benchmark suite
    pub async fn run_benchmark_suite(&self) -> anyhow::Result<Vec<BenchmarkResults>> {
        info!("Starting Rust ↔ Rust benchmark suite");
        
        let mut results = Vec::new();
        
        // Test different message sizes
        let test_cases = vec![
            ("Small Messages (1KB)", 1000, 1024),
            ("Medium Messages (64KB)", 100, 64 * 1024),
            ("Large Messages (1MB)", 50, 1024 * 1024),
            ("Huge Messages (16MB)", 10, 16 * 1024 * 1024),
        ];
        
        for (test_name, message_count, message_size) in test_cases {
            info!("Running test: {}", test_name);
            
            match self.run_throughput_test(test_name, message_count, message_size).await {
                Ok(result) => {
                    result.print_summary();
                    results.push(result);
                }
                Err(e) => {
                    error!("Test {} failed: {}", test_name, e);
                }
            }
            
            // Wait between tests
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        
        // Run latency test
        info!("Running latency test");
        match self.run_latency_test().await {
            Ok(result) => {
                result.print_summary();
                results.push(result);
            }
            Err(e) => {
                error!("Latency test failed: {}", e);
            }
        }
        
        Ok(results)
    }
    
    /// Run throughput test
    async fn run_throughput_test(&self, test_name: &str, message_count: usize, message_size: usize) -> anyhow::Result<BenchmarkResults> {
        // Check connectivity
        if !self.transport.can_communicate_with(&self.receiver_node).await {
            anyhow::bail!("Cannot communicate with receiver node");
        }
        
        info!("Starting {} test: {} messages × {} bytes", test_name, message_count, message_size);
        
        let total_start = Instant::now();
        
        // Generate test messages
        let messages: Vec<BenchmarkMessage> = (0..message_count)
            .map(|i| BenchmarkMessage::new(i as u64, message_size))
            .collect();
        
        info!("Generated {} test messages", messages.len());
        
        // Send phase
        let send_start = Instant::now();
        let mut successful_sends = 0;
        
        for (i, message) in messages.iter().enumerate() {
            let serialized = bincode::serialize(message)?;
            
            match self.transport.send(&serialized, &self.receiver_node).await {
                Ok(()) => {
                    successful_sends += 1;
                    
                    if i % (message_count / 10).max(1) == 0 {
                        info!("Sent {}/{} messages", i + 1, message_count);
                    }
                }
                Err(e) => {
                    warn!("Failed to send message {}: {}", i, e);
                }
            }
        }
        
        let send_duration = send_start.elapsed();
        info!("Send phase completed: {}/{} messages in {:.3}s", 
              successful_sends, message_count, send_duration.as_secs_f64());
        
        // Receive phase
        let receive_start = Instant::now();
        let mut successful_receives = 0;
        let mut received_messages = Vec::new();
        
        for i in 0..message_count {
            match timeout(
                Duration::from_secs(5),
                self.transport.receive(&self.sender_node, 5000)
            ).await {
                Ok(Ok(data)) => {
                    match bincode::deserialize::<BenchmarkMessage>(&data) {
                        Ok(message) => {
                            received_messages.push(message);
                            successful_receives += 1;
                            
                            if i % (message_count / 10).max(1) == 0 {
                                info!("Received {}/{} messages", i + 1, message_count);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to deserialize message {}: {}", i, e);
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("Failed to receive message {}: {}", i, e);
                }
                Err(_) => {
                    warn!("Timeout receiving message {}", i);
                }
            }
        }
        
        let receive_duration = receive_start.elapsed();
        let total_duration = total_start.elapsed();
        
        info!("Receive phase completed: {}/{} messages in {:.3}s", 
              successful_receives, message_count, receive_duration.as_secs_f64());
        
        // Verify message integrity
        let mut integrity_ok = true;
        for received in received_messages.iter() {
            if let Some(original) = messages.get(received.id as usize) {
                if received.data != original.data {
                    warn!("Data mismatch for message {}", received.id);
                    integrity_ok = false;
                }
            }
        }
        
        if integrity_ok {
            info!("✓ Message integrity check passed");
        } else {
            warn!("✗ Message integrity check failed");
        }
        
        // Calculate metrics
        let total_bytes = successful_sends * message_size;
        let send_throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / send_duration.as_secs_f64();
        let receive_throughput_mbps = (successful_receives * message_size) as f64 / (1024.0 * 1024.0) / receive_duration.as_secs_f64();
        let overall_throughput_mbps = (total_bytes as f64 * 2.0) / (1024.0 * 1024.0) / total_duration.as_secs_f64();
        
        // Calculate average latency (simplified)
        let average_latency_us = if successful_receives > 0 {
            total_duration.as_micros() as f64 / successful_receives as f64
        } else {
            0.0
        };
        
        Ok(BenchmarkResults {
            test_name: test_name.to_string(),
            message_count,
            message_size,
            total_duration,
            send_duration,
            receive_duration,
            successful_sends,
            successful_receives,
            send_throughput_mbps,
            receive_throughput_mbps,
            overall_throughput_mbps,
            average_latency_us,
        })
    }
    
    /// Run latency test
    async fn run_latency_test(&self) -> anyhow::Result<BenchmarkResults> {
        let test_name = "Latency Test (Round-trip)";
        let iterations = 1000;
        let message_size = 64; // Small messages for latency test
        
        info!("Starting latency test: {} round-trips", iterations);
        
        let total_start = Instant::now();
        let mut latencies = Vec::new();
        let mut successful_sends = 0;
        let mut successful_receives = 0;
        
        for i in 0..iterations {
            let message = BenchmarkMessage::new(i as u64, message_size);
            let serialized = bincode::serialize(&message)?;
            
            let round_trip_start = Instant::now();
            
            // Send
            match self.transport.send(&serialized, &self.receiver_node).await {
                Ok(()) => {
                    successful_sends += 1;
                    
                    // Receive
                    match timeout(
                        Duration::from_secs(1),
                        self.transport.receive(&self.sender_node, 1000)
                    ).await {
                        Ok(Ok(_)) => {
                            let latency = round_trip_start.elapsed();
                            latencies.push(latency);
                            successful_receives += 1;
                        }
                        _ => {
                            warn!("Failed to receive response for iteration {}", i);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to send message {}: {}", i, e);
                }
            }
            
            if i % 100 == 0 && i > 0 {
                info!("Completed {} latency tests", i);
            }
        }
        
        let total_duration = total_start.elapsed();
        
        // Calculate latency statistics
        let average_latency_us = if !latencies.is_empty() {
            latencies.iter().map(|d| d.as_micros() as f64).sum::<f64>() / latencies.len() as f64
        } else {
            0.0
        };
        
        latencies.sort();
        let min_latency_us = latencies.first().map(|d| d.as_micros() as f64).unwrap_or(0.0);
        let max_latency_us = latencies.last().map(|d| d.as_micros() as f64).unwrap_or(0.0);
        let p50_latency_us = if !latencies.is_empty() {
            latencies[latencies.len() / 2].as_micros() as f64
        } else {
            0.0
        };
        let p95_latency_us = if !latencies.is_empty() {
            latencies[(latencies.len() as f64 * 0.95) as usize].as_micros() as f64
        } else {
            0.0
        };
        let p99_latency_us = if !latencies.is_empty() {
            latencies[(latencies.len() as f64 * 0.99) as usize].as_micros() as f64
        } else {
            0.0
        };
        
        info!("Latency statistics:");
        info!("  Successful round-trips: {}/{}", latencies.len(), iterations);
        info!("  Min latency: {:.2} μs", min_latency_us);
        info!("  Max latency: {:.2} μs", max_latency_us);
        info!("  Avg latency: {:.2} μs", average_latency_us);
        info!("  P50 latency: {:.2} μs", p50_latency_us);
        info!("  P95 latency: {:.2} μs", p95_latency_us);
        info!("  P99 latency: {:.2} μs", p99_latency_us);
        
        Ok(BenchmarkResults {
            test_name: test_name.to_string(),
            message_count: iterations,
            message_size,
            total_duration,
            send_duration: total_duration, // For latency test, these are the same
            receive_duration: total_duration,
            successful_sends,
            successful_receives,
            send_throughput_mbps: 0.0, // Not relevant for latency test
            receive_throughput_mbps: 0.0,
            overall_throughput_mbps: 0.0,
            average_latency_us,
        })
    }
    
    /// Get transport metrics
    pub async fn get_transport_metrics(&self) -> TransportMetrics {
        self.transport.get_metrics().await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("🚀 Rust ↔ Rust Performance Benchmark");
    info!("======================================");
    
    let benchmark = RustRustBenchmark::new();
    
    // Run complete benchmark suite
    let results = benchmark.run_benchmark_suite().await?;
    
    // Display final summary
    println!("\n🎯 BENCHMARK SUMMARY");
    println!("====================");
    
    for result in &results {
        println!("{}: {:.2} MB/s overall, {:.2} μs avg latency", 
                 result.test_name,
                 result.overall_throughput_mbps,
                 result.average_latency_us);
    }
    
    // Display transport metrics
    let metrics = benchmark.get_transport_metrics().await;
    println!("\n📊 TRANSPORT METRICS");
    println!("====================");
    println!("Messages sent: {}", metrics.messages_sent);
    println!("Messages received: {}", metrics.messages_received);
    println!("Bytes sent: {} ({:.2} MB)", metrics.bytes_sent, metrics.bytes_sent as f64 / (1024.0 * 1024.0));
    println!("Bytes received: {} ({:.2} MB)", metrics.bytes_received, metrics.bytes_received as f64 / (1024.0 * 1024.0));
    println!("Average throughput: {:.2} MB/s", metrics.average_throughput_mbps);
    println!("Error count: {}", metrics.error_count);
    
    if let Some(ref last_error) = metrics.last_error {
        println!("Last error: {}", last_error);
    }
    
    info!("✅ Rust ↔ Rust benchmark completed successfully!");
    
    Ok(())
}
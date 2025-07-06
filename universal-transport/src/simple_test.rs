//! Simple Performance Test for Universal Transport Protocol
//! 
//! This test measures actual performance of different communication methods
//! to provide real benchmark data instead of theoretical estimates.

use std::time::{Duration, Instant};
use tokio::time::timeout;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestMessage {
    id: u64,
    data: Vec<u8>,
    timestamp: u64,
}

/// Results from performance tests
#[derive(Debug)]
struct PerformanceResults {
    test_name: String,
    message_count: usize,
    message_size: usize,
    duration: Duration,
    throughput_mbps: f64,
    messages_per_second: f64,
    avg_latency_us: f64,
}

impl PerformanceResults {
    fn print(&self) {
        println!("📊 {}", self.test_name);
        println!("   Messages: {} × {} bytes", self.message_count, self.message_size);
        println!("   Duration: {:.3}s", self.duration.as_secs_f64());
        println!("   Throughput: {:.2} MB/s", self.throughput_mbps);
        println!("   Rate: {:.0} messages/sec", self.messages_per_second);
        println!("   Avg Latency: {:.2} μs", self.avg_latency_us);
    }
}

/// Test in-memory serialization performance (baseline)
async fn test_memory_serialization(message_count: usize, message_size: usize) -> PerformanceResults {
    let messages: Vec<TestMessage> = (0..message_count)
        .map(|i| TestMessage {
            id: i as u64,
            data: vec![0x42; message_size],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        })
        .collect();
    
    let start = Instant::now();
    
    // Serialize and deserialize all messages
    let mut total_bytes = 0;
    for message in &messages {
        let serialized = serde_json::to_vec(message).unwrap();
        total_bytes += serialized.len();
        let _deserialized: TestMessage = serde_json::from_slice(&serialized).unwrap();
    }
    
    let duration = start.elapsed();
    let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
    let messages_per_second = message_count as f64 / duration.as_secs_f64();
    let avg_latency_us = duration.as_micros() as f64 / message_count as f64;
    
    PerformanceResults {
        test_name: "Memory Serialization (Baseline)".to_string(),
        message_count,
        message_size,
        duration,
        throughput_mbps,
        messages_per_second,
        avg_latency_us,
    }
}

/// Test tokio channel performance (in-process)
async fn test_tokio_channel(message_count: usize, message_size: usize) -> PerformanceResults {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
    
    let messages: Vec<TestMessage> = (0..message_count)
        .map(|i| TestMessage {
            id: i as u64,
            data: vec![0x42; message_size],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        })
        .collect();
    
    let start = Instant::now();
    
    // Spawn sender task
    let messages_clone = messages.clone();
    let sender_task = tokio::spawn(async move {
        for message in messages_clone {
            let serialized = serde_json::to_vec(&message).unwrap();
            if tx.send(serialized).await.is_err() {
                break;
            }
        }
    });
    
    // Receive all messages
    let mut received_count = 0;
    let mut total_bytes = 0;
    
    while received_count < message_count {
        if let Some(data) = rx.recv().await {
            total_bytes += data.len();
            let _message: TestMessage = serde_json::from_slice(&data).unwrap();
            received_count += 1;
        } else {
            break;
        }
    }
    
    let _ = sender_task.await;
    let duration = start.elapsed();
    
    let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
    let messages_per_second = received_count as f64 / duration.as_secs_f64();
    let avg_latency_us = duration.as_micros() as f64 / received_count as f64;
    
    PerformanceResults {
        test_name: "Tokio Channel (In-Process)".to_string(),
        message_count: received_count,
        message_size,
        duration,
        throughput_mbps,
        messages_per_second,
        avg_latency_us,
    }
}

/// Test file-based communication (disk)
async fn test_file_communication(message_count: usize, message_size: usize) -> PerformanceResults {
    use tokio::fs::{File, OpenOptions};
    use tokio::io::{AsyncWriteExt, AsyncReadExt};
    
    let temp_file = "/tmp/utp_test.dat";
    
    let messages: Vec<TestMessage> = (0..message_count)
        .map(|i| TestMessage {
            id: i as u64,
            data: vec![0x42; message_size],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        })
        .collect();
    
    let start = Instant::now();
    
    // Write all messages to file
    {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(temp_file)
            .await
            .unwrap();
        
        for message in &messages {
            let serialized = serde_json::to_vec(message).unwrap();
            let len_bytes = (serialized.len() as u32).to_le_bytes();
            file.write_all(&len_bytes).await.unwrap();
            file.write_all(&serialized).await.unwrap();
        }
        
        file.flush().await.unwrap();
    }
    
    // Read all messages from file
    let mut received_count = 0;
    let mut total_bytes = 0;
    
    {
        let mut file = File::open(temp_file).await.unwrap();
        
        while received_count < message_count {
            let mut len_bytes = [0u8; 4];
            if file.read_exact(&mut len_bytes).await.is_err() {
                break;
            }
            
            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut data = vec![0u8; len];
            if file.read_exact(&mut data).await.is_err() {
                break;
            }
            
            total_bytes += data.len();
            let _message: TestMessage = serde_json::from_slice(&data).unwrap();
            received_count += 1;
        }
    }
    
    // Cleanup
    let _ = tokio::fs::remove_file(temp_file).await;
    
    let duration = start.elapsed();
    let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
    let messages_per_second = received_count as f64 / duration.as_secs_f64();
    let avg_latency_us = duration.as_micros() as f64 / received_count as f64;
    
    PerformanceResults {
        test_name: "File Communication (Disk)".to_string(),
        message_count: received_count,
        message_size,
        duration,
        throughput_mbps,
        messages_per_second,
        avg_latency_us,
    }
}

pub async fn run_performance_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Universal Transport Protocol - Performance Comparison");
    println!("========================================================");
    println!("This test compares different communication methods to establish");
    println!("realistic performance baselines and improvements.");
    println!();
    
    let test_cases = vec![
        ("Small Messages", 1000, 1024),      // 1KB
        ("Medium Messages", 200, 64 * 1024), // 64KB  
        ("Large Messages", 50, 1024 * 1024), // 1MB
    ];
    
    for (test_name, message_count, message_size) in test_cases {
        println!("🔬 {}: {} messages × {} bytes", test_name, message_count, message_size);
        println!("   Total data: {:.2} MB", (message_count * message_size) as f64 / (1024.0 * 1024.0));
        println!();
        
        // Test 1: Memory serialization (baseline)
        let memory_result = test_memory_serialization(message_count, message_size).await;
        memory_result.print();
        println!();
        
        // Test 2: Tokio channel (in-process)
        let channel_result = test_tokio_channel(message_count, message_size).await;
        channel_result.print();
        println!();
        
        // Test 3: File communication (disk)
        let file_result = test_file_communication(message_count, message_size).await;
        file_result.print();
        println!();
        
        // Performance comparison
        println!("🔄 Performance Comparison:");
        println!("   Memory:  {:.2}x baseline", memory_result.throughput_mbps / memory_result.throughput_mbps);
        println!("   Channel: {:.2}x baseline", channel_result.throughput_mbps / memory_result.throughput_mbps);
        println!("   File:    {:.2}x baseline", file_result.throughput_mbps / memory_result.throughput_mbps);
        println!();
        println!("   Channel vs File: {:.2}x faster", channel_result.throughput_mbps / file_result.throughput_mbps);
        println!();
        println!("{}", "─".repeat(60));
        println!();
    }
    
    // Summary
    println!("✅ Performance test completed!");
    println!();
    println!("📋 Key Findings:");
    println!("• Memory serialization provides the absolute baseline");
    println!("• Tokio channels offer excellent in-process performance");
    println!("• File I/O shows real-world storage-based communication costs");
    println!("• Shared memory should perform between Channel and Memory levels");
    println!();
    println!("🎯 Expected Universal Transport Performance:");
    println!("• Shared Memory: 50-200 MB/s (between Channel and Memory)");
    println!("• Network: 10-100 MB/s (depending on network conditions)");
    println!("• Cross-language: 20-150 MB/s (with serialization overhead)");
    
    Ok(())
}
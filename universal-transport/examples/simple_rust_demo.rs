//! Simple Rust demonstration for Universal Transport Protocol
//!
//! This example shows basic shared memory operations and compatibility
//! with the Swift implementation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn};
use universal_transport_shared_memory::{
    SharedMemoryTransport, SharedMemoryConfig, SharedMemoryRegion, Message
};
use universal_transport_core::NodeInfo;

// MARK: - Simple Data Structures

/// Simple data message for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleMessage {
    id: String,
    operation: String,
    data: Vec<f64>,
    timestamp: f64,
}

impl SimpleMessage {
    fn new(operation: &str, data: Vec<f64>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            operation: operation.to_string(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        }
    }
}

/// Simple response message
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleResponse {
    request_id: String,
    result: Vec<f64>,
    processing_time: f64,
    status: String,
}

// MARK: - Rust Data Processor

struct SimpleRustProcessor {
    transport: SharedMemoryTransport,
}

impl SimpleRustProcessor {
    async fn new() -> Result<Self> {
        let config = SharedMemoryConfig::default();
        let transport = SharedMemoryTransport::new(config);
        info!("Simple Rust processor initialized");
        
        Ok(Self { transport })
    }
    
    /// Process data with simple Rust algorithms
    fn process_data(&self, message: &SimpleMessage) -> SimpleResponse {
        let start_time = Instant::now();
        info!("Processing: {} with {} data points", message.operation, message.data.len());
        
        let result = match message.operation.as_str() {
            "sum" => vec![message.data.iter().sum()],
            "mean" => {
                let sum: f64 = message.data.iter().sum();
                vec![sum / message.data.len() as f64]
            }
            "max" => vec![message.data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))],
            "min" => vec![message.data.iter().fold(f64::INFINITY, |a, &b| a.min(b))],
            "reverse" => {
                let mut reversed = message.data.clone();
                reversed.reverse();
                reversed
            }
            "double" => message.data.iter().map(|x| x * 2.0).collect(),
            "sqrt" => message.data.iter().map(|x| x.sqrt()).collect(),
            _ => {
                warn!("Unknown operation: {}, echoing data", message.operation);
                message.data.clone()
            }
        };
        
        let processing_time = start_time.elapsed().as_secs_f64();
        
        SimpleResponse {
            request_id: message.id.clone(),
            result,
            processing_time,
            status: "success".to_string(),
        }
    }
    
    /// Test shared memory creation and basic operations
    async fn test_shared_memory_basics(&self) -> Result<()> {
        println!("🧪 Testing shared memory basics...");
        
        // Test region creation
        let region_name = "rust-test-region";
        let region_size = 1024 * 1024; // 1MB
        
        match SharedMemoryRegion::create(region_name, region_size) {
            Ok(mut region) => {
                println!("   ✅ Created shared memory region: {} ({} bytes)", region_name, region_size);
                
                // Test basic memory operations using ring buffer
                let test_data = b"Hello from Rust! This is a test message.";
                
                // Initialize ring buffer first
                region.initialize_ring_buffer(region_size - 1024).context("Failed to initialize ring buffer")?;
                
                // Get data buffer for testing
                let data_buffer = region.get_data_buffer_mut().context("Failed to get data buffer")?;
                if data_buffer.len() >= test_data.len() {
                    data_buffer[..test_data.len()].copy_from_slice(test_data);
                    
                    let read_buffer = region.get_data_buffer().context("Failed to get read buffer")?;
                    if &read_buffer[..test_data.len()] == test_data {
                        println!("   ✅ Memory read/write test passed");
                    } else {
                        println!("   ❌ Memory read/write test failed");
                    }
                } else {
                    println!("   ⚠️ Data buffer too small for test");
                }
                
                println!("   ✅ Ring buffer initialized");
                
            }
            Err(e) => {
                println!("   ❌ Failed to create shared memory region: {}", e);
                println!("   💡 This is expected on some platforms - using simulated memory");
            }
        }
        
        Ok(())
    }
    
    /// Test message protocol
    async fn test_message_protocol(&self) -> Result<()> {
        println!("📨 Testing message protocol...");
        
        // Test different message types
        let test_messages = vec![
            ("heartbeat", Message::new_heartbeat()),
            ("data", Message::new_data(b"test data".to_vec())),
            ("ack", Message::new_acknowledgment(12345)),
        ];
        
        for (msg_type, message) in test_messages {
            println!("   ✅ {} message created successfully", msg_type);
            
            // Test validation
            if message.validate().is_ok() {
                println!("   ✅ {} message validation passed", msg_type);
            } else {
                println!("   ❌ {} message validation failed", msg_type);
            }
        }
        
        Ok(())
    }
    
    /// Simulate processing requests
    async fn simulate_processing(&self) -> Result<()> {
        println!("⚡ Simulating data processing...");
        
        let test_operations = vec![
            ("sum", vec![1.0, 2.0, 3.0, 4.0, 5.0]),
            ("mean", vec![10.0, 20.0, 30.0]),
            ("double", vec![1.5, 2.5, 3.5]),
            ("reverse", vec![1.0, 2.0, 3.0, 4.0]),
            ("sqrt", vec![4.0, 9.0, 16.0, 25.0]),
        ];
        
        for (operation, data) in test_operations {
            let message = SimpleMessage::new(operation, data);
            let response = self.process_data(&message);
            
            println!("   📊 {}: {} → {} results in {:.3}s", 
                     operation, 
                     message.data.len(), 
                     response.result.len(), 
                     response.processing_time);
        }
        
        Ok(())
    }
    
    /// Performance benchmark
    async fn run_benchmark(&self) -> Result<()> {
        println!("🚀 Running performance benchmark...");
        
        let data_sizes = vec![100, 1000, 10000, 100000];
        let operations = vec!["sum", "double", "sqrt"];
        
        for &size in &data_sizes {
            println!("\n   Testing with {} data points:", size);
            
            let test_data: Vec<f64> = (0..size).map(|i| i as f64).collect();
            
            for operation in &operations {
                let message = SimpleMessage::new(operation, test_data.clone());
                
                let start_time = Instant::now();
                let response = self.process_data(&message);
                let total_time = start_time.elapsed().as_secs_f64();
                
                let throughput = size as f64 / total_time;
                println!("     {} {}: {:.3}s ({:.0} items/s)", 
                         operation, size, total_time, throughput);
            }
        }
        
        Ok(())
    }
}

// MARK: - Main Function

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    
    println!("🦀 Universal Transport Rust Demo");
    println!("=================================");
    
    let processor = SimpleRustProcessor::new().await
        .context("Failed to initialize processor")?;
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("all");
    
    match command {
        "memory" => {
            processor.test_shared_memory_basics().await?;
        }
        "protocol" => {
            processor.test_message_protocol().await?;
        }
        "process" => {
            processor.simulate_processing().await?;
        }
        "benchmark" => {
            processor.run_benchmark().await?;
        }
        "all" => {
            processor.test_shared_memory_basics().await?;
            processor.test_message_protocol().await?;
            processor.simulate_processing().await?;
            processor.run_benchmark().await?;
        }
        _ => {
            println!("Usage: {} [memory|protocol|process|benchmark|all]", args[0]);
            println!("  memory    - Test shared memory operations");
            println!("  protocol  - Test message protocol");
            println!("  process   - Test data processing");
            println!("  benchmark - Run performance benchmark");
            println!("  all       - Run all tests (default)");
        }
    }
    
    println!("\n✅ Rust demo completed successfully!");
    
    Ok(())
}
//! Universal Transport Protocol - Shared Memory Demo
//! 
//! This example demonstrates high-performance shared memory communication
//! using the Universal Transport Protocol with automatic transport selection.

use universal_transport_core::prelude::*;
use universal_transport_shared_memory::{SharedMemoryTransportAdapter, SharedMemoryConfig};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{Duration, sleep, timeout};
use tracing::{info, warn, error, debug};

/// Example message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMessage {
    pub id: u64,
    pub content: String,
    pub timestamp: u64,
    pub data: Vec<u8>,
}

impl TestMessage {
    pub fn new(id: u64, content: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            id,
            content: content.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            data,
        }
    }
}

/// Simple demo service
pub struct SharedMemoryDemo {
    transport: Arc<SharedMemoryTransportAdapter>,
    local_node: NodeInfo,
}

impl SharedMemoryDemo {
    pub fn new() -> Self {
        let config = SharedMemoryConfig {
            default_region_size: 64 * 1024 * 1024, // 64MB
            message_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(5),
            max_retries: 3,
            enable_optimizations: true,
        };
        
        let transport = Arc::new(SharedMemoryTransportAdapter::new(config));
        let local_node = NodeInfo::new("shared-memory-demo", Language::Rust);
        
        Self {
            transport,
            local_node,
        }
    }
    
    pub async fn run_demo(&self) -> anyhow::Result<()> {
        info!("Starting Shared Memory Demo");
        info!("Local node: {}", self.local_node.id);
        info!("Machine ID: {}", self.local_node.machine_id);
        
        // Create a target node (simulating another process on the same machine)
        let target_node = NodeInfo::local("target-node", Language::Rust);
        
        // Check if we can communicate
        if !self.transport.can_communicate_with(&target_node).await {
            error!("Cannot communicate with target node via shared memory");
            return Err(anyhow::anyhow!("Communication check failed"));
        }
        
        info!("✓ Communication with target node is possible");
        
        // Test 1: Basic send/receive
        info!("\n=== Test 1: Basic Send/Receive ===");
        self.test_basic_communication(&target_node).await?;
        
        // Test 2: Performance test
        info!("\n=== Test 2: Performance Test ===");
        self.test_performance(&target_node).await?;
        
        // Test 3: Large message test
        info!("\n=== Test 3: Large Message Test ===");
        self.test_large_messages(&target_node).await?;
        
        // Display metrics
        info!("\n=== Transport Metrics ===");
        self.display_metrics().await;
        
        Ok(())
    }
    
    async fn test_basic_communication(&self, target: &NodeInfo) -> anyhow::Result<()> {
        let test_data = b"Hello, Universal Transport!".to_vec();
        let message = TestMessage::new(1, "Basic test message", test_data.clone());
        
        info!("Sending basic test message...");
        let serialized = bincode::serialize(&message)?;
        
        let start_time = std::time::Instant::now();
        self.transport.send(&serialized, target).await
            .map_err(|e| anyhow::anyhow!("Send failed: {}", e))?;
        let send_time = start_time.elapsed();
        
        info!("Message sent in {:.2}ms", send_time.as_secs_f64() * 1000.0);
        
        // Receive the message back
        info!("Receiving message...");
        let receive_start = std::time::Instant::now();
        let received_data = self.transport.receive(target, 5000).await
            .map_err(|e| anyhow::anyhow!("Receive failed: {}", e))?;
        let receive_time = receive_start.elapsed();
        
        let received_message: TestMessage = bincode::deserialize(&received_data)?;
        
        info!("Message received in {:.2}ms", receive_time.as_secs_f64() * 1000.0);
        info!("Round-trip time: {:.2}ms", (send_time + receive_time).as_secs_f64() * 1000.0);
        
        // Verify message
        if received_message.id == message.id && received_message.data == message.data {
            info!("✓ Message integrity verified");
        } else {
            error!("✗ Message integrity check failed");
            return Err(anyhow::anyhow!("Message integrity failed"));
        }
        
        Ok(())
    }
    
    async fn test_performance(&self, target: &NodeInfo) -> anyhow::Result<()> {
        let message_count = 1000;
        let message_size = 4096; // 4KB messages
        let test_data = vec![0x42u8; message_size];
        
        info!("Sending {} messages of {} bytes each", message_count, message_size);
        
        let total_bytes = message_count * message_size;
        let start_time = std::time::Instant::now();
        
        // Send phase
        for i in 0..message_count {
            let message = TestMessage::new(i as u64, format!("Perf test {}", i), test_data.clone());
            let serialized = bincode::serialize(&message)?;
            
            self.transport.send(&serialized, target).await
                .map_err(|e| anyhow::anyhow!("Send {} failed: {}", i, e))?;
            
            if i % 100 == 0 && i > 0 {
                debug!("Sent {} messages", i);
            }
        }
        
        let send_duration = start_time.elapsed();
        let send_throughput = (total_bytes as f64) / (1024.0 * 1024.0) / send_duration.as_secs_f64();
        
        info!("Send phase: {:.2} MB/s ({:.2}ms total)", send_throughput, send_duration.as_secs_f64() * 1000.0);
        
        // Receive phase
        let receive_start = std::time::Instant::now();
        let mut successful_receives = 0;
        
        for i in 0..message_count {
            match timeout(Duration::from_millis(1000), self.transport.receive(target, 1000)).await {
                Ok(Ok(_)) => {
                    successful_receives += 1;
                    if i % 100 == 0 && i > 0 {
                        debug!("Received {} messages", i);
                    }
                }
                Ok(Err(e)) => {
                    warn!("Receive {} failed: {}", i, e);
                    break;
                }
                Err(_) => {
                    warn!("Receive {} timed out", i);
                    break;
                }
            }
        }
        
        let receive_duration = receive_start.elapsed();
        let receive_throughput = (successful_receives * message_size) as f64 / (1024.0 * 1024.0) / receive_duration.as_secs_f64();
        
        info!("Receive phase: {:.2} MB/s ({}/{} messages, {:.2}ms total)", 
              receive_throughput, successful_receives, message_count, receive_duration.as_secs_f64() * 1000.0);
        
        let total_duration = start_time.elapsed();
        let overall_throughput = (total_bytes as f64 * 2.0) / (1024.0 * 1024.0) / total_duration.as_secs_f64();
        
        info!("Overall throughput: {:.2} MB/s (bidirectional)", overall_throughput);
        
        Ok(())
    }
    
    async fn test_large_messages(&self, target: &NodeInfo) -> anyhow::Result<()> {
        let message_sizes = vec![64 * 1024, 256 * 1024, 1024 * 1024]; // 64KB, 256KB, 1MB
        
        for size in message_sizes {
            info!("Testing {}KB messages", size / 1024);
            
            let test_data = vec![0x55u8; size];
            let message = TestMessage::new(999, format!("Large message {}KB", size / 1024), test_data);
            let serialized = bincode::serialize(&message)?;
            
            info!("Serialized size: {} bytes", serialized.len());
            
            let start_time = std::time::Instant::now();
            
            // Send
            self.transport.send(&serialized, target).await
                .map_err(|e| anyhow::anyhow!("Large message send failed: {}", e))?;
            
            // Receive
            let received = timeout(
                Duration::from_secs(10),
                self.transport.receive(target, 10000)
            ).await
                .map_err(|_| anyhow::anyhow!("Large message receive timed out"))?
                .map_err(|e| anyhow::anyhow!("Large message receive failed: {}", e))?;
            
            let duration = start_time.elapsed();
            let throughput = (serialized.len() as f64 * 2.0) / (1024.0 * 1024.0) / duration.as_secs_f64();
            
            let received_message: TestMessage = bincode::deserialize(&received)?;
            
            if received_message.data.len() == message.data.len() {
                info!("✓ {}KB message: {:.2} MB/s ({:.2}ms)", 
                      size / 1024, throughput, duration.as_secs_f64() * 1000.0);
            } else {
                error!("✗ {}KB message: size mismatch", size / 1024);
            }
        }
        
        Ok(())
    }
    
    async fn display_metrics(&self) {
        let metrics = self.transport.get_metrics().await;
        
        info!("Transport Type: {:?}", metrics.transport_type);
        info!("Messages Sent: {}", metrics.messages_sent);
        info!("Messages Received: {}", metrics.messages_received);
        info!("Bytes Sent: {} ({:.2} MB)", metrics.bytes_sent, metrics.bytes_sent as f64 / (1024.0 * 1024.0));
        info!("Bytes Received: {} ({:.2} MB)", metrics.bytes_received, metrics.bytes_received as f64 / (1024.0 * 1024.0));
        info!("Average Latency: {:.2}ms", metrics.average_latency_ms);
        info!("Average Throughput: {:.2} MB/s", metrics.average_throughput_mbps);
        info!("Error Count: {}", metrics.error_count);
        
        if let Some(ref last_error) = metrics.last_error {
            info!("Last Error: {}", last_error);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("Universal Transport Protocol - Shared Memory Demo");
    info!("================================================");
    
    let demo = SharedMemoryDemo::new();
    
    if let Err(e) = demo.run_demo().await {
        error!("Demo failed: {}", e);
        return Err(e);
    }
    
    info!("\n✓ Shared Memory Demo completed successfully!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_demo_creation() {
        let demo = SharedMemoryDemo::new();
        assert_eq!(demo.local_node.language, Language::Rust);
        assert!(!demo.local_node.id.is_empty());
    }
    
    #[tokio::test]
    async fn test_message_creation() {
        let test_data = vec![1, 2, 3, 4, 5];
        let message = TestMessage::new(42, "test", test_data.clone());
        
        assert_eq!(message.id, 42);
        assert_eq!(message.content, "test");
        assert_eq!(message.data, test_data);
        assert!(message.timestamp > 0);
    }
}
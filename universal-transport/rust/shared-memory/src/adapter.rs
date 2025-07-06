//! Adapter to integrate SharedMemoryTransport with core Transport trait

use crate::{SharedMemoryTransport, SharedMemoryConfig, SharedMemoryError};
use universal_transport_core::{
    Transport, NodeInfo, TransportType, TransportMetrics, TransportError, Result as CoreResult
};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn, instrument};

/// Adapter that implements the core Transport trait for SharedMemoryTransport
pub struct SharedMemoryTransportAdapter {
    /// Inner shared memory transport
    inner: SharedMemoryTransport,
    /// Metrics tracking
    metrics: Arc<TransportMetricsTracker>,
}

/// Metrics tracker for shared memory transport
struct TransportMetricsTracker {
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    error_count: AtomicU64,
    total_latency_ms: AtomicU64,
    total_operations: AtomicU64,
    last_error: parking_lot::Mutex<Option<String>>,
}

impl TransportMetricsTracker {
    fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            total_operations: AtomicU64::new(0),
            last_error: parking_lot::Mutex::new(None),
        }
    }
    
    fn record_send(&self, bytes: usize, latency_ms: f64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms as u64, Ordering::Relaxed);
        self.total_operations.fetch_add(1, Ordering::Relaxed);
    }
    
    fn record_receive(&self, bytes: usize, latency_ms: f64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms as u64, Ordering::Relaxed);
        self.total_operations.fetch_add(1, Ordering::Relaxed);
    }
    
    fn record_error(&self, error: &str) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        *self.last_error.lock() = Some(error.to_string());
    }
    
    fn get_metrics(&self) -> TransportMetrics {
        let messages_sent = self.messages_sent.load(Ordering::Relaxed);
        let messages_received = self.messages_received.load(Ordering::Relaxed);
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let error_count = self.error_count.load(Ordering::Relaxed);
        let total_latency_ms = self.total_latency_ms.load(Ordering::Relaxed);
        let total_operations = self.total_operations.load(Ordering::Relaxed);
        
        let average_latency_ms = if total_operations > 0 {
            total_latency_ms as f64 / total_operations as f64
        } else {
            0.0
        };
        
        // Calculate throughput in MB/s
        let total_bytes = bytes_sent + bytes_received;
        let total_time_seconds = if total_operations > 0 && average_latency_ms > 0.0 {
            (total_operations as f64 * average_latency_ms) / 1000.0
        } else {
            1.0
        };
        
        let average_throughput_mbps = if total_time_seconds > 0.0 {
            (total_bytes as f64) / (1024.0 * 1024.0 * total_time_seconds)
        } else {
            0.0
        };
        
        TransportMetrics {
            transport_type: TransportType::SharedMemory,
            messages_sent,
            messages_received,
            bytes_sent,
            bytes_received,
            average_latency_ms,
            average_throughput_mbps,
            error_count,
            last_error: self.last_error.lock().clone(),
        }
    }
}

impl SharedMemoryTransportAdapter {
    /// Create a new adapter with the given configuration
    pub fn new(config: SharedMemoryConfig) -> Self {
        Self {
            inner: SharedMemoryTransport::new(config),
            metrics: Arc::new(TransportMetricsTracker::new()),
        }
    }
    
    /// Create with default configuration
    pub fn new_default() -> Self {
        Self::new(SharedMemoryConfig::default())
    }
    
    /// Get the region name for a node
    fn get_region_name(&self, node: &NodeInfo) -> String {
        // Use shared memory name if available, otherwise construct from node info
        node.shared_memory_name.clone().unwrap_or_else(|| {
            format!("utp_{}_{}", node.machine_id, node.id)
        })
    }
    
    /// Check if two nodes are on the same machine
    fn is_same_machine(&self, node: &NodeInfo) -> bool {
        // Check if machine_id matches current machine
        // For now, we'll assume same machine if machine_id matches our local machine id
        // This should be improved with actual machine identification
        get_local_machine_id() == node.machine_id
    }
}

#[async_trait]
impl Transport for SharedMemoryTransportAdapter {
    #[instrument(skip(self, data))]
    async fn send(&self, data: &[u8], destination: &NodeInfo) -> CoreResult<()> {
        let start_time = std::time::Instant::now();
        
        // Check if destination is on the same machine
        if !self.is_same_machine(destination) {
            return Err(TransportError::TransportNotAvailable(TransportType::SharedMemory));
        }
        
        let region_name = self.get_region_name(destination);
        
        debug!("Sending {} bytes to shared memory region: {}", data.len(), region_name);
        
        // Initialize region if it doesn't exist
        if !self.inner.region_exists(&region_name).await {
            self.inner.initialize_region(&region_name, None).await
                .map_err(|e| TransportError::SharedMemory(e.to_string()))?;
        }
        
        // Send data
        match self.inner.send_to_region(&region_name, data).await {
            Ok(()) => {
                let latency = start_time.elapsed().as_secs_f64() * 1000.0;
                self.metrics.record_send(data.len(), latency);
                debug!("Successfully sent {} bytes in {:.2}ms", data.len(), latency);
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.metrics.record_error(&error_msg);
                warn!("Failed to send data: {}", error_msg);
                Err(TransportError::SharedMemory(error_msg))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn receive(&self, source: &NodeInfo, timeout_ms: u64) -> CoreResult<Bytes> {
        let start_time = std::time::Instant::now();
        
        // Check if source is on the same machine
        if !self.is_same_machine(source) {
            return Err(TransportError::TransportNotAvailable(TransportType::SharedMemory));
        }
        
        let region_name = self.get_region_name(source);
        let timeout_duration = tokio::time::Duration::from_millis(timeout_ms);
        
        debug!("Receiving from shared memory region: {} (timeout: {}ms)", region_name, timeout_ms);
        
        // Initialize region if it doesn't exist
        if !self.inner.region_exists(&region_name).await {
            self.inner.initialize_region(&region_name, None).await
                .map_err(|e| TransportError::SharedMemory(e.to_string()))?;
        }
        
        // Receive data
        match self.inner.receive_from_region(&region_name, timeout_duration).await {
            Ok(data) => {
                let latency = start_time.elapsed().as_secs_f64() * 1000.0;
                self.metrics.record_receive(data.len(), latency);
                debug!("Successfully received {} bytes in {:.2}ms", data.len(), latency);
                Ok(data)
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.metrics.record_error(&error_msg);
                
                // Convert timeout errors to appropriate core error type
                if error_msg.contains("timed out") || error_msg.contains("Timeout") {
                    warn!("Receive operation timed out after {}ms", timeout_ms);
                    Err(TransportError::Timeout { timeout_ms })
                } else {
                    warn!("Failed to receive data: {}", error_msg);
                    Err(TransportError::SharedMemory(error_msg))
                }
            }
        }
    }
    
    async fn can_communicate_with(&self, node: &NodeInfo) -> bool {
        // Can only communicate with nodes on the same machine
        if !self.is_same_machine(node) {
            return false;
        }
        
        let region_name = self.get_region_name(node);
        
        // Check if we can access or create the region
        if self.inner.region_exists(&region_name).await {
            return true;
        }
        
        // Try to initialize the region to see if it's possible
        match self.inner.initialize_region(&region_name, Some(4096)).await {
            Ok(()) => {
                debug!("Successfully initialized test region for node {}", node.id);
                true
            }
            Err(e) => {
                debug!("Cannot communicate with node {}: {}", node.id, e);
                false
            }
        }
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::SharedMemory
    }
    
    async fn get_metrics(&self) -> TransportMetrics {
        self.metrics.get_metrics()
    }
}

/// Get the local machine identifier
fn get_local_machine_id() -> String {
    // This is a simplified implementation
    // In a real implementation, this should use a consistent machine identifier
    // such as MAC address, hostname, or a stored UUID
    
    use std::env;
    
    // Try to get a consistent identifier
    if let Ok(hostname) = env::var("HOSTNAME") {
        return hostname;
    }
    
    if let Ok(computername) = env::var("COMPUTERNAME") {
        return computername;
    }
    
    // Fallback to hostname command result or a default
    match hostname::get() {
        Ok(name) => name.to_string_lossy().to_string(),
        Err(_) => "localhost".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use universal_transport_core::Language;
    use tokio_test;
    
    fn create_test_node(id: &str) -> NodeInfo {
        NodeInfo {
            id: id.to_string(),
            language: Language::Rust,
            machine_id: get_local_machine_id(),
            endpoint: None,
            shared_memory_name: Some(format!("test_{}", id)),
            metadata: std::collections::HashMap::new(),
            capabilities: universal_transport_core::NodeCapabilities::default(),
        }
    }
    
    #[tokio::test]
    async fn test_adapter_creation() {
        let adapter = SharedMemoryTransportAdapter::new_default();
        assert_eq!(adapter.transport_type(), TransportType::SharedMemory);
    }
    
    #[tokio::test]
    async fn test_same_machine_detection() {
        let adapter = SharedMemoryTransportAdapter::new_default();
        let local_node = create_test_node("local");
        
        assert!(adapter.is_same_machine(&local_node));
        
        let mut remote_node = local_node.clone();
        remote_node.machine_id = "different_machine".to_string();
        
        assert!(!adapter.is_same_machine(&remote_node));
    }
    
    #[tokio::test]
    async fn test_can_communicate_with() {
        let adapter = SharedMemoryTransportAdapter::new_default();
        let local_node = create_test_node("test_communication");
        
        assert!(adapter.can_communicate_with(&local_node).await);
        
        let mut remote_node = local_node.clone();
        remote_node.machine_id = "remote_machine".to_string();
        
        assert!(!adapter.can_communicate_with(&remote_node).await);
    }
    
    #[tokio::test]
    async fn test_send_receive() {
        let adapter = SharedMemoryTransportAdapter::new_default();
        let node = create_test_node("send_receive_test");
        
        let test_data = b"Hello, Universal Transport!";
        
        // Send data
        adapter.send(test_data, &node).await.unwrap();
        
        // Receive data
        let received = adapter.receive(&node, 5000).await.unwrap();
        assert_eq!(received.as_ref(), test_data);
        
        // Check metrics
        let metrics = adapter.get_metrics().await;
        assert_eq!(metrics.messages_sent, 1);
        assert_eq!(metrics.messages_received, 1);
        assert_eq!(metrics.bytes_sent, test_data.len() as u64);
        assert_eq!(metrics.bytes_received, test_data.len() as u64);
    }
    
    #[tokio::test]
    async fn test_metrics_tracking() {
        let adapter = SharedMemoryTransportAdapter::new_default();
        let node = create_test_node("metrics_test");
        
        // Initial metrics should be zero
        let initial_metrics = adapter.get_metrics().await;
        assert_eq!(initial_metrics.messages_sent, 0);
        assert_eq!(initial_metrics.error_count, 0);
        
        // Send some data
        let test_data = b"test data for metrics";
        adapter.send(test_data, &node).await.unwrap();
        adapter.receive(&node, 1000).await.unwrap();
        
        // Check updated metrics
        let metrics = adapter.get_metrics().await;
        assert_eq!(metrics.messages_sent, 1);
        assert_eq!(metrics.messages_received, 1);
        assert!(metrics.average_latency_ms > 0.0);
        assert!(metrics.average_throughput_mbps >= 0.0);
    }
}
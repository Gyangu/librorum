//! Performance metrics and monitoring

use crate::{TransportType, NodeInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicU64, AtomicU32, Ordering}};
use std::time::{SystemTime, Duration, Instant};
use tokio::sync::RwLock;

/// Global metrics collector
pub struct MetricsCollector {
    /// Transport-specific metrics
    transport_metrics: Arc<RwLock<HashMap<TransportType, TransportMetricsState>>>,
    /// Node-specific metrics
    node_metrics: Arc<RwLock<HashMap<String, NodeMetricsState>>>,
    /// System-wide counters
    global_counters: GlobalCounters,
    /// Start time for uptime calculation
    start_time: SystemTime,
}

/// Global system counters
struct GlobalCounters {
    /// Total messages sent across all transports
    total_messages_sent: AtomicU64,
    /// Total messages received across all transports
    total_messages_received: AtomicU64,
    /// Total bytes sent across all transports
    total_bytes_sent: AtomicU64,
    /// Total bytes received across all transports
    total_bytes_received: AtomicU64,
    /// Total errors across all transports
    total_errors: AtomicU64,
}

/// Transport-specific metrics state
struct TransportMetricsState {
    /// Messages sent counter
    messages_sent: AtomicU64,
    /// Messages received counter
    messages_received: AtomicU64,
    /// Bytes sent counter
    bytes_sent: AtomicU64,
    /// Bytes received counter
    bytes_received: AtomicU64,
    /// Error counter
    error_count: AtomicU64,
    /// Latency samples (for calculating average)
    latency_samples: Arc<RwLock<Vec<f64>>>,
    /// Throughput samples (for calculating average)
    throughput_samples: Arc<RwLock<Vec<f64>>>,
    /// Last error message
    last_error: Arc<RwLock<Option<String>>>,
    /// Last operation timestamp
    last_operation: Arc<RwLock<Option<SystemTime>>>,
}

/// Node-specific metrics state
struct NodeMetricsState {
    /// Node information
    node_info: NodeInfo,
    /// Messages sent to this node
    messages_sent: AtomicU64,
    /// Messages received from this node
    messages_received: AtomicU64,
    /// Bytes sent to this node
    bytes_sent: AtomicU64,
    /// Bytes received from this node
    bytes_received: AtomicU64,
    /// Communication errors with this node
    error_count: AtomicU64,
    /// Average latency to this node
    average_latency_ms: Arc<RwLock<f64>>,
    /// Last successful communication
    last_success: Arc<RwLock<Option<SystemTime>>>,
    /// Transport type usage frequency
    transport_usage: Arc<RwLock<HashMap<TransportType, u64>>>,
}

impl Default for GlobalCounters {
    fn default() -> Self {
        Self {
            total_messages_sent: AtomicU64::new(0),
            total_messages_received: AtomicU64::new(0),
            total_bytes_sent: AtomicU64::new(0),
            total_bytes_received: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
        }
    }
}

impl Default for TransportMetricsState {
    fn default() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            latency_samples: Arc::new(RwLock::new(Vec::new())),
            throughput_samples: Arc::new(RwLock::new(Vec::new())),
            last_error: Arc::new(RwLock::new(None)),
            last_operation: Arc::new(RwLock::new(None)),
        }
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            transport_metrics: Arc::new(RwLock::new(HashMap::new())),
            node_metrics: Arc::new(RwLock::new(HashMap::new())),
            global_counters: GlobalCounters::default(),
            start_time: SystemTime::now(),
        }
    }
    
    /// Record a send operation
    pub async fn record_send(
        &self,
        transport_type: TransportType,
        destination: &NodeInfo,
        bytes: usize,
        latency_ms: f64,
        success: bool,
        error: Option<String>,
    ) {
        // Update global counters
        self.global_counters.total_messages_sent.fetch_add(1, Ordering::SeqCst);
        if success {
            self.global_counters.total_bytes_sent.fetch_add(bytes as u64, Ordering::SeqCst);
        } else {
            self.global_counters.total_errors.fetch_add(1, Ordering::SeqCst);
        }
        
        // Update transport-specific metrics
        self.update_transport_metrics(transport_type, bytes, latency_ms, success, error.clone(), true).await;
        
        // Update node-specific metrics
        self.update_node_metrics(&destination.id, destination, bytes, latency_ms, success, transport_type, true).await;
    }
    
    /// Record a receive operation
    pub async fn record_receive(
        &self,
        transport_type: TransportType,
        source: &NodeInfo,
        bytes: usize,
        latency_ms: f64,
        success: bool,
        error: Option<String>,
    ) {
        // Update global counters
        self.global_counters.total_messages_received.fetch_add(1, Ordering::SeqCst);
        if success {
            self.global_counters.total_bytes_received.fetch_add(bytes as u64, Ordering::SeqCst);
        } else {
            self.global_counters.total_errors.fetch_add(1, Ordering::SeqCst);
        }
        
        // Update transport-specific metrics
        self.update_transport_metrics(transport_type, bytes, latency_ms, success, error.clone(), false).await;
        
        // Update node-specific metrics
        self.update_node_metrics(&source.id, source, bytes, latency_ms, success, transport_type, false).await;
    }
    
    /// Update transport-specific metrics
    async fn update_transport_metrics(
        &self,
        transport_type: TransportType,
        bytes: usize,
        latency_ms: f64,
        success: bool,
        error: Option<String>,
        is_send: bool,
    ) {
        let mut metrics = self.transport_metrics.write().await;
        let state = metrics.entry(transport_type).or_default();
        
        if is_send {
            state.messages_sent.fetch_add(1, Ordering::SeqCst);
            if success {
                state.bytes_sent.fetch_add(bytes as u64, Ordering::SeqCst);
            }
        } else {
            state.messages_received.fetch_add(1, Ordering::SeqCst);
            if success {
                state.bytes_received.fetch_add(bytes as u64, Ordering::SeqCst);
            }
        }
        
        if success {
            // Add latency sample (keep only last 100 samples)
            let mut latency_samples = state.latency_samples.write().await;
            latency_samples.push(latency_ms);
            if latency_samples.len() > 100 {
                latency_samples.remove(0);
            }
            
            // Calculate and add throughput sample
            let throughput_mbps = (bytes as f64) / (1024.0 * 1024.0) / (latency_ms / 1000.0);
            let mut throughput_samples = state.throughput_samples.write().await;
            throughput_samples.push(throughput_mbps);
            if throughput_samples.len() > 100 {
                throughput_samples.remove(0);
            }
            
            // Update last operation time
            *state.last_operation.write().await = Some(SystemTime::now());
        } else {
            state.error_count.fetch_add(1, Ordering::SeqCst);
            *state.last_error.write().await = error;
        }
    }
    
    /// Update node-specific metrics
    async fn update_node_metrics(
        &self,
        node_id: &str,
        node_info: &NodeInfo,
        bytes: usize,
        latency_ms: f64,
        success: bool,
        transport_type: TransportType,
        is_send: bool,
    ) {
        let mut metrics = self.node_metrics.write().await;
        let state = metrics.entry(node_id.to_string()).or_insert_with(|| {
            NodeMetricsState {
                node_info: node_info.clone(),
                messages_sent: AtomicU64::new(0),
                messages_received: AtomicU64::new(0),
                bytes_sent: AtomicU64::new(0),
                bytes_received: AtomicU64::new(0),
                error_count: AtomicU64::new(0),
                average_latency_ms: Arc::new(RwLock::new(0.0)),
                last_success: Arc::new(RwLock::new(None)),
                transport_usage: Arc::new(RwLock::new(HashMap::new())),
            }
        });
        
        if is_send {
            state.messages_sent.fetch_add(1, Ordering::SeqCst);
            if success {
                state.bytes_sent.fetch_add(bytes as u64, Ordering::SeqCst);
            }
        } else {
            state.messages_received.fetch_add(1, Ordering::SeqCst);
            if success {
                state.bytes_received.fetch_add(bytes as u64, Ordering::SeqCst);
            }
        }
        
        if success {
            // Update average latency using exponential moving average
            let mut avg_latency = state.average_latency_ms.write().await;
            if *avg_latency == 0.0 {
                *avg_latency = latency_ms;
            } else {
                *avg_latency = 0.9 * *avg_latency + 0.1 * latency_ms;
            }
            
            // Update last success time
            *state.last_success.write().await = Some(SystemTime::now());
            
            // Update transport usage
            let mut usage = state.transport_usage.write().await;
            *usage.entry(transport_type).or_insert(0) += 1;
        } else {
            state.error_count.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    /// Get transport metrics
    pub async fn get_transport_metrics(&self, transport_type: TransportType) -> Option<TransportMetricsSummary> {
        let metrics = self.transport_metrics.read().await;
        if let Some(state) = metrics.get(&transport_type) {
            let latency_samples = state.latency_samples.read().await;
            let throughput_samples = state.throughput_samples.read().await;
            
            let average_latency = if latency_samples.is_empty() {
                0.0
            } else {
                latency_samples.iter().sum::<f64>() / latency_samples.len() as f64
            };
            
            let average_throughput = if throughput_samples.is_empty() {
                0.0
            } else {
                throughput_samples.iter().sum::<f64>() / throughput_samples.len() as f64
            };
            
            let last_error = state.last_error.read().await.clone();
            let last_operation = *state.last_operation.read().await;
            
            Some(TransportMetricsSummary {
                transport_type,
                messages_sent: state.messages_sent.load(Ordering::SeqCst),
                messages_received: state.messages_received.load(Ordering::SeqCst),
                bytes_sent: state.bytes_sent.load(Ordering::SeqCst),
                bytes_received: state.bytes_received.load(Ordering::SeqCst),
                average_latency_ms: average_latency,
                average_throughput_mbps: average_throughput,
                error_count: state.error_count.load(Ordering::SeqCst),
                last_error,
                last_operation,
            })
        } else {
            None
        }
    }
    
    /// Get node metrics
    pub async fn get_node_metrics(&self, node_id: &str) -> Option<NodeMetricsSummary> {
        let metrics = self.node_metrics.read().await;
        if let Some(state) = metrics.get(node_id) {
            let average_latency = *state.average_latency_ms.read().await;
            let last_success = *state.last_success.read().await;
            let transport_usage = state.transport_usage.read().await.clone();
            
            Some(NodeMetricsSummary {
                node_id: node_id.to_string(),
                node_info: state.node_info.clone(),
                messages_sent: state.messages_sent.load(Ordering::SeqCst),
                messages_received: state.messages_received.load(Ordering::SeqCst),
                bytes_sent: state.bytes_sent.load(Ordering::SeqCst),
                bytes_received: state.bytes_received.load(Ordering::SeqCst),
                average_latency_ms: average_latency,
                error_count: state.error_count.load(Ordering::SeqCst),
                last_success,
                transport_usage,
            })
        } else {
            None
        }
    }
    
    /// Get global metrics summary
    pub async fn get_global_metrics(&self) -> GlobalMetricsSummary {
        let uptime = self.start_time.elapsed().unwrap_or_default();
        
        GlobalMetricsSummary {
            total_messages_sent: self.global_counters.total_messages_sent.load(Ordering::SeqCst),
            total_messages_received: self.global_counters.total_messages_received.load(Ordering::SeqCst),
            total_bytes_sent: self.global_counters.total_bytes_sent.load(Ordering::SeqCst),
            total_bytes_received: self.global_counters.total_bytes_received.load(Ordering::SeqCst),
            total_errors: self.global_counters.total_errors.load(Ordering::SeqCst),
            uptime_seconds: uptime.as_secs(),
            active_transports: self.transport_metrics.read().await.len(),
            active_nodes: self.node_metrics.read().await.len(),
        }
    }
    
    /// Get all transport metrics
    pub async fn get_all_transport_metrics(&self) -> Vec<TransportMetricsSummary> {
        let mut summaries = Vec::new();
        let metrics = self.transport_metrics.read().await;
        
        for transport_type in metrics.keys() {
            if let Some(summary) = self.get_transport_metrics(*transport_type).await {
                summaries.push(summary);
            }
        }
        
        summaries
    }
    
    /// Get all node metrics
    pub async fn get_all_node_metrics(&self) -> Vec<NodeMetricsSummary> {
        let mut summaries = Vec::new();
        let metrics = self.node_metrics.read().await;
        
        for node_id in metrics.keys() {
            if let Some(summary) = self.get_node_metrics(node_id).await {
                summaries.push(summary);
            }
        }
        
        summaries
    }
    
    /// Clear all metrics
    pub async fn clear_metrics(&self) {
        self.transport_metrics.write().await.clear();
        self.node_metrics.write().await.clear();
        
        // Reset global counters
        self.global_counters.total_messages_sent.store(0, Ordering::SeqCst);
        self.global_counters.total_messages_received.store(0, Ordering::SeqCst);
        self.global_counters.total_bytes_sent.store(0, Ordering::SeqCst);
        self.global_counters.total_bytes_received.store(0, Ordering::SeqCst);
        self.global_counters.total_errors.store(0, Ordering::SeqCst);
    }
    
    /// Export metrics to JSON
    pub async fn export_to_json(&self) -> serde_json::Result<String> {
        let export_data = MetricsExport {
            global: self.get_global_metrics().await,
            transports: self.get_all_transport_metrics().await,
            nodes: self.get_all_node_metrics().await,
            timestamp: SystemTime::now(),
        };
        
        serde_json::to_string_pretty(&export_data)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Transport metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportMetricsSummary {
    pub transport_type: TransportType,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub average_latency_ms: f64,
    pub average_throughput_mbps: f64,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub last_operation: Option<SystemTime>,
}

/// Node metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetricsSummary {
    pub node_id: String,
    pub node_info: NodeInfo,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub average_latency_ms: f64,
    pub error_count: u64,
    pub last_success: Option<SystemTime>,
    pub transport_usage: HashMap<TransportType, u64>,
}

/// Global metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetricsSummary {
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_errors: u64,
    pub uptime_seconds: u64,
    pub active_transports: usize,
    pub active_nodes: usize,
}

/// Metrics export structure
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsExport {
    pub global: GlobalMetricsSummary,
    pub transports: Vec<TransportMetricsSummary>,
    pub nodes: Vec<NodeMetricsSummary>,
    pub timestamp: SystemTime,
}

/// Performance measurement utility
pub struct PerformanceMeasurement {
    start_time: Instant,
    operation_name: String,
}

impl PerformanceMeasurement {
    /// Start a new performance measurement
    pub fn start(operation_name: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            operation_name: operation_name.into(),
        }
    }
    
    /// Finish the measurement and return the elapsed time
    pub fn finish(self) -> (String, Duration) {
        (self.operation_name, self.start_time.elapsed())
    }
    
    /// Get elapsed time without finishing
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let global_metrics = collector.get_global_metrics().await;
        
        assert_eq!(global_metrics.total_messages_sent, 0);
        assert_eq!(global_metrics.total_messages_received, 0);
        assert_eq!(global_metrics.active_transports, 0);
        assert_eq!(global_metrics.active_nodes, 0);
    }

    #[tokio::test]
    async fn test_record_send() {
        let collector = MetricsCollector::new();
        let destination = NodeInfo::new("test_node", Language::Rust);
        
        collector.record_send(
            TransportType::SharedMemory,
            &destination,
            1024,
            5.0,
            true,
            None,
        ).await;
        
        let global_metrics = collector.get_global_metrics().await;
        assert_eq!(global_metrics.total_messages_sent, 1);
        assert_eq!(global_metrics.total_bytes_sent, 1024);
        assert_eq!(global_metrics.active_transports, 1);
        assert_eq!(global_metrics.active_nodes, 1);
        
        let transport_metrics = collector.get_transport_metrics(TransportType::SharedMemory).await.unwrap();
        assert_eq!(transport_metrics.messages_sent, 1);
        assert_eq!(transport_metrics.bytes_sent, 1024);
        assert_eq!(transport_metrics.average_latency_ms, 5.0);
        
        let node_metrics = collector.get_node_metrics("test_node").await.unwrap();
        assert_eq!(node_metrics.messages_sent, 1);
        assert_eq!(node_metrics.bytes_sent, 1024);
    }

    #[tokio::test]
    async fn test_record_error() {
        let collector = MetricsCollector::new();
        let destination = NodeInfo::new("test_node", Language::Rust);
        
        collector.record_send(
            TransportType::SharedMemory,
            &destination,
            1024,
            0.0,
            false,
            Some("Test error".to_string()),
        ).await;
        
        let global_metrics = collector.get_global_metrics().await;
        assert_eq!(global_metrics.total_errors, 1);
        assert_eq!(global_metrics.total_bytes_sent, 0); // No bytes sent on error
        
        let transport_metrics = collector.get_transport_metrics(TransportType::SharedMemory).await.unwrap();
        assert_eq!(transport_metrics.error_count, 1);
        assert_eq!(transport_metrics.last_error, Some("Test error".to_string()));
    }

    #[test]
    fn test_performance_measurement() {
        let measurement = PerformanceMeasurement::start("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let (operation_name, duration) = measurement.finish();
        assert_eq!(operation_name, "test_operation");
        assert!(duration.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_metrics_export() {
        let collector = MetricsCollector::new();
        let destination = NodeInfo::new("test_node", Language::Rust);
        
        collector.record_send(
            TransportType::SharedMemory,
            &destination,
            1024,
            5.0,
            true,
            None,
        ).await;
        
        let json = collector.export_to_json().await.unwrap();
        assert!(json.contains("test_node"));
        assert!(json.contains("SharedMemory"));
        assert!(json.contains("1024"));
    }
}
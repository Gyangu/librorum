//! Core transport abstractions

use crate::{NodeInfo, TransportError, Result};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Core transport trait that all transport implementations must implement
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send data to a specific destination
    async fn send(&self, data: &[u8], destination: &NodeInfo) -> Result<()>;
    
    /// Receive data from a specific source (with timeout)
    async fn receive(&self, source: &NodeInfo, timeout_ms: u64) -> Result<Bytes>;
    
    /// Check if the transport can communicate with the given node
    async fn can_communicate_with(&self, node: &NodeInfo) -> bool;
    
    /// Get the transport type identifier
    fn transport_type(&self) -> TransportType;
    
    /// Get performance metrics for this transport
    async fn get_metrics(&self) -> TransportMetrics;
}

/// High-level universal transport interface
#[async_trait]
pub trait UniversalTransport: Send + Sync {
    /// Send structured data to a destination
    async fn send<T>(&self, data: &T, destination: &NodeInfo) -> Result<()>
    where
        T: Serialize + Send + Sync;
    
    /// Receive structured data from a source
    async fn receive<T>(&self, source: &NodeInfo, timeout_ms: u64) -> Result<T>
    where
        T: for<'de> Deserialize<'de> + Send;
    
    /// Broadcast data to multiple destinations
    async fn broadcast<T>(&self, data: &T, destinations: &[NodeInfo]) -> Result<Vec<Result<()>>>
    where
        T: Serialize + Send + Sync;
    
    /// Get information about available transports
    async fn available_transports(&self) -> Vec<TransportInfo>;
}

/// Transport type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    /// Shared memory transport (same machine)
    SharedMemory,
    /// Swift-optimized network protocol
    SwiftNetwork,
    /// Rust-optimized network protocol
    RustNetwork,
    /// Universal compatibility protocol
    Universal,
}

/// Transport performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportMetrics {
    pub transport_type: TransportType,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub average_latency_ms: f64,
    pub average_throughput_mbps: f64,
    pub error_count: u64,
    pub last_error: Option<String>,
}

/// Transport information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportInfo {
    pub transport_type: TransportType,
    pub is_available: bool,
    pub supported_platforms: Vec<String>,
    pub performance_tier: PerformanceTier,
    pub description: String,
}

/// Performance tier classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceTier {
    /// Extreme performance (shared memory)
    Extreme,
    /// High performance (optimized network)
    High,
    /// Medium performance (standard network)
    Medium,
    /// Compatibility focus
    Compatibility,
}
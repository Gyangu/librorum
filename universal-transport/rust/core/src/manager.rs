//! Transport manager for coordinating different transport implementations

use crate::{
    Transport, UniversalTransport, NodeInfo, TransportStrategy, TransportType, 
    TransportError, Result, StrategySelector, StrategyPreferences
};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn, error, instrument};

/// Transport manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportManagerConfig {
    /// Strategy selection preferences
    pub strategy_preferences: StrategyPreferences,
    /// Enable automatic fallback
    pub enable_fallback: bool,
    /// Fallback timeout in milliseconds
    pub fallback_timeout_ms: u64,
    /// Enable transport health monitoring
    pub enable_health_monitoring: bool,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
}

impl Default for TransportManagerConfig {
    fn default() -> Self {
        Self {
            strategy_preferences: StrategyPreferences::default(),
            enable_fallback: true,
            fallback_timeout_ms: 5000,
            enable_health_monitoring: true,
            health_check_interval_seconds: 30,
        }
    }
}

/// Transport manager that coordinates multiple transport implementations
pub struct TransportManager {
    /// Strategy selector for choosing optimal transports
    strategy_selector: Arc<RwLock<StrategySelector>>,
    /// Available transport implementations
    transports: HashMap<TransportType, Arc<dyn Transport>>,
    /// Configuration
    config: TransportManagerConfig,
    /// Transport health status
    transport_health: Arc<RwLock<HashMap<TransportType, TransportHealth>>>,
}

/// Health status of a transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportHealth {
    /// Is the transport currently healthy
    pub is_healthy: bool,
    /// Last successful operation timestamp
    pub last_success: Option<std::time::SystemTime>,
    /// Last error encountered
    pub last_error: Option<String>,
    /// Consecutive failure count
    pub consecutive_failures: u32,
    /// Total operations count
    pub total_operations: u64,
    /// Successful operations count
    pub successful_operations: u64,
}

impl Default for TransportHealth {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_success: None,
            last_error: None,
            consecutive_failures: 0,
            total_operations: 0,
            successful_operations: 0,
        }
    }
}

impl TransportManager {
    /// Create a new transport manager
    pub fn new(config: TransportManagerConfig) -> Self {
        let strategy_selector = StrategySelector::new(config.strategy_preferences.clone());
        
        Self {
            strategy_selector: Arc::new(RwLock::new(strategy_selector)),
            transports: HashMap::new(),
            config,
            transport_health: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create with default configuration
    pub fn new_default() -> Self {
        Self::new(TransportManagerConfig::default())
    }
    
    /// Register a transport implementation
    pub async fn register_transport(&mut self, transport_type: TransportType, transport: Arc<dyn Transport>) {
        debug!("Registering transport: {:?}", transport_type);
        self.transports.insert(transport_type, transport);
        
        // Initialize health status
        let mut health = self.transport_health.write().await;
        health.insert(transport_type, TransportHealth::default());
    }
    
    /// Get optimal transport strategy for communication
    #[instrument(skip(self))]
    pub async fn get_strategy(&self, source: &NodeInfo, destination: &NodeInfo, data_size: usize) -> Result<TransportStrategy> {
        let selector = self.strategy_selector.read().await;
        selector.select_strategy(source, destination, data_size)
    }
    
    /// Send data using the optimal transport strategy
    #[instrument(skip(self, data))]
    pub async fn send_with_strategy(&self, data: &[u8], destination: &NodeInfo, strategy: &TransportStrategy) -> Result<()> {
        let transport_type = strategy.transport_type();
        
        // Check if transport is healthy
        if !self.is_transport_healthy(transport_type).await {
            if self.config.enable_fallback {
                return self.send_with_fallback(data, destination).await;
            } else {
                return Err(TransportError::TransportNotAvailable(transport_type));
            }
        }
        
        // Get the transport implementation
        let transport = self.transports.get(&transport_type)
            .ok_or_else(|| TransportError::TransportNotAvailable(transport_type))?;
        
        let start_time = std::time::Instant::now();
        
        // Attempt to send
        match transport.send(data, destination).await {
            Ok(()) => {
                let latency = start_time.elapsed().as_secs_f64() * 1000.0;
                let throughput = (data.len() as f64) / (1024.0 * 1024.0) / start_time.elapsed().as_secs_f64();
                
                // Update performance and health
                self.update_performance(&destination.id, transport_type, latency, throughput, true).await;
                self.update_health(transport_type, true, None).await;
                
                debug!("Successfully sent {} bytes using {:?}", data.len(), transport_type);
                Ok(())
            }
            Err(e) => {
                // Update performance and health
                self.update_health(transport_type, false, Some(e.to_string())).await;
                
                if self.config.enable_fallback {
                    warn!("Primary transport failed, attempting fallback: {}", e);
                    self.send_with_fallback(data, destination).await
                } else {
                    Err(e)
                }
            }
        }
    }
    
    /// Send data with automatic fallback
    async fn send_with_fallback(&self, data: &[u8], destination: &NodeInfo) -> Result<()> {
        let selector = self.strategy_selector.read().await;
        let recommended_transports = selector.get_recommended_transports(destination);
        
        for transport_type in recommended_transports {
            if let Some(transport) = self.transports.get(&transport_type) {
                if self.is_transport_healthy(transport_type).await {
                    match transport.send(data, destination).await {
                        Ok(()) => {
                            debug!("Fallback successful using {:?}", transport_type);
                            return Ok(());
                        }
                        Err(e) => {
                            warn!("Fallback transport {:?} failed: {}", transport_type, e);
                            self.update_health(transport_type, false, Some(e.to_string())).await;
                        }
                    }
                }
            }
        }
        
        Err(TransportError::Internal("All transport fallbacks failed".to_string()))
    }
    
    /// Receive data using the optimal transport strategy
    #[instrument(skip(self))]
    pub async fn receive_with_strategy(&self, source: &NodeInfo, strategy: &TransportStrategy, timeout_ms: u64) -> Result<Bytes> {
        let transport_type = strategy.transport_type();
        
        // Check if transport is healthy
        if !self.is_transport_healthy(transport_type).await {
            if self.config.enable_fallback {
                return self.receive_with_fallback(source, timeout_ms).await;
            } else {
                return Err(TransportError::TransportNotAvailable(transport_type));
            }
        }
        
        // Get the transport implementation
        let transport = self.transports.get(&transport_type)
            .ok_or_else(|| TransportError::TransportNotAvailable(transport_type))?;
        
        let start_time = std::time::Instant::now();
        
        // Attempt to receive
        match transport.receive(source, timeout_ms).await {
            Ok(data) => {
                let latency = start_time.elapsed().as_secs_f64() * 1000.0;
                let throughput = (data.len() as f64) / (1024.0 * 1024.0) / start_time.elapsed().as_secs_f64();
                
                // Update performance and health
                self.update_performance(&source.id, transport_type, latency, throughput, true).await;
                self.update_health(transport_type, true, None).await;
                
                debug!("Successfully received {} bytes using {:?}", data.len(), transport_type);
                Ok(data)
            }
            Err(e) => {
                // Update health
                self.update_health(transport_type, false, Some(e.to_string())).await;
                
                if self.config.enable_fallback {
                    warn!("Primary transport failed, attempting fallback: {}", e);
                    self.receive_with_fallback(source, timeout_ms).await
                } else {
                    Err(e)
                }
            }
        }
    }
    
    /// Receive data with automatic fallback
    async fn receive_with_fallback(&self, source: &NodeInfo, timeout_ms: u64) -> Result<Bytes> {
        let selector = self.strategy_selector.read().await;
        let recommended_transports = selector.get_recommended_transports(source);
        
        for transport_type in recommended_transports {
            if let Some(transport) = self.transports.get(&transport_type) {
                if self.is_transport_healthy(transport_type).await {
                    match transport.receive(source, timeout_ms).await {
                        Ok(data) => {
                            debug!("Fallback receive successful using {:?}", transport_type);
                            return Ok(data);
                        }
                        Err(e) => {
                            warn!("Fallback transport {:?} failed: {}", transport_type, e);
                            self.update_health(transport_type, false, Some(e.to_string())).await;
                        }
                    }
                }
            }
        }
        
        Err(TransportError::Internal("All transport fallbacks failed".to_string()))
    }
    
    /// Check if a transport is healthy
    async fn is_transport_healthy(&self, transport_type: TransportType) -> bool {
        let health = self.transport_health.read().await;
        health.get(&transport_type)
            .map(|h| h.is_healthy)
            .unwrap_or(false)
    }
    
    /// Update transport health status
    async fn update_health(&self, transport_type: TransportType, success: bool, error: Option<String>) {
        let mut health_map = self.transport_health.write().await;
        let health = health_map.entry(transport_type).or_default();
        
        health.total_operations += 1;
        
        if success {
            health.successful_operations += 1;
            health.consecutive_failures = 0;
            health.is_healthy = true;
            health.last_success = Some(std::time::SystemTime::now());
        } else {
            health.consecutive_failures += 1;
            health.last_error = error;
            
            // Mark as unhealthy after 3 consecutive failures
            if health.consecutive_failures >= 3 {
                health.is_healthy = false;
            }
        }
    }
    
    /// Update performance metrics
    async fn update_performance(&self, node_id: &str, transport_type: TransportType, latency_ms: f64, throughput_mbps: f64, success: bool) {
        let mut selector = self.strategy_selector.write().await;
        selector.update_performance(node_id, transport_type, latency_ms, throughput_mbps, success);
    }
    
    /// Get available transports
    pub async fn get_available_transports(&self) -> Vec<crate::TransportInfo> {
        let mut transports = Vec::new();
        let health = self.transport_health.read().await;
        
        for (transport_type, transport) in &self.transports {
            let is_healthy = health.get(transport_type)
                .map(|h| h.is_healthy)
                .unwrap_or(false);
            
            let metrics = transport.get_metrics().await;
            
            let info = crate::TransportInfo {
                transport_type: *transport_type,
                is_available: is_healthy,
                supported_platforms: self.get_supported_platforms(*transport_type),
                performance_tier: self.get_performance_tier(*transport_type),
                description: self.get_transport_description(*transport_type),
            };
            
            transports.push(info);
        }
        
        transports
    }
    
    /// Get supported platforms for a transport type
    fn get_supported_platforms(&self, transport_type: TransportType) -> Vec<String> {
        match transport_type {
            TransportType::SharedMemory => vec!["macOS".to_string(), "Linux".to_string(), "Windows".to_string()],
            TransportType::SwiftNetwork => vec!["macOS".to_string(), "iOS".to_string(), "Linux".to_string()],
            TransportType::RustNetwork => vec!["macOS".to_string(), "Linux".to_string(), "Windows".to_string()],
            TransportType::Universal => vec!["All".to_string()],
        }
    }
    
    /// Get performance tier for a transport type
    fn get_performance_tier(&self, transport_type: TransportType) -> crate::PerformanceTier {
        match transport_type {
            TransportType::SharedMemory => crate::PerformanceTier::Extreme,
            TransportType::SwiftNetwork | TransportType::RustNetwork => crate::PerformanceTier::High,
            TransportType::Universal => crate::PerformanceTier::Medium,
        }
    }
    
    /// Get transport description
    fn get_transport_description(&self, transport_type: TransportType) -> String {
        match transport_type {
            TransportType::SharedMemory => "High-performance shared memory transport for local communication".to_string(),
            TransportType::SwiftNetwork => "Swift-optimized network protocol with MessagePack serialization".to_string(),
            TransportType::RustNetwork => "Rust-optimized network protocol with bincode serialization".to_string(),
            TransportType::Universal => "Universal compatibility protocol using Protocol Buffers".to_string(),
        }
    }
    
    /// Check if can communicate with a node
    pub async fn can_communicate_with(&self, node: &NodeInfo) -> bool {
        for transport in self.transports.values() {
            if transport.can_communicate_with(node).await {
                return true;
            }
        }
        false
    }
    
    /// Get transport health information
    pub async fn get_transport_health(&self) -> HashMap<TransportType, TransportHealth> {
        self.transport_health.read().await.clone()
    }
    
    /// Reset transport health for a specific transport
    pub async fn reset_transport_health(&self, transport_type: TransportType) {
        let mut health = self.transport_health.write().await;
        health.insert(transport_type, TransportHealth::default());
    }
    
    /// Update strategy preferences
    pub async fn update_strategy_preferences(&self, preferences: StrategyPreferences) {
        let mut selector = self.strategy_selector.write().await;
        selector.update_preferences(preferences);
    }
}

/// Implement UniversalTransport trait for TransportManager
#[async_trait]
impl UniversalTransport for TransportManager {
    async fn send<T>(&self, data: &T, destination: &NodeInfo) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        // Serialize data
        let serialized = bincode::serialize(data)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        
        // Get optimal strategy
        let strategy = self.get_strategy(&NodeInfo::new("local", crate::Language::Rust), destination, serialized.len()).await?;
        
        // Send using the strategy
        self.send_with_strategy(&serialized, destination, &strategy).await
    }
    
    async fn receive<T>(&self, source: &NodeInfo, timeout_ms: u64) -> Result<T>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        // Get optimal strategy
        let strategy = self.get_strategy(source, &NodeInfo::new("local", crate::Language::Rust), 0).await?;
        
        // Receive using the strategy
        let data = self.receive_with_strategy(source, &strategy, timeout_ms).await?;
        
        // Deserialize data
        bincode::deserialize(&data)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }
    
    async fn broadcast<T>(&self, data: &T, destinations: &[NodeInfo]) -> Result<Vec<Result<()>>>
    where
        T: Serialize + Send + Sync,
    {
        // Serialize data once
        let serialized = bincode::serialize(data)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        
        // Send to all destinations concurrently
        let futures = destinations.iter().map(|destination| {
            let serialized = serialized.clone();
            async move {
                let strategy = self.get_strategy(&NodeInfo::new("local", crate::Language::Rust), destination, serialized.len()).await?;
                self.send_with_strategy(&serialized, destination, &strategy).await
            }
        });
        
        let results = futures::future::join_all(futures).await;
        Ok(results)
    }
    
    async fn available_transports(&self) -> Vec<crate::TransportInfo> {
        self.get_available_transports().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    // Mock transport for testing
    struct MockTransport {
        transport_type: TransportType,
        should_fail: bool,
    }
    
    #[async_trait]
    impl Transport for MockTransport {
        async fn send(&self, _data: &[u8], _destination: &NodeInfo) -> Result<()> {
            if self.should_fail {
                Err(TransportError::Network("Mock failure".to_string()))
            } else {
                Ok(())
            }
        }
        
        async fn receive(&self, _source: &NodeInfo, _timeout_ms: u64) -> Result<Bytes> {
            if self.should_fail {
                Err(TransportError::Network("Mock failure".to_string()))
            } else {
                Ok(Bytes::from_static(b"test data"))
            }
        }
        
        async fn can_communicate_with(&self, _node: &NodeInfo) -> bool {
            true
        }
        
        fn transport_type(&self) -> TransportType {
            self.transport_type
        }
        
        async fn get_metrics(&self) -> crate::TransportMetrics {
            crate::TransportMetrics {
                transport_type: self.transport_type,
                messages_sent: 0,
                messages_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                average_latency_ms: 10.0,
                average_throughput_mbps: 100.0,
                error_count: 0,
                last_error: None,
            }
        }
    }

    #[tokio::test]
    async fn test_transport_manager_creation() {
        let manager = TransportManager::new_default();
        assert!(manager.transports.is_empty());
    }

    #[tokio::test]
    async fn test_transport_registration() {
        let mut manager = TransportManager::new_default();
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::SharedMemory,
            should_fail: false,
        });
        
        manager.register_transport(TransportType::SharedMemory, mock_transport).await;
        assert_eq!(manager.transports.len(), 1);
        assert!(manager.is_transport_healthy(TransportType::SharedMemory).await);
    }

    #[tokio::test]
    async fn test_successful_send() {
        let mut manager = TransportManager::new_default();
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::SharedMemory,
            should_fail: false,
        });
        
        manager.register_transport(TransportType::SharedMemory, mock_transport).await;
        
        let destination = NodeInfo::new("test", Language::Rust);
        let strategy = TransportStrategy::SharedMemory {
            region_name: "test_region".to_string(),
        };
        
        let result = manager.send_with_strategy(b"test data", &destination, &strategy).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_tracking() {
        let mut manager = TransportManager::new_default();
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::SharedMemory,
            should_fail: true,
        });
        
        manager.register_transport(TransportType::SharedMemory, mock_transport).await;
        
        let destination = NodeInfo::new("test", Language::Rust);
        let strategy = TransportStrategy::SharedMemory {
            region_name: "test_region".to_string(),
        };
        
        // First failure should not mark as unhealthy
        let _ = manager.send_with_strategy(b"test data", &destination, &strategy).await;
        assert!(manager.is_transport_healthy(TransportType::SharedMemory).await);
        
        // After 3 failures, should be marked as unhealthy
        let _ = manager.send_with_strategy(b"test data", &destination, &strategy).await;
        let _ = manager.send_with_strategy(b"test data", &destination, &strategy).await;
        
        let health = manager.get_transport_health().await;
        let shared_mem_health = &health[&TransportType::SharedMemory];
        assert!(!shared_mem_health.is_healthy);
        assert_eq!(shared_mem_health.consecutive_failures, 3);
    }
}
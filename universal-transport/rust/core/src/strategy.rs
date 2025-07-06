//! Transport strategy selection

use crate::{NodeInfo, TransportType, TransportError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transport strategy selector
pub struct StrategySelector {
    /// Performance history for different nodes
    performance_history: HashMap<String, PerformanceHistory>,
    /// Strategy preferences
    preferences: StrategyPreferences,
}

/// Strategy selection preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPreferences {
    /// Prefer shared memory for local communication
    pub prefer_shared_memory: bool,
    /// Prefer language-specific protocols
    pub prefer_language_optimization: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Minimum data size for shared memory (bytes)
    pub shared_memory_threshold: usize,
    /// Maximum acceptable latency (ms)
    pub max_acceptable_latency: f64,
}

impl Default for StrategyPreferences {
    fn default() -> Self {
        Self {
            prefer_shared_memory: true,
            prefer_language_optimization: true,
            enable_performance_monitoring: true,
            shared_memory_threshold: 1024, // 1KB
            max_acceptable_latency: 100.0, // 100ms
        }
    }
}

/// Performance history for a node
#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    /// Transport type to performance metrics mapping
    pub metrics: HashMap<TransportType, PerformanceMetrics>,
    /// Last update timestamp
    pub last_updated: std::time::SystemTime,
}

/// Performance metrics for a transport type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Average throughput in MB/s
    pub avg_throughput_mbps: f64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Number of samples
    pub sample_count: u64,
}

/// Transport strategy enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportStrategy {
    /// Shared memory with region name
    SharedMemory { region_name: String },
    /// Swift-optimized network
    SwiftNetwork { endpoint: String },
    /// Rust-optimized network
    RustNetwork { endpoint: String },
    /// Universal compatibility protocol
    Universal { endpoint: String },
}

impl StrategySelector {
    /// Create a new strategy selector
    pub fn new(preferences: StrategyPreferences) -> Self {
        Self {
            performance_history: HashMap::new(),
            preferences,
        }
    }
    
    /// Create with default preferences
    pub fn new_default() -> Self {
        Self::new(StrategyPreferences::default())
    }
    
    /// Select the optimal transport strategy for communication
    pub fn select_strategy(
        &self,
        source: &NodeInfo,
        destination: &NodeInfo,
        data_size: usize,
    ) -> Result<TransportStrategy> {
        // 1. Check if same machine - prefer shared memory
        if self.preferences.prefer_shared_memory && destination.is_local_machine() {
            if data_size >= self.preferences.shared_memory_threshold {
                let region_name = source.get_shared_memory_name(destination);
                return Ok(TransportStrategy::SharedMemory { region_name });
            }
        }
        
        // 2. Check for existing performance data
        if let Some(best_strategy) = self.get_best_performing_strategy(destination) {
            return Ok(best_strategy);
        }
        
        // 3. Default selection based on node characteristics
        self.select_default_strategy(destination)
    }
    
    /// Get the best performing strategy based on history
    fn get_best_performing_strategy(&self, destination: &NodeInfo) -> Option<TransportStrategy> {
        let history = self.performance_history.get(&destination.id)?;
        
        let mut best_score = f64::NEG_INFINITY;
        let mut best_transport = None;
        
        for (transport_type, metrics) in &history.metrics {
            // Calculate performance score (higher is better)
            let score = self.calculate_performance_score(metrics);
            
            if score > best_score {
                best_score = score;
                best_transport = Some(*transport_type);
            }
        }
        
        best_transport.and_then(|transport_type| {
            self.transport_type_to_strategy(transport_type, destination)
        })
    }
    
    /// Calculate performance score for metrics
    fn calculate_performance_score(&self, metrics: &PerformanceMetrics) -> f64 {
        // Weighted score: success rate (50%) + latency factor (30%) + throughput factor (20%)
        let success_weight = 0.5;
        let latency_weight = 0.3;
        let throughput_weight = 0.2;
        
        // Success rate component (0-1)
        let success_score = metrics.success_rate;
        
        // Latency component (lower is better, normalized to 0-1)
        let latency_score = if metrics.avg_latency_ms <= self.preferences.max_acceptable_latency {
            1.0 - (metrics.avg_latency_ms / self.preferences.max_acceptable_latency).min(1.0)
        } else {
            0.0
        };
        
        // Throughput component (logarithmic scale)
        let throughput_score = (metrics.avg_throughput_mbps.log10() / 3.0).min(1.0).max(0.0);
        
        success_weight * success_score + 
        latency_weight * latency_score + 
        throughput_weight * throughput_score
    }
    
    /// Convert transport type to strategy
    fn transport_type_to_strategy(&self, transport_type: TransportType, destination: &NodeInfo) -> Option<TransportStrategy> {
        match transport_type {
            TransportType::SharedMemory => {
                if destination.is_local_machine() {
                    let region_name = destination.get_shared_memory_name(destination);
                    Some(TransportStrategy::SharedMemory { region_name })
                } else {
                    None
                }
            }
            TransportType::SwiftNetwork => {
                destination.endpoint.as_ref().map(|endpoint| {
                    TransportStrategy::SwiftNetwork { endpoint: endpoint.clone() }
                })
            }
            TransportType::RustNetwork => {
                destination.endpoint.as_ref().map(|endpoint| {
                    TransportStrategy::RustNetwork { endpoint: endpoint.clone() }
                })
            }
            TransportType::Universal => {
                destination.endpoint.as_ref().map(|endpoint| {
                    TransportStrategy::Universal { endpoint: endpoint.clone() }
                })
            }
        }
    }
    
    /// Select default strategy based on node characteristics
    fn select_default_strategy(&self, destination: &NodeInfo) -> Result<TransportStrategy> {
        // Get endpoint for network communication
        let endpoint = destination.endpoint.as_ref()
            .ok_or_else(|| TransportError::Configuration(
                "No endpoint specified for remote node".to_string()
            ))?;
        
        // Select based on language if optimization is enabled
        if self.preferences.prefer_language_optimization {
            match destination.language {
                crate::Language::Swift => {
                    Ok(TransportStrategy::SwiftNetwork { endpoint: endpoint.clone() })
                }
                crate::Language::Rust => {
                    Ok(TransportStrategy::RustNetwork { endpoint: endpoint.clone() })
                }
            }
        } else {
            // Use universal protocol
            Ok(TransportStrategy::Universal { endpoint: endpoint.clone() })
        }
    }
    
    /// Update performance history with new measurements
    pub fn update_performance(
        &mut self,
        node_id: &str,
        transport_type: TransportType,
        latency_ms: f64,
        throughput_mbps: f64,
        success: bool,
    ) {
        let history = self.performance_history
            .entry(node_id.to_string())
            .or_insert_with(|| PerformanceHistory {
                metrics: HashMap::new(),
                last_updated: std::time::SystemTime::now(),
            });
        
        let metrics = history.metrics
            .entry(transport_type)
            .or_insert_with(|| PerformanceMetrics {
                avg_latency_ms: 0.0,
                avg_throughput_mbps: 0.0,
                success_rate: 0.0,
                sample_count: 0,
            });
        
        // Update metrics using exponential moving average
        let alpha = 0.1; // Smoothing factor
        
        if metrics.sample_count == 0 {
            // First sample
            metrics.avg_latency_ms = latency_ms;
            metrics.avg_throughput_mbps = throughput_mbps;
            metrics.success_rate = if success { 1.0 } else { 0.0 };
        } else {
            // Exponential moving average
            metrics.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * metrics.avg_latency_ms;
            metrics.avg_throughput_mbps = alpha * throughput_mbps + (1.0 - alpha) * metrics.avg_throughput_mbps;
            
            let success_value = if success { 1.0 } else { 0.0 };
            metrics.success_rate = alpha * success_value + (1.0 - alpha) * metrics.success_rate;
        }
        
        metrics.sample_count += 1;
        history.last_updated = std::time::SystemTime::now();
    }
    
    /// Get performance history for a node
    pub fn get_performance_history(&self, node_id: &str) -> Option<&PerformanceHistory> {
        self.performance_history.get(node_id)
    }
    
    /// Clear old performance history
    pub fn cleanup_old_history(&mut self, max_age: std::time::Duration) {
        let cutoff = std::time::SystemTime::now() - max_age;
        
        self.performance_history.retain(|_, history| {
            history.last_updated > cutoff
        });
    }
    
    /// Get strategy preferences
    pub fn get_preferences(&self) -> &StrategyPreferences {
        &self.preferences
    }
    
    /// Update strategy preferences
    pub fn update_preferences(&mut self, preferences: StrategyPreferences) {
        self.preferences = preferences;
    }
    
    /// Get recommended transport types for a destination
    pub fn get_recommended_transports(&self, destination: &NodeInfo) -> Vec<TransportType> {
        let mut transports = Vec::new();
        
        // Add shared memory if local
        if destination.is_local_machine() {
            transports.push(TransportType::SharedMemory);
        }
        
        // Add network transports if endpoint available
        if destination.endpoint.is_some() {
            if self.preferences.prefer_language_optimization {
                match destination.language {
                    crate::Language::Swift => transports.push(TransportType::SwiftNetwork),
                    crate::Language::Rust => transports.push(TransportType::RustNetwork),
                }
            }
            
            // Always include universal as fallback
            transports.push(TransportType::Universal);
        }
        
        transports
    }
}

impl TransportStrategy {
    /// Get the transport type for this strategy
    pub fn transport_type(&self) -> TransportType {
        match self {
            TransportStrategy::SharedMemory { .. } => TransportType::SharedMemory,
            TransportStrategy::SwiftNetwork { .. } => TransportType::SwiftNetwork,
            TransportStrategy::RustNetwork { .. } => TransportType::RustNetwork,
            TransportStrategy::Universal { .. } => TransportType::Universal,
        }
    }
    
    /// Get the endpoint for network strategies
    pub fn endpoint(&self) -> Option<&str> {
        match self {
            TransportStrategy::SharedMemory { .. } => None,
            TransportStrategy::SwiftNetwork { endpoint } => Some(endpoint),
            TransportStrategy::RustNetwork { endpoint } => Some(endpoint),
            TransportStrategy::Universal { endpoint } => Some(endpoint),
        }
    }
    
    /// Get the region name for shared memory strategies
    pub fn region_name(&self) -> Option<&str> {
        match self {
            TransportStrategy::SharedMemory { region_name } => Some(region_name),
            _ => None,
        }
    }
    
    /// Check if this strategy is suitable for the given data size
    pub fn is_suitable_for_size(&self, data_size: usize) -> bool {
        match self {
            TransportStrategy::SharedMemory { .. } => {
                // Shared memory is good for all sizes but especially large data
                true
            }
            TransportStrategy::SwiftNetwork { .. } | 
            TransportStrategy::RustNetwork { .. } => {
                // Optimized protocols are good for medium to large data
                data_size >= 256 // 256 bytes
            }
            TransportStrategy::Universal { .. } => {
                // Universal protocol works for all sizes
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    #[test]
    fn test_strategy_selector_creation() {
        let selector = StrategySelector::new_default();
        assert!(selector.preferences.prefer_shared_memory);
        assert!(selector.preferences.prefer_language_optimization);
    }

    #[test]
    fn test_local_shared_memory_strategy() {
        let selector = StrategySelector::new_default();
        let source = NodeInfo::new("source", Language::Rust);
        let destination = NodeInfo::new("dest", Language::Swift);
        
        let strategy = selector.select_strategy(&source, &destination, 2048).unwrap();
        
        match strategy {
            TransportStrategy::SharedMemory { .. } => (),
            _ => panic!("Expected shared memory strategy for local communication"),
        }
    }

    #[test]
    fn test_remote_network_strategy() {
        let selector = StrategySelector::new_default();
        let source = NodeInfo::new("source", Language::Rust);
        let destination = NodeInfo::remote("dest", Language::Swift, "127.0.0.1:8080");
        
        let strategy = selector.select_strategy(&source, &destination, 1024).unwrap();
        
        match strategy {
            TransportStrategy::SwiftNetwork { endpoint } => {
                assert_eq!(endpoint, "127.0.0.1:8080");
            }
            _ => panic!("Expected Swift network strategy for remote Swift node"),
        }
    }

    #[test]
    fn test_performance_update() {
        let mut selector = StrategySelector::new_default();
        
        selector.update_performance("node1", TransportType::SharedMemory, 1.0, 500.0, true);
        selector.update_performance("node1", TransportType::Universal, 50.0, 10.0, true);
        
        let history = selector.get_performance_history("node1").unwrap();
        assert_eq!(history.metrics.len(), 2);
        
        let shared_mem_metrics = &history.metrics[&TransportType::SharedMemory];
        assert_eq!(shared_mem_metrics.avg_latency_ms, 1.0);
        assert_eq!(shared_mem_metrics.success_rate, 1.0);
    }

    #[test]
    fn test_transport_strategy_properties() {
        let shared_mem_strategy = TransportStrategy::SharedMemory {
            region_name: "test_region".to_string(),
        };
        
        assert_eq!(shared_mem_strategy.transport_type(), TransportType::SharedMemory);
        assert_eq!(shared_mem_strategy.region_name(), Some("test_region"));
        assert_eq!(shared_mem_strategy.endpoint(), None);
        assert!(shared_mem_strategy.is_suitable_for_size(1024));
        
        let network_strategy = TransportStrategy::SwiftNetwork {
            endpoint: "127.0.0.1:8080".to_string(),
        };
        
        assert_eq!(network_strategy.transport_type(), TransportType::SwiftNetwork);
        assert_eq!(network_strategy.endpoint(), Some("127.0.0.1:8080"));
        assert_eq!(network_strategy.region_name(), None);
    }
}
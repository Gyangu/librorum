//! Cache Policy Configuration

use std::time::Duration;

/// Cache eviction strategies
#[derive(Debug, Clone)]
pub enum EvictionStrategy {
    LRU,
    LFU,
    FIFO,
    Random,
}

/// Cache prefetch strategies
#[derive(Debug, Clone)]
pub enum PrefetchStrategy {
    None,
    Sequential,
    Predictive,
}

/// Cache policy configuration
#[derive(Debug, Clone)]
pub struct CachePolicy {
    pub max_size: usize,
    pub ttl: Duration,
    pub eviction_strategy: EvictionStrategy,
    pub prefetch_strategy: PrefetchStrategy,
    pub enable_distributed: bool,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            max_size: 100 * 1024 * 1024, // 100MB
            ttl: Duration::from_secs(3600), // 1 hour
            eviction_strategy: EvictionStrategy::LRU,
            prefetch_strategy: PrefetchStrategy::None,
            enable_distributed: false,
        }
    }
}
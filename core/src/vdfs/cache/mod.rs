//! Cache Management Layer
//! 
//! Provides local and distributed caching for the VDFS system to improve
//! performance and reduce network overhead.

use crate::vdfs::{VDFSResult, CacheKey, CacheValue, ChunkId, VirtualPath};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

pub mod cache_manager;
pub mod lru_cache;
pub mod cache_policy;
pub mod sync;

pub use cache_manager::CacheManager;
pub use lru_cache::LRUCache;
pub use cache_policy::{CachePolicy, EvictionStrategy, PrefetchStrategy};
pub use sync::CacheSyncManager;

/// Local cache interface
#[async_trait]
pub trait LocalCache: Send + Sync {
    async fn get(&self, key: &CacheKey) -> Option<CacheValue>;
    async fn put(&self, key: CacheKey, value: CacheValue) -> VDFSResult<()>;
    async fn invalidate(&self, key: &CacheKey) -> VDFSResult<()>;
    async fn clear(&self) -> VDFSResult<()>;
    async fn size(&self) -> usize;
    async fn capacity(&self) -> usize;
}

/// Distributed cache interface
#[async_trait]
pub trait DistributedCache: Send + Sync {
    async fn get(&self, key: &CacheKey) -> VDFSResult<Option<CacheValue>>;
    async fn put(&self, key: CacheKey, value: CacheValue) -> VDFSResult<()>;
    async fn invalidate(&self, key: &CacheKey) -> VDFSResult<()>;
    async fn invalidate_pattern(&self, pattern: &str) -> VDFSResult<()>;
    async fn sync_with_peers(&self) -> VDFSResult<()>;
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub value: CacheValue,
    pub created: SystemTime,
    pub accessed: SystemTime,
    pub access_count: u64,
    pub size: usize,
}

impl CacheEntry {
    pub fn new(key: CacheKey, value: CacheValue) -> Self {
        let now = SystemTime::now();
        let size = Self::estimate_size(&value);
        
        Self {
            key,
            value,
            created: now,
            accessed: now,
            access_count: 1,
            size,
        }
    }
    
    pub fn access(&mut self) {
        self.accessed = SystemTime::now();
        self.access_count += 1;
    }
    
    pub fn is_expired(&self, ttl: Duration) -> bool {
        if let Ok(elapsed) = self.created.elapsed() {
            elapsed > ttl
        } else {
            true
        }
    }
    
    fn estimate_size(value: &CacheValue) -> usize {
        match value {
            CacheValue::FileData(data) => data.len(),
            CacheValue::ChunkData(data) => data.len(),
            CacheValue::FileMetadata(_) => 1024, // Estimate
            CacheValue::DirectoryListing(entries) => entries.len() * 256, // Estimate
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            size: 0,
            capacity: 0,
            hit_rate: 0.0,
        }
    }
    
    pub fn record_hit(&mut self) {
        self.hits += 1;
        self.update_hit_rate();
    }
    
    pub fn record_miss(&mut self) {
        self.misses += 1;
        self.update_hit_rate();
    }
    
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }
    
    fn update_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hit_rate = self.hits as f64 / total as f64;
        }
    }
}
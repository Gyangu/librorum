//! VDFS 高性能缓存系统
//! 
//! 提供多层缓存架构，支持：
//! - 混合缓存粒度：小文件整体缓存，大文件分块缓存
//! - Write-back 策略：先写缓存，异步同步到存储
//! - 节点内共享：同一节点内进程间共享缓存
//! - 双层架构：内存 + 磁盘缓存

use crate::vdfs::{VDFSResult, VDFSError, CacheKey, CacheValue, FileId, ChunkId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

pub mod cache_manager;
pub mod memory_cache;
pub mod disk_cache;
pub mod lru_cache;
pub mod cache_policy;
pub mod sync;

pub use cache_manager::CacheManager;
pub use memory_cache::MemoryCache;
pub use disk_cache::DiskCache;
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

/// 增强的缓存条目，支持 Write-back 和混合粒度
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub value: CacheValue,
    pub created: SystemTime,
    pub accessed: SystemTime,
    pub access_count: u64,
    pub size: usize,
    /// 是否为脏数据（需要写回存储）
    pub dirty: bool,
    /// 写回优先级
    pub writeback_priority: u8,
    /// 最后写入时间
    pub last_written: Option<SystemTime>,
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
            dirty: false,
            writeback_priority: 0,
            last_written: None,
        }
    }
    
    pub fn access(&mut self) {
        self.accessed = SystemTime::now();
        self.access_count += 1;
    }
    
    pub fn mark_dirty(&mut self, priority: u8) {
        self.dirty = true;
        self.writeback_priority = priority;
        self.last_written = Some(SystemTime::now());
    }
    
    pub fn mark_clean(&mut self) {
        self.dirty = false;
        self.writeback_priority = 0;
    }
    
    pub fn is_expired(&self, ttl: Duration) -> bool {
        if let Ok(elapsed) = self.created.elapsed() {
            elapsed > ttl
        } else {
            true
        }
    }
    
    /// 计算写回紧急程度（分数越高越紧急）
    pub fn writeback_urgency(&self) -> f64 {
        if !self.dirty {
            return 0.0;
        }
        
        let base_score = self.writeback_priority as f64;
        
        // 考虑写入时间间隔
        if let Some(last_written) = self.last_written {
            if let Ok(elapsed) = last_written.elapsed() {
                let age_factor = elapsed.as_secs_f64() / 60.0; // 每分钟增加权重
                return base_score + age_factor;
            }
        }
        
        base_score
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

/// 增强的缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f64,
    /// Write-back 统计
    pub dirty_entries: u64,
    pub writebacks: u64,
    pub writeback_errors: u64,
    /// 内存/磁盘分层统计
    pub memory_hits: u64,
    pub disk_hits: u64,
    pub memory_size: usize,
    pub disk_size: usize,
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
            dirty_entries: 0,
            writebacks: 0,
            writeback_errors: 0,
            memory_hits: 0,
            disk_hits: 0,
            memory_size: 0,
            disk_size: 0,
        }
    }
    
    pub fn record_hit(&mut self, from_memory: bool) {
        self.hits += 1;
        if from_memory {
            self.memory_hits += 1;
        } else {
            self.disk_hits += 1;
        }
        self.update_hit_rate();
    }
    
    pub fn record_miss(&mut self) {
        self.misses += 1;
        self.update_hit_rate();
    }
    
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }
    
    pub fn record_writeback(&mut self, success: bool) {
        if success {
            self.writebacks += 1;
            if self.dirty_entries > 0 {
                self.dirty_entries -= 1;
            }
        } else {
            self.writeback_errors += 1;
        }
    }
    
    pub fn record_dirty_entry(&mut self) {
        self.dirty_entries += 1;
    }
    
    fn update_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hit_rate = self.hits as f64 / total as f64;
        }
    }
    
    pub fn memory_hit_rate(&self) -> f64 {
        if self.hits == 0 {
            0.0
        } else {
            self.memory_hits as f64 / self.hits as f64
        }
    }
    
    pub fn disk_hit_rate(&self) -> f64 {
        if self.hits == 0 {
            0.0
        } else {
            self.disk_hits as f64 / self.hits as f64
        }
    }
}

/// 混合缓存粒度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridCacheConfig {
    /// 文件大小阈值，小于此值的文件整体缓存 (字节)
    pub file_threshold: usize,
    /// 块大小阈值，大于此值的文件分块缓存 (字节)
    pub chunk_size: usize,
    /// 小文件预取策略
    pub prefetch_small_files: bool,
    /// 块预取策略
    pub prefetch_chunks: bool,
}

impl Default for HybridCacheConfig {
    fn default() -> Self {
        Self {
            file_threshold: 4 * 1024 * 1024, // 4MB
            chunk_size: 1 * 1024 * 1024,     // 1MB
            prefetch_small_files: true,
            prefetch_chunks: false,
        }
    }
}

/// Write-back 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteBackConfig {
    /// 写回间隔
    pub interval: Duration,
    /// 批量写回大小
    pub batch_size: usize,
    /// 最大脏数据比例 (0.0 - 1.0)
    pub max_dirty_ratio: f64,
    /// 写回超时
    pub timeout: Duration,
    /// 重试次数
    pub retry_count: u32,
}

impl Default for WriteBackConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),     // 30秒间隔
            batch_size: 100,                      // 批量100条
            max_dirty_ratio: 0.2,                 // 最多20%脏数据
            timeout: Duration::from_secs(10),     // 10秒超时
            retry_count: 3,                       // 重试3次
        }
    }
}
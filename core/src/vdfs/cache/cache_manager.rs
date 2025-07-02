//! 高性能多层缓存管理器
//! 
//! 特性：
//! - 双层缓存：内存 + 磁盘
//! - 混合缓存粒度：智能选择文件/块级缓存
//! - Write-back 策略：异步写回存储
//! - 自动负载均衡和热点检测

use crate::vdfs::{VDFSResult, VDFSError, CacheKey, CacheValue, FileId, ChunkId};
use crate::vdfs::cache::{
    LocalCache, DistributedCache, CachePolicy, CacheStats, 
    HybridCacheConfig, WriteBackConfig, MemoryCache, DiskCache
};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, sleep};

/// 高性能多层缓存管理器
pub struct CacheManager {
    /// L1 缓存：内存缓存 (最快)
    memory_cache: Arc<MemoryCache>,
    /// L2 缓存：磁盘缓存 (持久化, 可选)
    disk_cache: Option<Arc<DiskCache>>,
    /// 分布式缓存 (可选)
    distributed_cache: Option<Box<dyn DistributedCache>>,
    /// 缓存策略配置
    policy: CachePolicy,
    /// 混合缓存配置
    hybrid_config: HybridCacheConfig,
    /// Write-back 配置
    writeback_config: WriteBackConfig,
    /// 合并统计信息
    combined_stats: Arc<Mutex<CacheStats>>,
    /// Write-back 任务控制
    writeback_shutdown: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl CacheManager {
    /// 创建仅内存缓存的缓存管理器
    pub async fn new_memory_only(
        memory_cache: MemoryCache,
        policy: CachePolicy,
    ) -> VDFSResult<Self> {
        let hybrid_config = HybridCacheConfig::default();
        let writeback_config = WriteBackConfig::default();
        
        let manager = Self {
            memory_cache: Arc::new(memory_cache),
            disk_cache: None,
            distributed_cache: None,
            policy,
            hybrid_config,
            writeback_config,
            combined_stats: Arc::new(Mutex::new(CacheStats::new())),
            writeback_shutdown: Arc::new(Mutex::new(None)),
        };

        Ok(manager)
    }

    /// 创建新的缓存管理器
    pub async fn new(
        memory_cache: MemoryCache,
        disk_cache: DiskCache,
        distributed_cache: Option<Box<dyn DistributedCache>>,
        policy: CachePolicy,
    ) -> VDFSResult<Self> {
        let hybrid_config = HybridCacheConfig::default();
        let writeback_config = WriteBackConfig::default();
        
        let manager = Self {
            memory_cache: Arc::new(memory_cache),
            disk_cache: Some(Arc::new(disk_cache)),
            distributed_cache,
            policy,
            hybrid_config,
            writeback_config,
            combined_stats: Arc::new(Mutex::new(CacheStats::new())),
            writeback_shutdown: Arc::new(Mutex::new(None)),
        };

        // 启动 Write-back 后台任务
        manager.start_writeback_task().await?;

        Ok(manager)
    }

    /// 使用默认配置创建缓存管理器
    pub async fn with_default() -> VDFSResult<Self> {
        let memory_cache = MemoryCache::with_default();
        let disk_cache = DiskCache::with_default().await?;
        let policy = CachePolicy::default();
        
        Self::new(memory_cache, disk_cache, None, policy).await
    }

    /// 根据混合策略决定缓存粒度
    fn determine_cache_strategy(&self, data_size: usize) -> CacheStrategy {
        if data_size <= self.hybrid_config.file_threshold {
            CacheStrategy::WholeFile
        } else {
            CacheStrategy::Chunked {
                chunk_size: self.hybrid_config.chunk_size,
            }
        }
    }

    /// 智能缓存分层决策
    fn should_cache_in_memory(&self, data_size: usize, access_frequency: u64) -> bool {
        // 小文件优先内存缓存
        if data_size <= 1024 * 1024 {  // 1MB
            return true;
        }
        
        // 高频访问数据优先内存缓存
        if access_frequency > 10 {
            return true;
        }
        
        // 大文件默认磁盘缓存
        false
    }

    /// 启动 Write-back 后台任务
    async fn start_writeback_task(&self) -> VDFSResult<()> {
        let memory_cache = self.memory_cache.clone();
        let disk_cache = self.disk_cache.clone();
        let writeback_config = self.writeback_config.clone();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        
        // 保存关闭信号发送器
        *self.writeback_shutdown.lock().await = Some(shutdown_tx);

        // 启动后台任务 (仅在有磁盘缓存时)
        if disk_cache.is_some() {
            tokio::spawn(async move {
                let mut interval = interval(writeback_config.interval);
                
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            // 执行 Write-back
                            if let Some(ref disk_cache) = disk_cache {
                                if let Err(e) = Self::execute_writeback(
                                    &memory_cache, 
                                    disk_cache, 
                                    &writeback_config
                                ).await {
                                    eprintln!("Write-back error: {}", e);
                                }
                            }
                        },
                        _ = &mut shutdown_rx => {
                            println!("Write-back task shutting down");
                            break;
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// 执行 Write-back 操作
    async fn execute_writeback(
        memory_cache: &MemoryCache,
        disk_cache: &DiskCache,
        config: &WriteBackConfig,
    ) -> VDFSResult<()> {
        // 获取内存中的脏数据
        let memory_dirty = memory_cache.get_dirty_entries().await;
        
        // 按紧急程度排序
        let mut sorted_entries = memory_dirty;
        sorted_entries.sort_by(|a, b| {
            b.writeback_urgency().partial_cmp(&a.writeback_urgency()).unwrap()
        });

        // 批量写回
        let batch_size = config.batch_size.min(sorted_entries.len());
        for chunk in sorted_entries.chunks(batch_size) {
            for entry in chunk {
                // 写回到磁盘缓存
                match disk_cache.put(entry.key.clone(), entry.value.clone()).await {
                    Ok(_) => {
                        // 标记写回成功
                        let _ = memory_cache.mark_writeback_complete(&entry.key).await;
                    }
                    Err(e) => {
                        eprintln!("Failed to write back {}: {}", 
                                format!("{:?}", entry.key), e);
                        let _ = memory_cache.mark_writeback_failed(&entry.key).await;
                    }
                }
            }
            
            // 批次之间短暂休息
            sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// 智能缓存获取
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // L1: 内存缓存
        if let Some(value) = self.memory_cache.get(key).await {
            self.update_combined_stats(true, true).await;
            return Some(value);
        }
        
        // L2: 磁盘缓存 (如果可用)
        if let Some(ref disk_cache) = self.disk_cache {
            if let Some(value) = disk_cache.get(key).await {
                // 根据策略决定是否提升到内存缓存
                if let CacheValue::FileData(ref data) | CacheValue::ChunkData(ref data) = value {
                    if self.should_cache_in_memory(data.len(), 1) {
                        let _ = self.memory_cache.put(key.clone(), value.clone()).await;
                    }
                }
                
                self.update_combined_stats(true, false).await;
                return Some(value);
            }
        }
        
        // L3: 分布式缓存 (如果可用)
        if let Some(distributed) = &self.distributed_cache {
            if let Ok(Some(value)) = distributed.get(key).await {
                // 缓存到本地
                let _ = self.put(key.clone(), value.clone()).await;
                return Some(value);
            }
        }
        
        // 缓存未命中
        self.update_combined_stats(false, false).await;
        None
    }

    /// 智能缓存写入
    pub async fn put(&self, key: CacheKey, value: CacheValue) -> VDFSResult<()> {
        let data_size = match &value {
            CacheValue::FileData(data) | CacheValue::ChunkData(data) => data.len(),
            _ => 1024, // 估算大小
        };

        // 决定缓存策略
        let cache_strategy = self.determine_cache_strategy(data_size);
        
        match cache_strategy {
            CacheStrategy::WholeFile => {
                // 小文件：优先内存缓存
                if self.should_cache_in_memory(data_size, 1) {
                    self.memory_cache.put(key.clone(), value.clone()).await?;
                } else if let Some(ref disk_cache) = self.disk_cache {
                    disk_cache.put(key.clone(), value.clone()).await?;
                } else {
                    // 没有磁盘缓存，回退到内存缓存
                    self.memory_cache.put(key.clone(), value.clone()).await?;
                }
            }
            CacheStrategy::Chunked { chunk_size: _ } => {
                // 大文件：磁盘缓存，热点数据可能提升到内存
                if let Some(ref disk_cache) = self.disk_cache {
                    disk_cache.put(key.clone(), value.clone()).await?;
                } else {
                    // 没有磁盘缓存，回退到内存缓存
                    self.memory_cache.put(key.clone(), value.clone()).await?;
                }
            }
        }

        // 分布式缓存 (如果可用)
        if let Some(distributed) = &self.distributed_cache {
            distributed.put(key, value).await?;
        }
        
        Ok(())
    }

    /// 失效缓存
    pub async fn invalidate(&self, key: &CacheKey) -> VDFSResult<()> {
        // 从所有层级移除
        let _ = self.memory_cache.invalidate(key).await;
        if let Some(ref disk_cache) = self.disk_cache {
            let _ = disk_cache.invalidate(key).await;
        }
        
        if let Some(distributed) = &self.distributed_cache {
            distributed.invalidate(key).await?;
        }
        
        Ok(())
    }

    /// 清空缓存
    pub async fn clear(&self) -> VDFSResult<()> {
        self.memory_cache.clear().await?;
        if let Some(ref disk_cache) = self.disk_cache {
            disk_cache.clear().await?;
        }
        
        // 重置统计
        *self.combined_stats.lock().await = CacheStats::new();
        
        Ok(())
    }

    /// 获取合并的统计信息
    pub async fn get_stats(&self) -> CacheStats {
        let memory_stats = self.memory_cache.get_stats().await;
        let disk_stats = if let Some(ref disk_cache) = self.disk_cache {
            disk_cache.get_stats().await
        } else {
            CacheStats::new()
        };
        
        let mut combined = self.combined_stats.lock().await;
        
        // 合并统计信息
        combined.memory_hits = memory_stats.hits;
        combined.disk_hits = disk_stats.hits;
        combined.hits = memory_stats.hits + disk_stats.hits;
        combined.misses = memory_stats.misses + disk_stats.misses;
        combined.memory_size = memory_stats.memory_size;
        combined.disk_size = disk_stats.disk_size;
        combined.size = memory_stats.size + disk_stats.size;
        combined.dirty_entries = memory_stats.dirty_entries + disk_stats.dirty_entries;
        combined.writebacks = memory_stats.writebacks + disk_stats.writebacks;
        combined.writeback_errors = memory_stats.writeback_errors + disk_stats.writeback_errors;
        
        combined.clone()
    }

    /// 更新合并统计
    async fn update_combined_stats(&self, hit: bool, from_memory: bool) {
        let mut stats = self.combined_stats.lock().await;
        if hit {
            stats.record_hit(from_memory);
        } else {
            stats.record_miss();
        }
    }

    /// 手动触发 Write-back
    pub async fn flush(&self) -> VDFSResult<()> {
        if let Some(ref disk_cache) = self.disk_cache {
            Self::execute_writeback(
                &self.memory_cache,
                disk_cache,
                &self.writeback_config,
            ).await
        } else {
            Ok(())
        }
    }

    /// 关闭缓存管理器
    pub async fn shutdown(&self) -> VDFSResult<()> {
        // 发送关闭信号
        if let Some(shutdown_tx) = self.writeback_shutdown.lock().await.take() {
            let _ = shutdown_tx.send(());
        }

        // 最后一次 Write-back
        self.flush().await?;

        Ok(())
    }

    /// 健康检查
    pub async fn health_check(&self) -> CacheHealthStatus {
        let stats = self.get_stats().await;
        let disk_capacity = if let Some(ref disk_cache) = self.disk_cache {
            disk_cache.capacity().await as f64
        } else {
            1.0 // 避免除零
        };
        
        CacheHealthStatus {
            memory_usage_ratio: stats.memory_size as f64 / self.memory_cache.capacity().await as f64,
            disk_usage_ratio: stats.disk_size as f64 / disk_capacity,
            hit_rate: stats.hit_rate,
            dirty_ratio: stats.dirty_entries as f64 / (stats.size as f64 + 1.0),
            healthy: (stats.hits + stats.misses == 0 || stats.hit_rate > 0.1) && 
                     (stats.writebacks == 0 || stats.writeback_errors < stats.writebacks / 10),
        }
    }
}

/// 缓存策略
#[derive(Debug, Clone)]
enum CacheStrategy {
    /// 整个文件缓存
    WholeFile,
    /// 分块缓存
    Chunked { chunk_size: usize },
}

/// 缓存健康状态
#[derive(Debug, Clone)]
pub struct CacheHealthStatus {
    pub memory_usage_ratio: f64,
    pub disk_usage_ratio: f64,
    pub hit_rate: f64,
    pub dirty_ratio: f64,
    pub healthy: bool,
}

impl Drop for CacheManager {
    fn drop(&mut self) {
        // 尝试优雅关闭 (非阻塞)
        if let Some(shutdown_tx) = self.writeback_shutdown.try_lock() 
            .ok()
            .and_then(|mut guard| guard.take()) {
            let _ = shutdown_tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_manager_multi_tier() {
        let temp_dir = TempDir::new().unwrap();
        
        let memory_cache = MemoryCache::with_default();
        let disk_cache_config = crate::vdfs::cache::disk_cache::DiskCacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let disk_cache = DiskCache::new(disk_cache_config).await.unwrap();
        let policy = CachePolicy::default();
        
        let cache_manager = CacheManager::new(memory_cache, disk_cache, None, policy).await.unwrap();
        
        let key = CacheKey::FileData(FileId::new_v4());
        let value = CacheValue::FileData(vec![1, 2, 3, 4, 5]);

        // 测试写入
        cache_manager.put(key.clone(), value.clone()).await.unwrap();
        
        // 测试读取
        let retrieved = cache_manager.get(&key).await;
        assert!(retrieved.is_some());
        
        // 多次读取以提高命中率
        for _ in 0..5 {
            let _ = cache_manager.get(&key).await;
        }
        
        // 测试统计
        let stats = cache_manager.get_stats().await;
        assert!(stats.hits > 0);
        println!("Cache stats: hits={}, misses={}, hit_rate={}", stats.hits, stats.misses, stats.hit_rate);
        
        // 健康检查
        let health = cache_manager.health_check().await;
        println!("Health check: hit_rate={}, healthy={}", health.hit_rate, health.healthy);
        assert!(health.healthy);
        
        // 关闭
        cache_manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_hybrid_caching_strategy() {
        let cache_manager = CacheManager::with_default().await.unwrap();
        
        // 小文件 (应该进入内存缓存)
        let small_key = CacheKey::FileData(FileId::new_v4());
        let small_value = CacheValue::FileData(vec![1; 1024]); // 1KB
        cache_manager.put(small_key.clone(), small_value).await.unwrap();
        
        // 大文件 (应该进入磁盘缓存)
        let large_key = CacheKey::FileData(FileId::new_v4());
        let large_value = CacheValue::FileData(vec![1; 10 * 1024 * 1024]); // 10MB
        cache_manager.put(large_key.clone(), large_value).await.unwrap();
        
        // 验证都能读取
        assert!(cache_manager.get(&small_key).await.is_some());
        assert!(cache_manager.get(&large_key).await.is_some());
        
        cache_manager.shutdown().await.unwrap();
    }
}
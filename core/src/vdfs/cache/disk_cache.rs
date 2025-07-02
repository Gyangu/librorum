//! 高性能磁盘缓存实现
//! 
//! 特性：
//! - 基于 Sled 的持久化存储
//! - 压缩存储节省空间
//! - 异步 I/O 操作
//! - 自动清理和空间管理

use crate::vdfs::{VDFSResult, VDFSError, CacheKey, CacheValue};
use crate::vdfs::cache::{CacheEntry, CacheStats, HybridCacheConfig, WriteBackConfig, LocalCache};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

/// 磁盘缓存条目 (序列化版本)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiskCacheEntry {
    value: CacheValue,
    created: u64,       // SystemTime as secs since UNIX_EPOCH
    accessed: u64,      // SystemTime as secs since UNIX_EPOCH
    access_count: u64,
    size: usize,
    dirty: bool,
    writeback_priority: u8,
    last_written: Option<u64>,
}

impl DiskCacheEntry {
    fn from_cache_entry(entry: &CacheEntry) -> Self {
        Self {
            value: entry.value.clone(),
            created: Self::system_time_to_secs(entry.created),
            accessed: Self::system_time_to_secs(entry.accessed),
            access_count: entry.access_count,
            size: entry.size,
            dirty: entry.dirty,
            writeback_priority: entry.writeback_priority,
            last_written: entry.last_written.map(Self::system_time_to_secs),
        }
    }

    fn to_cache_entry(&self, key: CacheKey) -> CacheEntry {
        CacheEntry {
            key,
            value: self.value.clone(),
            created: Self::secs_to_system_time(self.created),
            accessed: Self::secs_to_system_time(self.accessed),
            access_count: self.access_count,
            size: self.size,
            dirty: self.dirty,
            writeback_priority: self.writeback_priority,
            last_written: self.last_written.map(Self::secs_to_system_time),
        }
    }

    fn system_time_to_secs(time: SystemTime) -> u64 {
        time.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn secs_to_system_time(secs: u64) -> SystemTime {
        std::time::UNIX_EPOCH + Duration::from_secs(secs)
    }
}

/// 磁盘缓存配置
#[derive(Debug, Clone)]
pub struct DiskCacheConfig {
    /// 缓存目录路径
    pub cache_dir: PathBuf,
    /// 最大磁盘使用量 (字节)
    pub max_disk_size: usize,
    /// 压缩级别 (0-9, 0为不压缩)
    pub compression_level: u32,
    /// TTL 过期时间
    pub ttl: Duration,
    /// 清理间隔
    pub cleanup_interval: Duration,
    /// 混合缓存配置
    pub hybrid_config: HybridCacheConfig,
    /// Write-back 配置
    pub writeback_config: WriteBackConfig,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from("./vdfs_disk_cache"),
            max_disk_size: 2 * 1024 * 1024 * 1024, // 2GB
            compression_level: 3,                   // 中等压缩
            ttl: Duration::from_secs(24 * 3600),    // 24小时
            cleanup_interval: Duration::from_secs(3600), // 1小时清理一次
            hybrid_config: HybridCacheConfig::default(),
            writeback_config: WriteBackConfig::default(),
        }
    }
}

/// 高性能磁盘缓存实现
pub struct DiskCache {
    /// Sled 数据库
    db: sled::Db,
    /// 统计信息
    stats: Arc<Mutex<CacheStats>>,
    /// 配置
    config: DiskCacheConfig,
    /// 最后清理时间
    last_cleanup: Arc<Mutex<SystemTime>>,
}

impl DiskCache {
    /// 创建新的磁盘缓存
    pub async fn new(config: DiskCacheConfig) -> VDFSResult<Self> {
        // 确保缓存目录存在
        if !config.cache_dir.exists() {
            tokio::fs::create_dir_all(&config.cache_dir).await
                .map_err(|e| VDFSError::StorageError(format!("Failed to create cache directory: {}", e)))?;
        }

        // 配置 Sled 数据库
        let mut sled_config = sled::Config::default()
            .path(&config.cache_dir)
            .cache_capacity(64 * 1024 * 1024)  // 64MB 缓存
            .flush_every_ms(Some(5000));        // 5秒刷盘
        
        // 不使用压缩避免依赖冲突
        // 注释掉压缩配置以避免zstd依赖冲突

        let db = sled_config.open()
            .map_err(|e| VDFSError::StorageError(format!("Failed to open disk cache: {}", e)))?;

        let cache = Self {
            db,
            stats: Arc::new(Mutex::new(CacheStats::new())),
            config,
            last_cleanup: Arc::new(Mutex::new(SystemTime::now())),
        };

        // 初始化统计信息
        cache.update_stats().await?;

        Ok(cache)
    }

    /// 使用默认配置创建磁盘缓存
    pub async fn with_default() -> VDFSResult<Self> {
        Self::new(DiskCacheConfig::default()).await
    }

    /// 序列化缓存键
    fn serialize_key(&self, key: &CacheKey) -> Vec<u8> {
        match key {
            CacheKey::FileMetadata(path) => format!("meta:{}", path.to_string()).into_bytes(),
            CacheKey::FileData(file_id) => format!("file:{}", file_id).into_bytes(),
            CacheKey::ChunkData(chunk_id) => format!("chunk:{}", hex::encode(chunk_id)).into_bytes(),
            CacheKey::DirectoryListing(path) => format!("dir:{}", path.to_string()).into_bytes(),
        }
    }

    /// 序列化缓存值
    fn serialize_value(&self, entry: &CacheEntry) -> VDFSResult<Vec<u8>> {
        let disk_entry = DiskCacheEntry::from_cache_entry(entry);
        
        let serialized = bincode::serialize(&disk_entry)
            .map_err(|e| VDFSError::SerializationError(format!("Failed to serialize cache entry: {}", e)))?;

        // 不使用压缩避免依赖冲突
        Ok(serialized)
    }

    /// 反序列化缓存值
    fn deserialize_value(&self, key: CacheKey, data: &[u8]) -> VDFSResult<CacheEntry> {
        // 不使用压缩避免依赖冲突
        let decompressed = data.to_vec();

        let disk_entry: DiskCacheEntry = bincode::deserialize(&decompressed)
            .map_err(|e| VDFSError::SerializationError(format!("Failed to deserialize cache entry: {}", e)))?;

        Ok(disk_entry.to_cache_entry(key))
    }

    /// 压缩数据
    fn compress_data(&self, data: &[u8]) -> VDFSResult<Vec<u8>> {
        use std::io::Write;
        
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::new(self.config.compression_level)
        );
        
        encoder.write_all(data)
            .map_err(|e| VDFSError::InternalError(format!("Compression write error: {}", e)))?;
        
        encoder.finish()
            .map_err(|e| VDFSError::InternalError(format!("Compression finish error: {}", e)))
    }

    /// 解压数据
    fn decompress_data(&self, data: &[u8]) -> VDFSResult<Vec<u8>> {
        use std::io::Read;
        
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| VDFSError::InternalError(format!("Decompression error: {}", e)))?;
        
        Ok(decompressed)
    }

    /// 更新统计信息
    async fn update_stats(&self) -> VDFSResult<()> {
        let mut stats = self.stats.lock().await;
        
        // 计算总大小和条目数
        let mut total_size = 0;
        let mut entry_count = 0;
        let mut dirty_count = 0;

        for result in self.db.iter() {
            let (key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate cache: {}", e)))?;
            
            total_size += key.len() + value.len();
            entry_count += 1;

            // 检查是否为脏数据 (需要解析数据，成本较高，可以优化)
            if let Ok(cache_key) = self.parse_key(&key) {
                if let Ok(entry) = self.deserialize_value(cache_key, &value) {
                    if entry.dirty {
                        dirty_count += 1;
                    }
                }
            }
        }

        stats.size = entry_count;
        stats.disk_size = total_size;
        stats.dirty_entries = dirty_count;

        Ok(())
    }

    /// 检查是否需要清理
    async fn should_cleanup(&self) -> bool {
        let last_cleanup = *self.last_cleanup.lock().await;
        if let Ok(elapsed) = last_cleanup.elapsed() {
            elapsed > self.config.cleanup_interval
        } else {
            true
        }
    }

    /// 清理过期和超量数据
    pub async fn cleanup(&self) -> VDFSResult<()> {
        let now = SystemTime::now();
        let mut expired_keys = Vec::new();
        let mut entries_with_scores = Vec::new();

        // 收集过期条目和计算淘汰分数
        for result in self.db.iter() {
            let (key_bytes, value_bytes) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for cleanup: {}", e)))?;
            
            if let Ok(cache_key) = self.parse_key(&key_bytes) {
                if let Ok(entry) = self.deserialize_value(cache_key.clone(), &value_bytes) {
                    // 检查是否过期
                    if entry.is_expired(self.config.ttl) {
                        expired_keys.push(key_bytes.clone());
                        continue;
                    }

                    // 计算淘汰分数 (跳过脏数据)
                    if !entry.dirty {
                        let score = self.calculate_eviction_score(&entry);
                        entries_with_scores.push((key_bytes.clone(), score, entry.size));
                    }
                }
            }
        }

        // 删除过期条目
        for key in expired_keys {
            self.db.remove(&key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to remove expired entry: {}", e)))?;
        }

        // 如果磁盘使用量超限，按分数淘汰数据
        let current_size = self.stats.lock().await.disk_size;
        if current_size > self.config.max_disk_size {
            // 按淘汰分数排序
            entries_with_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            let target_size = self.config.max_disk_size * 4 / 5; // 减少到80%
            let mut evicted_size = 0;

            for (key, _score, size) in entries_with_scores {
                self.db.remove(&key)
                    .map_err(|e| VDFSError::StorageError(format!("Failed to evict entry: {}", e)))?;
                
                evicted_size += size;
                
                if current_size - evicted_size <= target_size {
                    break;
                }
            }
        }

        // 刷盘并更新清理时间
        self.db.flush_async().await
            .map_err(|e| VDFSError::StorageError(format!("Failed to flush cache: {}", e)))?;

        *self.last_cleanup.lock().await = now;

        // 更新统计
        self.update_stats().await?;

        Ok(())
    }

    /// 解析序列化的键
    fn parse_key(&self, key_bytes: &[u8]) -> VDFSResult<CacheKey> {
        let key_str = String::from_utf8(key_bytes.to_vec())
            .map_err(|e| VDFSError::InternalError(format!("Invalid key UTF8: {}", e)))?;

        if let Some(path_str) = key_str.strip_prefix("meta:") {
            Ok(CacheKey::FileMetadata(crate::vdfs::VirtualPath::new(path_str)))
        } else if let Some(file_id_str) = key_str.strip_prefix("file:") {
            let file_id = file_id_str.parse()
                .map_err(|e| VDFSError::InternalError(format!("Invalid file ID: {}", e)))?;
            Ok(CacheKey::FileData(file_id))
        } else if let Some(chunk_hex) = key_str.strip_prefix("chunk:") {
            let chunk_bytes = hex::decode(chunk_hex)
                .map_err(|e| VDFSError::InternalError(format!("Invalid chunk hex: {}", e)))?;
            let mut chunk_id = [0u8; 32];
            if chunk_bytes.len() == 32 {
                chunk_id.copy_from_slice(&chunk_bytes);
                Ok(CacheKey::ChunkData(chunk_id))
            } else {
                Err(VDFSError::InternalError("Invalid chunk ID length".to_string()))
            }
        } else if let Some(path_str) = key_str.strip_prefix("dir:") {
            Ok(CacheKey::DirectoryListing(crate::vdfs::VirtualPath::new(path_str)))
        } else {
            Err(VDFSError::InternalError(format!("Unknown key format: {}", key_str)))
        }
    }

    /// 计算淘汰分数
    fn calculate_eviction_score(&self, entry: &CacheEntry) -> f64 {
        let now = SystemTime::now();
        
        // LRU 分数
        let lru_score = if let Ok(elapsed) = now.duration_since(entry.accessed) {
            elapsed.as_secs_f64()
        } else {
            0.0
        };

        // LFU 分数
        let lfu_score = 1.0 / (entry.access_count as f64 + 1.0);

        // 大小分数
        let size_score = entry.size as f64 / (1024.0 * 1024.0);

        0.5 * lru_score + 0.3 * lfu_score + 0.2 * size_score
    }

    /// 获取脏数据条目用于写回
    pub async fn get_dirty_entries(&self) -> VDFSResult<Vec<CacheEntry>> {
        let mut dirty_entries = Vec::new();

        for result in self.db.iter() {
            let (key_bytes, value_bytes) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for dirty entries: {}", e)))?;
            
            if let Ok(cache_key) = self.parse_key(&key_bytes) {
                if let Ok(entry) = self.deserialize_value(cache_key, &value_bytes) {
                    if entry.dirty {
                        dirty_entries.push(entry);
                    }
                }
            }
        }

        Ok(dirty_entries)
    }

    /// 标记写回完成
    pub async fn mark_writeback_complete(&self, key: &CacheKey) -> VDFSResult<()> {
        let key_bytes = self.serialize_key(key);
        
        if let Some(value_bytes) = self.db.get(&key_bytes)
            .map_err(|e| VDFSError::StorageError(format!("Failed to get entry for writeback: {}", e)))? {
            
            if let Ok(mut entry) = self.deserialize_value(key.clone(), &value_bytes) {
                entry.mark_clean();
                let serialized = self.serialize_value(&entry)?;
                
                self.db.insert(&key_bytes, serialized)
                    .map_err(|e| VDFSError::StorageError(format!("Failed to update writeback status: {}", e)))?;
                
                self.stats.lock().await.record_writeback(true);
            }
        }

        Ok(())
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.lock().await.clone()
    }
}

#[async_trait]
impl LocalCache for DiskCache {
    async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // 检查是否需要清理
        if self.should_cleanup().await {
            if let Err(e) = self.cleanup().await {
                eprintln!("Cache cleanup failed: {}", e);
            }
        }

        let key_bytes = self.serialize_key(key);
        
        match self.db.get(&key_bytes) {
            Ok(Some(value_bytes)) => {
                match self.deserialize_value(key.clone(), &value_bytes) {
                    Ok(mut entry) => {
                        // 检查是否过期
                        if entry.is_expired(self.config.ttl) {
                            // 异步删除过期条目
                            let _ = self.db.remove(&key_bytes);
                            self.stats.lock().await.record_miss();
                            return None;
                        }

                        // 更新访问信息
                        entry.access();
                        
                        // 异步更新到磁盘 (可以优化为批量更新)
                        if let Ok(serialized) = self.serialize_value(&entry) {
                            let _ = self.db.insert(&key_bytes, serialized);
                        }

                        self.stats.lock().await.record_hit(false);
                        Some(entry.value)
                    }
                    Err(_) => {
                        self.stats.lock().await.record_miss();
                        None
                    }
                }
            }
            _ => {
                self.stats.lock().await.record_miss();
                None
            }
        }
    }

    async fn put(&self, key: CacheKey, value: CacheValue) -> VDFSResult<()> {
        let mut entry = CacheEntry::new(key.clone(), value);
        
        // 如果是数据写入，标记为脏数据
        match &key {
            CacheKey::FileData(_) | CacheKey::ChunkData(_) => {
                entry.mark_dirty(3); // 磁盘缓存写回优先级较低
                self.stats.lock().await.record_dirty_entry();
            }
            _ => {}
        }

        let key_bytes = self.serialize_key(&key);
        let value_bytes = self.serialize_value(&entry)?;

        self.db.insert(&key_bytes, value_bytes)
            .map_err(|e| VDFSError::StorageError(format!("Failed to insert cache entry: {}", e)))?;

        // 更新统计 (简化版本，避免每次都重新计算)
        let mut stats = self.stats.lock().await;
        stats.size += 1;
        stats.disk_size += key_bytes.len() + entry.size;

        Ok(())
    }

    async fn invalidate(&self, key: &CacheKey) -> VDFSResult<()> {
        let key_bytes = self.serialize_key(key);
        
        self.db.remove(&key_bytes)
            .map_err(|e| VDFSError::StorageError(format!("Failed to invalidate cache entry: {}", e)))?;

        // 更新统计
        let mut stats = self.stats.lock().await;
        if stats.size > 0 {
            stats.size -= 1;
        }

        Ok(())
    }

    async fn clear(&self) -> VDFSResult<()> {
        self.db.clear()
            .map_err(|e| VDFSError::StorageError(format!("Failed to clear cache: {}", e)))?;

        // 重置统计
        let mut stats = self.stats.lock().await;
        *stats = CacheStats::new();

        Ok(())
    }

    async fn size(&self) -> usize {
        self.stats.lock().await.disk_size
    }

    async fn capacity(&self) -> usize {
        self.config.max_disk_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_disk_cache_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = DiskCacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let cache = DiskCache::new(config).await.unwrap();
        
        let key = CacheKey::FileData(crate::vdfs::FileId::new_v4());
        let value = CacheValue::FileData(vec![1, 2, 3, 4, 5]);

        // 测试写入
        cache.put(key.clone(), value.clone()).await.unwrap();
        
        // 测试读取
        let retrieved = cache.get(&key).await;
        assert!(retrieved.is_some());
        
        // 测试统计
        let stats = cache.get_stats().await;
        assert!(stats.size > 0);
    }

    #[tokio::test]
    async fn test_disk_cache_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        
        let key = CacheKey::FileData(crate::vdfs::FileId::new_v4());
        let value = CacheValue::FileData(vec![1, 2, 3, 4, 5]);

        // 创建缓存并写入数据
        {
            let config = DiskCacheConfig {
                cache_dir: cache_dir.clone(),
                ..Default::default()
            };
            let cache = DiskCache::new(config).await.unwrap();
            cache.put(key.clone(), value.clone()).await.unwrap();
        }

        // 重新打开缓存，验证数据持久化
        {
            let config = DiskCacheConfig {
                cache_dir: cache_dir.clone(),
                ..Default::default()
            };
            let cache = DiskCache::new(config).await.unwrap();
            let retrieved = cache.get(&key).await;
            assert!(retrieved.is_some());
        }
    }
}
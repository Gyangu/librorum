use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId, ChunkId};
use crate::vdfs::filesystem::FileMetadata;
use crate::vdfs::metadata::{FileInfo, ChunkMetadata, MetadataManager};
use async_trait::async_trait;
use rocksdb::{DB, Options, ColumnFamilyDescriptor, WriteBatch};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// RocksDB-based metadata manager - Facebook 生产级 LSM-Tree 存储引擎
pub struct RocksDBMetadataManager {
    db: Arc<DB>,
}

impl RocksDBMetadataManager {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> VDFSResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        
        // 性能优化配置
        opts.set_max_background_jobs(6);
        opts.set_max_subcompactions(2);
        opts.set_bytes_per_sync(1048576); // 1MB
        opts.set_write_buffer_size(128 * 1024 * 1024); // 128MB
        opts.set_max_write_buffer_number(3);
        opts.set_target_file_size_base(64 * 1024 * 1024); // 64MB
        opts.set_level_zero_file_num_compaction_trigger(4);
        opts.set_level_zero_slowdown_writes_trigger(20);
        opts.set_level_zero_stop_writes_trigger(36);
        opts.set_max_bytes_for_level_base(256 * 1024 * 1024); // 256MB
        opts.set_max_bytes_for_level_multiplier(10.0);
        
        // 压缩配置
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        // 定义列族
        let cf_names = vec![
            "files",
            "attributes", 
            "chunks",
            "file_replicas",
            "chunk_replicas",
            "path_index"
        ];

        let mut cf_opts = Options::default();
        cf_opts.set_max_write_buffer_number(3);
        cf_opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB

        let cfs: Vec<ColumnFamilyDescriptor> = cf_names.iter()
            .map(|name| ColumnFamilyDescriptor::new(*name, cf_opts.clone()))
            .collect();

        let db = DB::open_cf_descriptors(&opts, db_path, cfs)
            .map_err(|e| VDFSError::StorageError(format!("Failed to open RocksDB: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// 创建临时数据库用于测试
    pub async fn new_temp() -> VDFSResult<Self> {
        let temp_dir = tempfile::TempDir::new()
            .map_err(|e| VDFSError::StorageError(format!("Failed to create temp dir: {}", e)))?;
        
        let path = temp_dir.into_path();
        
        // 使用 spawn_blocking 处理阻塞的 RocksDB 创建
        let db = tokio::task::spawn_blocking(move || {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);
            
            let cf_names = vec![
                "files", "attributes", "chunks", 
                "file_replicas", "chunk_replicas", "path_index"
            ];

            let cf_opts = Options::default();
            let cfs: Vec<ColumnFamilyDescriptor> = cf_names.iter()
                .map(|name| ColumnFamilyDescriptor::new(*name, cf_opts.clone()))
                .collect();

            DB::open_cf_descriptors(&opts, path, cfs)
                .map_err(|e| VDFSError::StorageError(format!("Failed to open temp RocksDB: {}", e)))
        }).await
        .map_err(|e| VDFSError::InternalError(format!("Spawn blocking error: {}", e)))??;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    fn serialize<T: Serialize>(&self, value: &T) -> VDFSResult<Vec<u8>> {
        bincode::serialize(value)
            .map_err(|e| VDFSError::SerializationError(format!("Serialization failed: {}", e)))
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> VDFSResult<T> {
        bincode::deserialize(bytes)
            .map_err(|e| VDFSError::SerializationError(format!("Deserialization failed: {}", e)))
    }

    fn path_to_key(&self, path: &VirtualPath) -> Vec<u8> {
        format!("file:{}", path.to_string()).into_bytes()
    }

    fn file_id_to_key(&self, file_id: &FileId) -> Vec<u8> {
        format!("fid:{}", file_id.to_string()).into_bytes()
    }

    fn chunk_id_to_key(&self, chunk_id: &ChunkId) -> Vec<u8> {
        format!("chunk:{}", hex::encode(chunk_id)).into_bytes()
    }

    fn attr_key(&self, file_id: &FileId, attr_name: &str) -> Vec<u8> {
        format!("attr:{}:{}", file_id, attr_name).into_bytes()
    }

    fn replica_key(&self, file_id: &FileId) -> Vec<u8> {
        format!("replica:{}", file_id).into_bytes()
    }

    fn chunk_replica_key(&self, chunk_id: &ChunkId) -> Vec<u8> {
        format!("chunkrep:{}", hex::encode(chunk_id)).into_bytes()
    }

    async fn get_file_attributes(&self, file_id: &FileId) -> VDFSResult<HashMap<String, String>> {
        let prefix = format!("attr:{}:", file_id);
        let prefix_bytes = prefix.as_bytes();
        let mut attributes = HashMap::new();

        let iter = self.db.prefix_iterator(prefix_bytes);
        for result in iter {
            let (key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate attributes: {}", e)))?;
            
            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid key UTF8: {}", e)))?;
            
            // 提取属性名 (格式: "attr:file_id:attr_name")
            if let Some(attr_name) = key_str.split(':').nth(2) {
                let attr_value: String = self.deserialize(&value)?;
                attributes.insert(attr_name.to_string(), attr_value);
            }
        }

        Ok(attributes)
    }

    async fn set_file_attributes(&self, file_id: &FileId, attributes: &HashMap<String, String>) -> VDFSResult<()> {
        let mut batch = WriteBatch::default();
        
        // 删除旧属性
        let prefix = format!("attr:{}:", file_id);
        let prefix_bytes = prefix.as_bytes();
        let iter = self.db.prefix_iterator(prefix_bytes);
        for result in iter {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate old attributes: {}", e)))?;
            batch.delete(&key);
        }

        // 添加新属性
        for (attr_name, attr_value) in attributes {
            let key = self.attr_key(file_id, attr_name);
            let value = self.serialize(attr_value)?;
            batch.put(key, value);
        }

        self.db.write(batch)
            .map_err(|e| VDFSError::StorageError(format!("Failed to write attributes batch: {}", e)))?;

        Ok(())
    }

    fn system_time_to_timestamp(time: SystemTime) -> u64 {
        time.duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn timestamp_to_system_time(timestamp: u64) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(timestamp)
    }
}

#[async_trait]
impl MetadataManager for RocksDBMetadataManager {
    async fn get_file_info(&self, path: &VirtualPath) -> VDFSResult<FileInfo> {
        let key = self.path_to_key(path);
        
        if let Some(value) = self.db.get(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to get file info: {}", e)))? {
            
            let file_info: FileInfo = self.deserialize(&value)?;
            Ok(file_info)
        } else {
            Err(VDFSError::FileNotFound(path.clone()))
        }
    }

    async fn set_file_info(&self, path: &VirtualPath, mut file_info: FileInfo) -> VDFSResult<()> {
        // 更新路径
        file_info.metadata.path = path.clone();
        
        let mut batch = WriteBatch::default();
        
        // 序列化并存储文件信息
        let key = self.path_to_key(path);
        let value = self.serialize(&file_info)?;
        batch.put(&key, value);

        // 更新路径索引 (file_id -> path)
        let file_id_key = self.file_id_to_key(&file_info.metadata.id);
        batch.put(&file_id_key, &key);

        // 提交批量写入
        self.db.write(batch)
            .map_err(|e| VDFSError::StorageError(format!("Failed to write file info batch: {}", e)))?;

        // 异步存储文件属性
        self.set_file_attributes(&file_info.metadata.id, &file_info.metadata.custom_attributes).await?;

        // 存储文件副本
        if !file_info.replicas.is_empty() {
            let replica_key = self.replica_key(&file_info.metadata.id);
            let replica_value = self.serialize(&file_info.replicas)?;
            self.db.put(replica_key, replica_value)
                .map_err(|e| VDFSError::StorageError(format!("Failed to set replicas: {}", e)))?;
        }

        // 存储 chunks
        for chunk in &file_info.chunks {
            let chunk_key = format!("chunk:{}:{}", file_info.metadata.id, hex::encode(&chunk.id));
            let chunk_value = self.serialize(chunk)?;
            self.db.put(chunk_key.as_bytes(), chunk_value)
                .map_err(|e| VDFSError::StorageError(format!("Failed to set chunk: {}", e)))?;

            // 存储 chunk 副本
            if !chunk.replicas.is_empty() {
                let chunk_replica_key = self.chunk_replica_key(&chunk.id);
                let chunk_replica_value = self.serialize(&chunk.replicas)?;
                self.db.put(chunk_replica_key, chunk_replica_value)
                    .map_err(|e| VDFSError::StorageError(format!("Failed to set chunk replicas: {}", e)))?;
            }
        }

        // 强制刷盘
        self.db.flush()
            .map_err(|e| VDFSError::StorageError(format!("Failed to flush database: {}", e)))?;

        Ok(())
    }

    async fn delete_file_info(&self, path: &VirtualPath) -> VDFSResult<()> {
        let key = self.path_to_key(path);
        
        // 获取文件信息以便删除相关数据
        if let Some(value) = self.db.get(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to get file for deletion: {}", e)))? {
            
            let file_info: FileInfo = self.deserialize(&value)?;
            let mut batch = WriteBatch::default();
            
            // 删除主文件记录
            batch.delete(&key);

            // 删除路径索引
            let file_id_key = self.file_id_to_key(&file_info.metadata.id);
            batch.delete(&file_id_key);

            // 删除文件副本
            let replica_key = self.replica_key(&file_info.metadata.id);
            batch.delete(&replica_key);

            // 删除文件属性
            let attr_prefix = format!("attr:{}:", file_info.metadata.id);
            let attr_prefix_bytes = attr_prefix.as_bytes();
            let iter = self.db.prefix_iterator(attr_prefix_bytes);
            for result in iter {
                let (attr_key, _) = result
                    .map_err(|e| VDFSError::StorageError(format!("Failed to iterate attributes for deletion: {}", e)))?;
                batch.delete(&attr_key);
            }

            // 删除 chunks
            for chunk in &file_info.chunks {
                let chunk_key = format!("chunk:{}:{}", file_info.metadata.id, hex::encode(&chunk.id));
                batch.delete(chunk_key.as_bytes());

                // 删除 chunk 副本
                let chunk_replica_key = self.chunk_replica_key(&chunk.id);
                batch.delete(&chunk_replica_key);
            }

            // 提交批量删除
            self.db.write(batch)
                .map_err(|e| VDFSError::StorageError(format!("Failed to write deletion batch: {}", e)))?;
        }

        Ok(())
    }

    async fn file_exists(&self, path: &VirtualPath) -> VDFSResult<bool> {
        let key = self.path_to_key(path);
        let exists = self.db.get(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to check file existence: {}", e)))?
            .is_some();
        Ok(exists)
    }

    async fn get_chunk_mapping(&self, file_id: FileId) -> VDFSResult<Vec<ChunkId>> {
        // 通过文件ID获取文件信息，然后提取chunk IDs
        let file_id_key = self.file_id_to_key(&file_id);
        if let Some(path_key) = self.db.get(&file_id_key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to get path for file_id: {}", e)))? {
            
            if let Some(file_value) = self.db.get(&path_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to get file info: {}", e)))? {
                
                let file_info: FileInfo = self.deserialize(&file_value)?;
                Ok(file_info.chunks.into_iter().map(|chunk| chunk.id).collect())
            } else {
                Ok(Vec::new())
            }
        } else {
            Ok(Vec::new())
        }
    }

    async fn update_chunk_mapping(&self, _file_id: FileId, _chunks: Vec<ChunkId>) -> VDFSResult<()> {
        // This would be implemented through set_file_info
        Ok(())
    }

    async fn get_chunk_metadata(&self, chunk_id: ChunkId) -> VDFSResult<ChunkMetadata> {
        // 遍历所有 chunks 寻找匹配的 chunk_id
        let prefix = "chunk:".as_bytes();
        let iter = self.db.prefix_iterator(prefix);
        for result in iter {
            let (_, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate chunks: {}", e)))?;
            
            let chunk: ChunkMetadata = self.deserialize(&value)?;
            if chunk.id == chunk_id {
                return Ok(chunk);
            }
        }

        Err(VDFSError::InternalError("Chunk not found".to_string()))
    }

    async fn update_chunk_metadata(&self, _chunk_id: ChunkId, _metadata: ChunkMetadata) -> VDFSResult<()> {
        // This would be implemented through set_file_info
        Ok(())
    }

    async fn list_directory(&self, path: &VirtualPath) -> VDFSResult<Vec<VirtualPath>> {
        let file_prefix = "file:".as_bytes();
        let dir_prefix = if path.to_string() == "/" {
            "file:/".to_string()
        } else {
            format!("file:{}/", path.to_string())
        };

        let mut results = Vec::new();
        let mut seen_dirs = std::collections::HashSet::new();

        let iter = self.db.prefix_iterator(file_prefix);
        for result in iter {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate files: {}", e)))?;
            
            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;

            // 去掉 "file:" 前缀
            if let Some(file_path) = key_str.strip_prefix("file:") {
                if file_path.starts_with(&dir_prefix[5..]) && file_path != path.to_string() {
                    let relative_path = &file_path[dir_prefix.len() - 5..];
                    
                    // 检查是否是直接子项（不包含更多的 '/'）
                    if let Some(slash_pos) = relative_path.find('/') {
                        // 这是一个子目录
                        let dir_name = &relative_path[..slash_pos];
                        let dir_path = if path.to_string() == "/" {
                            format!("/{}", dir_name)
                        } else {
                            format!("{}/{}", path.to_string(), dir_name)
                        };
                        
                        if seen_dirs.insert(dir_path.clone()) {
                            results.push(VirtualPath::from(dir_path));
                        }
                    } else {
                        // 这是一个直接文件
                        results.push(VirtualPath::from(file_path));
                    }
                }
            }
        }

        Ok(results)
    }

    async fn create_directory(&self, path: &VirtualPath) -> VDFSResult<()> {
        let file_id = uuid::Uuid::new_v4();
        let now = SystemTime::now();
        
        let metadata = FileMetadata {
            id: file_id,
            path: path.clone(),
            size: 0,
            created: now,
            modified: now,
            accessed: now,
            permissions: crate::vdfs::FilePermissions::default(),
            checksum: None,
            mime_type: None,
            custom_attributes: HashMap::new(),
            is_directory: true,
        };

        let file_info = FileInfo {
            metadata,
            chunks: Vec::new(),
            replicas: Vec::new(),
            version: 1,
            checksum: String::new(),
        };

        self.set_file_info(path, file_info).await
    }

    async fn remove_directory(&self, path: &VirtualPath) -> VDFSResult<()> {
        let dir_prefix = format!("file:{}/", path.to_string());
        let self_key = format!("file:{}", path.to_string());
        let mut batch = WriteBatch::default();
        
        // 收集所有需要删除的路径
        let file_prefix = "file:".as_bytes();
        let iter = self.db.prefix_iterator(file_prefix);
        for result in iter {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for directory removal: {}", e)))?;
            
            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;

            if key_str == self_key || key_str.starts_with(&dir_prefix) {
                batch.delete(&key);
            }
        }

        self.db.write(batch)
            .map_err(|e| VDFSError::StorageError(format!("Failed to write directory removal batch: {}", e)))?;

        Ok(())
    }

    async fn find_files_by_pattern(&self, pattern: &str) -> VDFSResult<Vec<VirtualPath>> {
        let mut results = Vec::new();
        let file_prefix = "file:".as_bytes();
        
        let iter = self.db.prefix_iterator(file_prefix);
        for result in iter {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for pattern search: {}", e)))?;
            
            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;

            if let Some(file_path) = key_str.strip_prefix("file:") {
                if file_path.contains(pattern) {
                    results.push(VirtualPath::from(file_path));
                }
            }
        }

        Ok(results)
    }

    async fn find_files_by_size(&self, min_size: u64, max_size: u64) -> VDFSResult<Vec<VirtualPath>> {
        let mut results = Vec::new();
        let file_prefix = "file:".as_bytes();
        
        let iter = self.db.prefix_iterator(file_prefix);
        for result in iter {
            let (key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for size search: {}", e)))?;
            
            let file_info: FileInfo = self.deserialize(&value)?;
            
            if file_info.metadata.size >= min_size && file_info.metadata.size <= max_size {
                let key_str = String::from_utf8(key.to_vec())
                    .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;
                if let Some(file_path) = key_str.strip_prefix("file:") {
                    results.push(VirtualPath::from(file_path));
                }
            }
        }

        Ok(results)
    }

    async fn find_files_by_date(&self, start: SystemTime, end: SystemTime) -> VDFSResult<Vec<VirtualPath>> {
        let start_timestamp = Self::system_time_to_timestamp(start);
        let end_timestamp = Self::system_time_to_timestamp(end);
        let mut results = Vec::new();
        let file_prefix = "file:".as_bytes();
        
        let iter = self.db.prefix_iterator(file_prefix);
        for result in iter {
            let (key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for date search: {}", e)))?;
            
            let file_info: FileInfo = self.deserialize(&value)?;
            let modified_timestamp = Self::system_time_to_timestamp(file_info.metadata.modified);
            
            if modified_timestamp >= start_timestamp && modified_timestamp <= end_timestamp {
                let key_str = String::from_utf8(key.to_vec())
                    .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;
                if let Some(file_path) = key_str.strip_prefix("file:") {
                    results.push(VirtualPath::from(file_path));
                }
            }
        }

        Ok(results)
    }

    async fn verify_consistency(&self) -> VDFSResult<Vec<VirtualPath>> {
        let mut inconsistent_files = Vec::new();

        // 检查路径索引一致性
        let fid_prefix = "fid:".as_bytes();
        let iter = self.db.prefix_iterator(fid_prefix);
        for result in iter {
            let (file_id_key, path_key) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate path index: {}", e)))?;
            
            // 检查对应的文件是否存在
            if self.db.get(&path_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to check file consistency: {}", e)))?
                .is_none() {
                
                let file_id_str = String::from_utf8(file_id_key.to_vec())
                    .unwrap_or_else(|_| "invalid_file_id".to_string());
                inconsistent_files.push(VirtualPath::new(format!("orphaned_index:{}", file_id_str)));
            }
        }

        Ok(inconsistent_files)
    }

    async fn repair_metadata(&self, _path: &VirtualPath) -> VDFSResult<()> {
        let mut batch = WriteBatch::default();
        
        // 清理孤立的路径索引
        let fid_prefix = "fid:".as_bytes();
        let iter = self.db.prefix_iterator(fid_prefix);
        for result in iter {
            let (file_id_key, path_key) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for repair: {}", e)))?;
            
            if self.db.get(&path_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to check file for repair: {}", e)))?
                .is_none() {
                batch.delete(&file_id_key);
            }
        }

        self.db.write(batch)
            .map_err(|e| VDFSError::StorageError(format!("Failed to write repair batch: {}", e)))?;

        Ok(())
    }

    async fn rebuild_index(&self) -> VDFSResult<()> {
        let mut batch = WriteBatch::default();
        
        // 清空路径索引
        let fid_prefix = "fid:".as_bytes();
        let iter = self.db.prefix_iterator(fid_prefix);
        for result in iter {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for clear: {}", e)))?;
            batch.delete(&key);
        }

        // 重建路径索引
        let file_prefix = "file:".as_bytes();
        let iter = self.db.prefix_iterator(file_prefix);
        for result in iter {
            let (path_key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for rebuild: {}", e)))?;
            
            let file_info: FileInfo = self.deserialize(&value)?;
            let file_id_key = self.file_id_to_key(&file_info.metadata.id);
            
            batch.put(&file_id_key, &path_key);
        }

        self.db.write(batch)
            .map_err(|e| VDFSError::StorageError(format!("Failed to write rebuild batch: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdfs::{VirtualPath, FileId};
    use crate::vdfs::filesystem::FileMetadata;
    use std::collections::HashMap;
    use std::time::SystemTime;

    async fn create_test_database() -> RocksDBMetadataManager {
        RocksDBMetadataManager::new_temp().await.unwrap()
    }

    fn create_test_file_info(path: &str) -> FileInfo {
        let file_id = uuid::Uuid::new_v4();
        let now = SystemTime::now();
        
        let metadata = FileMetadata {
            id: file_id,
            path: VirtualPath::new(path),
            size: 1024,
            created: now,
            modified: now,
            accessed: now,
            permissions: crate::vdfs::FilePermissions::default(),
            checksum: Some("test_checksum".to_string()),
            mime_type: Some("text/plain".to_string()),
            custom_attributes: HashMap::new(),
            is_directory: false,
        };

        FileInfo {
            metadata,
            chunks: Vec::new(),
            replicas: Vec::new(),
            version: 1,
            checksum: "test_checksum".to_string(),
        }
    }

    #[tokio::test]
    async fn test_rocksdb_database_creation() {
        let _db = create_test_database();
        // If we get here without panicking, database creation succeeded
    }

    #[tokio::test]
    async fn test_rocksdb_file_crud_operations() {
        let db = create_test_database().await;
        let path = VirtualPath::new("/test/file.txt");
        
        // Test file doesn't exist initially
        assert!(!db.file_exists(&path).await.unwrap());
        
        // Create file info
        let file_info = create_test_file_info("/test/file.txt");
        
        // Set file info
        db.set_file_info(&path, file_info.clone()).await.unwrap();
        
        // Test file exists now
        assert!(db.file_exists(&path).await.unwrap());
        
        // Get file info
        let retrieved_info = db.get_file_info(&path).await.unwrap();
        assert_eq!(retrieved_info.metadata.id, file_info.metadata.id);
        assert_eq!(retrieved_info.metadata.size, file_info.metadata.size);
        assert_eq!(retrieved_info.version, file_info.version);
        
        // Delete file info
        db.delete_file_info(&path).await.unwrap();
        
        // Test file doesn't exist anymore
        assert!(!db.file_exists(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_rocksdb_directory_operations() {
        let db = create_test_database().await;
        let dir_path = VirtualPath::new("/test/directory");
        
        // Create directory
        db.create_directory(&dir_path).await.unwrap();
        
        // Verify directory exists
        assert!(db.file_exists(&dir_path).await.unwrap());
        
        // Get directory info
        let dir_info = db.get_file_info(&dir_path).await.unwrap();
        assert!(dir_info.metadata.is_directory);
        
        // Remove directory
        db.remove_directory(&dir_path).await.unwrap();
        
        // Verify directory is gone
        assert!(!db.file_exists(&dir_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_rocksdb_search_operations() {
        let db = create_test_database().await;
        
        // Create test files
        let paths = vec![
            "/test/file1.txt",
            "/test/file2.log", 
            "/test/subdir/file3.txt",
            "/other/file4.txt"
        ];
        
        for path in &paths {
            let file_info = create_test_file_info(path);
            let vpath = VirtualPath::new(*path);
            db.set_file_info(&vpath, file_info).await.unwrap();
        }
        
        // Test pattern search
        let results = db.find_files_by_pattern("txt").await.unwrap();
        assert_eq!(results.len(), 3); // Should find 3 .txt files
        
        let results = db.find_files_by_pattern("test").await.unwrap();
        assert_eq!(results.len(), 3); // Should find 3 files in /test/
        
        // Test size search
        let results = db.find_files_by_size(1000, 2000).await.unwrap();
        assert_eq!(results.len(), 4); // All files have size 1024
        
        let results = db.find_files_by_size(2000, 3000).await.unwrap();
        assert_eq!(results.len(), 0); // No files in this size range
    }

    #[tokio::test]
    async fn test_rocksdb_file_with_attributes() {
        let db = create_test_database().await;
        let path = VirtualPath::new("/test/file_with_attrs.txt");
        
        let mut file_info = create_test_file_info("/test/file_with_attrs.txt");
        file_info.metadata.custom_attributes.insert("key1".to_string(), "value1".to_string());
        file_info.metadata.custom_attributes.insert("key2".to_string(), "value2".to_string());
        
        // Set file info with attributes
        db.set_file_info(&path, file_info.clone()).await.unwrap();
        
        // Get file info and verify attributes
        let retrieved_info = db.get_file_info(&path).await.unwrap();
        assert_eq!(retrieved_info.metadata.custom_attributes.len(), 2);
        assert_eq!(retrieved_info.metadata.custom_attributes.get("key1"), Some(&"value1".to_string()));
        assert_eq!(retrieved_info.metadata.custom_attributes.get("key2"), Some(&"value2".to_string()));
    }
}
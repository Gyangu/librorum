use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId, ChunkId, NodeId};
use crate::vdfs::filesystem::FileMetadata;
use crate::vdfs::metadata::{FileInfo, ChunkMetadata, MetadataManager};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Sled-based metadata manager - 高性能 Rust 原生嵌入式数据库
pub struct SledMetadataManager {
    db: sled::Db,
    // 使用不同的 Tree 来分离数据类型
    files_tree: sled::Tree,           // 文件元数据
    attributes_tree: sled::Tree,      // 文件属性
    chunks_tree: sled::Tree,          // chunk 元数据
    file_replicas_tree: sled::Tree,   // 文件副本
    chunk_replicas_tree: sled::Tree,  // chunk 副本
    path_index_tree: sled::Tree,      // 路径索引
}

impl SledMetadataManager {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> VDFSResult<Self> {
        let config = sled::Config::default()
            .path(db_path)
            .cache_capacity(64 * 1024 * 1024) // 64MB 缓存
            .flush_every_ms(Some(1000))       // 每秒刷盘
            .compression_factor(4)            // 压缩
            .use_compression(true);

        let db = config.open()
            .map_err(|e| VDFSError::StorageError(format!("Failed to open Sled database: {}", e)))?;

        // 创建不同的存储树
        let files_tree = db.open_tree("files")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open files tree: {}", e)))?;
        let attributes_tree = db.open_tree("attributes")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open attributes tree: {}", e)))?;
        let chunks_tree = db.open_tree("chunks")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open chunks tree: {}", e)))?;
        let file_replicas_tree = db.open_tree("file_replicas")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open file_replicas tree: {}", e)))?;
        let chunk_replicas_tree = db.open_tree("chunk_replicas")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open chunk_replicas tree: {}", e)))?;
        let path_index_tree = db.open_tree("path_index")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open path_index tree: {}", e)))?;

        Ok(Self {
            db,
            files_tree,
            attributes_tree,
            chunks_tree,
            file_replicas_tree,
            chunk_replicas_tree,
            path_index_tree,
        })
    }

    /// 创建内存数据库用于测试
    pub fn new_temp() -> VDFSResult<Self> {
        let config = sled::Config::default()
            .temporary(true)
            .cache_capacity(16 * 1024 * 1024); // 16MB 缓存

        let db = config.open()
            .map_err(|e| VDFSError::StorageError(format!("Failed to open temp Sled database: {}", e)))?;

        let files_tree = db.open_tree("files")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open files tree: {}", e)))?;
        let attributes_tree = db.open_tree("attributes")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open attributes tree: {}", e)))?;
        let chunks_tree = db.open_tree("chunks")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open chunks tree: {}", e)))?;
        let file_replicas_tree = db.open_tree("file_replicas")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open file_replicas tree: {}", e)))?;
        let chunk_replicas_tree = db.open_tree("chunk_replicas")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open chunk_replicas tree: {}", e)))?;
        let path_index_tree = db.open_tree("path_index")
            .map_err(|e| VDFSError::StorageError(format!("Failed to open path_index tree: {}", e)))?;

        Ok(Self {
            db,
            files_tree,
            attributes_tree,
            chunks_tree,
            file_replicas_tree,
            chunk_replicas_tree,
            path_index_tree,
        })
    }

    fn serialize<T: Serialize + ?Sized>(&self, value: &T) -> VDFSResult<Vec<u8>> {
        bincode::serialize(value)
            .map_err(|e| VDFSError::SerializationError(format!("Serialization failed: {}", e)))
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> VDFSResult<T> {
        bincode::deserialize(bytes)
            .map_err(|e| VDFSError::SerializationError(format!("Deserialization failed: {}", e)))
    }

    fn path_to_key(&self, path: &VirtualPath) -> Vec<u8> {
        path.to_string().into_bytes()
    }

    fn file_id_to_key(&self, file_id: &FileId) -> Vec<u8> {
        file_id.to_string().into_bytes()
    }

    fn chunk_id_to_key(&self, chunk_id: &ChunkId) -> Vec<u8> {
        hex::encode(chunk_id).into_bytes()
    }

    async fn get_file_attributes(&self, file_id: &FileId) -> VDFSResult<HashMap<String, String>> {
        let prefix = format!("{}:", file_id);
        let mut attributes = HashMap::new();

        for result in self.attributes_tree.scan_prefix(prefix.as_bytes()) {
            let (key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to scan attributes: {}", e)))?;
            
            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid key UTF8: {}", e)))?;
            
            // 提取属性名 (格式: "file_id:attr_name")
            if let Some(attr_name) = key_str.split(':').nth(1) {
                let attr_value: String = self.deserialize(&value)?;
                attributes.insert(attr_name.to_string(), attr_value);
            }
        }

        Ok(attributes)
    }

    async fn set_file_attributes(&self, file_id: &FileId, attributes: &HashMap<String, String>) -> VDFSResult<()> {
        // 删除旧属性
        let prefix = format!("{}:", file_id);
        let old_keys: Vec<_> = self.attributes_tree
            .scan_prefix(prefix.as_bytes())
            .map(|result| result.map(|(key, _)| key))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VDFSError::StorageError(format!("Failed to scan old attributes: {}", e)))?;

        for key in old_keys {
            self.attributes_tree.remove(&key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to remove old attribute: {}", e)))?;
        }

        // 添加新属性
        for (attr_name, attr_value) in attributes {
            let key = format!("{}:{}", file_id, attr_name);
            let value = self.serialize(attr_value)?;
            self.attributes_tree.insert(key.as_bytes(), value)
                .map_err(|e| VDFSError::StorageError(format!("Failed to set attribute: {}", e)))?;
        }

        Ok(())
    }

    async fn get_file_replicas(&self, file_id: &FileId) -> VDFSResult<Vec<NodeId>> {
        let key = self.file_id_to_key(file_id);
        
        if let Some(value) = self.file_replicas_tree.get(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to get file replicas: {}", e)))? {
            let replicas: Vec<NodeId> = self.deserialize(&value)?;
            Ok(replicas)
        } else {
            Ok(Vec::new())
        }
    }

    async fn set_file_replicas(&self, file_id: &FileId, replicas: &[NodeId]) -> VDFSResult<()> {
        let key = self.file_id_to_key(file_id);
        let value = self.serialize(replicas)?;
        
        self.file_replicas_tree.insert(key, value)
            .map_err(|e| VDFSError::StorageError(format!("Failed to set file replicas: {}", e)))?;
        
        Ok(())
    }

    async fn get_chunks_for_file(&self, file_id: &FileId) -> VDFSResult<Vec<ChunkMetadata>> {
        let prefix = format!("{}:", file_id);
        let mut chunks = Vec::new();

        for result in self.chunks_tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to scan chunks: {}", e)))?;
            
            let chunk: ChunkMetadata = self.deserialize(&value)?;
            chunks.push(chunk);
        }

        // // 按 chunk index 排序
        // chunks.sort_by_key(|_chunk| {
        //     // 从 chunk ID 中提取索引，这里简化处理
        //     0 // TODO: 实现正确的排序逻辑
        // });

        Ok(chunks)
    }

    async fn get_chunk_replicas(&self, chunk_id: &ChunkId) -> VDFSResult<Vec<NodeId>> {
        let key = self.chunk_id_to_key(chunk_id);
        
        if let Some(value) = self.chunk_replicas_tree.get(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to get chunk replicas: {}", e)))? {
            let replicas: Vec<NodeId> = self.deserialize(&value)?;
            Ok(replicas)
        } else {
            Ok(Vec::new())
        }
    }

    async fn set_chunk_replicas(&self, chunk_id: &ChunkId, replicas: &[NodeId]) -> VDFSResult<()> {
        let key = self.chunk_id_to_key(chunk_id);
        let value = self.serialize(replicas)?;
        
        self.chunk_replicas_tree.insert(key, value)
            .map_err(|e| VDFSError::StorageError(format!("Failed to set chunk replicas: {}", e)))?;
        
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
impl MetadataManager for SledMetadataManager {
    async fn get_file_info(&self, path: &VirtualPath) -> VDFSResult<FileInfo> {
        let key = self.path_to_key(path);
        
        if let Some(value) = self.files_tree.get(&key)
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
        
        // 序列化并存储文件信息
        let key = self.path_to_key(path);
        let value = self.serialize(&file_info)?;
        
        self.files_tree.insert(&key, value)
            .map_err(|e| VDFSError::StorageError(format!("Failed to set file info: {}", e)))?;

        // 存储文件属性
        self.set_file_attributes(&file_info.metadata.id, &file_info.metadata.custom_attributes).await?;

        // 存储文件副本
        self.set_file_replicas(&file_info.metadata.id, &file_info.replicas).await?;

        // 存储 chunks 和 chunk 副本
        for chunk in &file_info.chunks {
            let chunk_key = format!("{}:{}", file_info.metadata.id, hex::encode(&chunk.id));
            let chunk_value = self.serialize(chunk)?;
            self.chunks_tree.insert(chunk_key.as_bytes(), chunk_value)
                .map_err(|e| VDFSError::StorageError(format!("Failed to set chunk: {}", e)))?;

            // 存储 chunk 副本
            self.set_chunk_replicas(&chunk.id, &chunk.replicas).await?;
        }

        // 更新路径索引 (file_id -> path)
        let file_id_key = self.file_id_to_key(&file_info.metadata.id);
        self.path_index_tree.insert(file_id_key, key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to update path index: {}", e)))?;

        // 刷盘确保持久化
        self.db.flush_async().await
            .map_err(|e| VDFSError::StorageError(format!("Failed to flush database: {}", e)))?;

        Ok(())
    }

    async fn delete_file_info(&self, path: &VirtualPath) -> VDFSResult<()> {
        let key = self.path_to_key(path);
        
        // 获取文件信息以便删除相关数据
        if let Some(value) = self.files_tree.get(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to get file for deletion: {}", e)))? {
            
            let file_info: FileInfo = self.deserialize(&value)?;
            
            // 删除文件属性
            let attr_prefix = format!("{}:", file_info.metadata.id);
            let attr_keys: Vec<_> = self.attributes_tree
                .scan_prefix(attr_prefix.as_bytes())
                .map(|result| result.map(|(key, _)| key))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| VDFSError::StorageError(format!("Failed to scan attributes for deletion: {}", e)))?;

            for attr_key in attr_keys {
                self.attributes_tree.remove(&attr_key)
                    .map_err(|e| VDFSError::StorageError(format!("Failed to delete attribute: {}", e)))?;
            }

            // 删除文件副本
            let file_id_key = self.file_id_to_key(&file_info.metadata.id);
            self.file_replicas_tree.remove(&file_id_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to delete file replicas: {}", e)))?;

            // 删除 chunks 和 chunk 副本
            let chunk_prefix = format!("{}:", file_info.metadata.id);
            let chunk_keys: Vec<_> = self.chunks_tree
                .scan_prefix(chunk_prefix.as_bytes())
                .map(|result| result.map(|(key, _)| key))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| VDFSError::StorageError(format!("Failed to scan chunks for deletion: {}", e)))?;

            for chunk_key in chunk_keys {
                self.chunks_tree.remove(&chunk_key)
                    .map_err(|e| VDFSError::StorageError(format!("Failed to delete chunk: {}", e)))?;
            }

            // 删除所有 chunk 副本
            for chunk in &file_info.chunks {
                let chunk_replica_key = self.chunk_id_to_key(&chunk.id);
                self.chunk_replicas_tree.remove(&chunk_replica_key)
                    .map_err(|e| VDFSError::StorageError(format!("Failed to delete chunk replicas: {}", e)))?;
            }

            // 删除路径索引
            self.path_index_tree.remove(&file_id_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to delete path index: {}", e)))?;
        }

        // 删除主文件记录
        self.files_tree.remove(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to delete file: {}", e)))?;

        Ok(())
    }

    async fn file_exists(&self, path: &VirtualPath) -> VDFSResult<bool> {
        let key = self.path_to_key(path);
        let exists = self.files_tree.contains_key(&key)
            .map_err(|e| VDFSError::StorageError(format!("Failed to check file existence: {}", e)))?;
        Ok(exists)
    }

    async fn get_chunk_mapping(&self, file_id: FileId) -> VDFSResult<Vec<ChunkId>> {
        let chunks = self.get_chunks_for_file(&file_id).await?;
        Ok(chunks.into_iter().map(|chunk| chunk.id).collect())
    }

    async fn update_chunk_mapping(&self, _file_id: FileId, _chunks: Vec<ChunkId>) -> VDFSResult<()> {
        // This would be implemented through set_file_info
        Ok(())
    }

    async fn get_chunk_metadata(&self, chunk_id: ChunkId) -> VDFSResult<ChunkMetadata> {
        // 搜索包含这个 chunk 的文件
        for result in self.chunks_tree.iter() {
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
        let prefix = if path.to_string() == "/" {
            String::new()
        } else {
            format!("{}/", path.to_string())
        };

        let mut results = Vec::new();
        let mut seen_dirs = std::collections::HashSet::new();

        for result in self.files_tree.iter() {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate files: {}", e)))?;
            
            let file_path = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;

            if file_path.starts_with(&prefix) && file_path != path.to_string() {
                let relative_path = &file_path[prefix.len()..];
                
                // 检查是否是直接子项（不包含更多的 '/'）
                if let Some(slash_pos) = relative_path.find('/') {
                    // 这是一个子目录
                    let dir_name = &relative_path[..slash_pos];
                    let dir_path = if prefix.is_empty() {
                        format!("/{}", dir_name)
                    } else {
                        format!("{}{}", prefix, dir_name)
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
        let prefix = format!("{}/", path.to_string());
        
        // 收集所有需要删除的路径
        let mut paths_to_delete = Vec::new();
        
        for result in self.files_tree.iter() {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for directory removal: {}", e)))?;
            
            let file_path = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;

            if file_path == path.to_string() || file_path.starts_with(&prefix) {
                paths_to_delete.push(VirtualPath::from(file_path));
            }
        }

        // 删除所有找到的路径
        for path_to_delete in paths_to_delete {
            self.delete_file_info(&path_to_delete).await?;
        }

        Ok(())
    }

    async fn find_files_by_pattern(&self, pattern: &str) -> VDFSResult<Vec<VirtualPath>> {
        let mut results = Vec::new();
        
        for result in self.files_tree.iter() {
            let (key, _) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for pattern search: {}", e)))?;
            
            let file_path = String::from_utf8(key.to_vec())
                .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;

            if file_path.contains(pattern) {
                results.push(VirtualPath::from(file_path));
            }
        }

        Ok(results)
    }

    async fn find_files_by_size(&self, min_size: u64, max_size: u64) -> VDFSResult<Vec<VirtualPath>> {
        let mut results = Vec::new();
        
        for result in self.files_tree.iter() {
            let (key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for size search: {}", e)))?;
            
            let file_info: FileInfo = self.deserialize(&value)?;
            
            if file_info.metadata.size >= min_size && file_info.metadata.size <= max_size {
                let file_path = String::from_utf8(key.to_vec())
                    .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;
                results.push(VirtualPath::from(file_path));
            }
        }

        Ok(results)
    }

    async fn find_files_by_date(&self, start: SystemTime, end: SystemTime) -> VDFSResult<Vec<VirtualPath>> {
        let start_timestamp = Self::system_time_to_timestamp(start);
        let end_timestamp = Self::system_time_to_timestamp(end);
        let mut results = Vec::new();
        
        for result in self.files_tree.iter() {
            let (key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for date search: {}", e)))?;
            
            let file_info: FileInfo = self.deserialize(&value)?;
            let modified_timestamp = Self::system_time_to_timestamp(file_info.metadata.modified);
            
            if modified_timestamp >= start_timestamp && modified_timestamp <= end_timestamp {
                let file_path = String::from_utf8(key.to_vec())
                    .map_err(|e| VDFSError::InternalError(format!("Invalid path UTF8: {}", e)))?;
                results.push(VirtualPath::from(file_path));
            }
        }

        Ok(results)
    }

    async fn verify_consistency(&self) -> VDFSResult<Vec<VirtualPath>> {
        let mut inconsistent_files = Vec::new();

        // 检查路径索引一致性
        for result in self.path_index_tree.iter() {
            let (file_id_key, path_key) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate path index: {}", e)))?;
            
            // 检查对应的文件是否存在
            if !self.files_tree.contains_key(&path_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to check file consistency: {}", e)))? {
                
                let file_id_str = String::from_utf8(file_id_key.to_vec())
                    .unwrap_or_else(|_| "invalid_file_id".to_string());
                inconsistent_files.push(VirtualPath::new(format!("orphaned_index:{}", file_id_str)));
            }
        }

        Ok(inconsistent_files)
    }

    async fn repair_metadata(&self, _path: &VirtualPath) -> VDFSResult<()> {
        // 清理孤立的路径索引
        let mut orphaned_indices = Vec::new();
        
        for result in self.path_index_tree.iter() {
            let (file_id_key, path_key) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for repair: {}", e)))?;
            
            if !self.files_tree.contains_key(&path_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to check file for repair: {}", e)))? {
                orphaned_indices.push(file_id_key);
            }
        }

        for orphaned_key in orphaned_indices {
            self.path_index_tree.remove(&orphaned_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to remove orphaned index: {}", e)))?;
        }

        Ok(())
    }

    async fn rebuild_index(&self) -> VDFSResult<()> {
        // 清空路径索引
        self.path_index_tree.clear()
            .map_err(|e| VDFSError::StorageError(format!("Failed to clear path index: {}", e)))?;

        // 重建路径索引
        for result in self.files_tree.iter() {
            let (path_key, value) = result
                .map_err(|e| VDFSError::StorageError(format!("Failed to iterate for rebuild: {}", e)))?;
            
            let file_info: FileInfo = self.deserialize(&value)?;
            let file_id_key = self.file_id_to_key(&file_info.metadata.id);
            
            self.path_index_tree.insert(&file_id_key, &path_key)
                .map_err(|e| VDFSError::StorageError(format!("Failed to rebuild index entry: {}", e)))?;
        }

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

    fn create_test_database() -> SledMetadataManager {
        SledMetadataManager::new_temp().unwrap()
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
    async fn test_sled_database_creation() {
        let _db = create_test_database();
        // If we get here without panicking, database creation succeeded
    }

    #[tokio::test]
    async fn test_sled_file_crud_operations() {
        let db = create_test_database();
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
    async fn test_sled_directory_operations() {
        let db = create_test_database();
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
    async fn test_sled_search_operations() {
        let db = create_test_database();
        
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
    async fn test_sled_file_with_attributes() {
        let db = create_test_database();
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
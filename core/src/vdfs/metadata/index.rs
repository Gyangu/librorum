//! Index Store Implementation

use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Index store trait for key-value operations
#[async_trait]
pub trait IndexStore: Send + Sync {
    async fn get(&self, key: &str) -> VDFSResult<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> VDFSResult<()>;
    async fn delete(&self, key: &str) -> VDFSResult<()>;
    async fn list_keys(&self, prefix: &str) -> VDFSResult<Vec<String>>;
    async fn clear(&self) -> VDFSResult<()>;
}

/// In-memory index store implementation
pub struct MemoryIndexStore {
    data: RwLock<HashMap<String, String>>,
}

impl MemoryIndexStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl IndexStore for MemoryIndexStore {
    async fn get(&self, key: &str) -> VDFSResult<Option<String>> {
        let data = self.data.read().unwrap();
        Ok(data.get(key).cloned())
    }
    
    async fn set(&self, key: &str, value: &str) -> VDFSResult<()> {
        let mut data = self.data.write().unwrap();
        data.insert(key.to_string(), value.to_string());
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> VDFSResult<()> {
        let mut data = self.data.write().unwrap();
        data.remove(key);
        Ok(())
    }
    
    async fn list_keys(&self, prefix: &str) -> VDFSResult<Vec<String>> {
        let data = self.data.read().unwrap();
        let mut keys: Vec<String> = data.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        keys.sort();
        Ok(keys)
    }
    
    async fn clear(&self) -> VDFSResult<()> {
        let mut data = self.data.write().unwrap();
        data.clear();
        Ok(())
    }
}

/// Index entry for file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub file_id: FileId,
    pub size: u64,
    pub modified: u64, // SystemTime as secs since UNIX_EPOCH
    pub mime_type: Option<String>,
    pub tags: Vec<String>,
}

/// High-performance file index with multiple access patterns
pub struct FileIndex {
    /// Core index store
    store: Box<dyn IndexStore>,
    /// Path to FileId mapping
    path_index: RwLock<HashMap<VirtualPath, FileId>>,
    /// FileId to Path reverse mapping
    id_index: RwLock<HashMap<FileId, VirtualPath>>,
    /// Size-based index for range queries
    size_index: RwLock<HashMap<u64, Vec<VirtualPath>>>,
    /// Extension-based index
    extension_index: RwLock<HashMap<String, Vec<VirtualPath>>>,
}

impl FileIndex {
    pub fn new() -> Self {
        Self {
            store: Box::new(MemoryIndexStore::new()),
            path_index: RwLock::new(HashMap::new()),
            id_index: RwLock::new(HashMap::new()),
            size_index: RwLock::new(HashMap::new()),
            extension_index: RwLock::new(HashMap::new()),
        }
    }
    
    pub fn with_store(store: Box<dyn IndexStore>) -> Self {
        Self {
            store,
            path_index: RwLock::new(HashMap::new()),
            id_index: RwLock::new(HashMap::new()),
            size_index: RwLock::new(HashMap::new()),
            extension_index: RwLock::new(HashMap::new()),
        }
    }
    
    /// Add file to all indexes
    pub async fn add_file(&self, path: &VirtualPath, file_id: FileId, size: u64, mime_type: Option<String>) -> VDFSResult<()> {
        // 1. Add to path index
        {
            let mut path_idx = self.path_index.write().unwrap();
            path_idx.insert(path.clone(), file_id);
        }
        
        // 2. Add to ID index
        {
            let mut id_idx = self.id_index.write().unwrap();
            id_idx.insert(file_id, path.clone());
        }
        
        // 3. Add to size index
        {
            let mut size_idx = self.size_index.write().unwrap();
            size_idx.entry(size).or_insert_with(Vec::new).push(path.clone());
        }
        
        // 4. Add to extension index
        if let Some(ext) = self.extract_extension(path) {
            let mut ext_idx = self.extension_index.write().unwrap();
            ext_idx.entry(ext).or_insert_with(Vec::new).push(path.clone());
        }
        
        // 5. Store metadata in key-value store
        let entry = IndexEntry {
            file_id,
            size,
            modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            mime_type,
            tags: vec![],
        };
        
        let entry_json = serde_json::to_string(&entry)
            .map_err(|e| VDFSError::InternalError(format!("Serialization error: {}", e)))?;
        
        self.store.set(&format!("file:{}", path.as_str()), &entry_json).await?;
        
        Ok(())
    }
    
    /// Remove file from all indexes
    pub async fn remove_file(&self, path: &VirtualPath) -> VDFSResult<()> {
        // Get file ID first
        let file_id = {
            let path_idx = self.path_index.read().unwrap();
            path_idx.get(path).copied()
        };
        
        if let Some(file_id) = file_id {
            // 1. Remove from path index
            {
                let mut path_idx = self.path_index.write().unwrap();
                path_idx.remove(path);
            }
            
            // 2. Remove from ID index
            {
                let mut id_idx = self.id_index.write().unwrap();
                id_idx.remove(&file_id);
            }
            
            // 3. Remove from size index
            {
                let mut size_idx = self.size_index.write().unwrap();
                for paths in size_idx.values_mut() {
                    paths.retain(|p| p != path);
                }
                // Clean up empty entries
                size_idx.retain(|_, paths| !paths.is_empty());
            }
            
            // 4. Remove from extension index
            if let Some(ext) = self.extract_extension(path) {
                let mut ext_idx = self.extension_index.write().unwrap();
                if let Some(paths) = ext_idx.get_mut(&ext) {
                    paths.retain(|p| p != path);
                    if paths.is_empty() {
                        ext_idx.remove(&ext);
                    }
                }
            }
            
            // 5. Remove from store
            self.store.delete(&format!("file:{}", path.as_str())).await?;
        }
        
        Ok(())
    }
    
    /// Find file by path
    pub async fn find_file(&self, path: &VirtualPath) -> VDFSResult<Option<FileId>> {
        let path_idx = self.path_index.read().unwrap();
        Ok(path_idx.get(path).copied())
    }
    
    /// Find file by ID
    pub async fn find_path(&self, file_id: FileId) -> VDFSResult<Option<VirtualPath>> {
        let id_idx = self.id_index.read().unwrap();
        Ok(id_idx.get(&file_id).cloned())
    }
    
    /// Find files by size range
    pub async fn find_files_by_size_range(&self, min_size: u64, max_size: u64) -> VDFSResult<Vec<VirtualPath>> {
        let size_idx = self.size_index.read().unwrap();
        let mut results = Vec::new();
        
        for (&size, paths) in size_idx.iter() {
            if size >= min_size && size <= max_size {
                results.extend(paths.iter().cloned());
            }
        }
        
        Ok(results)
    }
    
    /// Find files by extension
    pub async fn find_files_by_extension(&self, extension: &str) -> VDFSResult<Vec<VirtualPath>> {
        let ext_idx = self.extension_index.read().unwrap();
        Ok(ext_idx.get(extension).cloned().unwrap_or_default())
    }
    
    /// Get file metadata from store
    pub async fn get_file_metadata(&self, path: &VirtualPath) -> VDFSResult<Option<IndexEntry>> {
        if let Some(data) = self.store.get(&format!("file:{}", path.as_str())).await? {
            let entry: IndexEntry = serde_json::from_str(&data)
                .map_err(|e| VDFSError::InternalError(format!("Deserialization error: {}", e)))?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }
    
    /// Rebuild all indexes from store
    pub async fn rebuild_indexes(&self) -> VDFSResult<()> {
        // Clear all indexes
        {
            let mut path_idx = self.path_index.write().unwrap();
            path_idx.clear();
        }
        {
            let mut id_idx = self.id_index.write().unwrap();
            id_idx.clear();
        }
        {
            let mut size_idx = self.size_index.write().unwrap();
            size_idx.clear();
        }
        {
            let mut ext_idx = self.extension_index.write().unwrap();
            ext_idx.clear();
        }
        
        // Rebuild from store
        let keys = self.store.list_keys("file:").await?;
        for key in keys {
            if let Some(path_str) = key.strip_prefix("file:") {
                let path = VirtualPath::new(path_str);
                
                if let Some(entry) = self.get_file_metadata(&path).await? {
                    // Rebuild indexes
                    self.add_file(&path, entry.file_id, entry.size, entry.mime_type).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract file extension from path
    fn extract_extension(&self, path: &VirtualPath) -> Option<String> {
        if let Some(dot_pos) = path.as_str().rfind('.') {
            let ext = &path.as_str()[dot_pos + 1..];
            if !ext.is_empty() {
                return Some(ext.to_lowercase());
            }
        }
        None
    }
}
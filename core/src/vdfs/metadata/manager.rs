//! Metadata Manager Implementation

use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId, ChunkId, NodeId};
use crate::vdfs::metadata::{MetadataManager, FileInfo, ChunkMetadata};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use regex::Regex;
use std::time::SystemTime;

/// Simple in-memory metadata manager with full functionality
pub struct SimpleMetadataManager {
    /// File information storage by path
    files: RwLock<HashMap<VirtualPath, FileInfo>>,
    /// File ID to path mapping for efficient lookups
    file_id_map: RwLock<HashMap<FileId, VirtualPath>>,
    /// Chunk metadata storage
    chunks: RwLock<HashMap<ChunkId, ChunkMetadata>>,
    /// Directory structure tracking
    directories: RwLock<HashMap<VirtualPath, Vec<VirtualPath>>>,
}

impl SimpleMetadataManager {
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
            file_id_map: RwLock::new(HashMap::new()),
            chunks: RwLock::new(HashMap::new()),
            directories: RwLock::new(HashMap::new()),
        }
    }
    
    /// Helper method to update file ID mapping
    fn update_file_id_mapping(&self, file_id: FileId, path: &VirtualPath) {
        let mut id_map = self.file_id_map.write().unwrap();
        id_map.insert(file_id, path.clone());
    }
    
    /// Helper method to remove file ID mapping
    fn remove_file_id_mapping(&self, file_id: FileId) {
        let mut id_map = self.file_id_map.write().unwrap();
        id_map.remove(&file_id);
    }
}

#[async_trait]
impl MetadataManager for SimpleMetadataManager {
    async fn get_file_info(&self, path: &VirtualPath) -> VDFSResult<FileInfo> {
        let files = self.files.read().unwrap();
        files.get(path).cloned().ok_or_else(|| {
            VDFSError::FileNotFound(path.clone())
        })
    }
    
    async fn set_file_info(&self, path: &VirtualPath, info: FileInfo) -> VDFSResult<()> {
        let mut files = self.files.write().unwrap();
        
        // Update file ID mapping
        self.update_file_id_mapping(info.metadata.id, path);
        
        // Store chunk metadata
        for chunk in &info.chunks {
            let mut chunks = self.chunks.write().unwrap();
            chunks.insert(chunk.id, chunk.clone());
        }
        
        files.insert(path.clone(), info);
        Ok(())
    }
    
    async fn delete_file_info(&self, path: &VirtualPath) -> VDFSResult<()> {
        let mut files = self.files.write().unwrap();
        
        // Clean up associated data if file exists
        if let Some(info) = files.get(path) {
            let file_id = info.metadata.id;
            
            // Remove file ID mapping
            self.remove_file_id_mapping(file_id);
            
            // Remove chunk metadata
            let mut chunks = self.chunks.write().unwrap();
            for chunk in &info.chunks {
                chunks.remove(&chunk.id);
            }
        }
        
        files.remove(path);
        Ok(())
    }
    
    async fn file_exists(&self, path: &VirtualPath) -> VDFSResult<bool> {
        let files = self.files.read().unwrap();
        Ok(files.contains_key(path))
    }
    
    async fn get_chunk_mapping(&self, file_id: FileId) -> VDFSResult<Vec<ChunkId>> {
        // Use efficient file_id to path mapping
        let id_map = self.file_id_map.read().unwrap();
        let path = id_map.get(&file_id)
            .ok_or_else(|| VDFSError::FileNotFound(VirtualPath::new(format!("file_id:{}", file_id))))?;
        
        let files = self.files.read().unwrap();
        let info = files.get(path)
            .ok_or_else(|| VDFSError::FileNotFound(path.clone()))?;
        
        Ok(info.chunks.iter().map(|c| c.id).collect())
    }
    
    async fn update_chunk_mapping(&self, file_id: FileId, chunk_ids: Vec<ChunkId>) -> VDFSResult<()> {
        // Find the file and update its chunk mapping
        let id_map = self.file_id_map.read().unwrap();
        let path = id_map.get(&file_id)
            .ok_or_else(|| VDFSError::FileNotFound(VirtualPath::new(format!("file_id:{}", file_id))))?
            .clone();
        drop(id_map);
        
        let mut files = self.files.write().unwrap();
        if let Some(info) = files.get_mut(&path) {
            // Create new chunk metadata for missing chunks
            let chunks = self.chunks.read().unwrap();
            info.chunks = chunk_ids.iter().map(|&chunk_id| {
                chunks.get(&chunk_id).cloned().unwrap_or_else(|| {
                    // Create default chunk metadata if not found
                    ChunkMetadata {
                        id: chunk_id,
                        size: 0,
                        checksum: String::new(),
                        compressed: false,
                        replicas: vec![],
                        access_count: 0,
                        last_accessed: SystemTime::now(),
                    }
                })
            }).collect();
        }
        
        Ok(())
    }
    
    async fn get_chunk_metadata(&self, chunk_id: ChunkId) -> VDFSResult<ChunkMetadata> {
        let chunks = self.chunks.read().unwrap();
        chunks.get(&chunk_id).cloned()
            .ok_or_else(|| VDFSError::InternalError(format!("Chunk not found: {:?}", chunk_id)))
    }
    
    async fn update_chunk_metadata(&self, chunk_id: ChunkId, metadata: ChunkMetadata) -> VDFSResult<()> {
        let mut chunks = self.chunks.write().unwrap();
        chunks.insert(chunk_id, metadata);
        Ok(())
    }
    
    async fn list_directory(&self, path: &VirtualPath) -> VDFSResult<Vec<VirtualPath>> {
        let files = self.files.read().unwrap();
        let mut results = Vec::new();
        
        for file_path in files.keys() {
            if let Some(parent) = file_path.parent() {
                if parent == *path {
                    results.push(file_path.clone());
                }
            }
        }
        
        Ok(results)
    }
    
    async fn create_directory(&self, path: &VirtualPath) -> VDFSResult<()> {
        let mut directories = self.directories.write().unwrap();
        
        // Initialize directory entry if it doesn't exist
        if !directories.contains_key(path) {
            directories.insert(path.clone(), Vec::new());
        }
        
        // Update parent directory to include this directory
        if let Some(parent) = path.parent() {
            directories.entry(parent)
                .or_insert_with(Vec::new)
                .push(path.clone());
        }
        
        Ok(())
    }
    
    async fn remove_directory(&self, path: &VirtualPath) -> VDFSResult<()> {
        let mut directories = self.directories.write().unwrap();
        
        // Check if directory is empty
        if let Some(children) = directories.get(path) {
            if !children.is_empty() {
                return Err(VDFSError::InvalidPath(format!("Directory not empty: {}", path.as_str())));
            }
        }
        
        // Remove from parent directory
        if let Some(parent) = path.parent() {
            if let Some(parent_children) = directories.get_mut(&parent) {
                parent_children.retain(|child| child != path);
            }
        }
        
        // Remove the directory itself
        directories.remove(path);
        
        Ok(())
    }
    
    async fn find_files_by_pattern(&self, pattern: &str) -> VDFSResult<Vec<VirtualPath>> {
        let regex = Regex::new(pattern)
            .map_err(|e| VDFSError::InvalidPath(format!("Invalid regex pattern: {}", e)))?;
        
        let files = self.files.read().unwrap();
        let mut results = Vec::new();
        
        for path in files.keys() {
            if regex.is_match(path.as_str()) {
                results.push(path.clone());
            }
        }
        
        Ok(results)
    }
    
    async fn find_files_by_size(&self, min_size: u64, max_size: u64) -> VDFSResult<Vec<VirtualPath>> {
        let files = self.files.read().unwrap();
        let mut results = Vec::new();
        
        for (path, info) in files.iter() {
            let file_size = info.metadata.size;
            if file_size >= min_size && file_size <= max_size {
                results.push(path.clone());
            }
        }
        
        Ok(results)
    }
    
    async fn find_files_by_date(&self, start: SystemTime, end: SystemTime) -> VDFSResult<Vec<VirtualPath>> {
        let files = self.files.read().unwrap();
        let mut results = Vec::new();
        
        for (path, info) in files.iter() {
            let modified_time = info.metadata.modified;
            if modified_time >= start && modified_time <= end {
                results.push(path.clone());
            }
        }
        
        Ok(results)
    }
    
    async fn verify_consistency(&self) -> VDFSResult<Vec<VirtualPath>> {
        let mut inconsistent_files = Vec::new();
        
        let files = self.files.read().unwrap();
        let chunks = self.chunks.read().unwrap();
        let id_map = self.file_id_map.read().unwrap();
        
        // Check 1: Verify file ID mappings are consistent
        for (path, info) in files.iter() {
            if let Some(mapped_path) = id_map.get(&info.metadata.id) {
                if mapped_path != path {
                    inconsistent_files.push(path.clone());
                }
            } else {
                // File ID mapping is missing
                inconsistent_files.push(path.clone());
            }
        }
        
        // Check 2: Verify chunk metadata consistency
        for (path, info) in files.iter() {
            for chunk_meta in &info.chunks {
                if let Some(stored_chunk) = chunks.get(&chunk_meta.id) {
                    // Verify chunk metadata consistency
                    if stored_chunk.size != chunk_meta.size || 
                       stored_chunk.checksum != chunk_meta.checksum {
                        inconsistent_files.push(path.clone());
                        break;
                    }
                } else {
                    // Chunk metadata is missing
                    inconsistent_files.push(path.clone());
                    break;
                }
            }
        }
        
        // Check 3: Verify orphaned chunk metadata
        for chunk_id in chunks.keys() {
            let mut found = false;
            for info in files.values() {
                if info.chunks.iter().any(|c| c.id == *chunk_id) {
                    found = true;
                    break;
                }
            }
            if !found {
                // Orphaned chunk metadata - this is a system-level inconsistency
                // We'll report it by adding a special marker
                inconsistent_files.push(VirtualPath::new(format!("orphaned_chunk:{:?}", chunk_id)));
            }
        }
        
        // Remove duplicates
        inconsistent_files.sort();
        inconsistent_files.dedup();
        
        Ok(inconsistent_files)
    }
    
    async fn repair_metadata(&self, path: &VirtualPath) -> VDFSResult<()> {
        // Handle special case of orphaned chunks
        if path.as_str().starts_with("orphaned_chunk:") {
            let chunk_id_str = path.as_str().strip_prefix("orphaned_chunk:").unwrap();
            // Parse chunk ID from debug format (simplified)
            let mut chunks = self.chunks.write().unwrap();
            chunks.retain(|id, _| !format!("{:?}", id).contains(chunk_id_str));
            return Ok(());
        }
        
        let files = self.files.read().unwrap();
        if let Some(info) = files.get(path) {
            let file_id = info.metadata.id;
            
            // Repair file ID mapping
            self.update_file_id_mapping(file_id, path);
            
            // Repair chunk metadata
            let mut chunks = self.chunks.write().unwrap();
            for chunk in &info.chunks {
                chunks.insert(chunk.id, chunk.clone());
            }
        }
        
        Ok(())
    }
    
    async fn rebuild_index(&self) -> VDFSResult<()> {
        // Rebuild file ID mapping
        {
            let mut id_map = self.file_id_map.write().unwrap();
            id_map.clear();
            
            let files = self.files.read().unwrap();
            for (path, info) in files.iter() {
                id_map.insert(info.metadata.id, path.clone());
            }
        }
        
        // Rebuild chunk metadata index
        {
            let mut chunks = self.chunks.write().unwrap();
            chunks.clear();
            
            let files = self.files.read().unwrap();
            for info in files.values() {
                for chunk in &info.chunks {
                    chunks.insert(chunk.id, chunk.clone());
                }
            }
        }
        
        // Rebuild directory structure
        {
            let mut directories = self.directories.write().unwrap();
            directories.clear();
            
            let files = self.files.read().unwrap();
            for path in files.keys() {
                if let Some(parent) = path.parent() {
                    directories.entry(parent)
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                }
            }
        }
        
        Ok(())
    }
}
//! Virtual File System Implementation

use crate::vdfs::{VDFSResult, VDFSError, Chunk, ChunkId};
use crate::vdfs::filesystem::{VirtualFileSystem, FileHandle, FileMetadata};
use crate::vdfs::storage::{StorageBackend, DefaultChunkManager};
use crate::vdfs::metadata::{MetadataManager, FileInfo, ChunkMetadata};
use crate::vdfs::{VirtualPath, OpenMode, DirEntry, FileId};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use uuid::Uuid;

/// Virtual File System implementation with storage and metadata integration
pub struct VirtualFileSystemImpl {
    storage: Arc<dyn StorageBackend>,
    metadata: Arc<dyn MetadataManager>,
    chunk_manager: DefaultChunkManager,
    /// In-memory file table for quick lookups
    open_files: Arc<RwLock<HashMap<FileId, Arc<RwLock<FileHandle>>>>>,
    /// Directory structure cache
    directory_cache: Arc<RwLock<HashMap<VirtualPath, Vec<DirEntry>>>>,
}

impl VirtualFileSystemImpl {
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        metadata: Arc<dyn MetadataManager>,
        chunk_size: usize,
    ) -> Self {
        Self {
            storage,
            metadata,
            chunk_manager: DefaultChunkManager::new(chunk_size, false),
            open_files: Arc::new(RwLock::new(HashMap::new())),
            directory_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Helper method to split file data into chunks and store them
    #[allow(dead_code)]
    async fn store_file_data(&self, _file_id: FileId, data: &[u8]) -> VDFSResult<Vec<ChunkId>> {
        let chunks = self.chunk_manager.split_file(data)?;
        let mut chunk_ids = Vec::new();
        
        for chunk in chunks {
            let chunk_id = chunk.id;
            self.storage.store_chunk(chunk_id, &chunk.data).await?;
            chunk_ids.push(chunk_id);
        }
        
        Ok(chunk_ids)
    }
    
    /// Helper method to retrieve and reassemble file data from chunks
    #[allow(dead_code)]
    async fn retrieve_file_data(&self, chunk_ids: &[ChunkId]) -> VDFSResult<Vec<u8>> {
        let mut chunks = Vec::new();
        
        for &chunk_id in chunk_ids {
            let data = self.storage.retrieve_chunk(chunk_id).await?;
            let chunk = Chunk::new(data);
            
            // Verify chunk integrity
            if chunk.id != chunk_id {
                return Err(VDFSError::CorruptedData(hex::encode(chunk_id)));
            }
            
            chunks.push(chunk);
        }
        
        self.chunk_manager.reassemble_file(chunks)
    }
    
    /// Update directory cache when files are added/removed
    fn update_directory_cache(&self, path: &VirtualPath, entry: Option<DirEntry>) {
        let mut cache = self.directory_cache.write().unwrap();
        
        if let Some(parent_path) = path.parent() {
            let dir_entries = cache.entry(parent_path).or_insert_with(Vec::new);
            
            if let Some(new_entry) = entry {
                // Add or update entry
                if let Some(existing_idx) = dir_entries.iter().position(|e| e.path == *path) {
                    dir_entries[existing_idx] = new_entry;
                } else {
                    dir_entries.push(new_entry);
                }
            } else {
                // Remove entry
                dir_entries.retain(|e| e.path != *path);
            }
        }
    }
    
    /// Validate path format
    fn validate_path(&self, path: &VirtualPath) -> VDFSResult<()> {
        let path_str = path.as_str();
        
        if path_str.is_empty() {
            return Err(VDFSError::InvalidPath("Empty path".to_string()));
        }
        
        if !path_str.starts_with('/') {
            return Err(VDFSError::InvalidPath("Path must be absolute".to_string()));
        }
        
        // Check for invalid characters
        if path_str.contains('\0') {
            return Err(VDFSError::InvalidPath("Path contains null character".to_string()));
        }
        
        Ok(())
    }
}

#[async_trait]
impl VirtualFileSystem for VirtualFileSystemImpl {
    async fn create_file(&self, path: &VirtualPath) -> VDFSResult<FileHandle> {
        self.validate_path(path)?;
        
        // Check if file already exists
        if self.metadata.file_exists(path).await? {
            return Err(VDFSError::FileAlreadyExists(path.clone()));
        }
        
        // Create file metadata
        let metadata = FileMetadata::new_file(path.clone());
        let file_id = metadata.id;
        
        // Create file info with empty chunks
        let file_info = FileInfo {
            metadata: metadata.clone(),
            chunks: Vec::new(),
            replicas: Vec::new(),
            version: 1,
            checksum: String::new(),
        };
        
        // Store metadata
        self.metadata.set_file_info(path, file_info).await?;
        
        // Create file handle with backends
        let storage_weak = Arc::downgrade(&self.storage);
        let metadata_weak = Arc::downgrade(&self.metadata);
        let handle = FileHandle::new(file_id, path.clone(), OpenMode::Create, metadata.clone())
            .with_backends(storage_weak, metadata_weak, self.chunk_manager.clone());
        let handle_arc = Arc::new(RwLock::new(handle.clone()));
        
        // Track open file
        {
            let mut open_files = self.open_files.write().unwrap();
            open_files.insert(file_id, handle_arc);
        }
        
        // Update directory cache
        let dir_entry = DirEntry {
            name: path.file_name().unwrap_or("").to_string(),
            path: path.clone(),
            is_dir: false,
            size: 0,
            modified: metadata.modified,
        };
        self.update_directory_cache(path, Some(dir_entry));
        
        Ok(handle)
    }
    
    async fn open_file(&self, path: &VirtualPath, mode: OpenMode) -> VDFSResult<FileHandle> {
        self.validate_path(path)?;
        
        // Check if file exists
        let file_info = self.metadata.get_file_info(path).await?;
        let mut metadata = file_info.metadata;
        
        // Update access time
        metadata.update_accessed();
        
        // Create file handle with backends
        let storage_weak = Arc::downgrade(&self.storage);
        let metadata_weak = Arc::downgrade(&self.metadata);
        let handle = FileHandle::new(metadata.id, path.clone(), mode, metadata)
            .with_backends(storage_weak, metadata_weak, self.chunk_manager.clone());
        let handle_arc = Arc::new(RwLock::new(handle.clone()));
        
        // Track open file
        {
            let mut open_files = self.open_files.write().unwrap();
            open_files.insert(handle.id, handle_arc);
        }
        
        Ok(handle)
    }
    
    async fn delete_file(&self, path: &VirtualPath) -> VDFSResult<()> {
        self.validate_path(path)?;
        
        // Get file info to find chunks
        let file_info = self.metadata.get_file_info(path).await?;
        let chunk_ids: Vec<ChunkId> = file_info.chunks.iter().map(|c| c.id).collect();
        
        // Delete chunks from storage
        if !chunk_ids.is_empty() {
            self.storage.delete_chunks(chunk_ids).await?;
        }
        
        // Remove from metadata
        self.metadata.delete_file_info(path).await?;
        
        // Remove from open files if present
        {
            let mut open_files = self.open_files.write().unwrap();
            open_files.remove(&file_info.metadata.id);
        }
        
        // Update directory cache
        self.update_directory_cache(path, None);
        
        Ok(())
    }
    
    async fn move_file(&self, from: &VirtualPath, to: &VirtualPath) -> VDFSResult<()> {
        self.validate_path(from)?;
        self.validate_path(to)?;
        
        // Check if destination already exists
        if self.metadata.file_exists(to).await? {
            return Err(VDFSError::FileAlreadyExists(to.clone()));
        }
        
        // Ensure destination directory exists
        if let Some(parent_path) = to.parent() {
            if !self.metadata.file_exists(&parent_path).await? {
                self.create_dir(&parent_path).await?;
            }
        }
        
        // Get source file info
        let mut file_info = self.metadata.get_file_info(from).await?;
        let is_directory = file_info.metadata.is_directory;
        
        // If moving a directory, recursively move all contents first
        if is_directory {
            let entries = self.list_dir(from).await?;
            for entry in entries {
                let old_path = &entry.path;
                let relative_path = old_path.as_str().strip_prefix(from.as_str()).unwrap_or("");
                let new_path = if relative_path.is_empty() {
                    to.clone()
                } else {
                    VirtualPath::new(format!("{}{}", to.as_str(), relative_path))
                };
                
                // Recursively move each item
                self.move_file(old_path, &new_path).await?;
            }
        }
        
        // Store metadata values we need before moving file_info
        let file_size = file_info.metadata.size;
        
        // Update path in metadata
        file_info.metadata.path = to.clone();
        file_info.metadata.update_modified();
        file_info.version += 1;
        let modified_time = file_info.metadata.modified;
        
        // Store under new path
        self.metadata.set_file_info(to, file_info).await?;
        
        // Remove old path
        self.metadata.delete_file_info(from).await?;
        
        // Update directory cache
        self.update_directory_cache(from, None);
        let dir_entry = DirEntry {
            name: to.file_name().unwrap_or("").to_string(),
            path: to.clone(),
            is_dir: is_directory,
            size: file_size,
            modified: modified_time,
        };
        self.update_directory_cache(to, Some(dir_entry));
        
        Ok(())
    }
    
    async fn copy_file(&self, from: &VirtualPath, to: &VirtualPath) -> VDFSResult<()> {
        self.validate_path(from)?;
        self.validate_path(to)?;
        
        // Check if destination already exists
        if self.metadata.file_exists(to).await? {
            return Err(VDFSError::FileAlreadyExists(to.clone()));
        }
        
        // Get source file info
        let source_file_info = self.metadata.get_file_info(from).await?;
        
        // Create new metadata for destination
        let mut dest_metadata = source_file_info.metadata.clone();
        dest_metadata.id = Uuid::new_v4();
        dest_metadata.path = to.clone();
        dest_metadata.created = SystemTime::now();
        dest_metadata.modified = SystemTime::now();
        dest_metadata.accessed = SystemTime::now();
        
        // Copy chunk data (we could optimize this with reference counting later)
        let mut new_chunks = Vec::new();
        for chunk_metadata in &source_file_info.chunks {
            let chunk_data = self.storage.retrieve_chunk(chunk_metadata.id).await?;
            let new_chunk = Chunk::new(chunk_data);
            self.storage.store_chunk(new_chunk.id, &new_chunk.data).await?;
            
            let new_chunk_metadata = ChunkMetadata {
                id: new_chunk.id,
                size: new_chunk.size,
                checksum: new_chunk.checksum,
                compressed: new_chunk.compressed,
                replicas: chunk_metadata.replicas.clone(),
                access_count: 0,
                last_accessed: SystemTime::now(),
            };
            new_chunks.push(new_chunk_metadata);
        }
        
        // Create destination file info
        let dest_file_info = FileInfo {
            metadata: dest_metadata.clone(),
            chunks: new_chunks,
            replicas: source_file_info.replicas.clone(),
            version: 1,
            checksum: source_file_info.checksum.clone(),
        };
        
        // Store destination metadata
        self.metadata.set_file_info(to, dest_file_info).await?;
        
        // Update directory cache
        let dir_entry = DirEntry {
            name: to.file_name().unwrap_or("").to_string(),
            path: to.clone(),
            is_dir: false,
            size: dest_metadata.size,
            modified: dest_metadata.modified,
        };
        self.update_directory_cache(to, Some(dir_entry));
        
        Ok(())
    }
    
    async fn create_dir(&self, path: &VirtualPath) -> VDFSResult<()> {
        self.validate_path(path)?;
        
        // Check if already exists
        if self.metadata.file_exists(path).await? {
            return Err(VDFSError::FileAlreadyExists(path.clone()));
        }
        
        // Create directory metadata
        let metadata = FileMetadata::new_directory(path.clone());
        
        let file_info = FileInfo {
            metadata: metadata.clone(),
            chunks: Vec::new(),
            replicas: Vec::new(),
            version: 1,
            checksum: String::new(),
        };
        
        // Store metadata
        self.metadata.set_file_info(path, file_info).await?;
        
        // Initialize empty directory cache entry
        {
            let mut cache = self.directory_cache.write().unwrap();
            cache.insert(path.clone(), Vec::new());
        }
        
        // Update parent directory cache
        let dir_entry = DirEntry {
            name: path.file_name().unwrap_or("").to_string(),
            path: path.clone(),
            is_dir: true,
            size: 0,
            modified: metadata.modified,
        };
        self.update_directory_cache(path, Some(dir_entry));
        
        Ok(())
    }
    
    async fn list_dir(&self, path: &VirtualPath) -> VDFSResult<Vec<DirEntry>> {
        self.validate_path(path)?;
        
        // Check if directory exists
        let file_info = self.metadata.get_file_info(path).await?;
        if !file_info.metadata.is_directory {
            return Err(VDFSError::InvalidPath("Not a directory".to_string()));
        }
        
        // Try cache first
        {
            let cache = self.directory_cache.read().unwrap();
            if let Some(entries) = cache.get(path) {
                return Ok(entries.clone());
            }
        }
        
        // Fallback to metadata listing
        let child_paths = self.metadata.list_directory(path).await?;
        let mut entries = Vec::new();
        
        for child_path in child_paths {
            if let Ok(child_info) = self.metadata.get_file_info(&child_path).await {
                let entry = DirEntry {
                    name: child_path.file_name().unwrap_or("").to_string(),
                    path: child_path,
                    is_dir: child_info.metadata.is_directory,
                    size: child_info.metadata.size,
                    modified: child_info.metadata.modified,
                };
                entries.push(entry);
            }
        }
        
        // Update cache
        {
            let mut cache = self.directory_cache.write().unwrap();
            cache.insert(path.clone(), entries.clone());
        }
        
        Ok(entries)
    }
    
    async fn remove_dir(&self, path: &VirtualPath) -> VDFSResult<()> {
        self.validate_path(path)?;
        
        // Check if directory exists and is empty
        let entries = self.list_dir(path).await?;
        if !entries.is_empty() {
            return Err(VDFSError::PermissionDenied("Directory not empty".to_string()));
        }
        
        // Remove from metadata
        self.metadata.delete_file_info(path).await?;
        
        // Remove from cache
        {
            let mut cache = self.directory_cache.write().unwrap();
            cache.remove(path);
        }
        
        // Update parent directory cache
        self.update_directory_cache(path, None);
        
        Ok(())
    }
    
    async fn remove_dir_all(&self, path: &VirtualPath) -> VDFSResult<()> {
        self.validate_path(path)?;
        
        // Recursively remove all children
        let entries = self.list_dir(path).await?;
        for entry in entries {
            if entry.is_dir {
                self.remove_dir_all(&entry.path).await?;
            } else {
                self.delete_file(&entry.path).await?;
            }
        }
        
        // Remove the directory itself
        self.remove_dir(path).await?;
        
        Ok(())
    }
    
    async fn get_metadata(&self, path: &VirtualPath) -> VDFSResult<FileMetadata> {
        self.validate_path(path)?;
        
        let file_info = self.metadata.get_file_info(path).await?;
        Ok(file_info.metadata)
    }
    
    async fn set_metadata(&self, path: &VirtualPath, mut metadata: FileMetadata) -> VDFSResult<()> {
        self.validate_path(path)?;
        
        // Get current file info
        let mut file_info = self.metadata.get_file_info(path).await?;
        
        // Update metadata but preserve internal fields
        metadata.id = file_info.metadata.id;
        metadata.path = path.clone();
        metadata.modified = SystemTime::now();
        
        file_info.metadata = metadata;
        file_info.version += 1;
        
        // Store updated info
        self.metadata.set_file_info(path, file_info).await?;
        
        Ok(())
    }
    
    async fn exists(&self, path: &VirtualPath) -> VDFSResult<bool> {
        if let Err(_) = self.validate_path(path) {
            return Ok(false);
        }
        
        self.metadata.file_exists(path).await
    }
    
    async fn canonicalize(&self, path: &VirtualPath) -> VDFSResult<VirtualPath> {
        self.validate_path(path)?;
        
        // Simple canonicalization - resolve .. and . components
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        let mut canonical_parts = Vec::new();
        
        for part in parts {
            match part {
                "." => {
                    // Current directory, skip
                }
                ".." => {
                    // Parent directory
                    canonical_parts.pop();
                }
                _ => {
                    canonical_parts.push(part);
                }
            }
        }
        
        let canonical_path = if canonical_parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", canonical_parts.join("/"))
        };
        
        Ok(VirtualPath::new(canonical_path))
    }
    
    async fn resolve_link(&self, path: &VirtualPath) -> VDFSResult<VirtualPath> {
        // For now, just return the path as-is since we don't support symlinks yet
        self.validate_path(path)?;
        Ok(path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdfs::storage::LocalStorageBackend;
    use tempfile::TempDir;
    
    async fn create_test_vfs() -> VirtualFileSystemImpl {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(LocalStorageBackend::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
        ).unwrap());
        let metadata = Arc::new(crate::vdfs::metadata::SimpleMetadataManager::new());
        
        VirtualFileSystemImpl::new(storage, metadata, 1024)
    }
    
    #[tokio::test]
    async fn test_file_lifecycle() {
        let vfs = create_test_vfs().await;
        let path = VirtualPath::new("/test_file.txt");
        
        // File should not exist initially
        assert!(!vfs.exists(&path).await.unwrap());
        
        // Create file
        let _handle = vfs.create_file(&path).await.unwrap();
        assert!(vfs.exists(&path).await.unwrap());
        
        // Get metadata
        let metadata = vfs.get_metadata(&path).await.unwrap();
        assert_eq!(metadata.path, path);
        assert!(!metadata.is_directory);
        
        // Delete file
        vfs.delete_file(&path).await.unwrap();
        assert!(!vfs.exists(&path).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_directory_operations() {
        let vfs = create_test_vfs().await;
        let dir_path = VirtualPath::new("/test_dir");
        let file_path = VirtualPath::new("/test_dir/file.txt");
        
        // Create directory
        vfs.create_dir(&dir_path).await.unwrap();
        assert!(vfs.exists(&dir_path).await.unwrap());
        
        // Create file in directory
        let _handle = vfs.create_file(&file_path).await.unwrap();
        
        // List directory
        let entries = vfs.list_dir(&dir_path).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file.txt");
        assert!(!entries[0].is_dir);
        
        // Try to remove non-empty directory (should fail)
        assert!(vfs.remove_dir(&dir_path).await.is_err());
        
        // Remove file first
        vfs.delete_file(&file_path).await.unwrap();
        
        // Now remove directory
        vfs.remove_dir(&dir_path).await.unwrap();
        assert!(!vfs.exists(&dir_path).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_file_operations() {
        let vfs = create_test_vfs().await;
        let source_path = VirtualPath::new("/source.txt");
        let dest_path = VirtualPath::new("/dest.txt");
        let moved_path = VirtualPath::new("/moved.txt");
        
        // Create source file
        let _handle = vfs.create_file(&source_path).await.unwrap();
        
        // Copy file
        vfs.copy_file(&source_path, &dest_path).await.unwrap();
        assert!(vfs.exists(&dest_path).await.unwrap());
        assert!(vfs.exists(&source_path).await.unwrap()); // Original should still exist
        
        // Move file
        vfs.move_file(&source_path, &moved_path).await.unwrap();
        assert!(vfs.exists(&moved_path).await.unwrap());
        assert!(!vfs.exists(&source_path).await.unwrap()); // Original should be gone
    }
    
    #[tokio::test]
    async fn test_path_canonicalization() {
        let vfs = create_test_vfs().await;
        
        let complex_path = VirtualPath::new("/dir1/../dir2/./file.txt");
        let canonical = vfs.canonicalize(&complex_path).await.unwrap();
        assert_eq!(canonical.as_str(), "/dir2/file.txt");
        
        let root_path = VirtualPath::new("/dir/..");
        let canonical_root = vfs.canonicalize(&root_path).await.unwrap();
        assert_eq!(canonical_root.as_str(), "/");
    }
}
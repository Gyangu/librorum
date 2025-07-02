//! VDFS (Virtual Distributed File System) - Core Module
//! 
//! A high-performance distributed file system with virtual file abstraction,
//! chunk-based storage, metadata management, and distributed caching.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub mod filesystem;
pub mod storage;
pub mod metadata;
pub mod cache;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod comprehensive_tests;

#[cfg(test)]
mod integration_test;

// Re-export core types for convenience
pub use filesystem::{VirtualFileSystem, FileHandle, FileOperations, FileMetadata, VirtualFileSystemImpl};
pub use storage::{StorageBackend, ChunkManager, LocalStorageBackend, DefaultChunkManager};
pub use metadata::{MetadataManager, DefaultMetadataManager, SledMetadataManager, SimpleMetadataManager};
pub use cache::{CacheManager, MemoryCache, DiskCache, CachePolicy};

/// Result type for VDFS operations
pub type VDFSResult<T> = Result<T, VDFSError>;

/// Unique identifier for files
pub type FileId = Uuid;

/// Unique identifier for data chunks (SHA-256 hash)
pub type ChunkId = [u8; 32];

/// Node identifier in the distributed system
pub type NodeId = String;

/// Virtual path representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VirtualPath(String);

impl VirtualPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    pub fn parent(&self) -> Option<VirtualPath> {
        if let Some(parent) = std::path::Path::new(&self.0).parent() {
            Some(VirtualPath(parent.to_string_lossy().to_string()))
        } else {
            None
        }
    }
    
    pub fn join(&self, segment: &str) -> VirtualPath {
        let mut path = self.0.clone();
        if !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(segment);
        VirtualPath(path)
    }
    
    pub fn file_name(&self) -> Option<&str> {
        std::path::Path::new(&self.0)
            .file_name()
            .and_then(|name| name.to_str())
    }
}

impl fmt::Display for VirtualPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for VirtualPath {
    fn from(path: String) -> Self {
        Self(path)
    }
}

impl From<&str> for VirtualPath {
    fn from(path: &str) -> Self {
        Self(path.to_string())
    }
}

/// Data chunk representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub data: Vec<u8>,
    pub checksum: String,
    pub size: usize,
    pub compressed: bool,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Chunk {
    pub fn new(data: Vec<u8>) -> Self {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        
        let mut id = [0u8; 32];
        id.copy_from_slice(&hash);
        
        let checksum = hex::encode(&hash);
        let size = data.len();
        
        Self {
            id,
            data,
            checksum,
            size,
            compressed: false,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn verify_integrity(&self) -> bool {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        let computed_hash = hex::encode(hasher.finalize());
        
        self.checksum == computed_hash
    }
}

/// Chunk information without data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub id: ChunkId,
    pub size: usize,
    pub checksum: String,
    pub compressed: bool,
    pub replicas: Vec<NodeId>,
}

/// File permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_execute: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_execute: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_execute: bool,
}

impl Default for FilePermissions {
    fn default() -> Self {
        Self {
            owner_read: true,
            owner_write: true,
            owner_execute: false,
            group_read: true,
            group_write: false,
            group_execute: false,
            other_read: true,
            other_write: false,
            other_execute: false,
        }
    }
}

/// File open modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Read,
    Write,
    ReadWrite,
    Append,
    Create,
    CreateNew,
}

/// Directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: VirtualPath,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
}

/// Storage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub total_space: u64,
    pub used_space: u64,
    pub available_space: u64,
    pub chunk_count: usize,
    pub node_id: NodeId,
}

/// Cache key for caching operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheKey {
    FileMetadata(VirtualPath),
    FileData(FileId),
    ChunkData(ChunkId),
    DirectoryListing(VirtualPath),
}

/// Cache value for caching operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheValue {
    FileMetadata(FileMetadata),
    FileData(Vec<u8>),
    ChunkData(Vec<u8>),
    DirectoryListing(Vec<DirEntry>),
}

/// VDFS Error types
#[derive(Debug, thiserror::Error)]
pub enum VDFSError {
    #[error("File not found: {0}")]
    FileNotFound(VirtualPath),
    
    #[error("Directory not found: {0}")]
    DirectoryNotFound(VirtualPath),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("File already exists: {0}")]
    FileAlreadyExists(VirtualPath),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Corrupted data in chunk: {0}")]
    CorruptedData(String),
    
    #[error("Insufficient storage space")]
    InsufficientSpace,
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Configuration for VDFS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VDFSConfig {
    /// Root directory for local storage
    pub storage_path: PathBuf,
    
    /// Default chunk size in bytes
    pub chunk_size: usize,
    
    /// Enable compression
    pub enable_compression: bool,
    
    /// Memory cache size in bytes
    pub cache_memory_size: usize,
    
    /// Disk cache size in bytes  
    pub cache_disk_size: usize,
    
    /// Replication factor
    pub replication_factor: usize,
    
    /// Network timeout
    pub network_timeout: Duration,
}

impl Default for VDFSConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./vdfs_storage"),
            chunk_size: 8 * 1024 * 1024, // 8MB - 更大的chunk减少协议开销
            enable_compression: false,
            cache_memory_size: 64 * 1024 * 1024, // 64MB
            cache_disk_size: 512 * 1024 * 1024, // 512MB
            replication_factor: 3,
            network_timeout: Duration::from_secs(30),
        }
    }
}


/// Main VDFS instance
pub struct VDFS {
    config: VDFSConfig,
    filesystem: Box<dyn VirtualFileSystem>,
    storage: Box<dyn StorageBackend>,
    metadata: Box<dyn MetadataManager>,
    cache: CacheManager,
}

impl VDFS {
    /// Create a new VDFS instance with default components
    pub async fn new(config: VDFSConfig) -> VDFSResult<Self> {
        // Create storage backend
        let node_id = format!("node-{}", uuid::Uuid::new_v4());
        let storage_for_fs = Arc::new(LocalStorageBackend::new(config.storage_path.clone(), node_id.clone())?);
        let storage_for_vdfs = LocalStorageBackend::new(config.storage_path.clone(), node_id)?;
        
        // Create metadata manager (using simple in-memory manager to avoid Sled compression issues)
        let metadata_for_fs = Arc::new(SimpleMetadataManager::new());
        let metadata_for_vdfs = SimpleMetadataManager::new();
        
        // Create cache components (simplified to avoid dependency conflicts)
        let memory_config = cache::memory_cache::MemoryCacheConfig {
            max_memory: config.cache_memory_size,
            ..Default::default()
        };
        let memory_cache = MemoryCache::new(memory_config);
        
        // Skip disk cache for now to avoid compression dependency conflicts
        let cache_policy = CachePolicy::default();
        
        // Create cache manager with memory cache only
        let cache = CacheManager::new_memory_only(
            memory_cache,
            cache_policy,
        ).await?;
        
        // Create virtual file system
        let filesystem = Box::new(VirtualFileSystemImpl::new(
            storage_for_fs as Arc<dyn StorageBackend>,
            metadata_for_fs as Arc<dyn MetadataManager>,
            config.chunk_size,
        ));
        
        Ok(Self {
            config,
            filesystem,
            storage: Box::new(storage_for_vdfs),
            metadata: Box::new(metadata_for_vdfs),
            cache,
        })
    }
    
    /// Create a new VDFS instance with custom components
    pub fn with_components(
        config: VDFSConfig,
        filesystem: Box<dyn VirtualFileSystem>,
        storage: Box<dyn StorageBackend>, 
        metadata: Box<dyn MetadataManager>,
        cache: CacheManager,
    ) -> Self {
        Self {
            config,
            filesystem,
            storage,
            metadata,
            cache,
        }
    }
    
    /// Mount the file system
    pub async fn mount(&self) -> VDFSResult<()> {
        // Initialize storage backend
        // Start background services
        // Recovery if needed
        Ok(())
    }
    
    /// Unmount the file system
    pub async fn unmount(&self) -> VDFSResult<()> {
        // Flush caches
        // Stop background services
        // Cleanup resources
        Ok(())
    }
    
    /// Get file system statistics
    pub async fn stats(&self) -> VDFSResult<StorageInfo> {
        self.storage.get_storage_info().await
    }
}

// Convenience functions for common operations
impl VDFS {
    /// Create a new file
    pub async fn create_file(&self, path: &str) -> VDFSResult<FileHandle> {
        let vpath = VirtualPath::new(path);
        self.filesystem.create_file(&vpath).await
    }
    
    /// Open an existing file
    pub async fn open_file(&self, path: &str, mode: OpenMode) -> VDFSResult<FileHandle> {
        let vpath = VirtualPath::new(path);
        self.filesystem.open_file(&vpath, mode).await
    }
    
    /// Read entire file content
    pub async fn read_file(&self, path: &str) -> VDFSResult<Vec<u8>> {
        let vpath = VirtualPath::new(path);
        let mut handle = self.filesystem.open_file(&vpath, OpenMode::Read).await?;
        
        let metadata = self.filesystem.get_metadata(&vpath).await?;
        let mut buffer = vec![0u8; metadata.size as usize];
        handle.read(&mut buffer).await?;
        
        Ok(buffer)
    }
    
    /// Write entire file content
    pub async fn write_file(&self, path: &str, data: &[u8]) -> VDFSResult<()> {
        let vpath = VirtualPath::new(path);
        let mut handle = self.filesystem.create_file(&vpath).await?;
        handle.write(data).await?;
        handle.flush().await?;
        
        Ok(())
    }
    
    /// Delete a file
    pub async fn delete_file(&self, path: &str) -> VDFSResult<()> {
        let vpath = VirtualPath::new(path);
        self.filesystem.delete_file(&vpath).await
    }
    
    /// List directory contents
    pub async fn list_dir(&self, path: &str) -> VDFSResult<Vec<DirEntry>> {
        let vpath = VirtualPath::new(path);
        self.filesystem.list_dir(&vpath).await
    }
    
    /// Create a directory
    pub async fn create_dir(&self, path: &str) -> VDFSResult<()> {
        let vpath = VirtualPath::new(path);
        self.filesystem.create_dir(&vpath).await
    }
    
    /// Create a directory (alias for create_dir)
    pub async fn create_directory(&self, path: &str) -> VDFSResult<()> {
        self.create_dir(path).await
    }
    
    /// List directory contents (alias for list_dir)
    pub async fn list_directory(&self, path: &str) -> VDFSResult<Vec<DirEntry>> {
        self.list_dir(path).await
    }
    
    /// Get file metadata
    pub async fn get_metadata(&self, path: &str) -> VDFSResult<FileMetadata> {
        let vpath = VirtualPath::new(path);
        self.filesystem.get_metadata(&vpath).await
    }
}

#[cfg(test)]
mod basic_tests {
    use super::*;
    
    #[test]
    fn test_virtual_path() {
        let path = VirtualPath::new("/home/user/document.txt");
        assert_eq!(path.as_str(), "/home/user/document.txt");
        assert_eq!(path.file_name(), Some("document.txt"));
        
        let parent = path.parent().unwrap();
        assert_eq!(parent.as_str(), "/home/user");
        
        let joined = parent.join("another.txt");
        assert_eq!(joined.as_str(), "/home/user/another.txt");
    }
    
    #[test]
    fn test_chunk_creation_and_integrity() {
        let data = b"Hello, VDFS!".to_vec();
        let chunk = Chunk::new(data.clone());
        
        assert_eq!(chunk.data, data);
        assert_eq!(chunk.size, data.len());
        assert!(chunk.verify_integrity());
        
        // Test with modified data
        let mut modified_chunk = chunk.clone();
        modified_chunk.data[0] = modified_chunk.data[0].wrapping_add(1);
        assert!(!modified_chunk.verify_integrity());
    }
    
    #[test]
    fn test_file_permissions() {
        let perms = FilePermissions::default();
        assert!(perms.owner_read);
        assert!(perms.owner_write);
        assert!(!perms.owner_execute);
    }
    
    #[test]
    fn test_vdfs_config() {
        let config = VDFSConfig::default();
        assert_eq!(config.chunk_size, 1024 * 1024);
        assert_eq!(config.replication_factor, 3);
        assert!(!config.enable_compression);
    }
}
//! Virtual File System Interface Layer
//! 
//! Provides the main file system abstractions and interfaces that the VDFS
//! system implements. This layer defines the contract for file operations
//! and handles file lifecycle management.

use crate::vdfs::{VDFSResult, VirtualPath, FileId, OpenMode, DirEntry, FilePermissions};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

pub mod vfs;
pub mod file_handle;
pub mod path_resolver;
pub mod permissions;

pub use vfs::VirtualFileSystemImpl;
pub use file_handle::{FileHandle, FileOperations};
pub use path_resolver::PathResolver;
pub use permissions::PermissionManager;

/// File metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: FileId,
    pub path: VirtualPath,
    pub size: u64,
    pub created: SystemTime,
    pub modified: SystemTime,
    pub accessed: SystemTime,
    pub permissions: FilePermissions,
    pub checksum: Option<String>,
    pub mime_type: Option<String>,
    pub custom_attributes: std::collections::HashMap<String, String>,
    pub is_directory: bool,
}

impl FileMetadata {
    pub fn new_file(path: VirtualPath) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            path,
            size: 0,
            created: now,
            modified: now,
            accessed: now,
            permissions: FilePermissions::default(),
            checksum: None,
            mime_type: None,
            custom_attributes: std::collections::HashMap::new(),
            is_directory: false,
        }
    }
    
    pub fn new_directory(path: VirtualPath) -> Self {
        let mut metadata = Self::new_file(path);
        metadata.is_directory = true;
        metadata
    }
    
    pub fn update_modified(&mut self) {
        self.modified = SystemTime::now();
    }
    
    pub fn update_accessed(&mut self) {
        self.accessed = SystemTime::now();
    }
}

/// Virtual File System trait
#[async_trait]
pub trait VirtualFileSystem: Send + Sync {
    /// File operations
    async fn create_file(&self, path: &VirtualPath) -> VDFSResult<FileHandle>;
    async fn open_file(&self, path: &VirtualPath, mode: OpenMode) -> VDFSResult<FileHandle>;
    async fn delete_file(&self, path: &VirtualPath) -> VDFSResult<()>;
    async fn move_file(&self, from: &VirtualPath, to: &VirtualPath) -> VDFSResult<()>;
    async fn copy_file(&self, from: &VirtualPath, to: &VirtualPath) -> VDFSResult<()>;
    
    /// Directory operations
    async fn create_dir(&self, path: &VirtualPath) -> VDFSResult<()>;
    async fn list_dir(&self, path: &VirtualPath) -> VDFSResult<Vec<DirEntry>>;
    async fn remove_dir(&self, path: &VirtualPath) -> VDFSResult<()>;
    async fn remove_dir_all(&self, path: &VirtualPath) -> VDFSResult<()>;
    
    /// Metadata operations
    async fn get_metadata(&self, path: &VirtualPath) -> VDFSResult<FileMetadata>;
    async fn set_metadata(&self, path: &VirtualPath, metadata: FileMetadata) -> VDFSResult<()>;
    async fn exists(&self, path: &VirtualPath) -> VDFSResult<bool>;
    
    /// Path operations
    async fn canonicalize(&self, path: &VirtualPath) -> VDFSResult<VirtualPath>;
    async fn resolve_link(&self, path: &VirtualPath) -> VDFSResult<VirtualPath>;
}
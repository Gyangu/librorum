//! Metadata Management Layer
//! 
//! Handles file metadata, indexing, and consistency management for the VDFS system.

use crate::vdfs::{VDFSResult, VirtualPath, FileId, ChunkId, NodeId};
use crate::vdfs::filesystem::FileMetadata;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod manager;
pub mod index;
pub mod consistency;
pub mod database;
pub mod sled_manager;
pub mod rocksdb_manager;

#[cfg(test)]
pub mod performance_tests;

pub use manager::SimpleMetadataManager;
pub use database::DatabaseMetadataManager;
pub use sled_manager::SledMetadataManager;
pub use rocksdb_manager::RocksDBMetadataManager;
pub use index::{IndexStore, FileIndex};
pub use consistency::ConsistencyManager;

// 默认使用 Sled 作为元数据管理器
pub type DefaultMetadataManager = SledMetadataManager;

/// Complete file information including chunks and replicas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub metadata: FileMetadata,
    pub chunks: Vec<ChunkMetadata>,
    pub replicas: Vec<NodeId>,
    pub version: u64,
    pub checksum: String,
}

/// Metadata for individual chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub id: ChunkId,
    pub size: usize,
    pub checksum: String,
    pub compressed: bool,
    pub replicas: Vec<NodeId>,
    pub access_count: u64,
    pub last_accessed: std::time::SystemTime,
}

/// Metadata management interface
#[async_trait]
pub trait MetadataManager: Send + Sync {
    /// File metadata operations
    async fn get_file_info(&self, path: &VirtualPath) -> VDFSResult<FileInfo>;
    async fn set_file_info(&self, path: &VirtualPath, info: FileInfo) -> VDFSResult<()>;
    async fn delete_file_info(&self, path: &VirtualPath) -> VDFSResult<()>;
    async fn file_exists(&self, path: &VirtualPath) -> VDFSResult<bool>;
    
    /// Chunk mapping operations
    async fn get_chunk_mapping(&self, file_id: FileId) -> VDFSResult<Vec<ChunkId>>;
    async fn update_chunk_mapping(&self, file_id: FileId, chunks: Vec<ChunkId>) -> VDFSResult<()>;
    async fn get_chunk_metadata(&self, chunk_id: ChunkId) -> VDFSResult<ChunkMetadata>;
    async fn update_chunk_metadata(&self, chunk_id: ChunkId, metadata: ChunkMetadata) -> VDFSResult<()>;
    
    /// Directory operations
    async fn list_directory(&self, path: &VirtualPath) -> VDFSResult<Vec<VirtualPath>>;
    async fn create_directory(&self, path: &VirtualPath) -> VDFSResult<()>;
    async fn remove_directory(&self, path: &VirtualPath) -> VDFSResult<()>;
    
    /// Search operations
    async fn find_files_by_pattern(&self, pattern: &str) -> VDFSResult<Vec<VirtualPath>>;
    async fn find_files_by_size(&self, min_size: u64, max_size: u64) -> VDFSResult<Vec<VirtualPath>>;
    async fn find_files_by_date(&self, start: std::time::SystemTime, end: std::time::SystemTime) -> VDFSResult<Vec<VirtualPath>>;
    
    /// Consistency operations
    async fn verify_consistency(&self) -> VDFSResult<Vec<VirtualPath>>; // Returns inconsistent files
    async fn repair_metadata(&self, path: &VirtualPath) -> VDFSResult<()>;
    async fn rebuild_index(&self) -> VDFSResult<()>;
}
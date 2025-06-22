//! Storage Backend Layer
//! 
//! Provides storage abstractions and implementations for the VDFS system.
//! Handles data chunking, compression, and actual storage operations.

use crate::vdfs::{VDFSResult, ChunkId, Chunk, ChunkInfo, StorageInfo};
use async_trait::async_trait;

pub mod backend;
pub mod local_storage;
pub mod chunk_manager;
pub mod compression;

pub use local_storage::LocalStorageBackend;
pub use chunk_manager::DefaultChunkManager;
pub use compression::{CompressionAlgorithm, CompressionManager};

/// Storage backend trait for chunk-based operations
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Basic chunk operations
    async fn store_chunk(&self, chunk_id: ChunkId, data: &[u8]) -> VDFSResult<()>;
    async fn retrieve_chunk(&self, chunk_id: ChunkId) -> VDFSResult<Vec<u8>>;
    async fn delete_chunk(&self, chunk_id: ChunkId) -> VDFSResult<()>;
    async fn chunk_exists(&self, chunk_id: ChunkId) -> VDFSResult<bool>;
    
    /// Batch operations for efficiency
    async fn store_chunks(&self, chunks: Vec<(ChunkId, Vec<u8>)>) -> VDFSResult<()>;
    async fn retrieve_chunks(&self, chunk_ids: Vec<ChunkId>) -> VDFSResult<Vec<Option<Vec<u8>>>>;
    async fn delete_chunks(&self, chunk_ids: Vec<ChunkId>) -> VDFSResult<()>;
    
    /// Storage information
    async fn get_storage_info(&self) -> VDFSResult<StorageInfo>;
    async fn get_chunk_info(&self, chunk_id: ChunkId) -> VDFSResult<ChunkInfo>;
    async fn list_chunks(&self) -> VDFSResult<Vec<ChunkId>>;
    
    /// Maintenance operations
    async fn gc(&self) -> VDFSResult<usize>; // Returns number of chunks cleaned up
    async fn verify_integrity(&self) -> VDFSResult<Vec<ChunkId>>; // Returns corrupted chunks
    async fn repair_chunk(&self, chunk_id: ChunkId) -> VDFSResult<()>;
}

/// Chunk management for splitting and reassembling files
pub trait ChunkManager: Send + Sync {
    /// Split file data into chunks
    fn split_file(&self, data: &[u8]) -> VDFSResult<Vec<Chunk>>;
    
    /// Reassemble chunks into file data
    fn reassemble_file(&self, chunks: Vec<Chunk>) -> VDFSResult<Vec<u8>>;
    
    /// Deduplicate chunks based on content hash
    fn deduplicate(&self, chunks: &[Chunk]) -> Vec<ChunkId>;
    
    /// Calculate optimal chunk size for given data
    fn optimal_chunk_size(&self, data_size: usize) -> usize;
    
    /// Verify chunk integrity
    fn verify_chunk(&self, chunk: &Chunk) -> bool;
}
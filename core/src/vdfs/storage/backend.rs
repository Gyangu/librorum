//! Storage Backend Interface

use crate::vdfs::{VDFSResult, ChunkId, ChunkInfo, StorageInfo};
use async_trait::async_trait;

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store_chunk(&self, chunk_id: ChunkId, data: &[u8]) -> VDFSResult<()>;
    async fn retrieve_chunk(&self, chunk_id: ChunkId) -> VDFSResult<Vec<u8>>;
    async fn delete_chunk(&self, chunk_id: ChunkId) -> VDFSResult<()>;
    async fn chunk_exists(&self, chunk_id: ChunkId) -> VDFSResult<bool>;
    async fn store_chunks(&self, chunks: Vec<(ChunkId, Vec<u8>)>) -> VDFSResult<()>;
    async fn retrieve_chunks(&self, chunk_ids: Vec<ChunkId>) -> VDFSResult<Vec<Option<Vec<u8>>>>;
    async fn delete_chunks(&self, chunk_ids: Vec<ChunkId>) -> VDFSResult<()>;
    async fn get_storage_info(&self) -> VDFSResult<StorageInfo>;
    async fn get_chunk_info(&self, chunk_id: ChunkId) -> VDFSResult<ChunkInfo>;
    async fn list_chunks(&self) -> VDFSResult<Vec<ChunkId>>;
    async fn gc(&self) -> VDFSResult<usize>;
    async fn verify_integrity(&self) -> VDFSResult<Vec<ChunkId>>;
    async fn repair_chunk(&self, chunk_id: ChunkId) -> VDFSResult<()>;
}
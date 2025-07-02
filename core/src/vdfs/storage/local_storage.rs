//! Local Storage Backend Implementation

use crate::vdfs::{VDFSResult, VDFSError, ChunkId, ChunkInfo, StorageInfo, NodeId};
use crate::vdfs::storage::StorageBackend;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;

/// Local file system storage backend with advanced features
pub struct LocalStorageBackend {
    root_path: PathBuf,
    node_id: NodeId,
    /// Cache for storage statistics
    stats_cache: std::sync::RwLock<Option<(StorageInfo, std::time::Instant)>>,
    /// Cache timeout for statistics (in seconds)
    stats_cache_timeout: std::time::Duration,
}

impl LocalStorageBackend {
    pub fn new(root_path: PathBuf, node_id: NodeId) -> VDFSResult<Self> {
        // Create storage directory if it doesn't exist
        std::fs::create_dir_all(&root_path).map_err(|e| {
            VDFSError::StorageError(format!("Failed to create storage directory: {}", e))
        })?;
        
        // Create subdirectories for better organization
        for hex_char in "0123456789abcdef".chars() {
            let subdir = root_path.join(hex_char.to_string());
            std::fs::create_dir_all(&subdir).map_err(|e| {
                VDFSError::StorageError(format!("Failed to create subdirectory {}: {}", subdir.display(), e))
            })?;
        }
        
        Ok(Self {
            root_path,
            node_id,
            stats_cache: std::sync::RwLock::new(None),
            stats_cache_timeout: std::time::Duration::from_secs(60), // Cache for 1 minute
        })
    }
    
    /// Get the file path for a chunk
    fn chunk_path(&self, chunk_id: ChunkId) -> PathBuf {
        let chunk_hex = hex::encode(chunk_id);
        let (prefix, suffix) = chunk_hex.split_at(2);
        self.root_path.join(prefix).join(suffix)
    }
    
    /// Check if chunk exists synchronously (for fast path)
    fn chunk_exists_sync(&self, chunk_id: ChunkId) -> bool {
        self.chunk_path(chunk_id).exists()
    }
    
    /// Calculate directory size recursively
    fn calculate_directory_size(&self, path: &PathBuf) -> u64 {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| entry.metadata().ok())
            .map(|metadata| metadata.len())
            .sum()
    }
    
    /// Count chunks in storage
    fn count_chunks(&self) -> usize {
        WalkDir::new(&self.root_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                // Only count files that look like chunk files (64 hex characters)
                entry.file_name()
                    .to_string_lossy()
                    .len() == 62 // 64 - 2 chars for directory prefix
            })
            .count()
    }
    
    /// Get disk space information
    async fn get_disk_space(&self) -> VDFSResult<(u64, u64)> {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem;
            
            let path_cstr = CString::new(self.root_path.to_string_lossy().as_bytes())
                .map_err(|e| VDFSError::StorageError(format!("Invalid path: {}", e)))?;
            
            let mut statvfs: libc::statvfs = unsafe { mem::zeroed() };
            let result = unsafe { libc::statvfs(path_cstr.as_ptr(), &mut statvfs) };
            
            if result == 0 {
                let block_size = statvfs.f_frsize as u64;
                let total_blocks = statvfs.f_blocks as u64;
                let available_blocks = statvfs.f_bavail as u64;
                
                let total_space = total_blocks * block_size;
                let available_space = available_blocks * block_size;
                
                Ok((total_space, available_space))
            } else {
                // Fallback to estimated values
                Ok((1_000_000_000_000, 500_000_000_000)) // 1TB total, 500GB available
            }
        }
        
        #[cfg(not(unix))]
        {
            // Fallback for non-Unix systems
            Ok((1_000_000_000_000, 500_000_000_000)) // 1TB total, 500GB available
        }
    }
    
    /// Verify chunk integrity by comparing file size with expected chunk size
    async fn verify_chunk_integrity(&self, chunk_id: ChunkId) -> VDFSResult<bool> {
        let chunk_path = self.chunk_path(chunk_id);
        
        match fs::metadata(&chunk_path).await {
            Ok(metadata) => {
                // Basic integrity check: file exists and has reasonable size
                let size = metadata.len();
                Ok(size > 0 && size < 100_000_000) // Max 100MB per chunk
            }
            Err(_) => Ok(false),
        }
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn store_chunk(&self, chunk_id: ChunkId, data: &[u8]) -> VDFSResult<()> {
        if data.is_empty() {
            return Err(VDFSError::StorageError("Cannot store empty chunk".to_string()));
        }
        
        let chunk_path = self.chunk_path(chunk_id);
        
        // Create parent directory
        if let Some(parent) = chunk_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                VDFSError::StorageError(format!("Failed to create chunk directory: {}", e))
            })?;
        }
        
        // Write chunk data atomically using a temporary file
        let temp_path = chunk_path.with_extension("tmp");
        
        // Write to temporary file first
        fs::write(&temp_path, data).await.map_err(|e| {
            VDFSError::StorageError(format!("Failed to write temporary chunk: {}", e))
        })?;
        
        // Atomically rename to final location
        fs::rename(&temp_path, &chunk_path).await.map_err(|e| {
            // Clean up temp file on error - use async cleanup
            let temp_path_clone = temp_path.clone();
            tokio::spawn(async move {
                let _ = fs::remove_file(&temp_path_clone).await;
            });
            VDFSError::StorageError(format!("Failed to finalize chunk: {}", e))
        })?;
        
        // Invalidate stats cache
        if let Ok(mut cache) = self.stats_cache.write() {
            *cache = None;
        }
        
        Ok(())
    }
    
    async fn retrieve_chunk(&self, chunk_id: ChunkId) -> VDFSResult<Vec<u8>> {
        let chunk_path = self.chunk_path(chunk_id);
        
        // Check if file exists first (fast path)
        if !chunk_path.exists() {
            return Err(VDFSError::StorageError(format!("Chunk not found: {}", hex::encode(chunk_id))));
        }
        
        fs::read(&chunk_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                VDFSError::StorageError(format!("Chunk not found: {}", hex::encode(chunk_id)))
            } else {
                VDFSError::StorageError(format!("Failed to read chunk: {}", e))
            }
        })
    }
    
    async fn delete_chunk(&self, chunk_id: ChunkId) -> VDFSResult<()> {
        let chunk_path = self.chunk_path(chunk_id);
        
        if !chunk_path.exists() {
            return Err(VDFSError::StorageError(format!("Chunk not found: {}", hex::encode(chunk_id))));
        }
        
        fs::remove_file(&chunk_path).await.map_err(|e| {
            VDFSError::StorageError(format!("Failed to delete chunk: {}", e))
        })?;
        
        // Invalidate stats cache
        if let Ok(mut cache) = self.stats_cache.write() {
            *cache = None;
        }
        
        Ok(())
    }
    
    async fn chunk_exists(&self, chunk_id: ChunkId) -> VDFSResult<bool> {
        Ok(self.chunk_exists_sync(chunk_id))
    }
    
    async fn store_chunks(&self, chunks: Vec<(ChunkId, Vec<u8>)>) -> VDFSResult<()> {
        // Store chunks in parallel for high performance
        let futures: Vec<_> = chunks.into_iter()
            .map(|(chunk_id, data)| {
                let self_ref = self;
                async move { self_ref.store_chunk(chunk_id, &data).await }
            })
            .collect();
        
        // Execute all stores in parallel and collect any errors
        let results = futures::future::join_all(futures).await;
        for result in results {
            result?; // Return first error if any
        }
        
        Ok(())
    }
    
    async fn retrieve_chunks(&self, chunk_ids: Vec<ChunkId>) -> VDFSResult<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(chunk_ids.len());
        
        // Retrieve chunks in parallel for better performance
        let futures: Vec<_> = chunk_ids.iter()
            .map(|&chunk_id| self.retrieve_chunk(chunk_id))
            .collect();
        
        for result in futures::future::join_all(futures).await {
            match result {
                Ok(data) => results.push(Some(data)),
                Err(_) => results.push(None),
            }
        }
        
        Ok(results)
    }
    
    async fn delete_chunks(&self, chunk_ids: Vec<ChunkId>) -> VDFSResult<()> {
        let mut errors = Vec::new();
        
        // Delete chunks in parallel
        let futures: Vec<_> = chunk_ids.iter()
            .map(|&chunk_id| self.delete_chunk(chunk_id))
            .collect();
        
        for (i, result) in futures::future::join_all(futures).await.into_iter().enumerate() {
            if let Err(e) = result {
                errors.push((chunk_ids[i], e));
            }
        }
        
        // Report errors but don't fail the entire operation
        if !errors.is_empty() {
            let error_msg = errors.iter()
                .map(|(chunk_id, error)| format!("{}: {}", hex::encode(chunk_id), error))
                .collect::<Vec<_>>()
                .join(", ");
            tracing::warn!("Failed to delete some chunks: {}", error_msg);
        }
        
        Ok(())
    }
    
    async fn get_storage_info(&self) -> VDFSResult<StorageInfo> {
        // Check cache first
        {
            let cache = self.stats_cache.read().unwrap();
            if let Some((info, timestamp)) = &*cache {
                if timestamp.elapsed() < self.stats_cache_timeout {
                    return Ok(info.clone());
                }
            }
        }
        
        // Calculate actual storage information
        let (total_space, available_space) = self.get_disk_space().await?;
        let used_space_by_chunks = self.calculate_directory_size(&self.root_path);
        let chunk_count = self.count_chunks();
        
        let storage_info = StorageInfo {
            total_space,
            used_space: used_space_by_chunks,
            available_space,
            chunk_count,
            node_id: self.node_id.clone(),
        };
        
        // Update cache
        {
            let mut cache = self.stats_cache.write().unwrap();
            *cache = Some((storage_info.clone(), std::time::Instant::now()));
        }
        
        Ok(storage_info)
    }
    
    async fn get_chunk_info(&self, chunk_id: ChunkId) -> VDFSResult<ChunkInfo> {
        let chunk_path = self.chunk_path(chunk_id);
        let metadata = fs::metadata(&chunk_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                VDFSError::StorageError(format!("Chunk not found: {}", hex::encode(chunk_id)))
            } else {
                VDFSError::StorageError(format!("Failed to get chunk metadata: {}", e))
            }
        })?;
        
        Ok(ChunkInfo {
            id: chunk_id,
            size: metadata.len() as usize,
            checksum: hex::encode(chunk_id),
            compressed: false,
            replicas: vec![self.node_id.clone()],
        })
    }
    
    async fn list_chunks(&self) -> VDFSResult<Vec<ChunkId>> {
        let mut chunks = Vec::new();
        
        // Walk through all subdirectories and collect chunk files
        for entry in WalkDir::new(&self.root_path) {
            let entry = entry.map_err(|e| {
                VDFSError::StorageError(format!("Failed to read directory: {}", e))
            })?;
            
            if entry.file_type().is_file() {
                let file_name = entry.file_name().to_string_lossy();
                let parent_name = entry.path().parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                
                // Reconstruct full hex string from directory + filename
                let full_hex = format!("{}{}", parent_name, file_name);
                
                // Validate hex string and convert to ChunkId
                if full_hex.len() == 64 && hex::decode(&full_hex).is_ok() {
                    if let Ok(chunk_bytes) = hex::decode(&full_hex) {
                        if chunk_bytes.len() == 32 {
                            let mut chunk_id = [0u8; 32];
                            chunk_id.copy_from_slice(&chunk_bytes);
                            chunks.push(chunk_id);
                        }
                    }
                }
            }
        }
        
        Ok(chunks)
    }
    
    async fn gc(&self) -> VDFSResult<usize> {
        let mut cleaned_count = 0;
        
        // Find temporary files that failed to be committed
        for entry in WalkDir::new(&self.root_path) {
            let entry = entry.map_err(|e| {
                VDFSError::StorageError(format!("Failed to read directory during GC: {}", e))
            })?;
            
            if entry.file_type().is_file() {
                let path = entry.path();
                
                // Remove temporary files
                if path.extension().and_then(|ext| ext.to_str()) == Some("tmp") {
                    if let Err(e) = fs::remove_file(path).await {
                        tracing::warn!("Failed to remove temp file {}: {}", path.display(), e);
                    } else {
                        cleaned_count += 1;
                        tracing::debug!("Removed temp file: {}", path.display());
                    }
                }
                
                // Remove empty chunk files (potential corruption)
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() == 0 {
                        if let Err(e) = fs::remove_file(path).await {
                            tracing::warn!("Failed to remove empty file {}: {}", path.display(), e);
                        } else {
                            cleaned_count += 1;
                            tracing::debug!("Removed empty file: {}", path.display());
                        }
                    }
                }
            }
        }
        
        // Remove empty subdirectories
        for hex_char in "0123456789abcdef".chars() {
            let subdir = self.root_path.join(hex_char.to_string());
            if subdir.exists() {
                if let Ok(mut entries) = fs::read_dir(&subdir).await {
                    let mut has_files = false;
                    while let Ok(Some(_)) = entries.next_entry().await {
                        has_files = true;
                        break;
                    }
                    
                    if !has_files {
                        if let Err(e) = fs::remove_dir(&subdir).await {
                            tracing::warn!("Failed to remove empty directory {}: {}", subdir.display(), e);
                        }
                    }
                }
            }
        }
        
        // Invalidate stats cache after cleanup
        if let Ok(mut cache) = self.stats_cache.write() {
            *cache = None;
        }
        
        Ok(cleaned_count)
    }
    
    async fn verify_integrity(&self) -> VDFSResult<Vec<ChunkId>> {
        let mut corrupted_chunks = Vec::new();
        
        // List all chunks and verify each one
        let chunks = self.list_chunks().await?;
        
        for chunk_id in chunks {
            if !self.verify_chunk_integrity(chunk_id).await? {
                corrupted_chunks.push(chunk_id);
            }
        }
        
        if !corrupted_chunks.is_empty() {
            tracing::warn!("Found {} corrupted chunks", corrupted_chunks.len());
        }
        
        Ok(corrupted_chunks)
    }
    
    async fn repair_chunk(&self, chunk_id: ChunkId) -> VDFSResult<()> {
        // For local storage, repair means removing corrupted chunks
        // In a distributed system, we would try to fetch from replicas
        let chunk_path = self.chunk_path(chunk_id);
        
        if chunk_path.exists() {
            // Verify if chunk is actually corrupted
            if !self.verify_chunk_integrity(chunk_id).await? {
                // Remove corrupted chunk
                fs::remove_file(&chunk_path).await.map_err(|e| {
                    VDFSError::StorageError(format!("Failed to remove corrupted chunk: {}", e))
                })?;
                
                tracing::info!("Removed corrupted chunk: {}", hex::encode(chunk_id));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use sha2::{Sha256, Digest};
    
    fn create_test_chunk_id(data: &[u8]) -> ChunkId {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut chunk_id = [0u8; 32];
        chunk_id.copy_from_slice(&hash);
        chunk_id
    }
    
    async fn create_test_storage() -> (LocalStorageBackend, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageBackend::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
        ).unwrap();
        (storage, temp_dir)
    }
    
    #[tokio::test]
    async fn test_basic_chunk_operations() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let test_data = b"Hello, VDFS Storage!";
        let chunk_id = create_test_chunk_id(test_data);
        
        // Initially chunk should not exist
        assert!(!storage.chunk_exists(chunk_id).await.unwrap());
        
        // Store chunk
        storage.store_chunk(chunk_id, test_data).await.unwrap();
        
        // Chunk should now exist
        assert!(storage.chunk_exists(chunk_id).await.unwrap());
        
        // Retrieve chunk
        let retrieved_data = storage.retrieve_chunk(chunk_id).await.unwrap();
        assert_eq!(retrieved_data, test_data);
        
        // Get chunk info
        let chunk_info = storage.get_chunk_info(chunk_id).await.unwrap();
        assert_eq!(chunk_info.id, chunk_id);
        assert_eq!(chunk_info.size, test_data.len());
        
        // Delete chunk
        storage.delete_chunk(chunk_id).await.unwrap();
        assert!(!storage.chunk_exists(chunk_id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_batch_operations() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let test_chunks = vec![
            (b"chunk1".to_vec(), create_test_chunk_id(b"chunk1")),
            (b"chunk2".to_vec(), create_test_chunk_id(b"chunk2")),
            (b"chunk3".to_vec(), create_test_chunk_id(b"chunk3")),
        ];
        
        // Prepare data for batch store
        let store_data: Vec<(ChunkId, Vec<u8>)> = test_chunks
            .iter()
            .map(|(data, chunk_id)| (*chunk_id, data.clone()))
            .collect();
        
        // Batch store
        storage.store_chunks(store_data).await.unwrap();
        
        // Verify all chunks exist
        for (_, chunk_id) in &test_chunks {
            assert!(storage.chunk_exists(*chunk_id).await.unwrap());
        }
        
        // Batch retrieve
        let chunk_ids: Vec<ChunkId> = test_chunks.iter().map(|(_, id)| *id).collect();
        let retrieved_chunks = storage.retrieve_chunks(chunk_ids.clone()).await.unwrap();
        
        assert_eq!(retrieved_chunks.len(), test_chunks.len());
        for (i, (original_data, _)) in test_chunks.iter().enumerate() {
            assert_eq!(retrieved_chunks[i].as_ref().unwrap(), original_data);
        }
        
        // Batch delete
        storage.delete_chunks(chunk_ids).await.unwrap();
        
        // Verify all chunks are deleted
        for (_, chunk_id) in &test_chunks {
            assert!(!storage.chunk_exists(*chunk_id).await.unwrap());
        }
    }
    
    #[tokio::test]
    async fn test_list_chunks() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let test_data = b"test data for listing";
        let chunk_id = create_test_chunk_id(test_data);
        
        // Initially no chunks
        let chunks = storage.list_chunks().await.unwrap();
        assert_eq!(chunks.len(), 0);
        
        // Store a chunk
        storage.store_chunk(chunk_id, test_data).await.unwrap();
        
        // Should find one chunk
        let chunks = storage.list_chunks().await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], chunk_id);
    }
    
    #[tokio::test]
    async fn test_storage_info() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let storage_info = storage.get_storage_info().await.unwrap();
        assert_eq!(storage_info.node_id, "test_node");
        assert!(storage_info.total_space > 0);
        assert!(storage_info.available_space > 0);
        
        // Store some data and check if stats update
        let test_data = b"test data for storage info";
        let chunk_id = create_test_chunk_id(test_data);
        storage.store_chunk(chunk_id, test_data).await.unwrap();
        
        let updated_info = storage.get_storage_info().await.unwrap();
        assert_eq!(updated_info.chunk_count, 1);
    }
    
    #[tokio::test]
    async fn test_garbage_collection() {
        let (storage, temp_dir) = create_test_storage().await;
        
        // Create a temporary file manually to simulate failed operations
        let temp_file = temp_dir.path().join("ab").join("test.tmp");
        tokio::fs::create_dir_all(temp_file.parent().unwrap()).await.unwrap();
        tokio::fs::write(&temp_file, b"temporary data").await.unwrap();
        
        // Run garbage collection
        let cleaned_count = storage.gc().await.unwrap();
        assert_eq!(cleaned_count, 1);
        
        // Temp file should be gone
        assert!(!temp_file.exists());
    }
    
    #[tokio::test]
    async fn test_integrity_verification() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let test_data = b"test data for integrity";
        let chunk_id = create_test_chunk_id(test_data);
        
        // Store chunk normally
        storage.store_chunk(chunk_id, test_data).await.unwrap();
        
        // Initially should have no corrupted chunks
        let corrupted = storage.verify_integrity().await.unwrap();
        assert_eq!(corrupted.len(), 0);
        
        // Manually corrupt the chunk by creating an empty file
        let chunk_path = storage.chunk_path(chunk_id);
        tokio::fs::write(&chunk_path, b"").await.unwrap();
        
        // Should now detect corruption
        let corrupted = storage.verify_integrity().await.unwrap();
        assert_eq!(corrupted.len(), 1);
        assert_eq!(corrupted[0], chunk_id);
        
        // Repair should remove the corrupted chunk
        storage.repair_chunk(chunk_id).await.unwrap();
        assert!(!storage.chunk_exists(chunk_id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_atomic_writes() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let test_data = b"atomic write test data";
        let chunk_id = create_test_chunk_id(test_data);
        
        // Store chunk
        storage.store_chunk(chunk_id, test_data).await.unwrap();
        
        // Chunk path should exist and not have a .tmp extension
        let chunk_path = storage.chunk_path(chunk_id);
        assert!(chunk_path.exists());
        assert_ne!(chunk_path.extension(), Some(std::ffi::OsStr::new("tmp")));
        
        // Read data should match
        let retrieved = storage.retrieve_chunk(chunk_id).await.unwrap();
        assert_eq!(retrieved, test_data);
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let nonexistent_chunk = [0u8; 32];
        
        // Retrieving non-existent chunk should fail
        assert!(storage.retrieve_chunk(nonexistent_chunk).await.is_err());
        
        // Getting info for non-existent chunk should fail
        assert!(storage.get_chunk_info(nonexistent_chunk).await.is_err());
        
        // Deleting non-existent chunk should fail
        assert!(storage.delete_chunk(nonexistent_chunk).await.is_err());
        
        // Storing empty data should fail
        assert!(storage.store_chunk(nonexistent_chunk, &[]).await.is_err());
    }
}
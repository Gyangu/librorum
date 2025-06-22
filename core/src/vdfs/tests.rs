//! VDFS Integration Tests

#[cfg(test)]
mod integration_tests {
    use crate::vdfs::*;
    use crate::vdfs::filesystem::{VirtualFileSystemImpl, VirtualFileSystem};
    use crate::vdfs::storage::{LocalStorageBackend, DefaultChunkManager, StorageBackend};
    use crate::vdfs::metadata::{SimpleMetadataManager, MetadataManager};
    use tempfile::TempDir;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_virtual_file_system_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let storage: Arc<dyn StorageBackend> = Arc::new(LocalStorageBackend::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
        ).unwrap());
        let metadata: Arc<dyn MetadataManager> = Arc::new(SimpleMetadataManager::new());
        
        let vfs = VirtualFileSystemImpl::new(storage, metadata, 1024);
        let path = VirtualPath::new("/test/file.txt");
        
        // Test file creation
        let handle = vfs.create_file(&path).await.unwrap();
        assert_eq!(handle.path, path);
        assert_eq!(handle.mode, OpenMode::Create);
        
        // Test file metadata
        let metadata = vfs.get_metadata(&path).await.unwrap();
        assert_eq!(metadata.path, path);
        assert!(!metadata.is_directory);
    }
    
    #[tokio::test]
    async fn test_local_storage_backend() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageBackend::new(
            temp_dir.path().to_path_buf(), 
            "test_node".to_string()
        ).unwrap();
        
        // Test chunk storage and retrieval
        let test_data = b"Hello, VDFS World!";
        let chunk = Chunk::new(test_data.to_vec());
        let chunk_id = chunk.id;
        
        // Store chunk
        storage.store_chunk(chunk_id, &chunk.data).await.unwrap();
        
        // Verify chunk exists
        assert!(storage.chunk_exists(chunk_id).await.unwrap());
        
        // Retrieve chunk
        let retrieved_data = storage.retrieve_chunk(chunk_id).await.unwrap();
        assert_eq!(retrieved_data, test_data);
        
        // Delete chunk
        storage.delete_chunk(chunk_id).await.unwrap();
        assert!(!storage.chunk_exists(chunk_id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_chunk_manager() {
        let chunk_manager = DefaultChunkManager::new(1024, false);
        
        // Test file splitting
        let test_data = b"This is a test file that should be split into chunks.";
        let chunks = chunk_manager.split_file(test_data).unwrap();
        
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|chunk| chunk.verify_integrity()));
        
        // Test file reassembly
        let reassembled = chunk_manager.reassemble_file(chunks).unwrap();
        assert_eq!(reassembled, test_data);
    }
    
    #[tokio::test]
    async fn test_large_file_chunking() {
        let chunk_manager = DefaultChunkManager::new(1024, false); // 1KB chunks
        
        // Create a 5KB test file
        let large_data: Vec<u8> = (0..5120).map(|i| (i % 256) as u8).collect();
        
        // Split into chunks
        let chunks = chunk_manager.split_file(&large_data).unwrap();
        
        // Should have 5-6 chunks
        assert!(chunks.len() >= 5 && chunks.len() <= 6);
        
        // Verify all chunks
        for chunk in &chunks {
            assert!(chunk.verify_integrity());
            assert!(chunk.size <= 1024);
        }
        
        // Reassemble and verify
        let reassembled = chunk_manager.reassemble_file(chunks).unwrap();
        assert_eq!(reassembled, large_data);
    }
    
    #[test]
    fn test_chunk_deduplication() {
        let chunk_manager = DefaultChunkManager::new(1024, false);
        
        // Create identical chunks
        let data1 = b"identical content".to_vec();
        let data2 = b"identical content".to_vec();
        let data3 = b"different content".to_vec();
        
        let chunk1 = Chunk::new(data1);
        let chunk2 = Chunk::new(data2);
        let chunk3 = Chunk::new(data3);
        
        let chunks = vec![chunk1.clone(), chunk2, chunk3.clone()];
        let unique_ids = chunk_manager.deduplicate(&chunks);
        
        // Should have 2 unique chunks (chunk1 and chunk3)
        assert_eq!(unique_ids.len(), 2);
        assert!(unique_ids.contains(&chunk1.id));
        assert!(unique_ids.contains(&chunk3.id));
    }
    
    #[tokio::test]
    async fn test_storage_info() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageBackend::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string()
        ).unwrap();
        
        let info = storage.get_storage_info().await.unwrap();
        assert_eq!(info.node_id, "test_node");
        assert!(info.total_space > 0);
    }
}
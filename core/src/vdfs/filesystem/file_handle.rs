//! File Handle Implementation

use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId, OpenMode, NodeId};
use crate::vdfs::filesystem::FileMetadata;
use crate::vdfs::storage::{StorageBackend, DefaultChunkManager};
use crate::vdfs::metadata::{MetadataManager, FileInfo, ChunkMetadata};
use async_trait::async_trait;
use std::io::SeekFrom;
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use sha2::{Sha256, Digest};

/// File handle for VDFS operations with actual I/O capabilities
#[derive(Debug)]
pub struct FileHandle {
    pub id: FileId,
    pub path: VirtualPath,
    pub mode: OpenMode,
    pub position: u64,
    pub metadata: FileMetadata,
    
    // Weak references to avoid circular dependencies
    storage: Option<Weak<dyn StorageBackend>>,
    metadata_manager: Option<Weak<dyn MetadataManager>>,
    chunk_manager: Option<DefaultChunkManager>,
    
    // Buffer for write operations
    write_buffer: Vec<u8>,
    buffer_dirty: bool,
    
    // Version control and replica management
    current_version: u64,
    target_replicas: usize,
    available_nodes: Vec<NodeId>,
}

impl Clone for FileHandle {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            path: self.path.clone(),
            mode: self.mode,
            position: self.position,
            metadata: self.metadata.clone(),
            storage: None, // Reset weak references on clone
            metadata_manager: None,
            chunk_manager: None,
            write_buffer: Vec::new(),
            buffer_dirty: false,
            current_version: self.current_version,
            target_replicas: self.target_replicas,
            available_nodes: self.available_nodes.clone(),
        }
    }
}

impl FileHandle {
    pub fn new(id: FileId, path: VirtualPath, mode: OpenMode, metadata: FileMetadata) -> Self {
        Self {
            id,
            path,
            mode,
            position: 0,
            metadata,
            storage: None,
            metadata_manager: None,
            chunk_manager: None,
            write_buffer: Vec::new(),
            buffer_dirty: false,
            current_version: 1,
            target_replicas: 3, // Default replication factor
            available_nodes: Vec::new(),
        }
    }
    
    /// Initialize file handle with storage and metadata backends
    pub fn with_backends(
        mut self,
        storage: Weak<dyn StorageBackend>,
        metadata_manager: Weak<dyn MetadataManager>,
        chunk_manager: DefaultChunkManager,
    ) -> Self {
        self.storage = Some(storage);
        self.metadata_manager = Some(metadata_manager);
        self.chunk_manager = Some(chunk_manager);
        self
    }
    
    /// Get the storage backend if available
    fn get_storage(&self) -> VDFSResult<Arc<dyn StorageBackend>> {
        self.storage
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .ok_or_else(|| VDFSError::InternalError("Storage backend not available".to_string()))
    }
    
    /// Get the metadata manager if available
    fn get_metadata_manager(&self) -> VDFSResult<Arc<dyn MetadataManager>> {
        self.metadata_manager
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .ok_or_else(|| VDFSError::InternalError("Metadata manager not available".to_string()))
    }
    
    /// Get the chunk manager if available
    fn get_chunk_manager(&self) -> VDFSResult<&DefaultChunkManager> {
        self.chunk_manager
            .as_ref()
            .ok_or_else(|| VDFSError::InternalError("Chunk manager not available".to_string()))
    }
    
    /// Set available nodes for replica management
    pub fn set_available_nodes(&mut self, nodes: Vec<NodeId>) {
        self.available_nodes = nodes;
    }
    
    /// Set target replica count
    pub fn set_replica_count(&mut self, count: usize) {
        self.target_replicas = count;
    }
    
    /// Get current version
    pub fn version(&self) -> u64 {
        self.current_version
    }
    
    /// Increment version for next write operation
    fn increment_version(&mut self) {
        self.current_version += 1;
    }
    
    /// Calculate file checksum from current buffer or metadata
    fn calculate_file_checksum(&self) -> String {
        let mut hasher = Sha256::new();
        
        if self.buffer_dirty && !self.write_buffer.is_empty() {
            hasher.update(&self.write_buffer);
        } else {
            // For now, use metadata size as a simple checksum
            // In a real implementation, we'd read the actual file content
            hasher.update(&self.metadata.size.to_le_bytes());
            hasher.update(self.path.as_str().as_bytes());
        }
        
        hex::encode(hasher.finalize())
    }
    
    /// Select nodes for replica placement
    fn select_replica_nodes(&self) -> Vec<NodeId> {
        // Simple round-robin selection for now
        // In a real implementation, this would consider:
        // - Node load balancing
        // - Network topology
        // - Storage capacity
        // - Geographic distribution
        
        let mut selected = Vec::new();
        let replica_count = std::cmp::min(self.target_replicas, self.available_nodes.len());
        
        for i in 0..replica_count {
            let node_index = i % self.available_nodes.len();
            selected.push(self.available_nodes[node_index].clone());
        }
        
        selected
    }
    
    /// Load file content from storage
    async fn load_file_content(&self) -> VDFSResult<Vec<u8>> {
        let storage = self.get_storage()?;
        let metadata_manager = self.get_metadata_manager()?;
        let chunk_manager = self.get_chunk_manager()?;
        
        // Get file info to find chunks
        let file_info = metadata_manager.get_file_info(&self.path).await?;
        
        if file_info.chunks.is_empty() {
            return Ok(Vec::new());
        }
        
        // Retrieve all chunks
        let mut chunks = Vec::new();
        for chunk_metadata in &file_info.chunks {
            let chunk_data = storage.retrieve_chunk(chunk_metadata.id).await?;
            let chunk = crate::vdfs::Chunk::new(chunk_data);
            
            // Verify chunk integrity
            if !chunk.verify_integrity() || chunk.id != chunk_metadata.id {
                return Err(VDFSError::CorruptedData(hex::encode(chunk_metadata.id)));
            }
            
            chunks.push(chunk);
        }
        
        // Reassemble file
        chunk_manager.reassemble_file(chunks)
    }
    
    /// Save write buffer to storage
    async fn flush_write_buffer(&mut self) -> VDFSResult<()> {
        if !self.buffer_dirty || self.write_buffer.is_empty() {
            return Ok(());
        }
        
        let storage = self.get_storage()?;
        let metadata_manager = self.get_metadata_manager()?;
        
        // Increment version for this write operation
        self.increment_version();
        
        // Split data into chunks
        let chunks = {
            let chunk_manager = self.get_chunk_manager()?;
            chunk_manager.split_file(&self.write_buffer)?
        };
        
        // Select nodes for replica placement
        let replica_nodes = self.select_replica_nodes();
        
        // Store chunks with replica management
        let mut chunk_metadata_list = Vec::new();
        for chunk in chunks {
            // Store primary chunk
            storage.store_chunk(chunk.id, &chunk.data).await?;
            
            // Store replicas on other nodes (simulated for now)
            // In a real implementation, this would involve network calls to other nodes
            let mut chunk_replicas = vec!["primary_node".to_string()]; // Primary storage node
            
            for replica_node in &replica_nodes {
                if replica_node != "primary_node" {
                    // TODO: Implement actual network replication
                    // For now, just record the intended replica locations
                    chunk_replicas.push(replica_node.clone());
                }
            }
            
            let chunk_metadata = ChunkMetadata {
                id: chunk.id,
                size: chunk.size,
                checksum: chunk.checksum,
                compressed: chunk.compressed,
                replicas: chunk_replicas,
                access_count: 0,
                last_accessed: SystemTime::now(),
            };
            chunk_metadata_list.push(chunk_metadata);
        }
        
        // Update file metadata
        self.metadata.size = self.write_buffer.len() as u64;
        self.metadata.update_modified();
        
        // Calculate file checksum
        let file_checksum = self.calculate_file_checksum();
        
        // Update file info with proper versioning and replica management
        let file_info = FileInfo {
            metadata: self.metadata.clone(),
            chunks: chunk_metadata_list,
            replicas: replica_nodes.clone(),
            version: self.current_version,
            checksum: file_checksum,
        };
        
        metadata_manager.set_file_info(&self.path, file_info).await?;
        
        self.buffer_dirty = false;
        Ok(())
    }
    
    /// Check if the current mode allows reading
    fn can_read(&self) -> bool {
        matches!(self.mode, OpenMode::Read | OpenMode::ReadWrite | OpenMode::Create | OpenMode::CreateNew)
    }
    
    /// Check if the current mode allows writing
    fn can_write(&self) -> bool {
        matches!(
            self.mode,
            OpenMode::Write | OpenMode::ReadWrite | OpenMode::Create | OpenMode::CreateNew | OpenMode::Append
        )
    }
    
    /// Get replica health status
    pub async fn get_replica_health(&self) -> VDFSResult<Vec<(NodeId, bool)>> {
        let metadata_manager = self.get_metadata_manager()?;
        let file_info = metadata_manager.get_file_info(&self.path).await?;
        
        let mut health_status = Vec::new();
        for replica_node in &file_info.replicas {
            // In a real implementation, this would ping each node to check health
            // For now, we'll assume all nodes are healthy
            health_status.push((replica_node.clone(), true));
        }
        
        Ok(health_status)
    }
    
    /// Repair missing replicas
    pub async fn repair_replicas(&mut self) -> VDFSResult<()> {
        let metadata_manager = self.get_metadata_manager()?;
        let _storage = self.get_storage()?;
        
        let file_info = metadata_manager.get_file_info(&self.path).await?;
        let current_replica_count = file_info.replicas.len();
        
        if current_replica_count < self.target_replicas {
            let needed_replicas = self.target_replicas - current_replica_count;
            let available_nodes: Vec<_> = self.available_nodes
                .iter()
                .filter(|node| !file_info.replicas.contains(node))
                .take(needed_replicas)
                .cloned()
                .collect();
            
            // In a real implementation, this would copy data to new replica nodes
            // For now, we'll just update the metadata
            let mut updated_replicas = file_info.replicas.clone();
            updated_replicas.extend(available_nodes);
            
            let updated_file_info = FileInfo {
                replicas: updated_replicas,
                ..file_info
            };
            
            metadata_manager.set_file_info(&self.path, updated_file_info).await?;
        }
        
        Ok(())
    }
    
    /// Create a new version checkpoint
    pub async fn create_checkpoint(&mut self, _description: Option<String>) -> VDFSResult<u64> {
        // Force flush current changes
        if self.buffer_dirty {
            self.flush_write_buffer().await?;
        }
        
        let metadata_manager = self.get_metadata_manager()?;
        let current_file_info = metadata_manager.get_file_info(&self.path).await?;
        
        // In a real implementation, this would create a snapshot of the current state
        // For now, we'll just increment the version and store metadata
        self.increment_version();
        
        let checkpoint_info = FileInfo {
            version: self.current_version,
            ..current_file_info
        };
        
        // Store checkpoint metadata (in a real system, this would be in a separate namespace)
        let checkpoint_path = VirtualPath::new(&format!("{}@v{}", self.path.as_str(), self.current_version));
        metadata_manager.set_file_info(&checkpoint_path, checkpoint_info).await?;
        
        Ok(self.current_version)
    }
    
    /// Restore from a specific version
    pub async fn restore_version(&mut self, version: u64) -> VDFSResult<()> {
        let metadata_manager = self.get_metadata_manager()?;
        
        // Load the specific version
        let version_path = VirtualPath::new(&format!("{}@v{}", self.path.as_str(), version));
        let version_info = metadata_manager.get_file_info(&version_path).await?;
        
        // Update current file with version data
        metadata_manager.set_file_info(&self.path, version_info.clone()).await?;
        
        // Update handle state
        self.current_version = version;
        self.metadata = version_info.metadata;
        
        // Clear any pending changes
        self.write_buffer.clear();
        self.buffer_dirty = false;
        
        Ok(())
    }
}

/// File operations implementation
#[async_trait]
impl FileOperations for FileHandle {
    async fn read(&mut self, buf: &mut [u8]) -> VDFSResult<usize> {
        if !self.can_read() {
            return Err(VDFSError::PermissionDenied("File not open for reading".to_string()));
        }
        
        // For write-buffered files, read from buffer if available
        if self.buffer_dirty && !self.write_buffer.is_empty() {
            let start = self.position as usize;
            let end = std::cmp::min(start + buf.len(), self.write_buffer.len());
            
            if start >= self.write_buffer.len() {
                return Ok(0); // EOF
            }
            
            let bytes_to_copy = end - start;
            buf[..bytes_to_copy].copy_from_slice(&self.write_buffer[start..end]);
            self.position += bytes_to_copy as u64;
            return Ok(bytes_to_copy);
        }
        
        // Load content from storage
        let content = self.load_file_content().await?;
        
        let start = self.position as usize;
        if start >= content.len() {
            return Ok(0); // EOF
        }
        
        let end = std::cmp::min(start + buf.len(), content.len());
        let bytes_to_copy = end - start;
        
        buf[..bytes_to_copy].copy_from_slice(&content[start..end]);
        self.position += bytes_to_copy as u64;
        
        Ok(bytes_to_copy)
    }
    
    async fn write(&mut self, buf: &[u8]) -> VDFSResult<usize> {
        if !self.can_write() {
            return Err(VDFSError::PermissionDenied("File not open for writing".to_string()));
        }
        
        match self.mode {
            OpenMode::Append => {
                // For append mode, ensure we have existing content loaded first
                if self.write_buffer.is_empty() && !self.buffer_dirty {
                    // Load existing content
                    if let Ok(existing_content) = self.load_file_content().await {
                        self.write_buffer = existing_content;
                    }
                }
                // Append mode: add to the end
                self.write_buffer.extend_from_slice(buf);
                self.position = self.write_buffer.len() as u64;
            }
            _ => {
                // Overwrite mode: ensure buffer is large enough
                let end_pos = self.position as usize + buf.len();
                if end_pos > self.write_buffer.len() {
                    self.write_buffer.resize(end_pos, 0);
                }
                
                // Write data at current position
                let start = self.position as usize;
                self.write_buffer[start..start + buf.len()].copy_from_slice(buf);
                self.position += buf.len() as u64;
            }
        }
        
        self.buffer_dirty = true;
        Ok(buf.len())
    }
    
    async fn seek(&mut self, pos: SeekFrom) -> VDFSResult<u64> {
        let new_position = match pos {
            SeekFrom::Start(pos) => pos,
            SeekFrom::End(offset) => {
                let size = if self.buffer_dirty {
                    self.write_buffer.len() as u64
                } else {
                    self.metadata.size
                };
                
                if offset >= 0 {
                    size + offset as u64
                } else {
                    size.saturating_sub((-offset) as u64)
                }
            }
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.position + offset as u64
                } else {
                    self.position.saturating_sub((-offset) as u64)
                }
            }
        };
        
        self.position = new_position;
        Ok(self.position)
    }
    
    async fn flush(&mut self) -> VDFSResult<()> {
        if self.can_write() {
            self.flush_write_buffer().await?;
        }
        Ok(())
    }
    
    async fn sync(&mut self) -> VDFSResult<()> {
        // Flush is the same as sync for our implementation
        self.flush().await
    }
}

/// File operations trait
#[async_trait]
pub trait FileOperations {
    async fn read(&mut self, buf: &mut [u8]) -> VDFSResult<usize>;
    async fn write(&mut self, buf: &[u8]) -> VDFSResult<usize>;
    async fn seek(&mut self, pos: SeekFrom) -> VDFSResult<u64>;
    async fn flush(&mut self) -> VDFSResult<()>;
    async fn sync(&mut self) -> VDFSResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdfs::storage::LocalStorageBackend;
    use crate::vdfs::metadata::SimpleMetadataManager;
    use tempfile::TempDir;
    use uuid::Uuid;
    
    async fn create_test_handle() -> (FileHandle, Arc<dyn StorageBackend>, Arc<dyn MetadataManager>) {
        let temp_dir = TempDir::new().unwrap();
        let storage: Arc<dyn StorageBackend> = Arc::new(LocalStorageBackend::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
        ).unwrap());
        let metadata_manager: Arc<dyn MetadataManager> = Arc::new(SimpleMetadataManager::new());
        
        let path = VirtualPath::new("/test_file.txt");
        let metadata = crate::vdfs::filesystem::FileMetadata::new_file(path.clone());
        
        let handle = FileHandle::new(metadata.id, path, OpenMode::ReadWrite, metadata)
            .with_backends(
                Arc::downgrade(&storage),
                Arc::downgrade(&metadata_manager),
                DefaultChunkManager::new(1024, false),
            );
        
        (handle, storage, metadata_manager)
    }
    
    #[tokio::test]
    async fn test_file_write_and_read() {
        let (mut handle, _storage, _metadata) = create_test_handle().await;
        
        // Write data
        let test_data = b"Hello, VDFS File System!";
        let bytes_written = handle.write(test_data).await.unwrap();
        assert_eq!(bytes_written, test_data.len());
        
        // Flush to storage
        handle.flush().await.unwrap();
        
        // Seek back to beginning
        handle.seek(SeekFrom::Start(0)).await.unwrap();
        
        // Read data back
        let mut read_buffer = vec![0u8; test_data.len()];
        let bytes_read = handle.read(&mut read_buffer).await.unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&read_buffer[..bytes_read], test_data);
    }
    
    #[tokio::test]
    async fn test_file_seek_operations() {
        let (mut handle, _storage, _metadata) = create_test_handle().await;
        
        // Write test data
        let test_data = b"0123456789";
        handle.write(test_data).await.unwrap();
        
        // Test SeekFrom::Start
        let pos = handle.seek(SeekFrom::Start(5)).await.unwrap();
        assert_eq!(pos, 5);
        
        // Test SeekFrom::Current
        let pos = handle.seek(SeekFrom::Current(2)).await.unwrap();
        assert_eq!(pos, 7);
        
        // Test SeekFrom::End
        let pos = handle.seek(SeekFrom::End(-3)).await.unwrap();
        assert_eq!(pos, 7);
        
        // Read from position 7
        let mut buf = [0u8; 3];
        let bytes_read = handle.read(&mut buf).await.unwrap();
        assert_eq!(bytes_read, 3);
        assert_eq!(&buf, b"789");
    }
    
    #[tokio::test]
    async fn test_append_mode() {
        let temp_dir = TempDir::new().unwrap();
        let storage: Arc<dyn StorageBackend> = Arc::new(LocalStorageBackend::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
        ).unwrap());
        let metadata_manager: Arc<dyn MetadataManager> = Arc::new(SimpleMetadataManager::new());
        
        let path = VirtualPath::new("/append_test.txt");
        let metadata = crate::vdfs::filesystem::FileMetadata::new_file(path.clone());
        let file_id = metadata.id;
        
        let mut handle = FileHandle::new(file_id, path.clone(), OpenMode::Append, metadata.clone())
            .with_backends(
                Arc::downgrade(&storage),
                Arc::downgrade(&metadata_manager),
                DefaultChunkManager::new(1024, false),
            );
        
        // Write initial data
        handle.write(b"Hello").await.unwrap();
        handle.write(b" ").await.unwrap();
        handle.write(b"World").await.unwrap();
        
        // Flush to storage
        handle.flush().await.unwrap();
        
        // Create a new handle in read mode to read back the data
        let mut read_handle = FileHandle::new(file_id, path.clone(), OpenMode::Read, metadata)
            .with_backends(
                Arc::downgrade(&storage),
                Arc::downgrade(&metadata_manager),
                DefaultChunkManager::new(1024, false),
            );
        
        let mut buffer = vec![0u8; 11];
        let bytes_read = read_handle.read(&mut buffer).await.unwrap();
        assert_eq!(bytes_read, 11);
        assert_eq!(&buffer[..bytes_read], b"Hello World");
    }
    
    #[tokio::test]
    async fn test_version_control() {
        let (mut handle, _storage, _metadata) = create_test_handle().await;
        
        // Set up some available nodes for replica management
        handle.set_available_nodes(vec![
            "node1".to_string(),
            "node2".to_string(), 
            "node3".to_string()
        ]);
        handle.set_replica_count(2);
        
        // Check initial version
        assert_eq!(handle.version(), 1);
        
        // Write some data (this should increment version)
        let test_data_v1 = b"Version 1 data";
        handle.write(test_data_v1).await.unwrap();
        handle.flush().await.unwrap();
        assert_eq!(handle.version(), 2);
        
        // Create a checkpoint
        let checkpoint_version = handle.create_checkpoint(Some("Checkpoint v2".to_string())).await.unwrap();
        assert_eq!(checkpoint_version, 3);
        assert_eq!(handle.version(), 3);
        
        // Write more data
        handle.seek(SeekFrom::End(0)).await.unwrap();
        let test_data_v2 = b" - Updated in v3";
        handle.write(test_data_v2).await.unwrap();
        handle.flush().await.unwrap();
        assert_eq!(handle.version(), 4);
        
        // Test replica health check
        let health_status = handle.get_replica_health().await.unwrap();
        assert!(health_status.len() >= 1); // Should have at least one replica
        
        // Test replica repair
        handle.repair_replicas().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_replica_management() {
        let (mut handle, _storage, _metadata) = create_test_handle().await;
        
        // Configure replica settings
        let available_nodes = vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
            "node4".to_string(),
        ];
        handle.set_available_nodes(available_nodes.clone());
        handle.set_replica_count(3);
        
        // Write data to trigger replication
        let test_data = b"Test data for replication";
        handle.write(test_data).await.unwrap();
        handle.flush().await.unwrap();
        
        // Check that replica health monitoring works
        let health_status = handle.get_replica_health().await.unwrap();
        assert!(!health_status.is_empty());
        
        // Test that replica repair works without errors
        assert!(handle.repair_replicas().await.is_ok());
        
        // Verify that checksum calculation works
        handle.seek(SeekFrom::Start(0)).await.unwrap();
        let mut read_buffer = vec![0u8; test_data.len()];
        let bytes_read = handle.read(&mut read_buffer).await.unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&read_buffer, test_data);
    }
}
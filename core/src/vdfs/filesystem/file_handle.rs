//! File Handle Implementation

use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId, OpenMode};
use crate::vdfs::filesystem::FileMetadata;
use crate::vdfs::storage::{StorageBackend, DefaultChunkManager};
use crate::vdfs::metadata::{MetadataManager, FileInfo, ChunkMetadata};
use async_trait::async_trait;
use std::io::SeekFrom;
use std::sync::{Arc, Weak};
use std::time::SystemTime;

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
        let chunk_manager = self.get_chunk_manager()?;
        
        // Split data into chunks
        let chunks = chunk_manager.split_file(&self.write_buffer)?;
        
        // Store chunks
        let mut chunk_metadata_list = Vec::new();
        for chunk in chunks {
            storage.store_chunk(chunk.id, &chunk.data).await?;
            
            let chunk_metadata = ChunkMetadata {
                id: chunk.id,
                size: chunk.size,
                checksum: chunk.checksum,
                compressed: chunk.compressed,
                replicas: Vec::new(), // TODO: Add replica management
                access_count: 0,
                last_accessed: SystemTime::now(),
            };
            chunk_metadata_list.push(chunk_metadata);
        }
        
        // Update file metadata
        self.metadata.size = self.write_buffer.len() as u64;
        self.metadata.update_modified();
        
        // Update file info
        let file_info = FileInfo {
            metadata: self.metadata.clone(),
            chunks: chunk_metadata_list,
            replicas: Vec::new(),
            version: 1, // TODO: Proper versioning
            checksum: String::new(), // TODO: Calculate file checksum
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
}
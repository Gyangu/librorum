use crate::vdfs::{VDFSResult, VDFSError, VirtualPath, FileId, ChunkId, NodeId};
use crate::vdfs::filesystem::FileMetadata;
use crate::vdfs::metadata::{FileInfo, ChunkMetadata, MetadataManager};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DatabaseMetadataManager {
    pool: SqlitePool,
}

impl DatabaseMetadataManager {
    pub async fn new(database_url: &str) -> VDFSResult<Self> {
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Failed to connect to SQLite database: {}", e)))?;

        let manager = Self { pool };
        manager.initialize_schema().await?;
        
        Ok(manager)
    }

    async fn initialize_schema(&self) -> VDFSResult<()> {
        let schema = include_str!("schema.sql");
        
        // Split on semicolons but handle multi-line statements correctly
        let mut statements = Vec::new();
        let mut current_statement = String::new();
        let mut in_trigger = false;
        
        for line in schema.lines() {
            let trimmed_line = line.trim();
            
            // Skip comments and empty lines
            if trimmed_line.is_empty() || trimmed_line.starts_with("--") {
                continue;
            }
            
            current_statement.push_str(line);
            current_statement.push('\n');
            
            // Check for trigger start
            if trimmed_line.contains("CREATE TRIGGER") {
                in_trigger = true;
            }
            
            // Check for statement end
            if trimmed_line.ends_with(';') {
                if in_trigger && trimmed_line == "END;" {
                    in_trigger = false;
                    statements.push(current_statement.trim().to_string());
                    current_statement.clear();
                } else if !in_trigger {
                    statements.push(current_statement.trim().to_string());
                    current_statement.clear();
                }
            }
        }
        
        // Execute each statement
        for statement in statements {
            if !statement.is_empty() {
                sqlx::query(&statement)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| VDFSError::StorageError(format!("Failed to execute schema statement: {}", e)))?;
            }
        }
        
        Ok(())
    }

    fn system_time_to_timestamp(time: SystemTime) -> i64 {
        time.duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    fn timestamp_to_system_time(timestamp: i64) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64)
    }

    fn chunk_id_to_string(chunk_id: &ChunkId) -> String {
        hex::encode(chunk_id)
    }

    fn string_to_chunk_id(s: &str) -> VDFSResult<ChunkId> {
        let bytes = hex::decode(s).map_err(|e| VDFSError::InternalError(format!("Invalid chunk ID: {}", e)))?;
        if bytes.len() != 32 {
            return Err(VDFSError::InternalError("Invalid chunk ID length".to_string()));
        }
        let mut chunk_id = [0u8; 32];
        chunk_id.copy_from_slice(&bytes);
        Ok(chunk_id)
    }

    async fn get_file_attributes(&self, file_id: &FileId) -> VDFSResult<HashMap<String, String>> {
        let rows = sqlx::query("SELECT key, value FROM file_attributes WHERE file_id = ?")
            .bind(file_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let mut attributes = HashMap::new();
        for row in rows {
            let key: String = row.get("key");
            let value: String = row.get("value");
            attributes.insert(key, value);
        }

        Ok(attributes)
    }

    async fn get_file_replicas(&self, file_id: &FileId) -> VDFSResult<Vec<NodeId>> {
        let rows = sqlx::query("SELECT node_id FROM file_replicas WHERE file_id = ?")
            .bind(file_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let replicas = rows.into_iter()
            .map(|row| {
                let node_id_str: String = row.get("node_id");
                node_id_str
            })
            .collect();

        Ok(replicas)
    }

    async fn get_chunks_for_file(&self, file_id: &FileId) -> VDFSResult<Vec<ChunkMetadata>> {
        let rows = sqlx::query(
            "SELECT id, chunk_index, size, checksum, compressed, access_count, last_accessed_timestamp 
             FROM chunk_metadata WHERE file_id = ? ORDER BY chunk_index"
        )
        .bind(file_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let mut chunks = Vec::new();
        for row in rows {
            let chunk_id_str: String = row.get("id");
            let chunk_id = Self::string_to_chunk_id(&chunk_id_str)?;
            
            let replicas = self.get_chunk_replicas(&chunk_id).await?;

            let chunk = ChunkMetadata {
                id: chunk_id,
                size: row.get::<i64, _>("size") as usize,
                checksum: row.get("checksum"),
                compressed: row.get("compressed"),
                replicas,
                access_count: row.get::<i64, _>("access_count") as u64,
                last_accessed: Self::timestamp_to_system_time(row.get("last_accessed_timestamp")),
            };
            chunks.push(chunk);
        }

        Ok(chunks)
    }

    async fn get_chunk_replicas(&self, chunk_id: &ChunkId) -> VDFSResult<Vec<NodeId>> {
        let rows = sqlx::query("SELECT node_id FROM chunk_replicas WHERE chunk_id = ?")
            .bind(Self::chunk_id_to_string(chunk_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let replicas = rows.into_iter()
            .map(|row| {
                let node_id_str: String = row.get("node_id");
                node_id_str
            })
            .collect();

        Ok(replicas)
    }
}

#[async_trait]
impl MetadataManager for DatabaseMetadataManager {
    async fn get_file_info(&self, path: &VirtualPath) -> VDFSResult<FileInfo> {
        let row = sqlx::query(
            "SELECT id, path, size, created_timestamp, modified_timestamp, accessed_timestamp, 
             permissions, checksum, mime_type, is_directory, version 
             FROM file_metadata WHERE path = ?"
        )
        .bind(path.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        if let Some(row) = row {
            let file_id_str: String = row.get("id");
            let file_id = uuid::Uuid::parse_str(&file_id_str)
                .map_err(|e| VDFSError::InternalError(format!("Invalid UUID: {}", e)))?;
            
            let custom_attributes = self.get_file_attributes(&file_id).await.unwrap_or_default();
            let replicas = self.get_file_replicas(&file_id).await.unwrap_or_default();
            let chunks = self.get_chunks_for_file(&file_id).await.unwrap_or_default();

            let metadata = FileMetadata {
                id: file_id,
                path: path.clone(),
                size: row.get::<i64, _>("size") as u64,
                created: Self::timestamp_to_system_time(row.get("created_timestamp")),
                modified: Self::timestamp_to_system_time(row.get("modified_timestamp")),
                accessed: Self::timestamp_to_system_time(row.get("accessed_timestamp")),
                permissions: crate::vdfs::FilePermissions::default(),
                checksum: row.get("checksum"),
                mime_type: row.get("mime_type"),
                custom_attributes,
                is_directory: row.get("is_directory"),
            };

            let file_info = FileInfo {
                metadata,
                chunks,
                replicas,
                version: row.get::<i64, _>("version") as u64,
                checksum: row.get::<Option<String>, _>("checksum").unwrap_or_default(),
            };

            Ok(file_info)
        } else {
            Err(VDFSError::FileNotFound(path.clone()))
        }
    }

    async fn set_file_info(&self, path: &VirtualPath, file_info: FileInfo) -> VDFSResult<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO file_metadata 
             (id, path, size, created_timestamp, modified_timestamp, accessed_timestamp, 
              permissions, checksum, mime_type, is_directory, version) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(file_info.metadata.id.to_string())
        .bind(path.to_string())
        .bind(file_info.metadata.size as i64)
        .bind(Self::system_time_to_timestamp(file_info.metadata.created))
        .bind(Self::system_time_to_timestamp(file_info.metadata.modified))
        .bind(Self::system_time_to_timestamp(file_info.metadata.accessed))
        .bind(0i64) // TODO: implement proper permissions serialization
        .bind(&file_info.metadata.checksum)
        .bind(&file_info.metadata.mime_type)
        .bind(file_info.metadata.is_directory)
        .bind(file_info.version as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        // Store file attributes
        sqlx::query("DELETE FROM file_attributes WHERE file_id = ?")
            .bind(file_info.metadata.id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        for (key, value) in &file_info.metadata.custom_attributes {
            sqlx::query("INSERT INTO file_attributes (file_id, key, value) VALUES (?, ?, ?)")
                .bind(file_info.metadata.id.to_string())
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await
                .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;
        }

        // Store file replicas
        sqlx::query("DELETE FROM file_replicas WHERE file_id = ?")
            .bind(file_info.metadata.id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        for replica in &file_info.replicas {
            sqlx::query("INSERT INTO file_replicas (file_id, node_id) VALUES (?, ?)")
                .bind(file_info.metadata.id.to_string())
                .bind(replica)
                .execute(&self.pool)
                .await
                .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;
        }

        Ok(())
    }

    async fn delete_file_info(&self, path: &VirtualPath) -> VDFSResult<()> {
        sqlx::query("DELETE FROM file_metadata WHERE path = ?")
            .bind(path.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;
        Ok(())
    }

    async fn file_exists(&self, path: &VirtualPath) -> VDFSResult<bool> {
        let result = sqlx::query("SELECT 1 FROM file_metadata WHERE path = ? LIMIT 1")
            .bind(path.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;
        Ok(result.is_some())
    }

    async fn get_chunk_mapping(&self, file_id: FileId) -> VDFSResult<Vec<ChunkId>> {
        let chunks = self.get_chunks_for_file(&file_id).await?;
        Ok(chunks.into_iter().map(|c| c.id).collect())
    }

    async fn update_chunk_mapping(&self, _file_id: FileId, _chunks: Vec<ChunkId>) -> VDFSResult<()> {
        // This would be implemented through set_file_info
        Ok(())
    }

    async fn get_chunk_metadata(&self, chunk_id: ChunkId) -> VDFSResult<ChunkMetadata> {
        let row = sqlx::query(
            "SELECT size, checksum, compressed, access_count, last_accessed_timestamp 
             FROM chunk_metadata WHERE id = ?"
        )
        .bind(Self::chunk_id_to_string(&chunk_id))
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        if let Some(row) = row {
            let replicas = self.get_chunk_replicas(&chunk_id).await?;

            let chunk = ChunkMetadata {
                id: chunk_id,
                size: row.get::<i64, _>("size") as usize,
                checksum: row.get("checksum"),
                compressed: row.get("compressed"),
                replicas,
                access_count: row.get::<i64, _>("access_count") as u64,
                last_accessed: Self::timestamp_to_system_time(row.get("last_accessed_timestamp")),
            };

            Ok(chunk)
        } else {
            Err(VDFSError::InternalError("Chunk not found".to_string()))
        }
    }

    async fn update_chunk_metadata(&self, _chunk_id: ChunkId, _metadata: ChunkMetadata) -> VDFSResult<()> {
        // This would be implemented through set_file_info
        Ok(())
    }

    async fn list_directory(&self, path: &VirtualPath) -> VDFSResult<Vec<VirtualPath>> {
        let pattern = format!("{}/%", path.to_string());
        let rows = sqlx::query("SELECT path FROM file_metadata WHERE path LIKE ? AND path NOT LIKE ?")
            .bind(&pattern)
            .bind(format!("{}/%/%", path.to_string()))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let paths = rows.into_iter()
            .map(|row| {
                let path_str: String = row.get("path");
                VirtualPath::from(path_str)
            })
            .collect();

        Ok(paths)
    }

    async fn create_directory(&self, path: &VirtualPath) -> VDFSResult<()> {
        let file_id = uuid::Uuid::new_v4();
        let now = SystemTime::now();
        
        sqlx::query(
            "INSERT INTO file_metadata 
             (id, path, size, created_timestamp, modified_timestamp, accessed_timestamp, 
              permissions, checksum, mime_type, is_directory, version) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(file_id.to_string())
        .bind(path.to_string())
        .bind(0i64)
        .bind(Self::system_time_to_timestamp(now))
        .bind(Self::system_time_to_timestamp(now))
        .bind(Self::system_time_to_timestamp(now))
        .bind(0i64)
        .bind(None::<String>)
        .bind(None::<String>)
        .bind(true)
        .bind(1i64)
        .execute(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        Ok(())
    }

    async fn remove_directory(&self, path: &VirtualPath) -> VDFSResult<()> {
        sqlx::query("DELETE FROM file_metadata WHERE path LIKE ? || '/%' OR path = ?")
            .bind(path.to_string())
            .bind(path.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;
        Ok(())
    }

    async fn find_files_by_pattern(&self, pattern: &str) -> VDFSResult<Vec<VirtualPath>> {
        let rows = sqlx::query("SELECT path FROM file_metadata WHERE path LIKE ?")
            .bind(format!("%{}%", pattern))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let paths = rows.into_iter()
            .map(|row| {
                let path_str: String = row.get("path");
                VirtualPath::from(path_str)
            })
            .collect();

        Ok(paths)
    }

    async fn find_files_by_size(&self, min_size: u64, max_size: u64) -> VDFSResult<Vec<VirtualPath>> {
        let rows = sqlx::query("SELECT path FROM file_metadata WHERE size BETWEEN ? AND ?")
            .bind(min_size as i64)
            .bind(max_size as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let paths = rows.into_iter()
            .map(|row| {
                let path_str: String = row.get("path");
                VirtualPath::from(path_str)
            })
            .collect();

        Ok(paths)
    }

    async fn find_files_by_date(&self, start: SystemTime, end: SystemTime) -> VDFSResult<Vec<VirtualPath>> {
        let start_timestamp = Self::system_time_to_timestamp(start);
        let end_timestamp = Self::system_time_to_timestamp(end);

        let rows = sqlx::query("SELECT path FROM file_metadata WHERE modified_timestamp BETWEEN ? AND ?")
            .bind(start_timestamp)
            .bind(end_timestamp)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        let paths = rows.into_iter()
            .map(|row| {
                let path_str: String = row.get("path");
                VirtualPath::from(path_str)
            })
            .collect();

        Ok(paths)
    }

    async fn verify_consistency(&self) -> VDFSResult<Vec<VirtualPath>> {
        let mut inconsistent_files = Vec::new();

        // Find orphaned chunks
        let orphaned_chunks = sqlx::query(
            "SELECT c.file_id FROM chunk_metadata c 
             LEFT JOIN file_metadata f ON c.file_id = f.id 
             WHERE f.id IS NULL"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        for row in orphaned_chunks {
            let file_id: String = row.get("file_id");
            inconsistent_files.push(VirtualPath::new(format!("orphaned_chunks:{}", file_id)));
        }

        Ok(inconsistent_files)
    }

    async fn repair_metadata(&self, _path: &VirtualPath) -> VDFSResult<()> {
        // Clean up orphaned entries
        sqlx::query(
            "DELETE FROM chunk_replicas 
             WHERE chunk_id NOT IN (SELECT id FROM chunk_metadata)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        sqlx::query(
            "DELETE FROM chunk_metadata 
             WHERE file_id NOT IN (SELECT id FROM file_metadata)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        sqlx::query(
            "DELETE FROM file_attributes 
             WHERE file_id NOT IN (SELECT id FROM file_metadata)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        sqlx::query(
            "DELETE FROM file_replicas 
             WHERE file_id NOT IN (SELECT id FROM file_metadata)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        Ok(())
    }

    async fn rebuild_index(&self) -> VDFSResult<()> {
        sqlx::query("DELETE FROM file_search")
            .execute(&self.pool)
            .await
            .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        sqlx::query(
            "INSERT INTO file_search(file_id, path, attributes) 
             SELECT id, path, '' FROM file_metadata"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| VDFSError::StorageError(format!("Database error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdfs::{VirtualPath, FileId};
    use crate::vdfs::filesystem::FileMetadata;
    use std::collections::HashMap;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    async fn create_test_database() -> DatabaseMetadataManager {
        // Use in-memory SQLite database for testing
        let db_url = "sqlite::memory:";
        DatabaseMetadataManager::new(db_url).await.unwrap()
    }

    fn create_test_file_info(path: &str) -> FileInfo {
        let file_id = uuid::Uuid::new_v4();
        let now = SystemTime::now();
        
        let metadata = FileMetadata {
            id: file_id,
            path: VirtualPath::new(path),
            size: 1024,
            created: now,
            modified: now,
            accessed: now,
            permissions: crate::vdfs::FilePermissions::default(),
            checksum: Some("test_checksum".to_string()),
            mime_type: Some("text/plain".to_string()),
            custom_attributes: HashMap::new(),
            is_directory: false,
        };

        FileInfo {
            metadata,
            chunks: Vec::new(),
            replicas: Vec::new(),
            version: 1,
            checksum: "test_checksum".to_string(),
        }
    }

    #[tokio::test]
    async fn test_database_creation() {
        let _db = create_test_database().await;
        // If we get here without panicking, database creation succeeded
    }

    #[tokio::test]
    async fn test_file_crud_operations() {
        let db = create_test_database().await;
        let path = VirtualPath::new("/test/file.txt");
        
        // Test file doesn't exist initially
        assert!(!db.file_exists(&path).await.unwrap());
        
        // Create file info
        let file_info = create_test_file_info("/test/file.txt");
        
        // Set file info
        db.set_file_info(&path, file_info.clone()).await.unwrap();
        
        // Test file exists now
        assert!(db.file_exists(&path).await.unwrap());
        
        // Get file info
        let retrieved_info = db.get_file_info(&path).await.unwrap();
        assert_eq!(retrieved_info.metadata.id, file_info.metadata.id);
        assert_eq!(retrieved_info.metadata.size, file_info.metadata.size);
        assert_eq!(retrieved_info.version, file_info.version);
        
        // Delete file info
        db.delete_file_info(&path).await.unwrap();
        
        // Test file doesn't exist anymore
        assert!(!db.file_exists(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_directory_operations() {
        let db = create_test_database().await;
        let dir_path = VirtualPath::new("/test/directory");
        
        // Create directory
        db.create_directory(&dir_path).await.unwrap();
        
        // Verify directory exists
        assert!(db.file_exists(&dir_path).await.unwrap());
        
        // Get directory info
        let dir_info = db.get_file_info(&dir_path).await.unwrap();
        assert!(dir_info.metadata.is_directory);
        
        // Remove directory
        db.remove_directory(&dir_path).await.unwrap();
        
        // Verify directory is gone
        assert!(!db.file_exists(&dir_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_search_operations() {
        let db = create_test_database().await;
        
        // Create test files
        let paths = vec![
            "/test/file1.txt",
            "/test/file2.log", 
            "/test/subdir/file3.txt",
            "/other/file4.txt"
        ];
        
        for path in &paths {
            let file_info = create_test_file_info(path);
            let vpath = VirtualPath::new(*path);
            db.set_file_info(&vpath, file_info).await.unwrap();
        }
        
        // Test pattern search
        let results = db.find_files_by_pattern("txt").await.unwrap();
        assert_eq!(results.len(), 3); // Should find 3 .txt files
        
        let results = db.find_files_by_pattern("test").await.unwrap();
        assert_eq!(results.len(), 3); // Should find 3 files in /test/
        
        // Test size search
        let results = db.find_files_by_size(1000, 2000).await.unwrap();
        assert_eq!(results.len(), 4); // All files have size 1024
        
        let results = db.find_files_by_size(2000, 3000).await.unwrap();
        assert_eq!(results.len(), 0); // No files in this size range
    }

    #[tokio::test]
    async fn test_consistency_operations() {
        let db = create_test_database().await;
        
        // Create a file and verify consistency
        let path = VirtualPath::new("/test/file.txt");
        let file_info = create_test_file_info("/test/file.txt");
        db.set_file_info(&path, file_info).await.unwrap();
        
        // Verify consistency (should be empty - no issues)
        let issues = db.verify_consistency().await.unwrap();
        assert_eq!(issues.len(), 0);
        
        // Test repair metadata (should not fail)
        db.repair_metadata(&path).await.unwrap();
        
        // Test rebuild index (should not fail)
        db.rebuild_index().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_with_attributes() {
        let db = create_test_database().await;
        let path = VirtualPath::new("/test/file_with_attrs.txt");
        
        let mut file_info = create_test_file_info("/test/file_with_attrs.txt");
        file_info.metadata.custom_attributes.insert("key1".to_string(), "value1".to_string());
        file_info.metadata.custom_attributes.insert("key2".to_string(), "value2".to_string());
        
        // Set file info with attributes
        db.set_file_info(&path, file_info.clone()).await.unwrap();
        
        // Get file info and verify attributes
        let retrieved_info = db.get_file_info(&path).await.unwrap();
        assert_eq!(retrieved_info.metadata.custom_attributes.len(), 2);
        assert_eq!(retrieved_info.metadata.custom_attributes.get("key1"), Some(&"value1".to_string()));
        assert_eq!(retrieved_info.metadata.custom_attributes.get("key2"), Some(&"value2".to_string()));
    }
}
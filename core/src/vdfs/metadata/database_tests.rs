#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdfs::{VirtualPath, FileId};
    use crate::vdfs::filesystem::FileMetadata;
    use std::collections::HashMap;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    async fn create_test_database() -> DatabaseMetadataManager {
        let temp_file = NamedTempFile::new().unwrap();
        let db_url = format!("sqlite://{}", temp_file.path().to_str().unwrap());
        
        DatabaseMetadataManager::new(&db_url).await.unwrap()
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
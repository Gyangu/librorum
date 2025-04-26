#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_create_file() {
        let test_dir = Path::new("test_data");
        let test_file = test_dir.join("test.txt");
        
        // Clean up any existing test files
        if test_file.exists() {
            fs::remove_file(&test_file).unwrap();
        }
        if test_dir.exists() {
            fs::remove_dir(test_dir).unwrap();
        }
        
        // Create test directory
        fs::create_dir(test_dir).unwrap();
        
        // Test file creation
        let result = create_file(&test_file.to_string_lossy(), "test content");
        assert!(result.is_ok());
        assert!(test_file.exists());
        
        // Verify file contents
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "test content");
        
        // Clean up
        fs::remove_file(&test_file).unwrap();
        fs::remove_dir(test_dir).unwrap();
    }
} 
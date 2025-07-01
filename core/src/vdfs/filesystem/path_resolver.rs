//! Path Resolution Utilities

use crate::vdfs::{VDFSResult, VDFSError, VirtualPath};

/// Path resolver for virtual paths
pub struct PathResolver {
    /// Root path for the virtual file system
    root: VirtualPath,
    /// Whether to allow relative paths
    allow_relative: bool,
    /// Maximum path length
    max_path_length: usize,
}

impl PathResolver {
    pub fn new() -> Self {
        Self {
            root: VirtualPath::new("/"),
            allow_relative: true,
            max_path_length: 4096,
        }
    }
    
    pub fn with_root(root: VirtualPath) -> Self {
        Self {
            root,
            allow_relative: true,
            max_path_length: 4096,
        }
    }
    
    pub fn with_config(root: VirtualPath, allow_relative: bool, max_path_length: usize) -> Self {
        Self {
            root,
            allow_relative,
            max_path_length,
        }
    }
    
    /// Resolve a virtual path to canonical form
    pub fn resolve(&self, path: &VirtualPath) -> VDFSResult<VirtualPath> {
        let path_str = path.as_str();
        
        // Basic validation
        if !self.is_valid(path) {
            return Err(VDFSError::InvalidPath(format!("Invalid path: {}", path_str)));
        }
        
        // Convert to canonical form
        let canonical = self.canonicalize(path_str)?;
        
        // Ensure the path is within bounds
        if !self.is_within_bounds(&canonical) {
            return Err(VDFSError::InvalidPath(format!("Path outside bounds: {}", canonical)));
        }
        
        Ok(VirtualPath::new(canonical))
    }
    
    /// Check if path is valid
    pub fn is_valid(&self, path: &VirtualPath) -> bool {
        let path_str = path.as_str();
        
        // Check length
        if path_str.len() > self.max_path_length {
            return false;
        }
        
        // Check for empty path
        if path_str.is_empty() {
            return false;
        }
        
        // Check for null bytes
        if path_str.contains('\0') {
            return false;
        }
        
        // Check for relative paths if not allowed
        if !self.allow_relative && !path_str.starts_with('/') {
            return false;
        }
        
        // Check for invalid characters (platform-specific)
        if self.contains_invalid_chars(path_str) {
            return false;
        }
        
        // Check for invalid path components
        if self.contains_invalid_components(path_str) {
            return false;
        }
        
        true
    }
    
    /// Normalize a path by resolving '..' and '.' components
    pub fn normalize(&self, path: &str) -> VDFSResult<String> {
        let components: Vec<&str> = path.split('/').collect();
        let mut normalized = Vec::new();
        
        for component in components {
            match component {
                "" | "." => {
                    // Skip empty components and current directory references
                    continue;
                }
                ".." => {
                    // Go up one directory
                    if normalized.is_empty() {
                        // Can't go above root
                        if path.starts_with('/') {
                            continue;
                        } else {
                            return Err(VDFSError::InvalidPath("Path goes above root".to_string()));
                        }
                    } else {
                        normalized.pop();
                    }
                }
                component => {
                    normalized.push(component);
                }
            }
        }
        
        // Reconstruct path
        let mut result = if path.starts_with('/') {
            "/".to_string()
        } else {
            String::new()
        };
        
        if !normalized.is_empty() {
            if result == "/" {
                result.push_str(&normalized.join("/"));
            } else {
                result = normalized.join("/");
            }
        }
        
        // Ensure we have at least "/"
        if result.is_empty() {
            result = "/".to_string();
        }
        
        Ok(result)
    }
    
    /// Convert relative path to absolute
    pub fn make_absolute(&self, path: &str, base: &VirtualPath) -> VDFSResult<VirtualPath> {
        if path.starts_with('/') {
            // Already absolute
            return Ok(VirtualPath::new(path));
        }
        
        // Join with base path
        let absolute = if base.as_str().ends_with('/') {
            format!("{}{}", base.as_str(), path)
        } else {
            format!("{}/{}", base.as_str(), path)
        };
        
        let normalized = self.normalize(&absolute)?;
        Ok(VirtualPath::new(normalized))
    }
    
    /// Get the parent directory of a path
    pub fn parent(&self, path: &VirtualPath) -> Option<VirtualPath> {
        path.parent()
    }
    
    /// Join two paths
    pub fn join(&self, base: &VirtualPath, segment: &str) -> VDFSResult<VirtualPath> {
        // Validate the segment
        if segment.contains('/') || segment.contains('\0') || segment == "." || segment == ".." {
            return Err(VDFSError::InvalidPath(format!("Invalid path segment: {}", segment)));
        }
        
        let joined = base.join(segment);
        let normalized = self.normalize(joined.as_str())?;
        Ok(VirtualPath::new(normalized))
    }
    
    // Private helper methods
    
    fn canonicalize(&self, path: &str) -> VDFSResult<String> {
        // First normalize to resolve . and ..
        let normalized = self.normalize(path)?;
        
        // Additional canonicalization rules
        let mut canonical = normalized;
        
        // Remove trailing slashes except for root
        if canonical.len() > 1 && canonical.ends_with('/') {
            canonical.pop();
        }
        
        // Ensure absolute paths start with /
        if !canonical.starts_with('/') && !self.allow_relative {
            canonical = format!("/{}", canonical);
        }
        
        Ok(canonical)
    }
    
    fn is_within_bounds(&self, path: &str) -> bool {
        // Check if path is within the root boundary
        if self.root.as_str() == "/" {
            return true; // Root is /, so all absolute paths are valid
        }
        
        path.starts_with(self.root.as_str())
    }
    
    fn contains_invalid_chars(&self, path: &str) -> bool {
        // Platform-specific invalid characters
        #[cfg(windows)]
        {
            // Windows reserved characters
            const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
            path.chars().any(|c| INVALID_CHARS.contains(&c) || (c as u32) < 32)
        }
        
        #[cfg(not(windows))]
        {
            // Unix-like systems - only null byte is invalid (already checked)
            false
        }
    }
    
    fn contains_invalid_components(&self, path: &str) -> bool {
        let components: Vec<&str> = path.split('/').collect();
        
        for component in components {
            if component.is_empty() && path != "/" {
                continue; // Allow empty components from consecutive slashes
            }
            
            // Check for reserved names (platform-specific)
            #[cfg(windows)]
            {
                const RESERVED_NAMES: &[&str] = &[
                    "CON", "PRN", "AUX", "NUL",
                    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
                ];
                
                let component_upper = component.to_uppercase();
                if RESERVED_NAMES.contains(&component_upper.as_str()) {
                    return true;
                }
                
                // Check for names ending with period or space
                if component.ends_with('.') || component.ends_with(' ') {
                    return true;
                }
            }
            
            // Check component length
            if component.len() > 255 {
                return true;
            }
        }
        
        false
    }
}

impl Default for PathResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_validation() {
        let resolver = PathResolver::new();
        
        // Valid paths
        assert!(resolver.is_valid(&VirtualPath::new("/home/user/file.txt")));
        assert!(resolver.is_valid(&VirtualPath::new("relative/path")));
        assert!(resolver.is_valid(&VirtualPath::new("/")));
        
        // Invalid paths
        assert!(!resolver.is_valid(&VirtualPath::new(""))); // Empty
        assert!(!resolver.is_valid(&VirtualPath::new("path\0with\0null"))); // Null bytes
        
        // Test with restricted relative paths
        let strict_resolver = PathResolver::with_config(
            VirtualPath::new("/"), 
            false, // No relative paths
            1024
        );
        assert!(!strict_resolver.is_valid(&VirtualPath::new("relative/path")));
        assert!(strict_resolver.is_valid(&VirtualPath::new("/absolute/path")));
    }

    #[test]
    fn test_path_normalization() {
        let resolver = PathResolver::new();
        
        // Test basic normalization
        assert_eq!(resolver.normalize("/home/user/../user/file.txt").unwrap(), "/home/user/file.txt");
        assert_eq!(resolver.normalize("/home/./user/file.txt").unwrap(), "/home/user/file.txt");
        assert_eq!(resolver.normalize("/home//user//file.txt").unwrap(), "/home/user/file.txt");
        
        // Test root path
        assert_eq!(resolver.normalize("/").unwrap(), "/");
        assert_eq!(resolver.normalize("/../..").unwrap(), "/");
        
        // Test relative paths
        assert_eq!(resolver.normalize("user/../file.txt").unwrap(), "file.txt");
        assert_eq!(resolver.normalize("./user/file.txt").unwrap(), "user/file.txt");
    }

    #[test]
    fn test_path_resolution() {
        let resolver = PathResolver::new();
        
        // Test valid path resolution
        let path = VirtualPath::new("/home/user/../user/file.txt");
        let resolved = resolver.resolve(&path).unwrap();
        assert_eq!(resolved.as_str(), "/home/user/file.txt");
        
        // Test invalid path
        let invalid_path = VirtualPath::new("path\0with\0null");
        assert!(resolver.resolve(&invalid_path).is_err());
    }

    #[test]
    fn test_absolute_path_conversion() {
        let resolver = PathResolver::new();
        let base = VirtualPath::new("/home/user");
        
        let absolute = resolver.make_absolute("file.txt", &base).unwrap();
        assert_eq!(absolute.as_str(), "/home/user/file.txt");
        
        let already_absolute = resolver.make_absolute("/etc/config", &base).unwrap();
        assert_eq!(already_absolute.as_str(), "/etc/config");
    }

    #[test]
    fn test_path_joining() {
        let resolver = PathResolver::new();
        let base = VirtualPath::new("/home/user");
        
        let joined = resolver.join(&base, "documents").unwrap();
        assert_eq!(joined.as_str(), "/home/user/documents");
        
        // Test invalid segments
        assert!(resolver.join(&base, "../escape").is_err());
        assert!(resolver.join(&base, "with/slash").is_err());
    }

    #[test]
    fn test_bounded_resolution() {
        let resolver = PathResolver::with_root(VirtualPath::new("/home/user"));
        
        // Path within bounds
        let valid_path = VirtualPath::new("/home/user/documents/file.txt");
        assert!(resolver.resolve(&valid_path).is_ok());
        
        // Path outside bounds
        let invalid_path = VirtualPath::new("/etc/passwd");
        assert!(resolver.resolve(&invalid_path).is_err());
    }

    #[test]
    fn test_parent_directory() {
        let resolver = PathResolver::new();
        
        let path = VirtualPath::new("/home/user/file.txt");
        let parent = resolver.parent(&path).unwrap();
        assert_eq!(parent.as_str(), "/home/user");
        
        let root = VirtualPath::new("/");
        assert!(resolver.parent(&root).is_none());
    }
}
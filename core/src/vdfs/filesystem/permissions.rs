//! Permission Management

use crate::vdfs::{VDFSResult, VDFSError, FilePermissions};
use std::collections::HashMap;

/// User context for permission checks
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub group_id: String,
    pub groups: Vec<String>,
    pub is_admin: bool,
}

impl Default for UserContext {
    fn default() -> Self {
        Self {
            user_id: "default".to_string(),
            group_id: "default".to_string(),
            groups: vec!["default".to_string()],
            is_admin: false,
        }
    }
}

/// File operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Read,
    Write,
    Execute,
    Delete,
    Modify,
    Create,
}

impl Operation {
    pub fn from_str(op: &str) -> VDFSResult<Self> {
        match op.to_lowercase().as_str() {
            "read" | "r" => Ok(Operation::Read),
            "write" | "w" => Ok(Operation::Write),
            "execute" | "x" => Ok(Operation::Execute),
            "delete" | "d" => Ok(Operation::Delete),
            "modify" | "m" => Ok(Operation::Modify),
            "create" | "c" => Ok(Operation::Create),
            _ => Err(VDFSError::InternalError(format!("Unknown operation: {}", op))),
        }
    }
}

/// Permission manager
pub struct PermissionManager {
    /// Access Control List (ACL) for additional permissions
    acl: HashMap<String, HashMap<String, Vec<Operation>>>,
    /// Default umask for new files
    umask: u16,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            acl: HashMap::new(),
            umask: 0o022, // Default umask: rw-r--r--
        }
    }
    
    pub fn with_umask(umask: u16) -> Self {
        Self {
            acl: HashMap::new(),
            umask,
        }
    }
    
    /// Check if operation is allowed for a user
    pub fn check_permission(
        &self, 
        perms: &FilePermissions, 
        operation: &str,
        user: &UserContext,
        file_owner: &str,
        file_group: &str
    ) -> VDFSResult<bool> {
        // Admin users can do anything
        if user.is_admin {
            return Ok(true);
        }
        
        let op = Operation::from_str(operation)?;
        
        // Note: This method doesn't have access to file_path for ACL checks
        // Use check_permission_with_path for ACL-enabled permission checking
        
        // Determine user relationship to file
        let relationship = if user.user_id == file_owner {
            UserRelationship::Owner
        } else if user.group_id == file_group || user.groups.contains(&file_group.to_string()) {
            UserRelationship::Group
        } else {
            UserRelationship::Other
        };
        
        // Check standard Unix permissions
        self.check_unix_permission(perms, op, relationship)
    }
    
    /// Check if operation is allowed for a user with ACL support
    pub fn check_permission_with_path(
        &self, 
        perms: &FilePermissions, 
        operation: &str,
        user: &UserContext,
        file_owner: &str,
        file_group: &str,
        file_path: &str
    ) -> VDFSResult<bool> {
        // Admin users can do anything
        if user.is_admin {
            return Ok(true);
        }
        
        let op = Operation::from_str(operation)?;
        
        // Check ACL first (takes precedence)
        if let Some(acl_result) = self.check_acl(&user.user_id, file_path, op) {
            return Ok(acl_result);
        }
        
        // Determine user relationship to file
        let relationship = if user.user_id == file_owner {
            UserRelationship::Owner
        } else if user.group_id == file_group || user.groups.contains(&file_group.to_string()) {
            UserRelationship::Group
        } else {
            UserRelationship::Other
        };
        
        // Check standard Unix permissions
        self.check_unix_permission(perms, op, relationship)
    }
    
    /// Add ACL entry for a user on a specific file
    pub fn add_acl_entry(&mut self, file_path: &str, user_id: &str, operations: Vec<Operation>) {
        let file_acl = self.acl.entry(file_path.to_string()).or_insert_with(HashMap::new);
        file_acl.insert(user_id.to_string(), operations);
    }
    
    /// Remove ACL entry
    pub fn remove_acl_entry(&mut self, file_path: &str, user_id: &str) {
        if let Some(file_acl) = self.acl.get_mut(file_path) {
            file_acl.remove(user_id);
            if file_acl.is_empty() {
                self.acl.remove(file_path);
            }
        }
    }
    
    /// Convert Unix mode to FilePermissions
    pub fn mode_to_permissions(mode: u16) -> FilePermissions {
        FilePermissions {
            owner_read: (mode & 0o400) != 0,
            owner_write: (mode & 0o200) != 0,
            owner_execute: (mode & 0o100) != 0,
            group_read: (mode & 0o040) != 0,
            group_write: (mode & 0o020) != 0,
            group_execute: (mode & 0o010) != 0,
            other_read: (mode & 0o004) != 0,
            other_write: (mode & 0o002) != 0,
            other_execute: (mode & 0o001) != 0,
        }
    }
    
    /// Convert FilePermissions to Unix mode
    pub fn permissions_to_mode(perms: &FilePermissions) -> u16 {
        let mut mode = 0u16;
        
        if perms.owner_read { mode |= 0o400; }
        if perms.owner_write { mode |= 0o200; }
        if perms.owner_execute { mode |= 0o100; }
        if perms.group_read { mode |= 0o040; }
        if perms.group_write { mode |= 0o020; }
        if perms.group_execute { mode |= 0o010; }
        if perms.other_read { mode |= 0o004; }
        if perms.other_write { mode |= 0o002; }
        if perms.other_execute { mode |= 0o001; }
        
        mode
    }
    
    /// Apply umask to permissions
    pub fn apply_umask(&self, perms: &FilePermissions) -> FilePermissions {
        let mode = Self::permissions_to_mode(perms);
        let masked_mode = mode & !self.umask;
        Self::mode_to_permissions(masked_mode)
    }
    
    /// Create default permissions for new files
    pub fn default_file_permissions(&self) -> FilePermissions {
        let default_mode = 0o644; // rw-r--r--
        let masked_mode = default_mode & !self.umask;
        Self::mode_to_permissions(masked_mode)
    }
    
    /// Create default permissions for new directories
    pub fn default_directory_permissions(&self) -> FilePermissions {
        let default_mode = 0o755; // rwxr-xr-x
        let masked_mode = default_mode & !self.umask;
        Self::mode_to_permissions(masked_mode)
    }
    
    // Private helper methods
    
    fn check_acl(&self, user_id: &str, file_path: &str, operation: Operation) -> Option<bool> {
        self.acl
            .get(file_path)?
            .get(user_id)
            .map(|ops| ops.contains(&operation))
    }
    
    fn check_unix_permission(
        &self,
        perms: &FilePermissions,
        operation: Operation,
        relationship: UserRelationship
    ) -> VDFSResult<bool> {
        let allowed = match (relationship, operation) {
            // Owner permissions
            (UserRelationship::Owner, Operation::Read) => perms.owner_read,
            (UserRelationship::Owner, Operation::Write) => perms.owner_write,
            (UserRelationship::Owner, Operation::Execute) => perms.owner_execute,
            (UserRelationship::Owner, Operation::Delete) => perms.owner_write, // Need write permission to delete
            (UserRelationship::Owner, Operation::Modify) => perms.owner_write,
            (UserRelationship::Owner, Operation::Create) => perms.owner_write,
            
            // Group permissions
            (UserRelationship::Group, Operation::Read) => perms.group_read,
            (UserRelationship::Group, Operation::Write) => perms.group_write,
            (UserRelationship::Group, Operation::Execute) => perms.group_execute,
            (UserRelationship::Group, Operation::Delete) => perms.group_write,
            (UserRelationship::Group, Operation::Modify) => perms.group_write,
            (UserRelationship::Group, Operation::Create) => perms.group_write,
            
            // Other permissions
            (UserRelationship::Other, Operation::Read) => perms.other_read,
            (UserRelationship::Other, Operation::Write) => perms.other_write,
            (UserRelationship::Other, Operation::Execute) => perms.other_execute,
            (UserRelationship::Other, Operation::Delete) => perms.other_write,
            (UserRelationship::Other, Operation::Modify) => perms.other_write,
            (UserRelationship::Other, Operation::Create) => perms.other_write,
        };
        
        Ok(allowed)
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UserRelationship {
    Owner,
    Group,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_conversion() {
        let mode = 0o755; // rwxr-xr-x
        let perms = PermissionManager::mode_to_permissions(mode);
        
        assert!(perms.owner_read);
        assert!(perms.owner_write);
        assert!(perms.owner_execute);
        assert!(perms.group_read);
        assert!(!perms.group_write);
        assert!(perms.group_execute);
        assert!(perms.other_read);
        assert!(!perms.other_write);
        assert!(perms.other_execute);
        
        let converted_mode = PermissionManager::permissions_to_mode(&perms);
        assert_eq!(mode, converted_mode);
    }

    #[test]
    fn test_owner_permissions() {
        let manager = PermissionManager::new();
        let perms = PermissionManager::mode_to_permissions(0o644); // rw-r--r--
        
        let owner = UserContext {
            user_id: "alice".to_string(),
            group_id: "users".to_string(),
            groups: vec!["users".to_string()],
            is_admin: false,
        };
        
        // Owner should be able to read and write
        assert!(manager.check_permission(&perms, "read", &owner, "alice", "users").unwrap());
        assert!(manager.check_permission(&perms, "write", &owner, "alice", "users").unwrap());
        assert!(!manager.check_permission(&perms, "execute", &owner, "alice", "users").unwrap());
    }

    #[test]
    fn test_group_permissions() {
        let manager = PermissionManager::new();
        let perms = PermissionManager::mode_to_permissions(0o644); // rw-r--r--
        
        let group_user = UserContext {
            user_id: "bob".to_string(),
            group_id: "users".to_string(),
            groups: vec!["users".to_string()],
            is_admin: false,
        };
        
        // Group member should be able to read but not write
        assert!(manager.check_permission(&perms, "read", &group_user, "alice", "users").unwrap());
        assert!(!manager.check_permission(&perms, "write", &group_user, "alice", "users").unwrap());
    }

    #[test]
    fn test_other_permissions() {
        let manager = PermissionManager::new();
        let perms = PermissionManager::mode_to_permissions(0o644); // rw-r--r--
        
        let other_user = UserContext {
            user_id: "charlie".to_string(),
            group_id: "other".to_string(),
            groups: vec!["other".to_string()],
            is_admin: false,
        };
        
        // Other user should be able to read but not write
        assert!(manager.check_permission(&perms, "read", &other_user, "alice", "users").unwrap());
        assert!(!manager.check_permission(&perms, "write", &other_user, "alice", "users").unwrap());
    }

    #[test]
    fn test_admin_permissions() {
        let manager = PermissionManager::new();
        let perms = PermissionManager::mode_to_permissions(0o000); // No permissions
        
        let admin_user = UserContext {
            user_id: "admin".to_string(),
            group_id: "admin".to_string(),
            groups: vec!["admin".to_string()],
            is_admin: true,
        };
        
        // Admin should be able to do anything
        assert!(manager.check_permission(&perms, "read", &admin_user, "alice", "users").unwrap());
        assert!(manager.check_permission(&perms, "write", &admin_user, "alice", "users").unwrap());
        assert!(manager.check_permission(&perms, "execute", &admin_user, "alice", "users").unwrap());
    }

    #[test]
    fn test_acl_permissions() {
        let mut manager = PermissionManager::new();
        let perms = PermissionManager::mode_to_permissions(0o600); // rw-------
        
        let special_user = UserContext {
            user_id: "special".to_string(),
            group_id: "other".to_string(),
            groups: vec!["other".to_string()],
            is_admin: false,
        };
        
        // Initially, special user should not have access
        assert!(!manager.check_permission_with_path(&perms, "read", &special_user, "alice", "users", "/test/file").unwrap());
        
        // Add ACL entry for special user
        manager.add_acl_entry("/test/file", "special", vec![Operation::Read, Operation::Write]);
        
        // Now special user should have access via ACL
        assert!(manager.check_permission_with_path(&perms, "read", &special_user, "alice", "users", "/test/file").unwrap());
        assert!(manager.check_permission_with_path(&perms, "write", &special_user, "alice", "users", "/test/file").unwrap());
        assert!(!manager.check_permission_with_path(&perms, "execute", &special_user, "alice", "users", "/test/file").unwrap());
        
        // Test that the old method still works for basic Unix permissions
        assert!(!manager.check_permission(&perms, "read", &special_user, "alice", "users").unwrap());
    }

    #[test]
    fn test_umask() {
        let manager = PermissionManager::with_umask(0o022);
        
        let file_perms = manager.default_file_permissions();
        let dir_perms = manager.default_directory_permissions();
        
        // File permissions should be 644 (666 & !022)
        assert!(file_perms.owner_read && file_perms.owner_write && !file_perms.owner_execute);
        assert!(file_perms.group_read && !file_perms.group_write && !file_perms.group_execute);
        assert!(file_perms.other_read && !file_perms.other_write && !file_perms.other_execute);
        
        // Directory permissions should be 755 (777 & !022)
        assert!(dir_perms.owner_read && dir_perms.owner_write && dir_perms.owner_execute);
        assert!(dir_perms.group_read && !dir_perms.group_write && dir_perms.group_execute);
        assert!(dir_perms.other_read && !dir_perms.other_write && dir_perms.other_execute);
    }
}
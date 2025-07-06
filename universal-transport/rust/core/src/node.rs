//! Node information and discovery

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Information about a communication node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: String,
    /// Programming language of the node
    pub language: Language,
    /// Machine identifier (for detecting local vs remote)
    pub machine_id: String,
    /// Network endpoint (if remote)
    pub endpoint: Option<String>,
    /// Shared memory region name (if local)
    pub shared_memory_name: Option<String>,
    /// Additional node metadata
    pub metadata: HashMap<String, String>,
    /// Node capabilities
    pub capabilities: NodeCapabilities,
}

impl NodeInfo {
    /// Create a new node info
    pub fn new(id: impl Into<String>, language: Language) -> Self {
        Self {
            id: id.into(),
            language,
            machine_id: get_machine_id(),
            endpoint: None,
            shared_memory_name: None,
            metadata: HashMap::new(),
            capabilities: NodeCapabilities::default(),
        }
    }
    
    /// Create a local node (same machine)
    pub fn local(id: impl Into<String>, language: Language) -> Self {
        let mut node = Self::new(id, language);
        node.shared_memory_name = Some(format!("utp_{}", Uuid::new_v4()));
        node
    }
    
    /// Create a remote node
    pub fn remote(id: impl Into<String>, language: Language, endpoint: impl Into<String>) -> Self {
        let mut node = Self::new(id, language);
        node.endpoint = Some(endpoint.into());
        node.machine_id = format!("remote_{}", Uuid::new_v4());
        node
    }
    
    /// Check if this node is on the same machine
    pub fn is_local_machine(&self) -> bool {
        self.machine_id == get_machine_id()
    }
    
    /// Get the shared memory region name for communication with this node
    pub fn get_shared_memory_name(&self, other: &NodeInfo) -> String {
        let mut ids = vec![&self.id, &other.id];
        ids.sort();
        format!("utp_{}_{}", ids[0], ids[1])
    }
}

/// Programming language enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Swift,
}

/// Node capabilities
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Supported transport types
    pub supported_transports: Vec<crate::TransportType>,
    /// Maximum message size
    pub max_message_size: usize,
    /// Supports compression
    pub supports_compression: bool,
    /// Supports encryption
    pub supports_encryption: bool,
    /// Protocol version
    pub protocol_version: String,
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            supported_transports: vec![
                crate::TransportType::Universal,
            ],
            max_message_size: 64 * 1024 * 1024, // 64MB
            supports_compression: false,
            supports_encryption: false,
            protocol_version: crate::VERSION.to_string(),
        }
    }
}

/// Get the current machine identifier
pub fn get_machine_id() -> String {
    use std::sync::OnceLock;
    static MACHINE_ID: OnceLock<String> = OnceLock::new();
    
    MACHINE_ID.get_or_init(|| {
        // Try to get a stable machine identifier
        #[cfg(unix)]
        {
            use std::fs;
            if let Ok(id) = fs::read_to_string("/etc/machine-id") {
                return id.trim().to_string();
            }
        }
        
        // Fallback to hostname + process ID
        format!("{}_{}", 
                hostname::get().unwrap_or_default().to_string_lossy(),
                std::process::id())
    }).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = NodeInfo::new("test-node", Language::Rust);
        assert_eq!(node.id, "test-node");
        assert_eq!(node.language, Language::Rust);
        assert!(node.is_local_machine());
    }

    #[test]
    fn test_shared_memory_name() {
        let node1 = NodeInfo::new("node1", Language::Rust);
        let node2 = NodeInfo::new("node2", Language::Swift);
        
        let name = node1.get_shared_memory_name(&node2);
        assert!(name.starts_with("utp_"));
        assert!(name.contains("node1"));
        assert!(name.contains("node2"));
    }
}
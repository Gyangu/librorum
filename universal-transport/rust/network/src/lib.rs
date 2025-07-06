//! Universal Transport Protocol - Network Module
//! 
//! Network transport implementations for different protocols

pub mod protocol;
pub mod swift;
pub mod rust_transport;
pub mod universal;

pub use protocol::*;

/// Re-export transport implementations
pub use swift::SwiftNetworkTransport;
pub use rust_transport::RustNetworkTransport;
pub use universal::UniversalNetworkTransport;

/// Network transport configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Default timeout for operations
    pub default_timeout_ms: u64,
    /// Enable compression
    pub enable_compression: bool,
    /// Buffer size for network operations
    pub buffer_size: usize,
    /// Maximum message size
    pub max_message_size: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30000,
            enable_compression: false,
            buffer_size: 64 * 1024,
            max_message_size: 64 * 1024 * 1024,
        }
    }
}
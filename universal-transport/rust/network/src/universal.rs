//! Universal compatibility network transport

use crate::protocol::{NetworkMessageHeader, MessageType, UNIVERSAL_PROTOCOL_MAGIC, PROTOCOL_VERSION};
use async_trait::async_trait;
use bytes::Bytes;
// TODO: Import from core once circular dependency is resolved

/// Universal compatibility network transport
pub struct UniversalNetworkTransport {
    // TODO: Implement actual network transport
}

impl UniversalNetworkTransport {
    /// Create a new universal network transport
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for UniversalNetworkTransport {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implement Transport trait once core types are stabilized
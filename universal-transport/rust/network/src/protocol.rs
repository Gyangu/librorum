//! Network protocol definitions

use serde::{Deserialize, Serialize};

/// Network message header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessageHeader {
    /// Protocol magic number
    pub magic: u32,
    /// Protocol version
    pub version: u8,
    /// Message type
    pub message_type: MessageType,
    /// Payload size
    pub payload_size: u32,
    /// Sequence number
    pub sequence: u64,
    /// Checksum
    pub checksum: u32,
}

/// Network message types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Data,
    Heartbeat,
    Acknowledgment,
    Error,
}

/// Network protocol magic numbers
pub const SWIFT_PROTOCOL_MAGIC: u32 = 0x53574654; // "SWFT"
pub const RUST_PROTOCOL_MAGIC: u32 = 0x52555354;  // "RUST"
pub const UNIVERSAL_PROTOCOL_MAGIC: u32 = 0x554E4956; // "UNIV"

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;
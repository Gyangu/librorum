//! Universal Transport Protocol - Shared Memory Module
//! 
//! High-performance cross-platform shared memory transport implementation

pub mod platform;
pub mod transport;
pub mod region;
pub mod protocol;
pub mod error;
pub mod adapter;

pub use transport::*;
pub use region::*;
pub use protocol::*;
pub use error::*;
pub use adapter::*;

/// Re-export platform-specific implementations
pub use platform::*;

/// Current version of the shared memory protocol
pub const SHARED_MEMORY_VERSION: u8 = 1;

/// Default shared memory region size (64MB)
pub const DEFAULT_REGION_SIZE: usize = 64 * 1024 * 1024;

/// Shared memory protocol magic number
pub const SHARED_MEMORY_MAGIC: u32 = 0x534D454D; // "SMEM"
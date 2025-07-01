// Config module moved to librorum-shared
pub mod logger;
pub mod daemon;
pub mod node_manager;
pub mod proto;
pub mod vdfs;

// Re-export most common types for convenience
pub use node_manager::NodeManager;
pub use vdfs::{VDFS, VDFSConfig, VirtualPath};

// Re-export log macros
pub use tracing::{info, warn, error, debug, trace};
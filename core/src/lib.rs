pub mod config;
pub mod logger;
pub mod daemon;
pub mod node_manager;
pub mod proto;

// Re-export most common types for convenience
pub use config::NodeConfig;
pub use node_manager::NodeManager;

// Re-export log macros
pub use tracing::{info, warn, error, debug, trace};
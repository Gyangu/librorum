pub mod config;
pub mod proto;
pub mod utils;
pub mod data_portal;

// Re-export commonly used types
pub use config::NodeConfig;

// Re-export gRPC generated code
pub use proto::*;

// Re-export Data Portal components
pub use data_portal::{
    DataPortalServer, DataPortalClient, DataPortalConfig,
};
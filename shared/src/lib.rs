pub mod config;
pub mod proto;
pub mod utils;
pub mod transport;

// Re-export commonly used types
pub use config::NodeConfig;

// Re-export gRPC generated code
pub use proto::*;

// Re-export UTP transport types
pub use transport::{
    UtpManager, UtpConfig, UtpTransport, UtpResult, UtpError, 
    UtpEvent, UtpSession, UtpStats, TransportMode,
    UtpTransportFactory,
};

// Re-export UTP server and client
pub use transport::{
    server::{UtpServer, ServerStatus},
    client::{UtpClient, UploadResult, DownloadResult, ConnectionStatus},
};
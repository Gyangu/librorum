pub mod config;
pub mod proto;
pub mod utils;
pub mod transport;
pub mod data_portal;
pub mod zero_copy_server;

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

// Re-export Data Portal components
pub use data_portal::{
    DataPortalServer, DataPortalClient, DataPortalConfig,
};

// Re-export Zero Copy Server
pub use zero_copy_server::{
    ZeroCopyDataPortalServer, ZeroCopyHeader,
};
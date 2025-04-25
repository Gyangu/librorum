use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum VDFSError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Node error: {0}")]
    NodeError(String),

    #[error("Sync error: {0}")]
    SyncError(String),

    #[error("gRPC error: {0}")]
    Grpc(#[from] Status),
}

pub type Result<T> = std::result::Result<T, VDFSError>; 
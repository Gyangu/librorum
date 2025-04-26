use std::io;
use tonic::Status;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("File system error: {0}")]
    FileSystem(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Node error: {0}")]
    NodeError(String),
    
    #[error("Metadata error: {0}")]
    Metadata(String),
    
    #[error("Sync error: {0}")]
    Sync(String),
    
    #[error("gRPC error: {0}")]
    Grpc(#[from] Status),
    
    #[error("gRPC transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

// 为了兼容性，我们将 Error 类型别名为 VDFSError
pub type VDFSError = Error;

impl Error {
    pub fn new(msg: &str) -> Self {
        Self::Unknown(msg.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Unknown(s)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
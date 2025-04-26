use thiserror::Error;
use toml;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    
    #[error("gRPC transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("General error: {0}")]
    General(String),
}

// 为了兼容性，我们将 Error 类型别名为 VDFSError
pub type VDFSError = Error;

impl Error {
    pub fn new(msg: &str) -> Self {
        Self::General(msg.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::General(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
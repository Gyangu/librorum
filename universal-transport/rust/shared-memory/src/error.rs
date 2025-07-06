//! Shared memory specific error types

use thiserror::Error;

/// Shared memory error types
#[derive(Error, Debug)]
pub enum SharedMemoryError {
    /// Platform-specific error
    #[error("Platform error: {0}")]
    Platform(String),
    
    /// Region not found
    #[error("Shared memory region not found: {0}")]
    RegionNotFound(String),
    
    /// Region already exists
    #[error("Shared memory region already exists: {0}")]
    RegionExists(String),
    
    /// Invalid region size
    #[error("Invalid region size: {size}, must be between {min} and {max}")]
    InvalidSize { size: usize, min: usize, max: usize },
    
    /// Memory mapping failed
    #[error("Memory mapping failed: {0}")]
    MappingFailed(String),
    
    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Data corruption
    #[error("Data corruption detected: {0}")]
    DataCorruption(String),
    
    /// Timeout
    #[error("Operation timed out: {0}")]
    Timeout(String),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience type alias
pub type Result<T> = std::result::Result<T, SharedMemoryError>;

impl SharedMemoryError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            SharedMemoryError::Timeout(_) => true,
            SharedMemoryError::Io(err) => {
                matches!(err.kind(), 
                    std::io::ErrorKind::TimedOut |
                    std::io::ErrorKind::WouldBlock |
                    std::io::ErrorKind::Interrupted
                )
            }
            _ => false,
        }
    }
    
    /// Convert platform-specific error codes to SharedMemoryError
    pub fn from_platform_error(error: i32, message: impl Into<String>) -> Self {
        match error {
            13 => SharedMemoryError::PermissionDenied(message.into()), // EACCES
            2 => SharedMemoryError::RegionNotFound(message.into()),    // ENOENT
            17 => SharedMemoryError::RegionExists(message.into()),     // EEXIST
            _ => SharedMemoryError::Platform(format!("Error {}: {}", error, message.into())),
        }
    }
}
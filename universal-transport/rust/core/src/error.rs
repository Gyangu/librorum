//! Error types for Universal Transport Protocol

use thiserror::Error;

/// Transport error types
#[derive(Error, Debug)]
pub enum TransportError {
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Network errors
    #[error("Network error: {0}")]
    Network(String),
    
    /// Shared memory errors
    #[error("Shared memory error: {0}")]
    SharedMemory(String),
    
    /// Node not found
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
    /// Transport not available
    #[error("Transport not available: {0:?}")]
    TransportNotAvailable(crate::TransportType),
    
    /// Timeout error
    #[error("Operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Protocol version mismatch
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
    
    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    /// Resource exhausted
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
    
    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience type alias for Results
pub type Result<T> = std::result::Result<T, TransportError>;

impl TransportError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            TransportError::Timeout { .. } => true,
            TransportError::Network(_) => true,
            TransportError::ResourceExhausted(_) => true,
            TransportError::Io(err) => {
                matches!(err.kind(), 
                    std::io::ErrorKind::TimedOut |
                    std::io::ErrorKind::WouldBlock |
                    std::io::ErrorKind::Interrupted
                )
            }
            _ => false,
        }
    }
    
    /// Get error category for metrics
    pub fn category(&self) -> ErrorCategory {
        match self {
            TransportError::Io(_) => ErrorCategory::Io,
            TransportError::Serialization(_) => ErrorCategory::Serialization,
            TransportError::Network(_) => ErrorCategory::Network,
            TransportError::SharedMemory(_) => ErrorCategory::SharedMemory,
            TransportError::NodeNotFound(_) => ErrorCategory::Configuration,
            TransportError::TransportNotAvailable(_) => ErrorCategory::Configuration,
            TransportError::Timeout { .. } => ErrorCategory::Timeout,
            TransportError::Configuration(_) => ErrorCategory::Configuration,
            TransportError::VersionMismatch { .. } => ErrorCategory::Protocol,
            TransportError::Authentication(_) => ErrorCategory::Security,
            TransportError::PermissionDenied(_) => ErrorCategory::Security,
            TransportError::ResourceExhausted(_) => ErrorCategory::Resource,
            TransportError::InvalidData(_) => ErrorCategory::Protocol,
            TransportError::Internal(_) => ErrorCategory::Internal,
        }
    }
}

/// Error categories for metrics and handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Io,
    Network,
    SharedMemory,
    Serialization,
    Protocol,
    Security,
    Configuration,
    Resource,
    Timeout,
    Internal,
}

/// Convert from common error types
impl From<bincode::Error> for TransportError {
    fn from(err: bincode::Error) -> Self {
        TransportError::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for TransportError {
    fn from(err: serde_json::Error) -> Self {
        TransportError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recovery() {
        let timeout_error = TransportError::Timeout { timeout_ms: 1000 };
        assert!(timeout_error.is_recoverable());
        
        let config_error = TransportError::Configuration("Invalid config".to_string());
        assert!(!config_error.is_recoverable());
    }
    
    #[test]
    fn test_error_categories() {
        let network_error = TransportError::Network("Connection failed".to_string());
        assert_eq!(network_error.category(), ErrorCategory::Network);
        
        let timeout_error = TransportError::Timeout { timeout_ms: 1000 };
        assert_eq!(timeout_error.category(), ErrorCategory::Timeout);
    }
}
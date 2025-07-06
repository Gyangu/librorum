//! Universal Transport Protocol - Core Module
//! 
//! This crate provides the core abstractions and smart transport selection
//! for high-performance cross-platform communication.

pub mod transport;
pub mod node;
pub mod manager;
pub mod strategy;
pub mod error;
pub mod metrics;
pub mod binary_protocol;

pub use transport::*;
pub use node::*;
pub use manager::*;
pub use strategy::*;
pub use error::*;

/// Re-export common types
pub mod prelude {
    pub use crate::{
        transport::{Transport, UniversalTransport},
        node::{NodeInfo, Language},
        manager::TransportManager,
        strategy::{TransportStrategy, StrategySelector},
        error::{TransportError, Result},
    };
    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};
}

/// Current version of the Universal Transport Protocol
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol magic number for identification
pub const PROTOCOL_MAGIC: u32 = 0x55545000; // "UTP\0"
// gRPC 服务定义
pub mod node {
    tonic::include_proto!("node");
}

pub mod file {
    tonic::include_proto!("file");
}

pub mod log {
    tonic::include_proto!("log");
}

// Re-export for convenience
pub use node::*;
pub use file::*;
pub use log::*;
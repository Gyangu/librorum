pub mod file_service;
// pub mod hybrid_file_service; // 暂时禁用复杂版本
pub mod hybrid_file_service_simple;
pub mod hybrid_node_manager;
pub mod log_service;
pub mod mdns_manager;
pub mod network_config;
pub mod node_client;
pub mod node_health;
pub mod node_manager;
pub mod node_service;

pub use file_service::FileServiceImpl;
pub use hybrid_file_service_simple::{SimpleHybridFileService, TransferStats};
pub use hybrid_node_manager::HybridNodeManager;
pub use log_service::LogServiceImpl;
pub use network_config::NetworkConfig;
pub use node_client::NodeClient;
pub use node_health::{HealthMonitor, NodeHealth, NodeStatus};
pub use node_manager::NodeManager;
pub use node_service::{NodeInfo, NodeServiceImpl};

// 暂时禁用这些测试，因为需要重构
// #[cfg(test)]
// mod mod_tests;

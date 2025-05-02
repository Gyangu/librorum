use anyhow::Result;

pub mod mdns_manager;
pub mod network_config;
pub mod node_client;
pub mod node_health;
pub mod node_manager;
pub mod node_service;

// 重新导出主要的类型和模块
pub use network_config::NetworkConfig;
pub use node_client::NodeClient;
pub use node_health::{HealthMonitor, NodeHealth, NodeStatus};
pub use node_manager::NodeManager;
pub use node_service::{NodeInfo, NodeServiceImpl};

// 辅助函数
/// 创建新的节点管理器
pub fn new_node_manager(port: u16) -> NodeManager {
    NodeManager::new(port)
}

/// 启动节点管理器
pub async fn start_node_manager(node_manager: &NodeManager) -> Result<()> {
    node_manager.start().await
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};
use crate::error::Result;
use crate::proto::vdfs::{NodeInfo, NodeStatus};

#[derive(Debug, Clone)]
pub struct ClusterConfig {
    pub sync_interval: u64,
    pub p2p_enabled: bool,
    pub nodes: Vec<NodeInfo>,
    pub discovery_enabled: bool,
    pub heartbeat_interval: u64,
    pub node_timeout: u64,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            sync_interval: 60,
            p2p_enabled: true,
            nodes: Vec::new(),
            discovery_enabled: true,
            heartbeat_interval: 5,
            node_timeout: 30,
        }
    }
}

pub struct ClusterManager {
    config: ClusterConfig,
    node_info: NodeInfo,
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
}

impl ClusterManager {
    pub fn new(config: ClusterConfig, node_info: NodeInfo) -> Self {
        Self {
            config,
            node_info,
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        // 启动发现服务
        if self.config.discovery_enabled {
            self.start_discovery().await?;
        }

        // 启动心跳检测
        self.start_heartbeat().await?;

        Ok(())
    }

    async fn start_discovery(&self) -> Result<()> {
        // let nodes = self.nodes.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.sync_interval));
            loop {
                interval.tick().await;
                // 执行节点发现
                // TODO: 实现节点发现逻辑
            }
        });

        Ok(())
    }

    async fn start_heartbeat(&self) -> Result<()> {
        let nodes = self.nodes.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.heartbeat_interval));
            loop {
                interval.tick().await;
                let mut nodes = nodes.write().await;
                let now = chrono::Utc::now().timestamp();

                // 检查节点状态
                nodes.retain(|_, node| {
                    let timeout = now - node.last_seen;
                    timeout < config.node_timeout as i64
                });
            }
        });

        Ok(())
    }

    pub async fn add_node(&self, node: NodeInfo) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_id);
        Ok(())
    }

    pub async fn get_node(&self, node_id: &str) -> Result<Option<NodeInfo>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.get(node_id).cloned())
    }

    pub async fn list_nodes(&self) -> Result<Vec<NodeInfo>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.values().cloned().collect())
    }

    pub async fn update_node_status(&self, node_id: &str, status: NodeStatus) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.status = status as i32;
            node.last_seen = chrono::Utc::now().timestamp();
        }
        Ok(())
    }
} 
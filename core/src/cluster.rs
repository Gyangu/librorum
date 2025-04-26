use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time;
use crate::proto::vdfs::{NodeInfo, NodeStatus, ClusterInfo};

const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const NODE_TIMEOUT: Duration = Duration::from_secs(30);

type NodeMap = HashMap<String, (NodeInfo, Instant)>;

#[derive(Clone)]
pub struct ClusterConfig {
    pub id: String,
    pub name: String,
    pub discovery_enabled: bool,
    pub discovery_port: i32,
    pub heartbeat_interval: Duration,
    pub node_timeout: Duration,
    pub join_token: Option<String>,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "VDFS Cluster".to_string(),
            discovery_enabled: true,
            discovery_port: 5353,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            node_timeout: NODE_TIMEOUT,
            join_token: None,
        }
    }
}

pub struct ClusterManager {
    pub config: ClusterConfig,
    nodes: Arc<Mutex<NodeMap>>,
    local_node: NodeInfo,
    heartbeat_tx: Option<mpsc::Sender<HeartbeatCommand>>,
}

enum HeartbeatCommand {
    Stop,
}

impl ClusterManager {
    pub fn new(config: ClusterConfig, local_node: NodeInfo) -> Self {
        Self {
            config,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            local_node,
            heartbeat_tx: None,
        }
    }

    pub async fn start(&mut self) {
        // 启动心跳定时器
        self.start_heartbeat_timer().await;
        
        // 如果启用了节点发现，启动发现服务
        if self.config.discovery_enabled {
            self.start_discovery_service().await;
        }
    }

    pub async fn stop(&mut self) {
        // 停止心跳定时器
        if let Some(tx) = &self.heartbeat_tx {
            let _ = tx.send(HeartbeatCommand::Stop).await;
            self.heartbeat_tx = None;
        }
    }

    pub fn get_cluster_info(&self) -> ClusterInfo {
        let nodes = self.nodes.lock().unwrap();
        
        ClusterInfo {
            nodes: nodes.values().map(|(info, _)| info.clone()).collect(),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    pub fn register_node(&self, node_info: NodeInfo) -> Result<(), String> {
        let mut nodes = self.nodes.lock().unwrap();
        let now = Instant::now();
        
        nodes.insert(node_info.id.clone(), (node_info, now));
        Ok(())
    }

    pub fn remove_node(&self, node_id: &str) -> Result<(), String> {
        let mut nodes = self.nodes.lock().unwrap();
        
        if nodes.remove(node_id).is_some() {
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    pub fn update_node_status(&self, node_id: &str, status: NodeStatus, last_seen: i64) -> Result<(), String> {
        let mut nodes = self.nodes.lock().unwrap();
        
        if let Some((node_info, last_heartbeat)) = nodes.get_mut(node_id) {
            node_info.status = status as i32;
            node_info.last_seen = last_seen;
            *last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    pub fn get_node(&self, node_id: &str) -> Option<NodeInfo> {
        let nodes = self.nodes.lock().unwrap();
        
        nodes.get(node_id).map(|(info, _)| info.clone())
    }

    pub fn get_all_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.lock().unwrap();
        
        nodes.values().map(|(info, _)| info.clone()).collect()
    }

    async fn start_heartbeat_timer(&mut self) {
        let (tx, mut rx) = mpsc::channel::<HeartbeatCommand>(1);
        self.heartbeat_tx = Some(tx);
        
        let nodes = self.nodes.clone();
        let interval = self.config.heartbeat_interval;
        let timeout = self.config.node_timeout;
        
        tokio::spawn(async move {
            let mut ticker = time::interval(interval);
            
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // 检查节点超时
                        let mut nodes_to_update = Vec::new();
                        {
                            let mut nodes_lock = nodes.lock().unwrap();
                            let now = Instant::now();
                            
                            for (id, (info, last_heartbeat)) in nodes_lock.iter_mut() {
                                if now.duration_since(*last_heartbeat) > timeout {
                                    if info.status != NodeStatus::NodeOffline as i32 {
                                        // 标记节点为离线
                                        info.status = NodeStatus::NodeOffline as i32;
                                        nodes_to_update.push(id.clone());
                                    }
                                }
                            }
                        }
                        
                        // 处理超时节点...
                    }
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            HeartbeatCommand::Stop => break,
                        }
                    }
                }
            }
        });
    }

    async fn start_discovery_service(&self) {
        // 实现节点发现服务，可使用 mDNS 或其他服务发现机制
        // 注意：这里只是框架，实际实现较复杂
    }
}

impl Drop for ClusterManager {
    fn drop(&mut self) {
        // 确保在析构时停止所有服务
        if let Some(tx) = &self.heartbeat_tx {
            // 在同步上下文中使用 try_send 而不是 send
            let _ = tx.try_send(HeartbeatCommand::Stop);
        }
    }
} 
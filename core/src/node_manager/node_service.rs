use crate::proto::node::{HeartbeatRequest, HeartbeatResponse};
use crate::proto::node::node_service_server::NodeService;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use chrono::Utc;

/// 节点信息结构体
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub system: String,
    pub last_seen: i64,
}

/// 节点服务器实现
#[derive(Debug)]
pub struct NodeServiceImpl {
    pub node_id: String,
    pub address: String,
    pub system_info: String,
    pub nodes: Arc<Mutex<Vec<NodeInfo>>>,
}

impl NodeServiceImpl {
    /// 创建新的节点服务实现
    pub fn new(node_id: String, address: String, system_info: String) -> Self {
        Self {
            node_id: node_id.clone(),
            address,
            system_info,
            nodes: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[tonic::async_trait]
impl NodeService for NodeServiceImpl {
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        let timestamp = Utc::now().timestamp();
        
        // 记录来自其他节点的心跳
        let remote_node = NodeInfo {
            id: req.node_id.clone(),
            address: req.address.clone(),
            system: req.system_info.clone(),
            last_seen: timestamp,
        };
        
        // 更新节点列表
        let mut nodes = self.nodes.lock().await;
        
        // 检查是否已存在该节点
        if let Some(idx) = nodes.iter().position(|n| n.id == remote_node.id) {
            nodes[idx] = remote_node;
            tracing::info!("更新节点: {}", req.node_id);
        } else {
            nodes.push(remote_node);
            tracing::info!("发现新节点: {}", req.node_id);
        }
        
        // 构造响应
        let reply = HeartbeatResponse {
            node_id: self.node_id.clone(),
            address: self.address.clone(),
            system_info: self.system_info.clone(),
            timestamp,
            status: true,
        };
        
        Ok(Response::new(reply))
    }
} 
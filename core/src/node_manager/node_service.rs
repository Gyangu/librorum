use crate::proto::node::node_service_server::NodeService;
use crate::proto::node::{HeartbeatRequest, HeartbeatResponse};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use tracing::{debug, info};

use crate::node_manager::node_health::HealthMonitor;

/// 节点信息结构体
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub system: String,
    pub last_seen: i64,
}

/// 节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum NodeConnectionStatus {
    /// 在线
    Online,
    /// 离线
    Offline,
}

/// 节点连接详情
#[derive(Debug, Clone)]
pub struct NodeConnection {
    /// 节点信息
    pub info: NodeInfo,
    /// 连接状态
    pub status: NodeConnectionStatus,
    /// 最后一次连接时间
    pub last_connection: i64,
    /// 连接成功次数
    pub success_count: u32,
    /// 连接失败次数
    pub failure_count: u32,
}

/// 节点服务器实现
#[derive(Debug)]
pub struct NodeServiceImpl {
    pub node_id: String,
    pub address: String,
    pub system_info: String,
    pub nodes: Arc<Mutex<HashMap<String, NodeConnection>>>,
    pub health_monitor: Option<Arc<HealthMonitor>>,
}

impl NodeServiceImpl {
    /// 创建新的节点服务实现
    pub fn new(node_id: String, address: String, system_info: String) -> Self {
        Self {
            node_id: node_id.clone(),
            address,
            system_info,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            health_monitor: None,
        }
    }

    /// 创建带有共享节点列表的服务实例
    pub fn with_shared_nodes(
        node_id: String, 
        address: String, 
        system_info: String, 
        shared_nodes: Arc<Mutex<HashMap<String, NodeConnection>>>
    ) -> Self {
        Self {
            node_id: node_id.clone(),
            address,
            system_info,
            nodes: shared_nodes,
            health_monitor: None,
        }
    }

    /// 设置健康监控器
    pub fn with_health_monitor(mut self, health_monitor: Arc<HealthMonitor>) -> Self {
        self.health_monitor = Some(health_monitor);
        self
    }

    /// 获取所有已知节点的连接状态
    pub async fn get_all_nodes(&self) -> Vec<NodeConnection> {
        let nodes = self.nodes.lock().await;
        nodes.values().cloned().collect()
    }

    /// 获取特定节点的连接状态
    pub async fn get_node(&self, node_id: &str) -> Option<NodeConnection> {
        let nodes = self.nodes.lock().await;
        nodes.values().find(|n| n.info.id == node_id).cloned()
    }

    /// 获取节点连接状态摘要
    pub async fn get_connection_summary(&self) -> String {
        let nodes = self.nodes.lock().await;

        if nodes.is_empty() {
            return "未发现任何连接过的节点".to_string();
        }

        let mut online_count = 0;
        let mut offline_count = 0;

        for conn in nodes.values() {
            match conn.status {
                NodeConnectionStatus::Online => online_count += 1,
                NodeConnectionStatus::Offline => offline_count += 1,
            }
        }

        let mut summary = format!(
            "共有 {} 个连接过的节点，在线: {}，离线: {}\n",
            nodes.len(),
            online_count,
            offline_count
        );

        // 添加节点详情
        summary.push_str("节点详情:\n");
        for conn in nodes.values() {
            let status_str = match conn.status {
                NodeConnectionStatus::Online => "在线",
                NodeConnectionStatus::Offline => "离线",
            };

            let last_seen_mins = (Utc::now().timestamp() - conn.last_connection) / 60;
            let last_seen = if last_seen_mins == 0 {
                "刚刚".to_string()
            } else {
                format!("{} 分钟前", last_seen_mins)
            };

            summary.push_str(&format!(
                "  - {}: {} | {} | 系统: {} | 最后连接: {} | 成功: {} | 失败: {}\n",
                conn.info.address,
                conn.info.id,
                status_str,
                conn.info.system,
                last_seen,
                conn.success_count,
                conn.failure_count
            ));
        }

        summary
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
        let remote_node_info = NodeInfo {
            id: req.node_id.clone(),
            address: req.address.clone(),
            system: req.system_info.clone(),
            last_seen: timestamp,
        };

        // 更新节点连接列表
        let mut nodes = self.nodes.lock().await;

        // 检查是否已存在该节点
        if let Some(conn) = nodes.get_mut(&req.address) {
            // 更新现有节点信息
            conn.info = remote_node_info;
            conn.status = NodeConnectionStatus::Online;
            conn.last_connection = timestamp;
            conn.success_count += 1;

            debug!("更新节点连接: {} ({})", req.node_id, req.address);

            // 同时通知健康监控器该节点在线
            if let Some(health_monitor) = &self.health_monitor {
                if let Err(e) = health_monitor.reset_node_status(&req.address) {
                    // 这里只记录错误，不中断处理
                    info!("在心跳处理中重置节点状态失败: {} - {}", req.address, e);
                }
            }
        } else {
            // 添加新节点
            let new_conn = NodeConnection {
                info: remote_node_info,
                status: NodeConnectionStatus::Online,
                last_connection: timestamp,
                success_count: 1,
                failure_count: 0,
            };

            nodes.insert(req.address.clone(), new_conn);
            debug!("发现新节点连接: {} ({})", req.node_id, req.address);

            // 同时通知健康监控器该节点在线
            if let Some(health_monitor) = &self.health_monitor {
                // 先添加节点，确保健康监控器知道该节点
                health_monitor.add_node(
                    req.node_id.clone(),
                    req.address.clone(),
                    req.system_info.clone(),
                );

                // 然后标记为在线
                if let Err(e) = health_monitor.reset_node_status(&req.address) {
                    // 这里只记录错误，不中断处理
                    info!("在心跳处理中重置新节点状态失败: {} - {}", req.address, e);
                }
            }
        }

        // 构造响应
        let reply = HeartbeatResponse {
            node_id: self.node_id.clone(),
            address: self.address.clone(),
            system_info: self.system_info.clone(),
            timestamp,
            status: true,
        };

        // 每收到10个心跳请求打印一次连接摘要
        static HEARTBEAT_COUNTER: AtomicU32 = AtomicU32::new(0);
        let counter = HEARTBEAT_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;

        if counter % 10 == 0 {
            let mut conn_summary = "Heartbeat request statistics:\n".to_string();
            conn_summary.push_str(&format!("Total heartbeat requests received: {}\n", counter));

            // Summarize connection success/failure counts for all nodes
            let mut total_success = 0;
            let mut total_failure = 0;
            for conn in nodes.values() {
                total_success += conn.success_count;
                total_failure += conn.failure_count;
            }

            conn_summary.push_str(&format!(
                "总连接成功: {}, 总连接失败: {}\n",
                total_success, total_failure
            ));

            info!("{}", conn_summary);
        }

        Ok(Response::new(reply))
    }
}

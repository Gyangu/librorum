use crate::proto::node::node_service_server::NodeService;
use crate::proto::node::{
    HeartbeatRequest, HeartbeatResponse,
    NodeListRequest, NodeListResponse, NodeInfo as ProtoNodeInfo,
    SystemHealthRequest, SystemHealthResponse,
    AddNodeRequest, AddNodeResponse,
    RemoveNodeRequest, RemoveNodeResponse,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn};

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

    async fn get_node_list(
        &self,
        request: Request<NodeListRequest>,
    ) -> Result<Response<NodeListResponse>, Status> {
        let req = request.into_inner();
        
        info!("收到节点列表请求, include_offline: {}", req.include_offline);
        
        let nodes = self.nodes.lock().await;
        let mut node_list = Vec::new();
        let mut online_count = 0;
        let mut offline_count = 0;
        
        for (address, conn) in nodes.iter() {
            let is_online = conn.status == NodeConnectionStatus::Online;
            let status = if is_online {
                online_count += 1;
                "online".to_string()
            } else {
                offline_count += 1;
                if conn.failure_count > 0 {
                    "error".to_string()
                } else {
                    "offline".to_string()
                }
            };
            
            // 根据请求决定是否包含离线节点
            if !req.include_offline && !is_online {
                continue;
            }
            
            let node_info = ProtoNodeInfo {
                node_id: conn.info.id.clone(),
                address: address.clone(),
                system_info: conn.info.system.clone(),
                status,
                last_heartbeat: conn.last_connection,
                connection_count: conn.success_count as i32,
                failure_count: conn.failure_count as i32,
                latency_ms: 0.0, // 暂时固定值
                is_online,
                discovered_at: conn.last_connection,
            };
            
            node_list.push(node_info);
        }
        
        let response = NodeListResponse {
            nodes: node_list,
            total_count: (online_count + offline_count) as i32,
            online_count: online_count as i32,
            offline_count: offline_count as i32,
        };
        
        info!("返回节点列表: {} 个节点 ({} 在线, {} 离线)", 
              response.total_count, response.online_count, response.offline_count);
        
        Ok(Response::new(response))
    }

    async fn get_system_health(
        &self,
        _request: Request<SystemHealthRequest>,
    ) -> Result<Response<SystemHealthResponse>, Status> {
        info!("收到系统健康状态请求");
        
        // 获取系统信息（简化实现）
        let memory_usage = 134_217_728i64; // 128MB 模拟值
        let cpu_usage = 15.5; // 模拟CPU使用率
        
        // 模拟存储信息（实际应该从VDFS获取）
        let total_storage = 1_073_741_824i64; // 1GB
        let used_storage = 268_435_456i64;    // 256MB
        let available_storage = total_storage - used_storage;
        
        // 计算网络延迟（基于当前节点连接）
        let nodes = self.nodes.lock().await;
        let avg_latency = if nodes.is_empty() {
            0.0
        } else {
            0.025 // 固定延迟值
        };
        
        // 计算运行时间（简化实现）
        let uptime = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        let response = SystemHealthResponse {
            total_storage,
            used_storage,
            available_storage,
            total_files: 150,           // 模拟数据
            total_chunks: 750,          // 模拟数据
            network_latency: avg_latency,
            error_count: nodes.values().map(|conn| conn.failure_count as i32).sum(),
            uptime_seconds: uptime % 86400, // 当日运行时间
            memory_usage,
            cpu_usage,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        info!("返回系统健康状态: {}MB 内存使用, {}% CPU", 
              response.memory_usage / 1024 / 1024, response.cpu_usage);
        
        Ok(Response::new(response))
    }

    async fn add_node(
        &self,
        request: Request<AddNodeRequest>,
    ) -> Result<Response<AddNodeResponse>, Status> {
        let req = request.into_inner();
        
        info!("收到添加节点请求: {}", req.address);
        
        // 验证地址格式
        if !self.is_valid_address(&req.address) {
            let response = AddNodeResponse {
                success: false,
                message: "无效的节点地址格式".to_string(),
                node: None,
            };
            return Ok(Response::new(response));
        }
        
        // 检查节点是否已存在
        let mut nodes = self.nodes.lock().await;
        if nodes.contains_key(&req.address) {
            let response = AddNodeResponse {
                success: false,
                message: "节点已存在".to_string(),
                node: None,
            };
            return Ok(Response::new(response));
        }
        
        // 创建新的节点连接
        let node_id = format!("manual.{}.librorum.local", req.address.replace(":", "_"));
        let node_info = NodeInfo {
            id: node_id.clone(),
            address: req.address.clone(),
            system: "Manually Added".to_string(),
            last_seen: chrono::Utc::now().timestamp(),
        };
        
        let conn = NodeConnection {
            info: node_info,
            status: NodeConnectionStatus::Offline,
            last_connection: chrono::Utc::now().timestamp(),
            success_count: 0,
            failure_count: 0,
        };
        
        let node_info = ProtoNodeInfo {
            node_id: node_id.clone(),
            address: req.address.clone(),
            system_info: conn.info.system.clone(),
            status: "connecting".to_string(),
            last_heartbeat: conn.last_connection,
            connection_count: 0,
            failure_count: 0,
            latency_ms: 0.0,
            is_online: false,
            discovered_at: conn.last_connection,
        };
        
        nodes.insert(req.address.clone(), conn);
        drop(nodes);
        
        let response = AddNodeResponse {
            success: true,
            message: "节点添加成功".to_string(),
            node: Some(node_info),
        };
        
        info!("成功添加节点: {} ({})", req.address, node_id);
        
        Ok(Response::new(response))
    }

    async fn remove_node(
        &self,
        request: Request<RemoveNodeRequest>,
    ) -> Result<Response<RemoveNodeResponse>, Status> {
        let req = request.into_inner();
        
        info!("收到移除节点请求: {}", req.node_id);
        
        let mut nodes = self.nodes.lock().await;
        
        // 查找并移除节点
        let mut found = false;
        let mut removed_address = String::new();
        
        nodes.retain(|address, conn| {
            if conn.info.id == req.node_id {
                found = true;
                removed_address = address.clone();
                false // 移除这个节点
            } else {
                true // 保留其他节点
            }
        });
        
        let response = if found {
            info!("成功移除节点: {} ({})", req.node_id, removed_address);
            RemoveNodeResponse {
                success: true,
                message: format!("节点 {} 已移除", req.node_id),
            }
        } else {
            warn!("未找到节点: {}", req.node_id);
            RemoveNodeResponse {
                success: false,
                message: format!("未找到节点: {}", req.node_id),
            }
        };
        
        Ok(Response::new(response))
    }
}

impl NodeServiceImpl {
    /// 验证地址格式
    fn is_valid_address(&self, address: &str) -> bool {
        let parts: Vec<&str> = address.split(':').collect();
        if parts.len() != 2 {
            return false;
        }
        
        // 验证端口
        if let Ok(port) = parts[1].parse::<u16>() {
            port > 0
        } else {
            false
        }
    }
}

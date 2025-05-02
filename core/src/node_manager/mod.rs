use crate::config::NodeConfig;
use crate::proto::node::node_service_server::NodeServiceServer;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use nanoid::nanoid;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tonic::transport::Server;
pub use tracing::{debug, info, warn};

mod mdns_manager;
pub mod node_client;
pub mod node_service;

use mdns_manager::MdnsManager;
use node_client::NodeClient;
use node_service::{NodeInfo, NodeServiceImpl};

/// 节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    /// 在线
    Online,
    /// 离线
    Offline,
    /// 未知
    Unknown,
}

/// 节点健康信息
#[derive(Debug, Clone)]
pub struct NodeHealth {
    /// 节点ID
    pub node_id: String,
    /// 节点地址
    pub address: String,
    /// 系统类型
    pub system_info: String,
    /// 最后一次心跳时间
    pub last_heartbeat: DateTime<Utc>,
    /// 连续失败次数
    pub failure_count: u32,
    /// 节点状态
    pub status: NodeStatus,
    /// 延迟(毫秒)
    pub latency_ms: Option<u64>,
}

impl NodeHealth {
    /// 创建新的节点健康信息
    pub fn new(node_id: String, address: String, system_info: String) -> Self {
        Self {
            node_id,
            address,
            system_info,
            last_heartbeat: Utc::now(),
            failure_count: 0,
            status: NodeStatus::Unknown,
            latency_ms: None,
        }
    }

    /// 更新节点状态为在线
    pub fn mark_online(&mut self, latency_ms: Option<u64>) {
        self.last_heartbeat = Utc::now();
        self.failure_count = 0;
        self.status = NodeStatus::Online;
        self.latency_ms = latency_ms;
    }

    /// 更新节点状态为离线
    pub fn mark_failure(&mut self) {
        self.failure_count += 1;
        // 如果连续失败超过3次，标记为离线
        if self.failure_count >= 3 {
            self.status = NodeStatus::Offline;
        }
    }

    /// 返回节点最后心跳是否超时
    pub fn is_timeout(&self, timeout_secs: i64) -> bool {
        let now = Utc::now();
        let diff = now.timestamp() - self.last_heartbeat.timestamp();
        diff > timeout_secs
    }
}

/// 节点管理器
pub struct NodeManager {
    /// 节点ID
    node_id: String,

    /// 绑定地址
    bind_address: String,

    /// 系统信息
    system_info: String,

    /// 已发现的节点列表
    discovered_nodes: Arc<std::sync::Mutex<Vec<String>>>,

    /// 已知节点列表
    known_nodes: Arc<Mutex<Vec<String>>>,

    /// 节点健康状态
    node_health: Arc<std::sync::Mutex<HashMap<String, NodeHealth>>>,

    /// 节点配置
    config: Option<NodeConfig>,
}

impl NodeManager {
    /// 创建新的节点管理器
    pub fn new(port: u16) -> Self {
        // 生成节点 ID
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let node_id = format!("{}.{}.librorum.local", nanoid!(10), device_name);

        // 获取绑定地址
        let bind_ip = "0.0.0.0"; // 绑定所有接口
        let bind_address = format!("{}:{}", bind_ip, port);

        // 获取系统信息
        let system_info = Self::get_system_info();

        // 创建节点管理器
        Self {
            node_id,
            bind_address,
            system_info,
            discovered_nodes: Arc::new(std::sync::Mutex::new(Vec::new())),
            known_nodes: Arc::new(Mutex::new(Vec::new())),
            node_health: Arc::new(std::sync::Mutex::new(HashMap::new())),
            config: None,
        }
    }

    /// 使用配置创建节点管理器
    pub fn with_config(config: NodeConfig) -> Self {
        let bind_address = config.bind_address();
        let _port = config.bind_port;

        // 生成节点 ID
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let node_id = format!("{}.{}.librorum.local", config.node_prefix, device_name);

        // 获取系统信息
        let system_info = Self::get_system_info();

        // 初始化已知节点列表
        let known_nodes = Arc::new(Mutex::new(Vec::new()));

        // 创建节点管理器
        Self {
            node_id,
            bind_address,
            system_info,
            discovered_nodes: Arc::new(std::sync::Mutex::new(Vec::new())),
            known_nodes,
            node_health: Arc::new(std::sync::Mutex::new(HashMap::new())),
            config: Some(config),
        }
    }

    /// 获取系统信息
    fn get_system_info() -> String {
        #[cfg(target_os = "windows")]
        {
            "Windows".to_string()
        }

        #[cfg(target_os = "macos")]
        {
            "macOS".to_string()
        }

        #[cfg(target_os = "linux")]
        {
            "Linux".to_string()
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            "Unknown".to_string()
        }
    }

    /// 启动节点管理器
    pub async fn start(&self) -> Result<()> {
        info!("开始启动节点服务: {}", self.bind_address);

        // 创建节点服务
        let node_service = NodeServiceImpl::new(
            self.node_id.clone(),
            self.bind_address.clone(),
            self.system_info.clone(),
        );

        // 获取端口
        let port = self
            .bind_address
            .split(':')
            .nth(1)
            .unwrap_or("50051")
            .parse()
            .unwrap_or(50051);

        // 创建mDNS管理器
        let mdns_manager = MdnsManager::new(self.node_id.clone(), port);

        // 注册mDNS服务
        if let Err(err) = mdns_manager.register() {
            warn!("mDNS服务注册失败: {}", err);
        } else {
            info!("mDNS服务注册成功");
        }

        // 启动mDNS服务发现
        let discovered_nodes = self.discovered_nodes.clone();
        let node_health = self.node_health.clone();

        // 定义服务发现回调
        let discovery_callback = move |node_id: String, address: String, _port: u16| {
            debug!("回调被调用: 节点ID={}, 地址={}", node_id, address);
            let mut nodes = discovered_nodes.lock().unwrap();
            
            // 跳过IPv6地址
            if address.matches(':').count() > 1 {
                debug!("忽略IPv6地址节点: {} ({})", node_id, address);
                return;
            }
            
            debug!("检查是否已存在: {}", address);
            if !nodes.contains(&address) {
                info!("发现新节点: {} ({})", node_id, address);
                nodes.push(address.clone());

                // 初始化节点健康状态
                let mut health_map = node_health.lock().unwrap();
                debug!("准备添加节点到健康状态映射: {}", address);
                
                // 尝试从节点ID中提取系统信息
                let system_info = if node_id.contains("windows") {
                    "Windows".to_string()
                } else if node_id.contains("macos") {
                    "macOS".to_string()
                } else if node_id.contains("linux") {
                    "Linux".to_string()
                } else {
                    "Unknown".to_string()
                };
                
                // 创建健康状态对象并设置为在线状态
                let mut health = NodeHealth::new(node_id, address.clone(), system_info);
                health.status = NodeStatus::Online;
                health_map.insert(address.clone(), health);
                
                info!("节点已加入健康状态跟踪: {}", address);
                debug!("当前健康映射大小: {}", health_map.len());
            } else {
                debug!("节点已存在，跳过添加: {}", address);
            }
        };

        // 定义服务移除回调
        let removed_nodes = self.discovered_nodes.clone();
        let removed_health = self.node_health.clone();
        let removed_callback = move |node_id: String| {
            // 尝试从已发现节点列表中移除
            let mut nodes = removed_nodes.lock().unwrap();
            if let Some(pos) = nodes.iter().position(|addr| addr.contains(&node_id)) {
                let addr = nodes.remove(pos);
                info!("节点已离线: {} ({})", node_id, addr);

                // 更新节点健康状态
                let mut health_map = removed_health.lock().unwrap();
                if let Some(health) = health_map.get_mut(&addr) {
                    health.status = NodeStatus::Offline;
                }
            }
        };

        // 异步启动mDNS服务发现
        if let Err(err) = mdns_manager
            .start_discovery(discovery_callback, removed_callback)
            .await
        {
            warn!("启动mDNS服务发现失败: {}", err);
        } else {
            info!("mDNS服务发现已启动");
        }

        // 如果有配置文件，加载已知节点
        if let Some(config) = &self.config {
            // 从配置文件载入已知节点
            let mut known_nodes = self.known_nodes.lock().await;
            for node in &config.known_nodes {
                if !known_nodes.contains(node) {
                    info!("从配置文件加载已知节点: {}", node);
                    known_nodes.push(node.clone());

                    // 初始化节点健康状态
                    let mut health_map = self.node_health.lock().unwrap();
                    health_map.insert(
                        node.clone(),
                        NodeHealth::new(
                            format!("unknown-{}", node),
                            node.clone(),
                            "Unknown".to_string(),
                        ),
                    );
                }
            }
        }

        // 创建gRPC服务
        let node_server = NodeServiceServer::new(node_service);

        // 解析绑定地址
        let addr = self
            .bind_address
            .parse::<SocketAddr>()
            .with_context(|| format!("无法解析地址: {}", self.bind_address))?;

        // 启动节点连接管理器
        let discovered_nodes_clone = Arc::clone(&self.discovered_nodes);
        let known_nodes_clone = Arc::clone(&self.known_nodes);
        let node_health_clone = Arc::clone(&self.node_health);
        let node_id_clone = self.node_id.clone();
        let bind_address_clone = self.bind_address.clone();
        let system_info_clone = self.system_info.clone();

        // 获取心跳间隔时间
        let heartbeat_interval = if let Some(config) = &self.config {
            config.heartbeat_interval
        } else {
            5
        };

        // 启动心跳任务
        tokio::spawn(async move {
            // 启动前短暂延迟，让服务完全启动
            tokio::time::sleep(Duration::from_secs(2)).await;
            info!("节点连接管理器启动，心跳间隔: {}秒", heartbeat_interval);

            let mut connect_interval = interval(Duration::from_secs(heartbeat_interval));

            loop {
                connect_interval.tick().await;

                // 获取发现的节点列表
                let discovered = { discovered_nodes_clone.lock().unwrap().clone() };

                // 获取已知节点列表
                let known = { known_nodes_clone.lock().await.clone() };

                // 显示发现的节点数量
                if !discovered.is_empty() {
                    debug!("发现节点列表: {} 个节点", discovered.len());
                    for (i, node) in discovered.iter().enumerate() {
                        debug!("  [{}] {}", i + 1, node);
                    }
                }

                // 显示已知节点数量
                if !known.is_empty() {
                    debug!("已知节点列表: {} 个节点", known.len());
                    for (i, node) in known.iter().enumerate() {
                        debug!("  [{}] {}", i + 1, node);
                    }
                }

                // 连接发现的节点并发送心跳
                for node_addr in discovered.iter().chain(known.iter()) {
                    // 确保只连接IPv4地址，跳过IPv6地址
                    if node_addr.matches(':').count() > 1 {
                        // IPv6地址包含多个冒号，而IPv4:PORT只包含一个
                        debug!("跳过IPv6地址: {}", node_addr);
                        continue;
                    }

                    // 跳过本地节点自身的地址
                    if node_addr.contains(&node_id_clone) {
                        debug!("跳过本地节点地址: {}", node_addr);
                        continue;
                    }
                    
                    let is_known = known.contains(node_addr);

                    // 检查节点状态，避免频繁重试已知的离线节点
                    let mut should_connect = true;
                    {
                        let health_map = node_health_clone.lock().unwrap();
                        if let Some(health) = health_map.get(node_addr) {
                            // 如果节点已知是离线的，且失败计数大于5，则降低检测频率
                            if health.status == NodeStatus::Offline && health.failure_count > 5 {
                                should_connect = health.failure_count % 5 == 0; // 每5次才尝试重连
                            }
                        }
                    }

                    // 使用辅助函数判断是否应当连接
                    let should_retry =
                        !is_known || (is_known && Self::should_retry_connection(node_addr));

                    if should_connect && should_retry {
                        // 尝试连接节点并发送心跳
                        let client = NodeClient::new(
                            node_id_clone.clone(),
                            bind_address_clone.clone(),
                            system_info_clone.clone(),
                        );

                        debug!("正在连接节点: {}", node_addr);
                        let start_time = std::time::Instant::now();

                        match client.send_heartbeat(node_addr).await {
                            Ok(response) => {
                                let elapsed = start_time.elapsed().as_millis() as u64;
                                let connection_type = if is_known {
                                    "刷新连接"
                                } else {
                                    "新建连接"
                                };
                                info!(
                                    "{}到节点成功: {} (节点ID: {}, 延迟: {}ms)",
                                    connection_type, node_addr, response.node_id, elapsed
                                );

                                // 只在未知节点时添加到已知列表
                                if !is_known {
                                    known_nodes_clone.lock().await.push(node_addr.clone());
                                }

                                // 更新节点健康状态
                                let mut health_map = node_health_clone.lock().unwrap();
                                if let Some(health) = health_map.get_mut(node_addr) {
                                    info!("更新节点健康状态: {} ({})", node_addr, response.node_id);
                                    health.mark_online(Some(elapsed));
                                    health.node_id = response.node_id.clone();
                                    health.system_info = response.system_info.clone();
                                } else {
                                    info!("创建新节点健康状态: {} ({})", node_addr, response.node_id);
                                    health_map.insert(node_addr.clone(), {
                                        let mut health = NodeHealth::new(
                                            response.node_id.clone(),
                                            node_addr.clone(),
                                            response.system_info.clone(),
                                        );
                                        health.mark_online(Some(elapsed));
                                        health
                                    });
                                }
                            }
                            Err(e) => {
                                warn!("心跳发送失败: {} - {}", node_addr, e);

                                // 更新节点健康状态
                                let mut health_map = node_health_clone.lock().unwrap();
                                if let Some(health) = health_map.get_mut(node_addr) {
                                    health.mark_failure();
                                    if health.status == NodeStatus::Offline {
                                        warn!(
                                            "节点已标记为离线: {}, 连续失败次数: {}",
                                            node_addr, health.failure_count
                                        );
                                    }
                                } else {
                                    let mut health = NodeHealth::new(
                                        format!("unknown-{}", node_addr),
                                        node_addr.clone(),
                                        "Unknown".to_string(),
                                    );
                                    health.mark_failure();
                                    health_map.insert(node_addr.clone(), health);
                                }
                            }
                        }
                    }
                }

                // 每10个心跳周期打印一次健康状态报告
                static mut REPORT_COUNTER: u32 = 0;
                unsafe {
                    REPORT_COUNTER += 1;
                    if REPORT_COUNTER % 10 == 0 || REPORT_COUNTER < 5 {
                        // 前5次和每10次后生成报告
                        let health_count = {
                            let health_map = node_health_clone.lock().unwrap();
                            health_map.len()
                        };
                        let report = Self::generate_health_report(&node_health_clone).await;
                        info!("节点健康状态报告 (总节点: {}):\n{}", health_count, report);
                    }
                }
            }
        });

        // 启动服务
        info!("节点服务启动，绑定地址: {}", addr);
        Server::builder()
            .add_service(node_server)
            .serve(addr)
            .await
            .with_context(|| "gRPC服务启动失败")?;

        Ok(())
    }

    /// 生成节点健康状态报告
    async fn generate_health_report(
        node_health: &Arc<std::sync::Mutex<HashMap<String, NodeHealth>>>,
    ) -> String {
        let health_map = node_health.lock().unwrap();
        debug!("生成健康报告：当前映射大小 = {}", health_map.len());

        if health_map.is_empty() {
            debug!("健康状态映射为空");
            return "未发现任何节点".to_string();
        }

        // 计算在线/离线节点数量
        let mut online_count = 0;
        let mut offline_count = 0;
        let mut unknown_count = 0;

        // 首先过滤出有效的节点（非IPv6地址）
        let valid_nodes: Vec<(&String, &NodeHealth)> = health_map.iter()
            .filter(|(addr, _)| addr.matches(':').count() <= 1) // 只保留IPv4地址
            .collect();
        
        debug!("有效节点数量: {}/{}", valid_nodes.len(), health_map.len());

        if valid_nodes.is_empty() {
            debug!("没有有效的IPv4节点");
            return "未发现任何IPv4节点".to_string();
        }

        for (_, health) in &valid_nodes {
            match health.status {
                NodeStatus::Online => online_count += 1,
                NodeStatus::Offline => offline_count += 1,
                NodeStatus::Unknown => unknown_count += 1,
            }
        }

        // 生成报告头部
        let mut report = format!(
            "发现 {} 个节点 (在线: {}, 离线: {}, 未知: {})\n",
            valid_nodes.len(),
            online_count,
            offline_count,
            unknown_count
        );

        // 为每个节点添加状态详情
        report.push_str("节点详情:\n");

        for (addr, health) in valid_nodes {
            let status_str = match health.status {
                NodeStatus::Online => "在线",
                NodeStatus::Offline => "离线",
                NodeStatus::Unknown => "未知",
            };

            let latency_str = health
                .latency_ms
                .map(|ms| format!("{} ms", ms))
                .unwrap_or_else(|| "未知".to_string());

            let last_seen = (Utc::now().timestamp() - health.last_heartbeat.timestamp()) / 60;
            let last_seen_str = if last_seen == 0 {
                "刚刚".to_string()
            } else {
                format!("{} 分钟前", last_seen)
            };

            report.push_str(&format!(
                "  - {}: {} | {} | 系统: {} | 延迟: {} | 最后心跳: {}\n",
                addr, health.node_id, status_str, health.system_info, latency_str, last_seen_str
            ));
        }

        report
    }

    /// 判断是否应该尝试重新连接节点
    fn should_retry_connection(node_addr: &str) -> bool {
        // 本地连接总是尝试
        if node_addr.starts_with("127.0.0.1") || node_addr.starts_with("localhost") {
            return true;
        }

        // 默认情况下总是尝试重新连接
        true
    }

    /// 连接到特定节点
    pub async fn connect_to_node(&self, address: String) -> Result<NodeInfo> {
        // 创建节点客户端
        let client = NodeClient::new(
            self.node_id.clone(),
            self.bind_address.clone(),
            self.system_info.clone(),
        );

        // 发送心跳
        let response = client.send_heartbeat(&address).await?;

        // 记录节点信息
        let node_info = NodeInfo {
            id: response.node_id.clone(),
            address: response.address.clone(),
            system: response.system_info.clone(),
            last_seen: response.timestamp,
        };

        // 添加到已知节点
        let mut known_nodes = self.known_nodes.lock().await;
        if !known_nodes.contains(&address) {
            known_nodes.push(address.clone());
        }

        Ok(node_info)
    }

    /// 获取节点ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 获取绑定地址
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// 获取系统信息
    pub fn system_info(&self) -> &str {
        &self.system_info
    }

    /// 添加已知节点
    pub async fn add_node(&self, address: String) -> Result<()> {
        // 添加到已知节点列表
        let mut known_nodes = self.known_nodes.lock().await;
        if !known_nodes.contains(&address) {
            known_nodes.push(address.clone());
        }

        // 尝试立即连接
        let result = self.connect_to_node(address.clone()).await;

        match result {
            Ok(node_info) => {
                info!(
                    "成功添加并连接到节点: {}, 节点ID: {}",
                    address, node_info.id
                );
                Ok(())
            }
            Err(e) => {
                warn!("添加节点成功，但首次连接失败: {}, 错误: {}", address, e);
                // 仍然视为成功，后续心跳机制会尝试重连
                Ok(())
            }
        }
    }

    /// 获取已知节点健康状态
    pub async fn get_nodes_health(&self) -> Vec<NodeHealth> {
        let health_map = self.node_health.lock().unwrap();
        health_map.values().cloned().collect()
    }

    /// 获取节点状态报告
    pub async fn get_health_report(&self) -> String {
        Self::generate_health_report(&self.node_health).await
    }
}
use anyhow::{Result, Context};
use nanoid::nanoid;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::net::SocketAddr;
use tonic::transport::Server;
use tokio::time::{Duration, interval};
use crate::proto::node::node_service_server::NodeServiceServer;
use crate::config::NodeConfig;

mod mdns_manager;
pub mod node_client;
pub mod node_service;

use mdns_manager::MdnsManager;
use node_service::{NodeServiceImpl, NodeInfo};
use node_client::NodeClient;

/// 节点管理器
pub struct NodeManager {
    /// 节点ID
    node_id: String,
    
    /// 绑定地址
    bind_address: String,
    
    /// 系统信息
    system_info: String,
    
    /// 已发现的节点列表
    discovered_nodes: Arc<Mutex<Vec<String>>>,
    
    /// 已知节点列表
    known_nodes: Arc<Mutex<Vec<String>>>,
    
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
            discovered_nodes: Arc::new(Mutex::new(Vec::new())),
            known_nodes: Arc::new(Mutex::new(Vec::new())),
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
            discovered_nodes: Arc::new(Mutex::new(Vec::new())),
            known_nodes,
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
        tracing::info!("开始启动节点服务: {}", self.bind_address);
        
        // 创建节点服务
        let node_service = NodeServiceImpl::new(
            self.node_id.clone(),
            self.bind_address.clone(),
            self.system_info.clone(),
        );
        
        // 获取端口
        let port = self.bind_address.split(':')
            .nth(1)
            .unwrap_or("50051")
            .parse()
            .unwrap_or(50051);
            
        // 创建mDNS管理器
        let mdns_manager = MdnsManager::new(
            self.node_id.clone(), 
            port
        );
        
        // 注册mDNS服务
        if let Err(err) = mdns_manager.register() {
            tracing::warn!("mDNS服务注册失败: {}", err);
        } else {
            tracing::info!("mDNS服务注册成功");
        }
        
        // 如果有配置文件，加载已知节点
        if let Some(config) = &self.config {
            // 从配置文件载入已知节点
            let mut known_nodes = self.known_nodes.lock().await;
            for node in &config.known_nodes {
                if !known_nodes.contains(node) {
                    tracing::info!("从配置文件加载已知节点: {}", node);
                    known_nodes.push(node.clone());
                }
            }
        }
        
        // 创建gRPC服务
        let node_server = NodeServiceServer::new(node_service);
        
        // 解析绑定地址
        let addr = self.bind_address.parse::<SocketAddr>()
            .with_context(|| format!("无法解析地址: {}", self.bind_address))?;
        
        // 启动服务发现任务
        let discovered_nodes_clone = Arc::clone(&self.discovered_nodes);
        let mdns_manager_clone = mdns_manager;
        
        // 获取发现间隔时间
        let discovery_interval = if let Some(config) = &self.config {
            config.discovery_interval
        } else {
            10
        };
        
        tokio::spawn(async move {
            let mut discover_interval = interval(Duration::from_secs(discovery_interval));
            
            loop {
                discover_interval.tick().await;
                
                tracing::debug!("执行节点发现...");
                if let Err(e) = mdns_manager_clone.discover(Arc::clone(&discovered_nodes_clone)).await {
                    tracing::error!("服务发现错误: {}", e);
                }
            }
        });
        
        // 启动节点连接管理器
        let discovered_nodes_clone = Arc::clone(&self.discovered_nodes);
        let known_nodes_clone = Arc::clone(&self.known_nodes);
        let node_id_clone = self.node_id.clone();
        let bind_address_clone = self.bind_address.clone();
        let system_info_clone = self.system_info.clone();
        
        // 获取心跳间隔时间
        let heartbeat_interval = if let Some(config) = &self.config {
            config.heartbeat_interval
        } else {
            5
        };
        
        tokio::spawn(async move {
            // 启动前短暂延迟，让服务完全启动
            tokio::time::sleep(Duration::from_secs(2)).await;
            tracing::info!("节点连接管理器启动，心跳间隔: {}秒", heartbeat_interval);
            
            let mut connect_interval = interval(Duration::from_secs(heartbeat_interval));
            
            loop {
                connect_interval.tick().await;
                
                // 获取发现的节点列表
                let discovered = {
                    discovered_nodes_clone.lock().await.clone()
                };
                
                // 获取已知节点列表
                let known = {
                    known_nodes_clone.lock().await.clone()
                };
                
                // 连接发现的节点并发送心跳
                for node_addr in discovered.iter() {
                    let is_known = known.contains(node_addr);
                    // 使用辅助函数判断是否应当连接
                    let should_connect = !is_known || (is_known && Self::should_retry_connection(node_addr));
                    
                    if should_connect {
                        // 尝试连接节点并发送心跳
                        let client = NodeClient::new(
                            node_id_clone.clone(),
                            bind_address_clone.clone(),
                            system_info_clone.clone(),
                        );
                        
                        tracing::debug!("正在连接节点: {}", node_addr);
                        match client.send_heartbeat(node_addr).await {
                            Ok(response) => {
                                let connection_type = if is_known { "刷新连接" } else { "新建连接" };
                                tracing::info!("{}到节点成功: {} (节点ID: {})", connection_type, node_addr, response.node_id);
                                
                                // 只在未知节点时添加到已知列表
                                if !is_known {
                                    known_nodes_clone.lock().await.push(node_addr.clone());
                                }
                                
                                // 成功连接日志
                                tracing::debug!("已成功与节点建立连接: {}", node_addr);
                            },
                            Err(e) => {
                                tracing::warn!("连接节点失败: {} - {}", node_addr, e);
                                
                                // TODO: 记录失败次数，在多次失败后从已知节点中移除
                            },
                        }
                    }
                }
            }
        });
        
        // 启动gRPC服务器
        tracing::info!("gRPC服务启动在 {}", addr);
        Server::builder()
            .add_service(node_server)
            .serve(addr)
            .await
            .with_context(|| "gRPC服务启动失败")
    }
    
    /// 判断是否应该重新尝试连接节点
    /// 对于本地节点和重要节点，即使已知也定期重试连接
    fn should_retry_connection(node_addr: &str) -> bool {
        // 对于本地节点总是尝试重连
        if node_addr.starts_with("127.0.0.1") || node_addr.starts_with("localhost") {
            return true;
        }
        
        // 特殊节点列表 - 这些节点即使已知也要定期尝试重连
        let important_nodes = [
            ".local:",     // mDNS地址
            "windows.local",
            "gy.local",
            "192.168.31.90",  // 已知Mac节点IP
            "192.168.31.91",
            "192.168.31.92",  // 已知Windows节点IP
            "192.168.31.93"
        ];
        
        // 检查是否匹配重要节点
        for important in &important_nodes {
            if node_addr.contains(important) {
                return true;
            }
        }
        
        // 默认不重试已知节点
        false
    }
    
    /// 连接到指定节点
    pub async fn connect_to_node(&self, address: String) -> Result<NodeInfo> {
        let client = NodeClient::new(
            self.node_id.clone(), 
            self.bind_address.clone(), 
            self.system_info.clone()
        );
        
        // 尝试发送心跳以测试连接
        tracing::info!("尝试连接到节点: {}", address);
        let response = client.send_heartbeat(&address).await?;
        
        // 使用响应中的节点信息
        let info = NodeInfo {
            id: response.node_id,
            address: address.clone(),
            system: response.system_info,
            last_seen: response.timestamp,
        };
            
        tracing::info!("连接成功: {} (节点ID: {})", address, info.id);
        
        Ok(info)
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
    
    /// 手动添加节点 (用于测试)
    pub async fn add_node(&self, address: String) -> Result<()> {
        let mut nodes = self.discovered_nodes.lock().await;
        if !nodes.contains(&address) {
            tracing::info!("手动添加节点: {}", address);
            nodes.push(address);
        }
        Ok(())
    }
}
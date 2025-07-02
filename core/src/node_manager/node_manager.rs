use librorum_shared::NodeConfig;
use crate::proto::file::file_service_server::FileServiceServer;
use crate::proto::log::log_service_server::LogServiceServer;
use crate::proto::node::node_service_server::NodeServiceServer;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::interval;
use tonic::transport::Server;
use tracing::{debug, info, warn};

use crate::node_manager::file_service::FileServiceImpl;
use crate::node_manager::log_service::LogServiceImpl;
use crate::node_manager::mdns_manager::MdnsManager;
use crate::node_manager::node_client::NodeClient;
use crate::node_manager::node_health::{HealthMonitor, NodeHealth, NodeStatus};
use crate::node_manager::node_service::{NodeInfo, NodeServiceImpl};

/// 节点管理器，负责协调所有节点管理相关的功能
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

    /// 健康监控器
    health_monitor: HealthMonitor,

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

        let node_id = format!("{}.{}.librorum.local", nanoid::nanoid!(10), device_name);

        // 获取绑定地址
        let bind_ip = "0.0.0.0"; // 绑定所有接口
        let bind_address = format!("{}:{}", bind_ip, port);

        // 获取系统信息
        let system_info = Self::get_system_info();

        // 创建健康监控器
        let health_monitor = HealthMonitor::new(60); // 默认60秒心跳超时

        // 创建节点管理器
        Self {
            node_id,
            bind_address,
            system_info,
            discovered_nodes: Arc::new(std::sync::Mutex::new(Vec::new())),
            known_nodes: Arc::new(Mutex::new(Vec::new())),
            health_monitor,
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

        // 创建健康监控器 - 这里使用默认的超时时间60秒
        let health_monitor = HealthMonitor::new(60);

        // 创建节点管理器
        Self {
            node_id,
            bind_address,
            system_info,
            discovered_nodes: Arc::new(std::sync::Mutex::new(Vec::new())),
            known_nodes,
            health_monitor,
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
        )
        // 传递健康监控器引用
        .with_health_monitor(Arc::new(self.health_monitor.clone()));

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

        // 启动健康监控
        self.start_health_monitor().await;

        // 启动mDNS服务发现
        let discovered_nodes = self.discovered_nodes.clone();
        let health_monitor = self.health_monitor.clone();

        // 定义服务发现回调
        let discovery_callback = move |node_id: String, address: String, _port: u16| {
            debug!("节点发现回调: 节点ID={}, 地址={}", node_id, address);

            // 跳过IPv6地址
            if address.matches(':').count() > 1 {
                debug!("忽略IPv6地址节点: {} ({})", node_id, address);
                return;
            }

            // 检查并添加节点
            let should_add = {
                let nodes = discovered_nodes.lock().unwrap();
                !nodes.contains(&address)
            };
            
            if should_add {
                info!("发现新节点: {} ({})", node_id, address);
                
                // 添加到已发现节点列表
                {
                    let mut nodes = discovered_nodes.lock().unwrap();
                    nodes.push(address.clone());
                }
                
                // 初始化节点健康状态
                health_monitor.add_node(node_id.clone(), address.clone(), String::new());
            } else {
                // 节点已存在，但可能状态不正确，重置健康状态
                debug!("节点已存在，尝试重置健康状态: {} ({})", node_id, address);
                
                // 获取当前节点健康状态
                let health_opt = health_monitor.get_node_health(&address);
                
                if let Some(health) = health_opt {
                    if health.status != NodeStatus::Online {
                        info!("重置离线节点状态: {} ({}), 当前失败计数: {}", 
                             node_id, address, health.failure_count);
                        
                        // 手动将节点标记为在线，强制重置离线状态
                        if let Err(e) = health_monitor.reset_node_status(&address) {
                            warn!("重置节点状态失败: {}", e);
                        }
                    }
                }
            }
        };

        // 定义服务移除回调
        let removed_callback = |node_id: String| {
            info!("收到节点移除通知: {}", node_id);
            // 节点离线逻辑由健康检查处理，这里可以增加额外的处理逻辑
        };

        // 启动服务发现
        if let Err(err) = mdns_manager
            .start_discovery(discovery_callback, removed_callback)
            .await
        {
            warn!("启动mDNS服务发现失败: {}", err);
        } else {
            info!("mDNS服务发现启动成功");
        }

        // 绑定地址
        let addr: SocketAddr = self
            .bind_address
            .parse()
            .with_context(|| format!("解析地址失败: {}", self.bind_address))?;

        // 创建文件服务并初始化VDFS
        let mut file_service = FileServiceImpl::new();
        
        // 初始化VDFS配置
        let vdfs_config = if let Some(config) = &self.config {
            crate::vdfs::VDFSConfig {
                storage_path: config.data_dir.clone(),
                chunk_size: 4096,
                enable_compression: false,
                cache_memory_size: 64 * 1024 * 1024, // 64MB
                cache_disk_size: 512 * 1024 * 1024, // 512MB
                replication_factor: 3,
                network_timeout: std::time::Duration::from_secs(30),
            }
        } else {
            crate::vdfs::VDFSConfig::default()
        };
        
        // 异步初始化VDFS
        if let Err(e) = file_service.init_vdfs(vdfs_config).await {
            warn!("Failed to initialize VDFS for FileService: {}", e);
            info!("FileService will fall back to memory storage");
        }

        // 创建日志服务
        let log_service = LogServiceImpl::new();
        log_service.init_sample_logs().await;

        // 启动gRPC服务器
        info!("启动gRPC服务器: {}", addr);
        Server::builder()
            .add_service(NodeServiceServer::new(node_service))
            .add_service(FileServiceServer::new(file_service))
            .add_service(LogServiceServer::new(log_service))
            .serve(addr)
            .await
            .with_context(|| format!("gRPC服务器启动失败: {}", addr))?;

        Ok(())
    }

    /// 启动健康监控任务
    async fn start_health_monitor(&self) {
        let health_monitor = self.health_monitor.clone();
        let discovered_nodes = self.discovered_nodes.clone();
        let node_id = self.node_id.clone();
        let address = self.bind_address.clone();
        let system_info = self.system_info.clone();

        // 设置心跳间隔时间
        let heartbeat_interval = match &self.config {
            Some(config) => config.heartbeat_interval,
            None => 30,
        };

        // 启动健康监控任务
        task::spawn(async move {
            let mut interval = interval(Duration::from_secs(heartbeat_interval as u64));

            // 创建节点客户端
            let client = NodeClient::new(node_id, address, system_info);

            loop {
                interval.tick().await;

                // 检查节点健康
                health_monitor.check_nodes_health();

                // 向已发现的节点发送心跳
                let nodes = {
                    let locked_nodes = discovered_nodes.lock().unwrap();
                    locked_nodes.clone()
                };

                for node_addr in nodes.iter() {
                    // 获取节点健康信息并决定是否重试连接
                    let should_retry = {
                        let health_opt = health_monitor.get_node_health(node_addr);
                        Self::should_retry_connection(&health_opt)
                    };
                    
                    if should_retry {
                        debug!("尝试向节点发送心跳: {}", node_addr);

                        // 发送心跳请求
                        match client.send_heartbeat(node_addr).await {
                            Ok(response) => {
                                debug!("心跳成功: {} -> {}", node_addr, response.node_id);
                                let latency = chrono::Utc::now().timestamp() - response.timestamp;
                                let latency_ms = if latency >= 0 {
                                    Some(latency as u64)
                                } else {
                                    None
                                };

                                // 更新节点状态
                                if let Err(e) =
                                    health_monitor.mark_node_online(node_addr, latency_ms)
                                {
                                    warn!("更新节点状态失败: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("心跳失败: {} - {}", node_addr, e);

                                // 更新节点失败状态
                                if let Err(e) = health_monitor.mark_node_failure(node_addr) {
                                    warn!("更新节点失败状态失败: {}", e);
                                }
                            }
                        }
                    } else {
                        debug!("跳过对节点的重试: {}", node_addr);
                    }
                }

                // 每隔5次循环打印健康报告
                static mut HEALTH_REPORT_COUNTER: u32 = 0;
                unsafe {
                    HEALTH_REPORT_COUNTER += 1;
                    if HEALTH_REPORT_COUNTER % 5 == 0 {
                        let report = health_monitor.generate_health_report();
                        info!("节点健康报告:\n{}", report);
                    }
                }
            }
        });
    }

    /// 检查是否应该重试连接
    fn should_retry_connection(health_opt: &Option<NodeHealth>) -> bool {
        // 从健康监控获取节点状态信息
        if let Some(health) = health_opt {
            // 已确认在线的节点，始终保持连接
            if health.status == NodeStatus::Online {
                return true;
            }

            let now = chrono::Utc::now();
            let offline_duration = (now - health.last_heartbeat).num_seconds() as u64;

            // 根据失败次数和离线时间决定重试策略
            match health.failure_count {
                0 => true, // 没有失败记录，始终尝试连接
                
                1..=3 => {
                    // 少量失败，较高频率重试 (每30秒)
                    debug!("节点 {} 失败次数较少 ({}), 保持定期重试", health.address, health.failure_count);
                    true
                },
                
                4..=10 => {
                    // 中等失败次数，采用指数退避策略
                    // 基础等待时间: 2^(failure_count-3) 分钟
                    let base_wait_mins = 1u64 << (health.failure_count as u64 - 3);
                    let wait_secs = base_wait_mins * 60;
                    
                    // 检查是否达到等待时间
                    let should_retry = offline_duration >= wait_secs;
                    
                    if should_retry {
                        debug!("节点 {} 离线 {} 秒后进行重试，失败次数: {}", 
                               health.address, offline_duration, health.failure_count);
                    } else {
                        debug!("节点 {} 跳过重试，需等待 {} 秒 (已等待 {} 秒)，失败次数: {}", 
                               health.address, wait_secs, offline_duration, health.failure_count);
                    }
                    
                    should_retry
                },
                
                _ => {
                    // 大量失败，可能是长期离线节点
                    // 1小时重试一次
                    let hour_secs = 3600u64;
                    let should_retry = offline_duration >= hour_secs;
                    
                    if should_retry {
                        info!("长期离线节点 {} 一小时后尝试重新连接，失败次数: {}", 
                              health.address, health.failure_count);
                    }
                    
                    should_retry
                }
            }
        } else {
            // 未知节点，默认尝试连接
            debug!("未知节点状态，默认尝试连接");
            true
        }
    }

    /// 连接到指定节点
    pub async fn connect_to_node(&self, address: String) -> Result<NodeInfo> {
        info!("尝试连接到节点: {}", address);

        // 创建节点客户端
        let client = NodeClient::new(
            self.node_id.clone(),
            self.bind_address.clone(),
            self.system_info.clone(),
        );

        // 发送心跳请求
        match client.send_heartbeat(&address).await {
            Ok(response) => {
                info!("成功连接到节点: {} ({})", response.node_id, address);

                // 添加到已知节点列表
                let mut known_nodes = self.known_nodes.lock().await;
                if !known_nodes.contains(&address) {
                    known_nodes.push(address.clone());
                }

                // 更新节点健康状态
                let latency = chrono::Utc::now().timestamp() - response.timestamp;
                let latency_ms = if latency >= 0 {
                    Some(latency as u64)
                } else {
                    None
                };

                self.health_monitor.add_node(
                    response.node_id.clone(),
                    address.clone(),
                    response.system_info.clone(),
                );

                if let Err(e) = self.health_monitor.mark_node_online(&address, latency_ms) {
                    warn!("更新节点状态失败: {}", e);
                }

                // 返回节点信息
                Ok(NodeInfo {
                    id: response.node_id,
                    address: response.address,
                    system: response.system_info,
                    last_seen: response.timestamp,
                })
            }
            Err(e) => {
                warn!("连接到节点失败: {} - {}", address, e);
                Err(e)
            }
        }
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

    /// 添加新节点
    pub async fn add_node(&self, address: String) -> Result<()> {
        // 检查是否已存在该节点
        {
            let nodes = self.discovered_nodes.lock().unwrap();
            if nodes.contains(&address) {
                info!("节点已存在: {}", address);
                return Ok(());
            }
        }

        // 连接到节点
        match self.connect_to_node(address.clone()).await {
            Ok(_) => {
                // 添加到已发现节点列表
                let mut nodes = self.discovered_nodes.lock().unwrap();
                nodes.push(address.clone());
                info!("成功添加节点: {}", address);
                Ok(())
            }
            Err(e) => {
                warn!("添加节点失败: {} - {}", address, e);
                Err(e)
            }
        }
    }

    /// 获取所有节点的健康状态
    pub fn get_nodes_health(&self) -> Vec<NodeHealth> {
        self.health_monitor.get_nodes_health()
    }

    /// 获取健康报告
    pub fn get_health_report(&self) -> String {
        self.health_monitor.generate_health_report()
    }
}

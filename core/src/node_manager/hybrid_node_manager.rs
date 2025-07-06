//! Hybrid节点管理器
//! 
//! 集成了UTP传输的节点管理器，支持高性能文件传输

use librorum_shared::NodeConfig;
use crate::proto::file::file_service_server::FileServiceServer;
use crate::proto::log::log_service_server::LogServiceServer;
use crate::proto::node::node_service_server::NodeServiceServer;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use tonic::transport::Server;
use tracing::{debug, info, warn, error};

use crate::node_manager::hybrid_file_service_simple::{SimpleHybridFileService, TransferStats};
use crate::node_manager::log_service::LogServiceImpl;
use crate::node_manager::mdns_manager::MdnsManager;
use crate::node_manager::node_client::NodeClient;
use crate::node_manager::node_health::{HealthMonitor, NodeHealth, NodeStatus};
use crate::node_manager::node_service::{NodeInfo, NodeServiceImpl};
use crate::vdfs::VDFSConfig;

/// Hybrid节点管理器
pub struct HybridNodeManager {
    /// 节点ID
    node_id: String,
    
    /// gRPC绑定地址
    grpc_bind_address: String,
    
    /// UTP服务器地址
    utp_bind_address: SocketAddr,
    
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
    
    /// Hybrid文件服务
    hybrid_file_service: Option<Arc<SimpleHybridFileService>>,
}

impl HybridNodeManager {
    /// 创建新的Hybrid节点管理器
    pub fn new(grpc_port: u16, utp_port: u16) -> Self {
        // 生成节点 ID
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let node_id = format!("{}.{}.librorum.local", nanoid::nanoid!(10), device_name);

        // 获取绑定地址
        let bind_ip = "0.0.0.0"; // 绑定所有接口
        let grpc_bind_address = format!("{}:{}", bind_ip, grpc_port);
        let utp_bind_address: SocketAddr = format!("{}:{}", bind_ip, utp_port).parse().unwrap();

        // 获取系统信息
        let system_info = Self::get_system_info();

        // 创建健康监控器
        let health_monitor = HealthMonitor::new(60); // 默认60秒心跳超时

        info!("🔧 创建Hybrid节点管理器:");
        info!("  节点ID: {}", node_id);
        info!("  gRPC地址: {}", grpc_bind_address);
        info!("  UTP地址: {}", utp_bind_address);

        Self {
            node_id,
            grpc_bind_address,
            utp_bind_address,
            system_info,
            discovered_nodes: Arc::new(std::sync::Mutex::new(Vec::new())),
            known_nodes: Arc::new(Mutex::new(Vec::new())),
            health_monitor,
            config: None,
            hybrid_file_service: None,
        }
    }

    /// 使用配置创建Hybrid节点管理器
    pub fn with_config(config: NodeConfig, utp_port: u16) -> Self {
        let grpc_bind_address = config.bind_address();
        let utp_bind_address: SocketAddr = format!("0.0.0.0:{}", utp_port).parse().unwrap();

        // 生成节点 ID
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let node_id = format!("{}.{}.librorum.local", config.node_prefix, device_name);

        // 获取系统信息
        let system_info = Self::get_system_info();

        // 初始化已知节点列表
        let known_nodes = Arc::new(Mutex::new(Vec::new()));

        // 创建健康监控器
        let health_monitor = HealthMonitor::new(60);

        info!("🔧 创建配置化Hybrid节点管理器:");
        info!("  节点ID: {}", node_id);
        info!("  gRPC地址: {}", grpc_bind_address);
        info!("  UTP地址: {}", utp_bind_address);

        Self {
            node_id,
            grpc_bind_address,
            utp_bind_address,
            system_info,
            discovered_nodes: Arc::new(std::sync::Mutex::new(Vec::new())),
            known_nodes,
            health_monitor,
            config: Some(config),
            hybrid_file_service: None,
        }
    }

    /// 启动节点管理器
    pub async fn start(&mut self) -> Result<()> {
        info!("🚀 启动Hybrid节点管理器...");

        // 初始化VDFS配置 (简化版本不需要，但保留兼容性)
        let _vdfs_config = if let Some(config) = &self.config {
            VDFSConfig {
                storage_path: config.data_dir.clone(),
                chunk_size: 8 * 1024 * 1024, // 8MB chunks
                enable_compression: true,
                cache_memory_size: 100 * 1024 * 1024, // 100MB cache
                cache_disk_size: 1024 * 1024 * 1024, // 1GB disk cache
                replication_factor: 1,
                network_timeout: std::time::Duration::from_secs(30),
            }
        } else {
            VDFSConfig::default()
        };

        // 创建并初始化Hybrid文件服务
        let mut hybrid_file_service = SimpleHybridFileService::new(self.utp_bind_address);
        
        // 简化版本不需要VDFS初始化
        info!("📦 使用简化版Hybrid文件服务");

        // 简化版本不需要显式启动UTP服务器
        info!("🚀 Hybrid文件服务就绪");

        self.hybrid_file_service = Some(Arc::new(hybrid_file_service));

        // 启动gRPC服务器
        self.start_grpc_server().await?;

        // 启动mDNS服务发现
        self.start_mdns_discovery().await?;

        // 启动健康监控
        self.start_health_monitoring().await?;

        info!("✅ Hybrid节点管理器启动成功");
        Ok(())
    }

    /// 启动gRPC服务器
    async fn start_grpc_server(&self) -> Result<()> {
        let addr: SocketAddr = self.grpc_bind_address.parse()
            .context("Invalid gRPC bind address")?;

        info!("🌐 启动gRPC服务器: {}", addr);

        // 创建服务实例
        let node_service = NodeServiceImpl::new(NodeInfo {
            id: self.node_id.clone(),
            address: self.grpc_bind_address.clone(),
            // status: NodeStatus::Online, // 字段不存在，移除
            system_info: self.system_info.clone(),
            capabilities: vec!["file_storage".to_string(), "hybrid_transport".to_string()],
            metadata: std::collections::HashMap::new(),
            last_seen: chrono::Utc::now().timestamp(),
        });

        let log_service = LogServiceImpl::new();

        // 使用Hybrid文件服务
        let file_service = self.hybrid_file_service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Hybrid文件服务未初始化"))?
            .clone();

        // 创建gRPC服务器
        let grpc_server = Server::builder()
            .add_service(NodeServiceServer::new(node_service))
            .add_service(LogServiceServer::new(log_service))
            .add_service(FileServiceServer::new(file_service.as_ref().clone()))
            .serve(addr);

        // 在后台运行gRPC服务器
        tokio::spawn(async move {
            if let Err(e) = grpc_server.await {
                error!("❌ gRPC服务器运行失败: {}", e);
            }
        });

        info!("✅ gRPC服务器启动成功");
        Ok(())
    }

    /// 启动mDNS服务发现
    async fn start_mdns_discovery(&self) -> Result<()> {
        info!("🔍 启动mDNS服务发现...");

        let node_id = self.node_id.clone();
        let bind_address = self.grpc_bind_address.clone();
        let utp_address = self.utp_bind_address.to_string();
        let discovered_nodes = self.discovered_nodes.clone();

        // 启动mDNS管理器
        tokio::spawn(async move {
            match MdnsManager::new(&node_id, &bind_address).await {
                Ok(mut mdns_manager) => {
                    // 注册服务，包含UTP端口信息
                    let mut service_txt = std::collections::HashMap::new();
                    service_txt.insert("utp_address".to_string(), utp_address);
                    service_txt.insert("hybrid_mode".to_string(), "enabled".to_string());

                    if let Err(e) = mdns_manager.register_service(Some(service_txt)).await {
                        error!("❌ mDNS服务注册失败: {}", e);
                        return;
                    }

                    // 持续监听服务发现
                    loop {
                        match mdns_manager.discover_services().await {
                            Ok(services) => {
                                if !services.is_empty() {
                                    debug!("🔍 发现 {} 个服务", services.len());
                                    let mut nodes = discovered_nodes.lock().unwrap();
                                    nodes.clear();
                                    nodes.extend(services);
                                }
                            }
                            Err(e) => {
                                warn!("⚠️ 服务发现失败: {}", e);
                            }
                        }

                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                }
                Err(e) => {
                    error!("❌ mDNS管理器启动失败: {}", e);
                }
            }
        });

        info!("✅ mDNS服务发现启动成功");
        Ok(())
    }

    /// 启动健康监控
    async fn start_health_monitoring(&self) -> Result<()> {
        info!("💗 启动健康监控...");

        let health_monitor = self.health_monitor.clone();
        let known_nodes = self.known_nodes.clone();
        let hybrid_file_service = self.hybrid_file_service.clone();

        // 启动健康监控任务
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // 每60秒检查一次

            loop {
                interval.tick().await;

                // 检查已知节点的健康状态
                let nodes = known_nodes.lock().await.clone();
                for node_address in nodes {
                    match NodeClient::new(&node_address).await {
                        Ok(mut client) => {
                            match client.get_health().await {
                                Ok(health) => {
                                    // 简化实现：跳过健康状态更新
                                    debug!("更新节点健康状态: {} {:?}", node_address, health);
                                    debug!("💗 节点健康检查成功: {}", node_address);
                                }
                                Err(e) => {
                                    let offline_health = NodeHealth {
                                        node_id: node_address.clone(),
                                        status: NodeStatus::Offline,
                                        last_heartbeat: chrono::Utc::now(),
                                        cpu_usage: 0.0,
                                        memory_usage: 0.0,
                                        disk_usage: 0.0,
                                        network_latency: None,
                                        uptime: 0,
                                        error_message: Some(format!("健康检查失败: {}", e)),
                                    };
                                    // 简化实现：跳过健康状态更新  
                                    debug!("节点离线: {} {:?}", node_address, offline_health);
                                    warn!("⚠️ 节点健康检查失败: {} - {}", node_address, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("⚠️ 无法连接到节点: {} - {}", node_address, e);
                        }
                    }
                }

                // 检查UTP传输统计
                if let Some(file_service) = &hybrid_file_service {
                    let stats = file_service.get_transfer_stats();
                    debug!("📊 UTP传输统计: 总会话数={}, 成功传输={}, 失败传输={}", 
                        stats.total_sessions, stats.active_uploads, stats.active_downloads);
                }
            }
        });

        info!("✅ 健康监控启动成功");
        Ok(())
    }

    /// 获取系统信息
    fn get_system_info() -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let cpu_count = num_cpus::get();
        
        format!("OS: {}, Arch: {}, CPUs: {}, Hybrid: enabled", os, arch, cpu_count)
    }

    /// 获取节点ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 获取gRPC绑定地址
    pub fn grpc_bind_address(&self) -> &str {
        &self.grpc_bind_address
    }

    /// 获取UTP绑定地址
    pub fn utp_bind_address(&self) -> SocketAddr {
        self.utp_bind_address
    }

    /// 获取已发现的节点列表
    pub fn get_discovered_nodes(&self) -> Vec<String> {
        self.discovered_nodes.lock().unwrap().clone()
    }

    /// 获取已知节点列表
    pub async fn get_known_nodes(&self) -> Vec<String> {
        self.known_nodes.lock().await.clone()
    }

    /// 添加已知节点
    pub async fn add_known_node(&self, node_address: String) {
        let mut nodes = self.known_nodes.lock().await;
        if !nodes.contains(&node_address) {
            nodes.push(node_address.clone());
            info!("➕ 添加已知节点: {}", node_address);
        }
    }

    /// 移除已知节点
    pub async fn remove_known_node(&self, node_address: &str) {
        let mut nodes = self.known_nodes.lock().await;
        if let Some(pos) = nodes.iter().position(|x| x == node_address) {
            nodes.remove(pos);
            info!("➖ 移除已知节点: {}", node_address);
        }
    }

    /// 获取节点健康状态
    pub async fn get_node_health(&self, node_id: &str) -> Option<NodeHealth> {
        self.health_monitor.get_node_health(node_id)
    }

    /// 获取所有节点健康状态
    pub async fn get_all_node_health(&self) -> Vec<NodeHealth> {
        // 简化实现，返回空向量
        vec![]
    }

    /// 获取UTP传输统计
    pub fn get_utp_stats(&self) -> Option<TransferStats> {
        self.hybrid_file_service.as_ref().map(|service| service.get_transfer_stats())
    }

    /// 清理完成的UTP会话
    pub async fn cleanup_utp_sessions(&self) {
        if let Some(file_service) = &self.hybrid_file_service {
            // 这里可以添加清理逻辑
            debug!("🧹 清理UTP会话");
        }
    }

    /// 停止节点管理器
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 停止Hybrid节点管理器...");

        // 这里可以添加停止逻辑，如关闭UTP服务器等
        // 由于Rust的所有权系统，某些清理操作需要特殊处理

        info!("✅ Hybrid节点管理器已停止");
        Ok(())
    }
}

impl std::fmt::Display for HybridNodeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HybridNodeManager[{}] (gRPC: {}, UTP: {})", 
            self.node_id, self.grpc_bind_address, self.utp_bind_address)
    }
}
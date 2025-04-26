pub mod proto;
pub mod fs;
pub mod service;
pub mod error;
pub mod config;
pub mod cluster;
pub mod discovery;
pub mod cli;
pub mod client;
pub mod metadata;
pub mod sync;
#[cfg(test)]
mod tests {
    mod config_test;
}

pub use fs::LocalFileSystem;
pub use service::VDFSServiceImpl;
pub use config::NodeConfig;
pub use cluster::ClusterManager;
pub use discovery::DiscoveryService;
pub use error::Error;
pub use cli::Cli;
pub use client::VDFSClient;

use std::sync::Arc;
use tonic::transport::Server;
use proto::vdfs::vdfs_service_server::VdfsServiceServer;

pub async fn start_server(
    config: NodeConfig,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 创建文件系统
    let fs = Arc::new(LocalFileSystem::new(&config.root_dir).await?);
    
    // 创建节点信息
    let node_info = proto::vdfs::NodeInfo {
        id: config.id.clone(),
        name: config.name.clone(),
        host: config.host.clone(),
        port: config.port as i32,
        status: proto::vdfs::NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };
    
    // 创建集群配置
    let cluster_config = cluster::ClusterConfig::default();
    
    // 创建集群管理器
    let cluster_manager = Arc::new(ClusterManager::new(cluster_config, node_info.clone()));
    
    // 创建服务实现
    let service = VDFSServiceImpl::with_cluster_manager(fs, cluster_manager).await?;
    
    // 创建服务器
    let addr = addr.parse().map_err(|e| 
        format!("Failed to parse address: {}", e)
    )?;
    let service = VdfsServiceServer::new(service);
    
    // 创建发现服务
    let mut discovery_service = DiscoveryService::new(node_info);
    if let Err(e) = discovery_service.start().await {
        eprintln!("Failed to start discovery service: {}", e);
    }
    
    println!("VDFS Server listening on {}", addr);
    
    Server::builder()
        .add_service(service)
        .serve(addr)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { 
            Box::new(error::Error::new(&format!("Server error: {}", e)))
        })?;
    
    // 停止发现服务
    if let Err(e) = discovery_service.stop().await {
        eprintln!("Failed to stop discovery service: {}", e);
    }
    
    Ok(())
} 
pub mod error;
pub mod fs;
pub mod config;
pub mod metadata;
pub mod sync;
pub mod proto;
pub mod service;

use std::net::SocketAddr;
use tonic::transport::Server;
use crate::config::{NodeConfig, ClusterConfig};
use crate::error::Result;
use crate::fs::LocalFileSystem;
use crate::metadata::MetadataStore;
use crate::service::VDFSServiceImpl;
use crate::sync::SyncManager;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;
use tokio::fs as tokio_fs;
use std::os::unix::fs::PermissionsExt;

pub async fn start_server(node_config: NodeConfig, cluster_config: ClusterConfig) -> Result<()> {
    // 初始化文件系统
    let fs = LocalFileSystem::new(&node_config.root_dir).await?;

    // 初始化数据库连接
    let root_dir = node_config.root_dir.canonicalize().map_err(|e| crate::error::VDFSError::Io(e))?;
    let db_dir = root_dir.join("db");
    tokio_fs::create_dir_all(&db_dir).await.map_err(|e| crate::error::VDFSError::Io(e))?;
    let db_path = db_dir.join("metadata.db");
    
    // 设置数据库目录权限
    let mut perms = tokio_fs::metadata(&db_dir).await?.permissions();
    perms.set_mode(0o755);
    tokio_fs::set_permissions(&db_dir, perms).await?;
    
    // 创建空的数据库文件
    if !db_path.exists() {
        tokio_fs::File::create(&db_path).await.map_err(|e| crate::error::VDFSError::Io(e))?;
    }
    
    let db_url = format!("sqlite:{}", db_path.display());
    println!("Database URL: {}", db_url);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| crate::error::VDFSError::Database(e))?;

    // 初始化元数据存储
    let metadata_store = MetadataStore::new(pool);
    metadata_store.init().await?;
    let metadata_store = Arc::new(RwLock::new(metadata_store));

    // 初始化同步管理器
    let sync_manager = SyncManager::new(
        metadata_store.clone(),
        cluster_config.clone(),
        node_config.clone(),
    );

    // 创建服务实例
    let service = VDFSServiceImpl::new(fs);

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], node_config.port));
    println!("VDFS server listening on {}", addr);

    Server::builder()
        .add_service(crate::proto::vdfs::vdfs_service_server::VdfsServiceServer::new(service))
        .serve(addr)
        .await
        .map_err(|e| crate::error::VDFSError::Grpc(tonic::Status::internal(e.to_string())))?;

    Ok(())
} 
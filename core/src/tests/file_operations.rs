use crate::config::{ClusterConfig, NodeConfig};
use crate::fs::LocalFileSystem;
use crate::metadata::MetadataStore;
use crate::service::VDFSServiceImpl;
use crate::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;

pub async fn setup_test_environment() -> (VDFSServiceImpl, String) {
    // 创建测试配置
    let node_config = NodeConfig {
        id: "test_node".to_string(),
        name: "Test Node".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        root_dir: std::env::temp_dir().join("vdfs_test"),
        max_file_size: 1024 * 1024,
        chunk_size: 1024,
        workers: 1,
    };

    let cluster_config = ClusterConfig {
        nodes: vec![node_config.clone()],
        sync_interval: 60,
        p2p_enabled: true,
    };

    // 创建文件系统实例
    let fs = Arc::new(LocalFileSystem::new(node_config.root_dir.clone()));
    
    // 创建元数据存储
    let metadata = Arc::new(RwLock::new(MetadataStore::new()));
    
    // 创建同步管理器
    let sync_manager = SyncManager::new(
        cluster_config,
        metadata.clone(),
        node_config.clone(),
    );
    
    // 创建服务实例
    let service = VDFSServiceImpl::new(fs, metadata, sync_manager);
    
    (service, node_config.id)
}

#[tokio::test]
async fn test_list_directory() {
    let (service, node_id) = setup_test_environment().await;
    
    // 创建测试目录
    let test_dir = std::env::temp_dir().join("vdfs_test/test_dir");
    std::fs::create_dir_all(&test_dir).unwrap();
    
    // 创建测试文件
    let test_file = test_dir.join("test.txt");
    std::fs::write(&test_file, "test content").unwrap();
    
    // 测试列出目录
    let request = tonic::Request::new(crate::proto::vdfs::ListDirectoryRequest {
        path: test_dir.to_string_lossy().into_owned(),
        node_id: node_id.clone(),
    });
    
    let response = service.list_directory(request).await.unwrap();
    let response = response.into_inner();
    
    assert!(!response.entries.is_empty());
    assert_eq!(response.entries[0].name, "test.txt");
}

#[tokio::test]
async fn test_create_file() {
    let (service, node_id) = setup_test_environment().await;
    
    // 测试创建文件
    let request = tonic::Request::new(crate::proto::vdfs::CreateFileRequest {
        path: "/test.txt".to_string(),
        r#type: crate::proto::vdfs::FileType::File as i32,
        node_id: node_id.clone(),
    });
    
    let response = service.create_file(request).await.unwrap();
    let response = response.into_inner();
    
    assert_eq!(response.info.name, "test.txt");
    assert_eq!(response.info.r#type, crate::proto::vdfs::FileType::File as i32);
}

#[tokio::test]
async fn test_delete_file() {
    let (service, node_id) = setup_test_environment().await;
    
    // 创建测试文件
    let test_file = std::env::temp_dir().join("vdfs_test/test.txt");
    std::fs::write(&test_file, "test content").unwrap();
    
    // 测试删除文件
    let request = tonic::Request::new(crate::proto::vdfs::DeleteFileRequest {
        path: test_file.to_string_lossy().into_owned(),
        node_id: node_id.clone(),
    });
    
    let response = service.delete_file(request).await.unwrap();
    let response = response.into_inner();
    
    assert!(response.success);
    assert!(!test_file.exists());
}

#[tokio::test]
async fn test_move_file() {
    let (service, node_id) = setup_test_environment().await;
    
    // 创建源文件
    let source_file = std::env::temp_dir().join("vdfs_test/source.txt");
    std::fs::write(&source_file, "test content").unwrap();
    
    // 创建目标目录
    let target_dir = std::env::temp_dir().join("vdfs_test/target");
    std::fs::create_dir_all(&target_dir).unwrap();
    
    // 测试移动文件
    let request = tonic::Request::new(crate::proto::vdfs::MoveFileRequest {
        source_path: source_file.to_string_lossy().into_owned(),
        target_path: target_dir.join("moved.txt").to_string_lossy().into_owned(),
        source_node: node_id.clone(),
        target_node: node_id.clone(),
    });
    
    let response = service.move_file(request).await.unwrap();
    let response = response.into_inner();
    
    assert!(response.success);
    assert!(!source_file.exists());
    assert!(target_dir.join("moved.txt").exists());
}

#[tokio::test]
async fn test_copy_file() {
    let (service, node_id) = setup_test_environment().await;
    
    // 创建源文件
    let source_file = std::env::temp_dir().join("vdfs_test/source.txt");
    std::fs::write(&source_file, "test content").unwrap();
    
    // 创建目标目录
    let target_dir = std::env::temp_dir().join("vdfs_test/target");
    std::fs::create_dir_all(&target_dir).unwrap();
    
    // 测试复制文件
    let request = tonic::Request::new(crate::proto::vdfs::CopyFileRequest {
        source_path: source_file.to_string_lossy().into_owned(),
        target_path: target_dir.join("copied.txt").to_string_lossy().into_owned(),
        source_node: node_id.clone(),
        target_node: node_id.clone(),
    });
    
    let response = service.copy_file(request).await.unwrap();
    let response = response.into_inner();
    
    assert!(response.success);
    assert!(source_file.exists());
    assert!(target_dir.join("copied.txt").exists());
}

#[tokio::test]
async fn test_read_write_file() {
    let (service, node_id) = setup_test_environment().await;
    
    // 创建测试文件
    let test_file = std::env::temp_dir().join("vdfs_test/test.txt");
    let content = "test content".as_bytes();
    
    // 写入文件
    let request = tonic::Request::new(crate::proto::vdfs::WriteFileRequest {
        path: test_file.to_string_lossy().into_owned(),
        offset: 0,
        data: content.to_vec(),
        node_id: node_id.clone(),
    });
    
    let response = service.write_file(request).await.unwrap();
    let response = response.into_inner();
    
    assert_eq!(response.bytes_written, content.len() as i64);
    
    // 读取文件
    let request = tonic::Request::new(crate::proto::vdfs::ReadFileRequest {
        path: test_file.to_string_lossy().into_owned(),
        offset: 0,
        length: content.len() as i64,
        node_id: node_id.clone(),
    });
    
    let mut response_stream = service.read_file(request).await.unwrap();
    let response = response_stream.message().await.unwrap().unwrap();
    
    assert_eq!(response.data, content);
} 
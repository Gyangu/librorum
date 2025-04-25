use crate::config::{ClusterConfig, NodeConfig};
use crate::fs::LocalFileSystem;
use crate::metadata::{FileMetadata, MetadataStore, NodeStatus};
use crate::service::VDFSServiceImpl;
use crate::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;

pub async fn setup_test_nodes() -> (VDFSServiceImpl, VDFSServiceImpl) {
    // 创建节点1配置
    let node1_config = NodeConfig {
        id: "node1".to_string(),
        name: "Node 1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        root_dir: std::env::temp_dir().join("vdfs_test/node1"),
        max_file_size: 1024 * 1024,
        chunk_size: 1024,
        workers: 1,
    };

    // 创建节点2配置
    let node2_config = NodeConfig {
        id: "node2".to_string(),
        name: "Node 2".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50052,
        root_dir: std::env::temp_dir().join("vdfs_test/node2"),
        max_file_size: 1024 * 1024,
        chunk_size: 1024,
        workers: 1,
    };

    let cluster_config = ClusterConfig {
        nodes: vec![node1_config.clone(), node2_config.clone()],
        sync_interval: 60,
        p2p_enabled: true,
    };

    // 创建节点1服务
    let fs1 = Arc::new(LocalFileSystem::new(node1_config.root_dir.clone()));
    let metadata1 = Arc::new(RwLock::new(MetadataStore::new()));
    let sync_manager1 = SyncManager::new(
        cluster_config.clone(),
        metadata1.clone(),
        node1_config.clone(),
    );
    let service1 = VDFSServiceImpl::new(fs1, metadata1, sync_manager1);

    // 创建节点2服务
    let fs2 = Arc::new(LocalFileSystem::new(node2_config.root_dir.clone()));
    let metadata2 = Arc::new(RwLock::new(MetadataStore::new()));
    let sync_manager2 = SyncManager::new(
        cluster_config,
        metadata2.clone(),
        node2_config.clone(),
    );
    let service2 = VDFSServiceImpl::new(fs2, metadata2, sync_manager2);

    (service1, service2)
}

#[tokio::test]
async fn test_metadata_sync() {
    let (service1, service2) = setup_test_nodes().await;

    // 在节点1上创建文件
    let request = tonic::Request::new(crate::proto::vdfs::CreateFileRequest {
        path: "/test.txt".to_string(),
        r#type: crate::proto::vdfs::FileType::File as i32,
        node_id: "node1".to_string(),
    });

    let response = service1.create_file(request).await.unwrap();
    let file_info = response.into_inner().info;

    // 同步元数据
    let request = tonic::Request::new(crate::proto::vdfs::SyncMetadataRequest {
        node_id: "node2".to_string(),
        last_sync_time: 0,
    });

    let mut response_stream = service1.sync_metadata(request).await.unwrap();
    let response = response_stream.message().await.unwrap().unwrap();

    // 验证节点2是否收到文件信息
    assert!(!response.files.is_empty());
    assert_eq!(response.files[0].id, file_info.id);
}

#[tokio::test]
async fn test_node_status() {
    let (service1, _) = setup_test_nodes().await;

    // 获取节点状态
    let request = tonic::Request::new(crate::proto::vdfs::GetNodeStatusRequest {
        node_id: "node1".to_string(),
    });

    let response = service1.get_node_status(request).await.unwrap();
    let status = response.into_inner().status;

    assert_eq!(status.id, "node1");
    assert!(status.is_online);
}

#[tokio::test]
async fn test_metadata_sync_with_multiple_files() {
    let (service1, service2) = setup_test_nodes().await;

    // 在节点1上创建多个文件
    let files = vec![
        ("file1.txt", "content1"),
        ("file2.txt", "content2"),
        ("file3.txt", "content3"),
    ];

    for (name, content) in files {
        let request = tonic::Request::new(crate::proto::vdfs::CreateFileRequest {
            path: format!("/{}", name),
            r#type: crate::proto::vdfs::FileType::File as i32,
            node_id: "node1".to_string(),
        });

        service1.create_file(request).await.unwrap();
    }

    // 同步元数据
    let request = tonic::Request::new(crate::proto::vdfs::SyncMetadataRequest {
        node_id: "node2".to_string(),
        last_sync_time: 0,
    });

    let mut response_stream = service1.sync_metadata(request).await.unwrap();
    let response = response_stream.message().await.unwrap().unwrap();

    // 验证节点2是否收到所有文件信息
    assert_eq!(response.files.len(), files.len());
    for (name, _) in files {
        assert!(response.files.iter().any(|f| f.name == name));
    }
}

#[tokio::test]
async fn test_metadata_sync_with_updates() {
    let (service1, service2) = setup_test_nodes().await;

    // 在节点1上创建文件
    let request = tonic::Request::new(crate::proto::vdfs::CreateFileRequest {
        path: "/test.txt".to_string(),
        r#type: crate::proto::vdfs::FileType::File as i32,
        node_id: "node1".to_string(),
    });

    let response = service1.create_file(request).await.unwrap();
    let file_info = response.into_inner().info;

    // 第一次同步
    let request = tonic::Request::new(crate::proto::vdfs::SyncMetadataRequest {
        node_id: "node2".to_string(),
        last_sync_time: 0,
    });

    let mut response_stream = service1.sync_metadata(request).await.unwrap();
    let response = response_stream.message().await.unwrap().unwrap();
    let sync_time = response.sync_time;

    // 修改文件
    let request = tonic::Request::new(crate::proto::vdfs::WriteFileRequest {
        path: "/test.txt".to_string(),
        offset: 0,
        data: "updated content".as_bytes().to_vec(),
        node_id: "node1".to_string(),
    });

    service1.write_file(request).await.unwrap();

    // 第二次同步
    let request = tonic::Request::new(crate::proto::vdfs::SyncMetadataRequest {
        node_id: "node2".to_string(),
        last_sync_time: sync_time,
    });

    let mut response_stream = service1.sync_metadata(request).await.unwrap();
    let response = response_stream.message().await.unwrap().unwrap();

    // 验证节点2是否收到更新
    assert!(!response.files.is_empty());
    assert_eq!(response.files[0].id, file_info.id);
    assert!(response.sync_time > sync_time);
} 
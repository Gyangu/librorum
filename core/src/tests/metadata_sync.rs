use crate::config::{ClusterConfig, NodeConfig};
use crate::fs::LocalFileSystem;
use crate::metadata::{FileMetadata, MetadataStore, NodeStatus};
use crate::service::VDFSServiceImpl;
use crate::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;
use std::path::PathBuf;
use tempfile::tempdir;
use tonic::Request;
use crate::proto::vdfs::{
    CreateFileRequest, WriteFileRequest, SyncMetadataRequest,
    GetNodeStatusRequest, FileType, NodeStatus,
};
use crate::proto::vdfs::vdfs_service_server::VdfsService;
use crate::cluster::ClusterManager;
use crate::proto::vdfs::NodeInfo;
use tokio_stream::StreamExt;
use futures::stream;

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
    let fs1 = LocalFileSystem::new(&node1_config.root_dir).await.unwrap();
    let service1 = VDFSServiceImpl::new(fs1);

    // 创建节点2服务
    let fs2 = LocalFileSystem::new(&node2_config.root_dir).await.unwrap();
    let service2 = VDFSServiceImpl::new(fs2);

    (service1, service2)
}

#[tokio::test]
async fn test_sync_metadata() {
    let (service1, service2) = setup_test_nodes().await;

    // 在节点1上创建文件
    let create_request = Request::new(CreateFileRequest {
        path: "/test.txt".to_string(),
        node_id: "node1".to_string(),
        r#type: FileType::File as i32,
    });
    service1.create_file(create_request).await.unwrap();

    // 写入文件内容
    let write_request = Request::new_streaming(stream::iter(vec![WriteFileRequest {
        path: "/test.txt".to_string(),
        offset: 0,
        data: b"Hello, World!".to_vec(),
        node_id: "node1".to_string(),
    }]));
    service1.write_file(write_request).await.unwrap();

    // 同步元数据
    let sync_request = Request::new(SyncMetadataRequest {
        node_id: "node2".to_string(),
        last_sync_time: 0,
    });
    let mut response_stream = service2.sync_metadata(sync_request).await.unwrap().into_inner();

    // 验证同步结果
    while let Some(response) = response_stream.next().await {
        let response = response.unwrap();
        assert!(!response.files.is_empty());
        let file = &response.files[0];
        assert_eq!(file.path, "/test.txt");
        assert_eq!(file.owner_node, "node1");
    }
}

#[tokio::test]
async fn test_metadata_sync() {
    // 创建两个临时目录，模拟两个节点
    let temp_dir1 = tempdir().unwrap();
    let temp_dir2 = tempdir().unwrap();
    let root_path1 = temp_dir1.path().to_path_buf();
    let root_path2 = temp_dir2.path().to_path_buf();

    // 创建两个文件系统实例
    let fs1 = Arc::new(LocalFileSystem::new(&root_path1).await.unwrap());
    let fs2 = Arc::new(LocalFileSystem::new(&root_path2).await.unwrap());

    // 创建节点信息
    let node_info1 = NodeInfo {
        id: "node1".to_string(),
        name: "node1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        status: NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    let node_info2 = NodeInfo {
        id: "node2".to_string(),
        name: "node2".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50052,
        status: NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    // 创建集群配置
    let cluster_config1 = ClusterConfig::default();
    let cluster_config2 = ClusterConfig::default();

    // 创建集群管理器
    let cluster_manager1 = ClusterManager::new(cluster_config1, node_info1.clone());
    let cluster_manager2 = ClusterManager::new(cluster_config2, node_info2.clone());

    // 创建服务实现
    let service1 = VDFSServiceImpl::with_cluster_manager(fs1, cluster_manager1);
    let service2 = VDFSServiceImpl::with_cluster_manager(fs2, cluster_manager2);

    // 在节点1上创建文件
    let request = Request::new(CreateFileRequest {
        path: "/test.txt".to_string(),
        r#type: FileType::File as i32,
        node_id: node_info1.id.clone(),
    });
    let response = service1.create_file(request).await.unwrap();
    let response = response.into_inner();
    assert!(response.info.is_some());
    let file_info = response.info.unwrap();
    assert_eq!(file_info.name, "test.txt");
    assert_eq!(file_info.r#type, FileType::File as i32);

    // 写入一些数据
    let write_request = WriteFileRequest {
        path: "/test.txt".to_string(),
        data: b"Hello, World!".to_vec(),
        offset: 0,
        node_id: node_info1.id.clone(),
    };
    let request = Request::new(write_request);
    let response = service1.write_file(request).await.unwrap();
    let response = response.into_inner();
    assert_eq!(response.bytes_written, 13);

    // 同步元数据到节点2
    let request = Request::new(SyncMetadataRequest {
        node_id: node_info2.id.clone(),
        last_sync_time: 0,
    });
    let response = service1.sync_metadata(request).await.unwrap();
    let response = response.into_inner();
    assert!(!response.files.is_empty());
    assert_eq!(response.files[0].name, "test.txt");

    // 检查节点状态
    let request = Request::new(GetNodeStatusRequest {
        node_id: node_info1.id.clone(),
    });
    let response = service1.get_node_status(request).await.unwrap();
    let status = response.into_inner();
    assert!(status.status.is_some());
    let node_status = status.status.unwrap();
    assert_eq!(node_status.id, node_info1.id);
    assert_eq!(node_status.status, NodeStatus::NodeOnline as i32);
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
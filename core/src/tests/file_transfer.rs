use crate::config::{ClusterConfig, NodeConfig};
use crate::fs::LocalFileSystem;
use crate::metadata::MetadataStore;
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
async fn test_drop_file() {
    let (service1, service2) = setup_test_nodes().await;

    // 在节点1上创建文件
    let request = tonic::Request::new(crate::proto::vdfs::CreateFileRequest {
        path: "/test.txt".to_string(),
        r#type: crate::proto::vdfs::FileType::File as i32,
        node_id: "node1".to_string(),
    });

    let response = service1.create_file(request).await.unwrap();
    let file_info = response.into_inner().info;

    // 写入文件内容
    let content = "test content".as_bytes();
    let request = tonic::Request::new(crate::proto::vdfs::WriteFileRequest {
        path: "/test.txt".to_string(),
        offset: 0,
        data: content.to_vec(),
        node_id: "node1".to_string(),
    });

    service1.write_file(request).await.unwrap();

    // 传输文件到节点2
    let request = tonic::Request::new(crate::proto::vdfs::DropFileRequest {
        file_id: file_info.id,
        source_node: "node1".to_string(),
        target_node: "node2".to_string(),
    });

    let mut response_stream = service1.drop_file(request).await.unwrap();
    
    // 接收文件信息
    let response = response_stream.message().await.unwrap().unwrap();
    let file_info = match response.response {
        Some(crate::proto::vdfs::drop_file_response::Response::FileInfo(info)) => info,
        _ => panic!("Expected file info"),
    };

    // 接收文件内容
    let response = response_stream.message().await.unwrap().unwrap();
    let chunk = match response.response {
        Some(crate::proto::vdfs::drop_file_response::Response::Chunk(data)) => data,
        _ => panic!("Expected chunk"),
    };

    // 在节点2上接收文件
    let mut request_stream = tokio_stream::iter(vec![
        crate::proto::vdfs::ReceiveFileRequest {
            request: Some(crate::proto::vdfs::receive_file_request::Request::FileInfo(file_info)),
        },
        crate::proto::vdfs::ReceiveFileRequest {
            request: Some(crate::proto::vdfs::receive_file_request::Request::Chunk(chunk)),
        },
    ]);

    let response = service2.receive_file(tonic::Request::new(request_stream)).await.unwrap();
    let response = response.into_inner();

    assert!(response.success);

    // 验证文件内容
    let request = tonic::Request::new(crate::proto::vdfs::ReadFileRequest {
        path: "/test.txt".to_string(),
        offset: 0,
        length: content.len() as i64,
        node_id: "node2".to_string(),
    });

    let mut response_stream = service2.read_file(request).await.unwrap();
    let response = response_stream.message().await.unwrap().unwrap();

    assert_eq!(response.data, content);
}

#[tokio::test]
async fn test_drop_large_file() {
    let (service1, service2) = setup_test_nodes().await;

    // 在节点1上创建大文件
    let request = tonic::Request::new(crate::proto::vdfs::CreateFileRequest {
        path: "/large.txt".to_string(),
        r#type: crate::proto::vdfs::FileType::File as i32,
        node_id: "node1".to_string(),
    });

    let response = service1.create_file(request).await.unwrap();
    let file_info = response.into_inner().info;

    // 写入大文件内容
    let content = vec![0u8; 1024 * 1024]; // 1MB
    let request = tonic::Request::new(crate::proto::vdfs::WriteFileRequest {
        path: "/large.txt".to_string(),
        offset: 0,
        data: content.clone(),
        node_id: "node1".to_string(),
    });

    service1.write_file(request).await.unwrap();

    // 传输文件到节点2
    let request = tonic::Request::new(crate::proto::vdfs::DropFileRequest {
        file_id: file_info.id,
        source_node: "node1".to_string(),
        target_node: "node2".to_string(),
    });

    let mut response_stream = service1.drop_file(request).await.unwrap();
    
    // 接收文件信息
    let response = response_stream.message().await.unwrap().unwrap();
    let file_info = match response.response {
        Some(crate::proto::vdfs::drop_file_response::Response::FileInfo(info)) => info,
        _ => panic!("Expected file info"),
    };

    // 接收文件内容
    let mut chunks = Vec::new();
    while let Some(response) = response_stream.message().await.unwrap() {
        if let Some(crate::proto::vdfs::drop_file_response::Response::Chunk(data)) = response.response {
            chunks.push(data);
        }
    }

    // 在节点2上接收文件
    let mut request_stream = tokio_stream::iter(
        std::iter::once(crate::proto::vdfs::ReceiveFileRequest {
            request: Some(crate::proto::vdfs::receive_file_request::Request::FileInfo(file_info)),
        })
        .chain(chunks.into_iter().map(|chunk| {
            crate::proto::vdfs::ReceiveFileRequest {
                request: Some(crate::proto::vdfs::receive_file_request::Request::Chunk(chunk)),
            }
        })),
    );

    let response = service2.receive_file(tonic::Request::new(request_stream)).await.unwrap();
    let response = response.into_inner();

    assert!(response.success);

    // 验证文件内容
    let request = tonic::Request::new(crate::proto::vdfs::ReadFileRequest {
        path: "/large.txt".to_string(),
        offset: 0,
        length: content.len() as i64,
        node_id: "node2".to_string(),
    });

    let mut response_stream = service2.read_file(request).await.unwrap();
    let response = response_stream.message().await.unwrap().unwrap();

    assert_eq!(response.data, content);
}

#[tokio::test]
async fn test_drop_multiple_files() {
    let (service1, service2) = setup_test_nodes().await;

    // 在节点1上创建多个文件
    let files = vec![
        ("file1.txt", "content1"),
        ("file2.txt", "content2"),
        ("file3.txt", "content3"),
    ];

    for (name, content) in files {
        // 创建文件
        let request = tonic::Request::new(crate::proto::vdfs::CreateFileRequest {
            path: format!("/{}", name),
            r#type: crate::proto::vdfs::FileType::File as i32,
            node_id: "node1".to_string(),
        });

        let response = service1.create_file(request).await.unwrap();
        let file_info = response.into_inner().info;

        // 写入文件内容
        let request = tonic::Request::new(crate::proto::vdfs::WriteFileRequest {
            path: format!("/{}", name),
            offset: 0,
            data: content.as_bytes().to_vec(),
            node_id: "node1".to_string(),
        });

        service1.write_file(request).await.unwrap();

        // 传输文件到节点2
        let request = tonic::Request::new(crate::proto::vdfs::DropFileRequest {
            file_id: file_info.id,
            source_node: "node1".to_string(),
            target_node: "node2".to_string(),
        });

        let mut response_stream = service1.drop_file(request).await.unwrap();
        
        // 接收文件信息
        let response = response_stream.message().await.unwrap().unwrap();
        let file_info = match response.response {
            Some(crate::proto::vdfs::drop_file_response::Response::FileInfo(info)) => info,
            _ => panic!("Expected file info"),
        };

        // 接收文件内容
        let response = response_stream.message().await.unwrap().unwrap();
        let chunk = match response.response {
            Some(crate::proto::vdfs::drop_file_response::Response::Chunk(data)) => data,
            _ => panic!("Expected chunk"),
        };

        // 在节点2上接收文件
        let mut request_stream = tokio_stream::iter(vec![
            crate::proto::vdfs::ReceiveFileRequest {
                request: Some(crate::proto::vdfs::receive_file_request::Request::FileInfo(file_info)),
            },
            crate::proto::vdfs::ReceiveFileRequest {
                request: Some(crate::proto::vdfs::receive_file_request::Request::Chunk(chunk)),
            },
        ]);

        let response = service2.receive_file(tonic::Request::new(request_stream)).await.unwrap();
        let response = response.into_inner();

        assert!(response.success);

        // 验证文件内容
        let request = tonic::Request::new(crate::proto::vdfs::ReadFileRequest {
            path: format!("/{}", name),
            offset: 0,
            length: content.len() as i64,
            node_id: "node2".to_string(),
        });

        let mut response_stream = service2.read_file(request).await.unwrap();
        let response = response_stream.message().await.unwrap().unwrap();

        assert_eq!(response.data, content.as_bytes());
    }
} 
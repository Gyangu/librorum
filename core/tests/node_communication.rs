use librorum_core::{
    config::NodeConfig,
    fs::LocalFileSystem,
    proto::vdfs::{
        vdfs_service_server::VdfsService,
        CreateFileRequest, DeleteFileRequest, FileType,
        ReadFileRequest, WriteFileRequest,
        GetFileInfoRequest, FileInfo,
        NodeInfo, NodeStatus, ClusterInfo,
        RegisterNodeRequest, RegisterNodeResponse,
        GetClusterInfoRequest, GetClusterInfoResponse,
        UpdateNodeStatusRequest, UpdateNodeStatusResponse,
        CreateDirectoryRequest, CreateDirectoryResponse,
        node_client::NodeClient,
        node_server::{Node, NodeServer},
    },
    service::VDFSServiceImpl,
};
use tonic::{Request, Response, Status, codec::{ProstCodec, Direction}, transport::Server};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use futures::stream::StreamExt;
use tonic::Streaming;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn setup_test_nodes() -> (VDFSServiceImpl, String, VDFSServiceImpl, String) {
    // Setup first node
    let temp_dir1 = PathBuf::from("/tmp/librorum_test/node1");
    std::fs::create_dir_all(&temp_dir1).unwrap();
    
    let config1 = NodeConfig {
        id: "test-node-1".to_string(),
        name: "Test Node 1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        root_dir: temp_dir1,
        max_file_size: 1024 * 1024,
        chunk_size: 1024,
        workers: 1,
    };

    let fs1 = LocalFileSystem::new(&config1.root_dir).await.unwrap();
    let service1 = VDFSServiceImpl::new(fs1);
    
    // Setup second node
    let temp_dir2 = PathBuf::from("/tmp/librorum_test/node2");
    std::fs::create_dir_all(&temp_dir2).unwrap();
    
    let config2 = NodeConfig {
        id: "test-node-2".to_string(),
        name: "Test Node 2".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50052,
        root_dir: temp_dir2,
        max_file_size: 1024 * 1024,
        chunk_size: 1024,
        workers: 1,
    };

    let fs2 = LocalFileSystem::new(&config2.root_dir).await.unwrap();
    let service2 = VDFSServiceImpl::new(fs2);
    
    (service1, config1.id, service2, config2.id)
}

#[tokio::test]
async fn test_file_transfer_between_nodes() -> Result<(), Box<dyn std::error::Error>> {
    let (service1, node1_id, service2, node2_id) = setup_test_nodes().await;
    
    // Test file path and content
    let test_path = "/test.txt";
    let test_content = b"Hello, World!".to_vec();
    
    // Create file on node 1
    let create_request = CreateFileRequest {
        path: test_path.to_string(),
        r#type: FileType::File as i32,
        node_id: node1_id.clone(),
    };
    
    service1.create_file(Request::new(create_request)).await?;
    
    // Write content on node 1
    let write_request = WriteFileRequest {
        path: test_path.to_string(),
        offset: 0,
        data: test_content.clone(),
        node_id: node1_id.clone(),
    };
    
    let (tx, rx) = mpsc::channel(1);
    tx.send(write_request).await?;
    drop(tx);
    
    let stream = ReceiverStream::new(rx);
    let streaming = Streaming::new(ProstCodec::new(), stream, Direction::Client, None);
    service1.write_file(Request::new(streaming)).await?;
    
    // Read content from node 2
    let read_request = ReadFileRequest {
        path: test_path.to_string(),
        offset: 0,
        length: test_content.len() as i64,
        node_id: node2_id.clone(),
    };
    
    let mut read_stream = service2.read_file(Request::new(read_request)).await?.into_inner();
    let read_response = read_stream.next().await.unwrap()?;
    
    assert_eq!(read_response.data, test_content);
    
    // Cleanup
    let delete_request = DeleteFileRequest {
        path: test_path.to_string(),
        node_id: node1_id.clone(),
    };
    
    service1.delete_file(Request::new(delete_request)).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_file_operations() -> Result<(), Box<dyn std::error::Error>> {
    let (service1, node1_id, service2, node2_id) = setup_test_nodes().await;
    
    // Test file path
    let test_path = "/test.txt";
    
    // Create file on node 1
    let create_request = CreateFileRequest {
        path: test_path.to_string(),
        r#type: FileType::File as i32,
        node_id: node1_id.clone(),
    };
    
    service1.create_file(Request::new(create_request)).await?;
    
    // Concurrent write operations
    let write_content1 = b"Hello from Node 1".to_vec();
    let write_content2 = b"Hello from Node 2".to_vec();
    
    let write_request1 = WriteFileRequest {
        path: test_path.to_string(),
        offset: 0,
        data: write_content1.clone(),
        node_id: node1_id.clone(),
    };
    
    let write_request2 = WriteFileRequest {
        path: test_path.to_string(),
        offset: write_content1.len() as i64,
        data: write_content2.clone(),
        node_id: node2_id.clone(),
    };
    
    let (tx1, rx1) = mpsc::channel(1);
    tx1.send(write_request1).await?;
    drop(tx1);
    
    let (tx2, rx2) = mpsc::channel(1);
    tx2.send(write_request2).await?;
    drop(tx2);
    
    let stream1 = ReceiverStream::new(rx1);
    let stream2 = ReceiverStream::new(rx2);
    
    let streaming1 = Streaming::new(ProstCodec::new(), stream1, Direction::Client, None);
    let streaming2 = Streaming::new(ProstCodec::new(), stream2, Direction::Client, None);
    
    let write_future1 = service1.write_file(Request::new(streaming1));
    let write_future2 = service2.write_file(Request::new(streaming2));
    
    let (write_response1, write_response2) = tokio::join!(write_future1, write_future2);
    
    assert_eq!(write_response1?.into_inner().bytes_written, write_content1.len() as i64);
    assert_eq!(write_response2?.into_inner().bytes_written, write_content2.len() as i64);
    
    // Read combined content
    let read_request = ReadFileRequest {
        path: test_path.to_string(),
        offset: 0,
        length: (write_content1.len() + write_content2.len()) as i64,
        node_id: node1_id.clone(),
    };
    
    let mut read_stream = service1.read_file(Request::new(read_request)).await?.into_inner();
    let read_response = read_stream.next().await.unwrap()?;
    
    let mut expected_content = write_content1.clone();
    expected_content.extend_from_slice(&write_content2);
    
    assert_eq!(read_response.data, expected_content);
    
    // Cleanup
    let delete_request = DeleteFileRequest {
        path: test_path.to_string(),
        node_id: node1_id.clone(),
    };
    
    service1.delete_file(Request::new(delete_request)).await?;
    
    Ok(())
}

#[derive(Debug)]
pub struct MockNodeService {
    node_info: Arc<Mutex<NodeInfo>>,
}

impl MockNodeService {
    pub fn new() -> Self {
        let node_info = NodeInfo {
            id: "test-node-1".to_string(),
            name: "Test Node".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            status: NodeStatus::Online as i32,
            last_seen: 0,
        };
        
        Self {
            node_info: Arc::new(Mutex::new(node_info)),
        }
    }
}

async fn test_node_registration() {
    let node = MockNodeService::new();
    let node_info = NodeInfo {
        id: "test-node-1".to_string(),
        name: "Test Node".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8000,
        status: NodeStatus::Online as i32,
        last_seen: 0,
    };

    let request = Request::new(RegisterNodeRequest {
        node_info: Some(node_info.clone()),
    });

    let response = node.register_node(request).await.unwrap();
    let result = response.into_inner();
    
    assert!(result.node_info.is_some());
    let registered_node = result.node_info.unwrap();
    assert_eq!(registered_node.id, node_info.id);
    assert_eq!(registered_node.host, node_info.host);
    assert_eq!(registered_node.port, node_info.port);
}

#[tokio::test]
async fn test_cluster_info() -> Result<(), Box<dyn std::error::Error>> {
    let (service1, _, _, _) = setup_test_nodes().await;
    
    // 注册两个节点在同一个服务实例中
    let node1_info = NodeInfo {
        id: "node1".to_string(),
        name: "Node 1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        status: NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    let node2_info = NodeInfo {
        id: "node2".to_string(),
        name: "Node 2".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50052,
        status: NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    // 注册节点1
    let request1 = RegisterNodeRequest {
        node_info: Some(node1_info.clone()),
    };
    service1.register_node(Request::new(request1)).await?;

    // 注册节点2到同一个服务实例
    let request2 = RegisterNodeRequest {
        node_info: Some(node2_info.clone()),
    };
    service1.register_node(Request::new(request2)).await?;

    // 获取集群信息
    let request = GetClusterInfoRequest {};
    let response = service1.get_cluster_info(Request::new(request)).await?;
    let cluster_info = response.into_inner().cluster_info.unwrap();

    // 验证集群信息
    assert_eq!(cluster_info.nodes.len(), 2);
    assert!(cluster_info.nodes.iter().any(|n| n.id == node1_info.id));
    assert!(cluster_info.nodes.iter().any(|n| n.id == node2_info.id));

    Ok(())
}

#[tokio::test]
async fn test_node_status_update() -> Result<(), Box<dyn std::error::Error>> {
    let (service1, _, _, _) = setup_test_nodes().await;

    // 注册节点
    let node_info = NodeInfo {
        id: "node1".to_string(),
        name: "Node 1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        status: NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    let request = RegisterNodeRequest {
        node_info: Some(node_info.clone()),
    };
    service1.register_node(Request::new(request)).await?;

    // 更新节点状态
    let update_request = UpdateNodeStatusRequest {
        node_id: "node1".to_string(),
        status: NodeStatus::NodeOffline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    let response = service1.update_node_status(Request::new(update_request)).await?;
    let response = response.into_inner();

    // 验证状态更新
    assert!(response.success);

    // 获取集群信息验证状态
    let request = GetClusterInfoRequest {};
    let response = service1.get_cluster_info(Request::new(request)).await?;
    let cluster_info = response.into_inner().cluster_info.unwrap();

    let updated_node = cluster_info.nodes.iter()
        .find(|n| n.id == "node1")
        .unwrap();

    assert_eq!(updated_node.status, NodeStatus::NodeOffline as i32);

    Ok(())
}

#[tokio::test]
async fn test_node_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let (service1, node1_id, service2, _) = setup_test_nodes().await;
    
    // Create discovery request for node1
    let node1_info = NodeInfo {
        id: node1_id.clone(),
        name: "Node 1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        status: NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    // Register node1
    let register_request = RegisterNodeRequest {
        node_info: Some(node1_info.clone()),
    };
    service1.register_node(Request::new(register_request)).await?;

    // Get cluster info from node2
    let cluster_request = GetClusterInfoRequest {};
    let cluster_response = service2.get_cluster_info(Request::new(cluster_request)).await?;
    let cluster_info = cluster_response.into_inner().cluster_info.unwrap();

    // Verify that node1 is discoverable from node2
    let discovered_nodes = &cluster_info.nodes;
    assert!(!discovered_nodes.is_empty(), "No nodes discovered");
    
    let discovered_node = discovered_nodes.iter()
        .find(|n| n.id == node1_id)
        .expect("Node 1 not found in discovered nodes");
    
    assert_eq!(discovered_node.host, "127.0.0.1");
    assert_eq!(discovered_node.port, 50051);
    assert_eq!(discovered_node.status, NodeStatus::NodeOnline as i32);

    Ok(())
}

#[tokio::test]
async fn test_node_heartbeat() -> Result<(), Box<dyn std::error::Error>> {
    let (service1, node1_id, service2, _) = setup_test_nodes().await;
    
    // Register node1
    let node1_info = NodeInfo {
        id: node1_id.clone(),
        name: "Node 1".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        status: NodeStatus::NodeOnline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };

    let register_request = RegisterNodeRequest {
        node_info: Some(node1_info.clone()),
    };
    service1.register_node(Request::new(register_request)).await?;

    // Simulate heartbeat by updating node status
    let status_request = UpdateNodeStatusRequest {
        node_id: node1_id.clone(),
        status: NodeStatus::NodeMaintenance as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };
    
    let status_response = service1.update_node_status(Request::new(status_request)).await?;
    assert!(status_response.into_inner().success);

    // Verify status update through cluster info
    let cluster_request = GetClusterInfoRequest {};
    let cluster_response = service2.get_cluster_info(Request::new(cluster_request)).await?;
    let cluster_info = cluster_response.into_inner().cluster_info.unwrap();

    let updated_node = cluster_info.nodes.iter()
        .find(|n| n.id == node1_id)
        .expect("Node 1 not found in cluster info");
    
    assert_eq!(updated_node.status, NodeStatus::NodeMaintenance as i32);

    // Simulate node going offline
    let offline_request = UpdateNodeStatusRequest {
        node_id: node1_id.clone(),
        status: NodeStatus::NodeOffline as i32,
        last_seen: chrono::Utc::now().timestamp(),
    };
    
    let offline_response = service1.update_node_status(Request::new(offline_request)).await?;
    assert!(offline_response.into_inner().success);

    // Verify offline status
    let cluster_request = GetClusterInfoRequest {};
    let cluster_response = service2.get_cluster_info(Request::new(cluster_request)).await?;
    let cluster_info = cluster_response.into_inner().cluster_info.unwrap();

    let offline_node = cluster_info.nodes.iter()
        .find(|n| n.id == node1_id)
        .expect("Node 1 not found in cluster info");
    
    assert_eq!(offline_node.status, NodeStatus::NodeOffline as i32);

    Ok(())
}

#[tonic::async_trait]
impl VdfsService for MockNodeService {
    type ReadFileStream = ReceiverStream<Result<ReadFileResponse, Status>>;
    type SyncMetadataStream = ReceiverStream<Result<SyncMetadataResponse, Status>>;
    type DropFileStream = ReceiverStream<Result<DropFileResponse, Status>>;

    async fn create_file(
        &self,
        request: Request<CreateFileRequest>,
    ) -> Result<Response<CreateFileResponse>, Status> {
        let request = request.into_inner();
        let node_info = self.node_info.lock().await;
        Ok(Response::new(CreateFileResponse {
            info: Some(FileInfo {
                id: format!("test-file-{}", request.path),
                name: request.path.split('/').last().unwrap_or("").to_string(),
                path: request.path,
                r#type: request.r#type,
                size: 0,
                created_at: chrono::Utc::now().timestamp(),
                modified_at: chrono::Utc::now().timestamp(),
                accessed_at: chrono::Utc::now().timestamp(),
                owner_node: node_info.id.clone(),
                available_nodes: vec![node_info.id.clone()],
                attributes: Default::default(),
            }),
        }))
    }

    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        let request = request.into_inner();
        let node_info = self.node_info.lock().await;
        Ok(Response::new(CreateDirectoryResponse {
            info: Some(FileInfo {
                id: format!("test-dir-{}", request.path),
                name: request.path.split('/').last().unwrap_or("").to_string(),
                path: request.path,
                r#type: FileType::Directory as i32,
                size: 0,
                created_at: chrono::Utc::now().timestamp(),
                modified_at: chrono::Utc::now().timestamp(),
                accessed_at: chrono::Utc::now().timestamp(),
                owner_node: node_info.id.clone(),
                available_nodes: vec![node_info.id.clone()],
                attributes: Default::default(),
            }),
        }))
    }

    async fn delete_file(
        &self,
        _request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        Ok(Response::new(DeleteFileResponse { success: true }))
    }

    async fn read_file(
        &self,
        _request: Request<ReadFileRequest>,
    ) -> Result<Response<Self::ReadFileStream>, Status> {
        let (tx, rx) = mpsc::channel(32);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn write_file(
        &self,
        request: Request<Streaming<WriteFileRequest>>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let mut stream = request.into_inner();
        let mut bytes_written = 0;
        while let Some(req) = stream.message().await? {
            bytes_written += req.data.len() as i64;
        }
        Ok(Response::new(WriteFileResponse { bytes_written }))
    }

    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let node_info = request.into_inner().node_info.unwrap();
        Ok(Response::new(RegisterNodeResponse {
            success: true,
            node_id: format!("test-node-{}", node_info.address),
            message: "Node registered successfully".to_string(),
        }))
    }

    async fn get_node_status(
        &self,
        request: Request<NodeInfo>,
    ) -> Result<Response<NodeStatus>, Status> {
        let node_info = request.into_inner();
        Ok(Response::new(NodeStatus {
            node_id: format!("test-node-{}", node_info.address),
            status: "ACTIVE".to_string(),
            connected_nodes: 1,
            total_storage: 1000,
            used_storage: 100,
        }))
    }
} 
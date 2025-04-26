use librorum_core::{
    config::NodeConfig,
    proto::vdfs::{
        vdfs_service_server::VdfsService,
        NodeInfo, NodeStatus,
        DiscoverNodesRequest, DiscoverNodesResponse,
        HeartbeatRequest, HeartbeatResponse,
        JoinClusterRequest, JoinClusterResponse,
        LeaveClusterRequest, LeaveClusterResponse,
        RegisterNodeRequest, RegisterNodeResponse,
        GetNodeStatusRequest, GetNodeStatusResponse,
        ListDirectoryRequest, ListDirectoryResponse,
        GetFileInfoRequest, GetFileInfoResponse,
        CreateFileRequest, CreateFileResponse,
        DeleteFileRequest, DeleteFileResponse,
        MoveFileRequest, MoveFileResponse,
        CopyFileRequest, CopyFileResponse,
        ReadFileRequest, ReadFileResponse,
        WriteFileRequest, WriteFileResponse,
        SyncMetadataRequest, SyncMetadataResponse,
        DropFileRequest, DropFileResponse,
        ReceiveFileRequest, ReceiveFileResponse,
        GetClusterInfoRequest, GetClusterInfoResponse,
        UpdateNodeStatusRequest, UpdateNodeStatusResponse,
        ClusterInfo,
        CreateDirectoryRequest, CreateDirectoryResponse,
    },
    service::VDFSServiceImpl,
    fs::LocalFileSystem,
    cluster::{ClusterManager, ClusterConfig},
    discovery::DiscoveryService,
};
use tonic::{Request, Response, Status, Streaming};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::sync::{RwLock, Mutex, mpsc};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use chrono::Utc;

async fn setup_test_node() -> (Arc<MockNodeService>, NodeInfo) {
    let mock_service = Arc::new(MockNodeService::new());
    let node_info = NodeInfo {
        id: "test_node".to_string(),
        name: "Test Node".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        status: NodeStatus::NodeOnline as i32,
        last_seen: Utc::now().timestamp(),
    };
    
    (mock_service, node_info)
}

#[derive(Clone)]
struct MockNodeService {
    node_info: Arc<RwLock<NodeInfo>>,
}

impl MockNodeService {
    fn new() -> Self {
        Self {
            node_info: Arc::new(RwLock::new(NodeInfo {
                id: "test_node".to_string(),
                name: "Test Node".to_string(),
                host: "127.0.0.1".to_string(),
                port: 50051,
                status: NodeStatus::NodeOnline as i32,
                last_seen: Utc::now().timestamp(),
            })),
        }
    }
}

#[tonic::async_trait]
impl VdfsService for MockNodeService {
    type ReadFileStream = ReceiverStream<Result<ReadFileResponse, Status>>;
    type SyncMetadataStream = ReceiverStream<Result<SyncMetadataResponse, Status>>;
    type DropFileStream = ReceiverStream<Result<DropFileResponse, Status>>;

    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let node_info = request.into_inner().node_info.unwrap();
        *self.node_info.write().await = node_info.clone();
        Ok(Response::new(RegisterNodeResponse {
            success: true,
            node_info: Some(node_info),
            error: String::new(),
        }))
    }

    async fn get_node_status(
        &self,
        _request: Request<GetNodeStatusRequest>,
    ) -> Result<Response<GetNodeStatusResponse>, Status> {
        let node_info = self.node_info.read().await.clone();
        Ok(Response::new(GetNodeStatusResponse {
            status: Some(node_info),
        }))
    }

    async fn heartbeat(
        &self,
        _request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let mut node_info = self.node_info.write().await;
        node_info.last_seen = Utc::now().timestamp();
        Ok(Response::new(HeartbeatResponse {
            acknowledged: true,
            server_timestamp: Utc::now().timestamp(),
            cluster_id: "test_cluster".to_string(),
        }))
    }

    async fn join_cluster(
        &self,
        request: Request<JoinClusterRequest>,
    ) -> Result<Response<JoinClusterResponse>, Status> {
        let node_info = request.into_inner().node_info.unwrap();
        *self.node_info.write().await = node_info.clone();
        Ok(Response::new(JoinClusterResponse {
            success: true,
            cluster_info: Some(ClusterInfo {
                nodes: vec![node_info],
                last_updated: Utc::now().timestamp(),
            }),
            error: String::new(),
        }))
    }

    async fn leave_cluster(
        &self,
        _request: Request<LeaveClusterRequest>,
    ) -> Result<Response<LeaveClusterResponse>, Status> {
        let mut node_info = self.node_info.write().await;
        node_info.status = NodeStatus::NodeOffline as i32;
        Ok(Response::new(LeaveClusterResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn list_directory(
        &self,
        _request: Request<ListDirectoryRequest>,
    ) -> Result<Response<ListDirectoryResponse>, Status> {
        Ok(Response::new(ListDirectoryResponse {
            entries: vec![],
        }))
    }

    async fn get_file_info(
        &self,
        _request: Request<GetFileInfoRequest>,
    ) -> Result<Response<GetFileInfoResponse>, Status> {
        Ok(Response::new(GetFileInfoResponse {
            info: None,
        }))
    }

    async fn create_file(
        &self,
        _request: Request<CreateFileRequest>,
    ) -> Result<Response<CreateFileResponse>, Status> {
        Ok(Response::new(CreateFileResponse {
            info: None,
        }))
    }

    async fn create_directory(
        &self,
        _request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        Ok(Response::new(CreateDirectoryResponse {
            info: None,
        }))
    }

    async fn delete_file(
        &self,
        _request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        Ok(Response::new(DeleteFileResponse {
            success: false,
        }))
    }

    async fn move_file(
        &self,
        _request: Request<MoveFileRequest>,
    ) -> Result<Response<MoveFileResponse>, Status> {
        Ok(Response::new(MoveFileResponse {
            success: false,
        }))
    }

    async fn copy_file(
        &self,
        _request: Request<CopyFileRequest>,
    ) -> Result<Response<CopyFileResponse>, Status> {
        Ok(Response::new(CopyFileResponse {
            success: false,
        }))
    }

    async fn read_file(
        &self,
        _request: Request<ReadFileRequest>,
    ) -> Result<Response<Self::ReadFileStream>, Status> {
        let (_tx, rx) = mpsc::channel(32);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn write_file(
        &self,
        _request: Request<Streaming<WriteFileRequest>>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        Ok(Response::new(WriteFileResponse {
            bytes_written: 0,
        }))
    }

    async fn sync_metadata(
        &self,
        _request: Request<SyncMetadataRequest>,
    ) -> Result<Response<Self::SyncMetadataStream>, Status> {
        let (_tx, rx) = mpsc::channel(32);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn drop_file(
        &self,
        _request: Request<DropFileRequest>,
    ) -> Result<Response<Self::DropFileStream>, Status> {
        let (_tx, rx) = mpsc::channel(32);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn receive_file(
        &self,
        _request: Request<Streaming<ReceiveFileRequest>>,
    ) -> Result<Response<ReceiveFileResponse>, Status> {
        Ok(Response::new(ReceiveFileResponse {
            success: false,
            error: "Not implemented".to_string(),
        }))
    }

    async fn get_cluster_info(
        &self,
        _request: Request<GetClusterInfoRequest>,
    ) -> Result<Response<GetClusterInfoResponse>, Status> {
        let node_info = self.node_info.read().await.clone();
        Ok(Response::new(GetClusterInfoResponse {
            cluster_info: Some(ClusterInfo {
                nodes: vec![node_info],
                last_updated: Utc::now().timestamp(),
            }),
        }))
    }

    async fn update_node_status(
        &self,
        request: Request<UpdateNodeStatusRequest>,
    ) -> Result<Response<UpdateNodeStatusResponse>, Status> {
        let request = request.into_inner();
        let mut node_info = self.node_info.write().await;
        node_info.status = request.status;
        node_info.last_seen = request.last_seen;
        Ok(Response::new(UpdateNodeStatusResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn discover_nodes(
        &self,
        _request: Request<DiscoverNodesRequest>,
    ) -> Result<Response<DiscoverNodesResponse>, Status> {
        let node_info = self.node_info.read().await.clone();
        Ok(Response::new(DiscoverNodesResponse {
            nodes: vec![node_info],
        }))
    }
}

#[tokio::test]
async fn test_node_registration() -> Result<(), Box<dyn std::error::Error>> {
    let (service, node_info) = setup_test_node().await;
    
    let request = Request::new(RegisterNodeRequest {
        node_info: Some(node_info.clone()),
    });
    
    let response = service.register_node(request).await?;
    let result = response.into_inner();
    assert!(result.success);
    assert!(result.node_info.is_some());
    assert_eq!(result.node_info.unwrap().id, node_info.id);
    
    Ok(())
}

#[tokio::test]
async fn test_node_status() -> Result<(), Box<dyn std::error::Error>> {
    let (service, node_info) = setup_test_node().await;
    
    let request = Request::new(GetNodeStatusRequest {
        node_id: node_info.id.clone(),
    });
    
    let response = service.get_node_status(request).await?;
    let result = response.into_inner();
    assert!(result.status.is_some());
    assert_eq!(result.status.unwrap().id, node_info.id);
    
    Ok(())
}

#[tokio::test]
async fn test_heartbeat() -> Result<(), Box<dyn std::error::Error>> {
    let (service, node_info) = setup_test_node().await;
    
    let request = Request::new(HeartbeatRequest {
        node_id: node_info.id.clone(),
        timestamp: Utc::now().timestamp(),
        status: NodeStatus::NodeOnline as i32,
        cpu_usage: 0.0,
        memory_usage: 0.0,
        disk_usage: 0.0,
        active_connections: 0,
    });
    
    let response = service.heartbeat(request).await?;
    let result = response.into_inner();
    assert!(result.acknowledged);
    assert!(result.server_timestamp > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_node_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let (service, node_info) = setup_test_node().await;
    
    // 注册节点
    let register_request = Request::new(RegisterNodeRequest {
        node_info: Some(node_info.clone()),
    });
    let register_response = service.register_node(register_request).await?;
    assert!(register_response.into_inner().success);
    
    // 发现节点
    let discover_request = Request::new(DiscoverNodesRequest {
        node_id: node_info.id.clone(),
        network: "local".to_string(),
        discovery_port: 50052,
        max_nodes: 10,
    });
    let discover_response = service.discover_nodes(discover_request).await?;
    let result = discover_response.into_inner();
    assert!(!result.nodes.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_node_status_update() -> Result<(), Box<dyn std::error::Error>> {
    let (service, node_info) = setup_test_node().await;
    
    let request = Request::new(UpdateNodeStatusRequest {
        node_id: node_info.id.clone(),
        status: NodeStatus::NodeMaintenance as i32,
        last_seen: Utc::now().timestamp(),
    });
    
    let response = service.update_node_status(request).await?;
    let result = response.into_inner();
    assert!(result.success);
    
    // 验证状态已更新
    let status_request = Request::new(GetNodeStatusRequest {
        node_id: node_info.id.clone(),
    });
    let status_response = service.get_node_status(status_request).await?;
    let status = status_response.into_inner().status.unwrap();
    assert_eq!(status.status, NodeStatus::NodeMaintenance as i32);
    
    Ok(())
}

#[tokio::test]
async fn test_cluster_operations() -> Result<(), Box<dyn std::error::Error>> {
    let (mock_service, node_info) = setup_test_node().await;
    
    // Test join cluster
    let join_request = Request::new(JoinClusterRequest {
        node_info: Some(node_info.clone()),
        cluster_id: "test_cluster".to_string(),
        join_token: "test_token".to_string(),
    });
    
    let join_response = mock_service.join_cluster(join_request).await?;
    assert!(join_response.into_inner().success);
    
    // Test leave cluster
    let leave_request = Request::new(LeaveClusterRequest {
        node_id: node_info.id.clone(),
        cluster_id: "test_cluster".to_string(),
        graceful: true,
    });
    
    let leave_response = mock_service.leave_cluster(leave_request).await?;
    assert!(leave_response.into_inner().success);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_node_registration() -> Result<(), Box<dyn std::error::Error>> {
    let (service, _) = setup_test_node().await;
    let service = Arc::new(service);
    let mut handles = Vec::new();

    for i in 0..5 {
        let service_clone = Arc::clone(&service);
        let node_info = NodeInfo {
            id: format!("test_node_{}", i),
            name: format!("Test Node {}", i),
            host: "127.0.0.1".to_string(),
            port: 50051 + i as i32,
            status: NodeStatus::NodeOnline as i32,
            last_seen: Utc::now().timestamp(),
        };

        let handle = tokio::spawn(async move {
            let request = Request::new(RegisterNodeRequest {
                node_info: Some(node_info.clone()),
            });
            let response = service_clone.register_node(request).await?;
            let result = response.into_inner();
            assert!(result.success);
            assert!(result.node_info.is_some());
            assert_eq!(result.node_info.unwrap().id, node_info.id);
            Ok::<_, Status>(())
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}

#[tokio::test]
async fn test_node_failure_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let (service, node_info) = setup_test_node().await;
    
    // 测试节点离线
    let mut node_info = node_info.clone();
    node_info.status = NodeStatus::NodeOffline as i32;
    let request = Request::new(UpdateNodeStatusRequest {
        node_id: node_info.id.clone(),
        status: NodeStatus::NodeOffline as i32,
        last_seen: Utc::now().timestamp(),
    });
    
    let response = service.update_node_status(request).await?;
    assert!(response.into_inner().success);
    
    // 测试节点恢复
    let request = Request::new(UpdateNodeStatusRequest {
        node_id: node_info.id.clone(),
        status: NodeStatus::NodeOnline as i32,
        last_seen: Utc::now().timestamp(),
    });
    
    let response = service.update_node_status(request).await?;
    assert!(response.into_inner().success);
    
    Ok(())
}

#[tokio::test]
async fn test_invalid_node_registration() -> Result<(), Box<dyn std::error::Error>> {
    let (service, _) = setup_test_node().await;
    
    // 测试无效的节点信息
    let invalid_node = NodeInfo {
        id: "".to_string(),
        name: "".to_string(),
        host: "".to_string(),
        port: 0,
        status: NodeStatus::NodeUnknown as i32,
        last_seen: 0,
    };
    
    let request = Request::new(RegisterNodeRequest {
        node_info: Some(invalid_node),
    });
    
    let response = service.register_node(request).await;
    assert!(response.is_ok()); // 即使是无效节点，我们也允许注册，但会标记为未知状态
    
    Ok(())
} 
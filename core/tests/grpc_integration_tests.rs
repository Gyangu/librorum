use librorum_core::proto::node::node_service_server::{NodeService, NodeServiceServer};
use librorum_core::proto::node::{HeartbeatRequest, HeartbeatResponse};
use librorum_core::node_manager::{NodeServiceImpl};
use tonic::{transport::Server, Request, Response, Status};
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 测试用的简单 gRPC 服务实现
#[derive(Debug)]
struct TestNodeService {
    pub responses: Arc<Mutex<Vec<HeartbeatRequest>>>,
}

impl TestNodeService {
    fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[tonic::async_trait]
impl NodeService for TestNodeService {
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        
        // 记录收到的请求
        self.responses.lock().await.push(req.clone());

        let response = HeartbeatResponse {
            node_id: "test_server".to_string(),
            address: "127.0.0.1:50051".to_string(),
            system_info: "Test System".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            status: true,
        };

        Ok(Response::new(response))
    }
}

/// 启动测试 gRPC 服务器
async fn start_test_server(port: u16) -> SocketAddr {
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();

    let service = TestNodeService::new();
    tokio::spawn(async move {
        Server::builder()
            .add_service(NodeServiceServer::new(service))
            .serve(addr)
            .await
            .unwrap();
    });

    // 等待服务器启动
    sleep(Duration::from_millis(100)).await;

    addr
}

#[tokio::test]
async fn test_basic_heartbeat() {
    let addr = start_test_server(50052).await;
    
    let node_client = librorum_core::node_manager::NodeClient::new(
        "test_client".to_string(),
        "127.0.0.1:50052".to_string(),
        "Test Client System".to_string(),
    );

    let result = node_client
        .send_heartbeat(&addr.to_string())
        .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.node_id, "test_server");
    assert!(response.status);
}

#[tokio::test]
async fn test_heartbeat_timeout() {
    let node_client = librorum_core::node_manager::NodeClient::new(
        "test_client".to_string(),
        "127.0.0.1:9999".to_string(),
        "Test Client System".to_string(),
    );

    // 尝试连接到不存在的服务器
    let result = node_client
        .send_heartbeat("127.0.0.1:9999")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_heartbeats() {
    let addr = start_test_server(50053).await;
    
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let addr_clone = addr.to_string();
        let handle = tokio::spawn(async move {
            let node_client = librorum_core::node_manager::NodeClient::new(
                format!("test_client_{}", i),
                "127.0.0.1:50053".to_string(),
                "Test Client System".to_string(),
            );

            node_client.send_heartbeat(&addr_clone).await
        });
        handles.push(handle);
    }

    // 等待所有请求完成
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 5);
}

#[tokio::test]
async fn test_node_service_impl() {
    let node_service = NodeServiceImpl::new(
        "test_node".to_string(),
        "127.0.0.1:50051".to_string(),
        "Test System".to_string(),
    );

    let request = Request::new(HeartbeatRequest {
        node_id: "remote_node".to_string(),
        address: "127.0.0.1:50052".to_string(),
        system_info: "Remote System".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    });

    let response = node_service.heartbeat(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.node_id, "test_node");
    assert_eq!(response.address, "127.0.0.1:50051");
    assert!(response.status);

    // 验证节点被添加到已知节点列表
    let nodes = node_service.get_all_nodes().await;
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].info.id, "remote_node");
}

#[tokio::test] 
async fn test_heartbeat_request_validation() {
    let node_service = NodeServiceImpl::new(
        "test_node".to_string(),
        "127.0.0.1:50051".to_string(),
        "Test System".to_string(),
    );

    // 测试空节点ID
    let request = Request::new(HeartbeatRequest {
        node_id: "".to_string(),
        address: "127.0.0.1:50052".to_string(),
        system_info: "Remote System".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    });

    let response = node_service.heartbeat(request).await;
    // 即使节点ID为空，服务也应该响应（实际项目中可能需要验证）
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_repeated_heartbeats_same_node() {
    let node_service = NodeServiceImpl::new(
        "test_node".to_string(),
        "127.0.0.1:50051".to_string(),
        "Test System".to_string(),
    );

    // 发送多次心跳给同一个节点
    for i in 0..3 {
        let request = Request::new(HeartbeatRequest {
            node_id: "remote_node".to_string(),
            address: "127.0.0.1:50052".to_string(),
            system_info: format!("Remote System {}", i),
            timestamp: chrono::Utc::now().timestamp(),
        });

        let response = node_service.heartbeat(request).await;
        assert!(response.is_ok());
    }

    // 应该只有一个节点记录，但是信息会更新
    let nodes = node_service.get_all_nodes().await;
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].info.id, "remote_node");
}
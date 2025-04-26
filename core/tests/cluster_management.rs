use tonic::{Request, Response, Status};
use librorum_core::proto::vdfs::{
    cluster_service_server::{ClusterService, ClusterServiceServer},
    JoinClusterRequest, JoinClusterResponse,
    LeaveClusterRequest, LeaveClusterResponse,
    GetClusterStatusRequest, ClusterStatus, NodeInfo,
};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
struct MockClusterService {
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
}

#[tonic::async_trait]
impl ClusterService for MockClusterService {
    async fn join_cluster(
        &self,
        request: Request<JoinClusterRequest>,
    ) -> Result<Response<JoinClusterResponse>, Status> {
        let req = request.into_inner();
        let node_info = req.node_info.ok_or_else(|| {
            Status::invalid_argument("Missing node info")
        })?;
        
        let mut nodes = self.nodes.write().await;
        nodes.insert(node_info.id.clone(), node_info);
        
        Ok(Response::new(JoinClusterResponse {
            success: true,
            message: "Successfully joined cluster".to_string(),
            cluster_id: "test-cluster".to_string(),
        }))
    }

    async fn leave_cluster(
        &self,
        request: Request<LeaveClusterRequest>,
    ) -> Result<Response<LeaveClusterResponse>, Status> {
        let req = request.into_inner();
        let mut nodes = self.nodes.write().await;
        nodes.remove(&req.node_id);
        
        Ok(Response::new(LeaveClusterResponse {
            success: true,
            message: "Successfully left cluster".to_string(),
        }))
    }

    async fn get_cluster_status(
        &self,
        _request: Request<GetClusterStatusRequest>,
    ) -> Result<Response<ClusterStatus>, Status> {
        let nodes = self.nodes.read().await;
        Ok(Response::new(ClusterStatus {
            cluster_id: "test-cluster".to_string(),
            total_nodes: nodes.len() as i32,
            active_nodes: nodes.len() as i32,
            health_status: "HEALTHY".to_string(),
            nodes: nodes.values().cloned().collect(),
        }))
    }
}

#[tokio::test]
async fn test_cluster_operations() -> Result<(), Box<dyn std::error::Error>> {
    // 启动测试服务器
    let addr = "[::1]:50052".parse::<SocketAddr>()?;
    let mock_service = MockClusterService::default();
    let server = ClusterServiceServer::new(mock_service);
    
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(server)
            .serve(addr)
            .await
            .unwrap();
    });
    
    // 等待服务器启动
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 创建客户端
    let mut client = librorum_core::proto::vdfs::cluster_service_client::ClusterServiceClient::connect(
        "http://[::1]:50052"
    ).await?;
    
    // 测试加入集群
    let node_info = NodeInfo {
        id: "test-node".to_string(),
        name: "Test Node".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50052,
        root_dir: "/tmp/test".to_string(),
        status: "ACTIVE".to_string(),
        last_seen: chrono::Utc::now().timestamp(),
    };
    
    let join_request = Request::new(JoinClusterRequest {
        node_info: Some(node_info.clone()),
    });
    
    let join_response = client.join_cluster(join_request).await?;
    assert!(join_response.get_ref().success);
    assert_eq!(join_response.get_ref().cluster_id, "test-cluster");
    
    // 测试获取集群状态
    let status_request = Request::new(GetClusterStatusRequest {});
    let status_response = client.get_cluster_status(status_request).await?;
    let status = status_response.get_ref();
    assert_eq!(status.cluster_id, "test-cluster");
    assert_eq!(status.total_nodes, 1);
    assert_eq!(status.active_nodes, 1);
    assert_eq!(status.health_status, "HEALTHY");
    
    // 测试离开集群
    let leave_request = Request::new(LeaveClusterRequest {
        node_id: "test-node".to_string(),
        cluster_id: "test-cluster".to_string(),
    });
    
    let leave_response = client.leave_cluster(leave_request).await?;
    assert!(leave_response.get_ref().success);
    
    // 验证节点已离开集群
    let status_request = Request::new(GetClusterStatusRequest {});
    let status_response = client.get_cluster_status(status_request).await?;
    let status = status_response.get_ref();
    assert_eq!(status.total_nodes, 0);
    assert_eq!(status.active_nodes, 0);
    
    Ok(())
} 
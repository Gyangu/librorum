use tonic::{Request, Response, Status};
use crate::fs::LocalFileSystem;
use crate::proto::vdfs::{
    vdfs_service_server::VdfsService,
    CreateFileRequest, CreateFileResponse,
    DeleteFileRequest, DeleteFileResponse,
    ReadFileRequest, ReadFileResponse,
    WriteFileRequest, WriteFileResponse,
    ListDirectoryRequest, ListDirectoryResponse,
    GetFileInfoRequest, GetFileInfoResponse,
    MoveFileRequest, MoveFileResponse,
    CopyFileRequest, CopyFileResponse,
    SyncMetadataRequest, SyncMetadataResponse,
    GetNodeStatusRequest, GetNodeStatusResponse,
    DropFileRequest, DropFileResponse,
    ReceiveFileRequest, ReceiveFileResponse,
    FileInfo, NodeStatus, FileType,
    drop_file_response,
    NodeInfo, ClusterInfo,
    RegisterNodeRequest, RegisterNodeResponse,
    GetClusterInfoRequest, GetClusterInfoResponse,
    UpdateNodeStatusRequest, UpdateNodeStatusResponse,
    DiscoverNodesRequest, DiscoverNodesResponse,
    HeartbeatRequest, HeartbeatResponse,
    JoinClusterRequest, JoinClusterResponse,
    LeaveClusterRequest, LeaveClusterResponse,
    CreateDirectoryRequest, CreateDirectoryResponse,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::Utc;
use crate::cluster::ClusterManager;

pub struct VDFSServiceImpl {
    fs: Arc<LocalFileSystem>,
    nodes: Arc<Mutex<HashMap<String, NodeInfo>>>,
    cluster_manager: Option<Arc<Mutex<ClusterManager>>>,
}

impl VDFSServiceImpl {
    pub fn new(fs: LocalFileSystem) -> Self {
        Self {
            fs: Arc::new(fs),
            nodes: Arc::new(Mutex::new(HashMap::new())),
            cluster_manager: None,
        }
    }
    
    pub fn with_cluster_manager(fs: LocalFileSystem, cluster_manager: ClusterManager) -> Self {
        Self {
            fs: Arc::new(fs),
            nodes: Arc::new(Mutex::new(HashMap::new())),
            cluster_manager: Some(Arc::new(Mutex::new(cluster_manager))),
        }
    }
    
    fn get_current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[tonic::async_trait]
impl VdfsService for VDFSServiceImpl {
    async fn create_file(
        &self,
        request: Request<CreateFileRequest>,
    ) -> std::result::Result<Response<CreateFileResponse>, Status> {
        let req = request.into_inner();
        self.fs.create_file(&req.path)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: req.path.split('/').last().unwrap_or("").to_string(),
            path: req.path,
            r#type: FileType::File as i32,
            size: 0,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            modified_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            accessed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            owner_node: req.node_id.clone(),
            available_nodes: vec![req.node_id],
            attributes: Default::default(),
        };

        Ok(Response::new(CreateFileResponse { info: Some(info) }))
    }

    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> std::result::Result<Response<CreateDirectoryResponse>, Status> {
        let req = request.into_inner();
        self.fs.create_dir(&req.path)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: req.path.split('/').last().unwrap_or("").to_string(),
            path: req.path,
            r#type: FileType::Directory as i32,
            size: 0,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            modified_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            accessed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            owner_node: req.node_id.clone(),
            available_nodes: vec![req.node_id],
            attributes: Default::default(),
        };

        Ok(Response::new(CreateDirectoryResponse { info: Some(info) }))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> std::result::Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        self.fs.delete_file(&req.path)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(DeleteFileResponse { success: true }))
    }

    type ReadFileStream = ReceiverStream<std::result::Result<ReadFileResponse, Status>>;

    async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> std::result::Result<Response<Self::ReadFileStream>, Status> {
        let (tx, rx) = mpsc::channel(4);
        let req = request.into_inner();

        let content = self.fs.read_file(&req.path)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        tokio::spawn(async move {
            tx.send(Ok(ReadFileResponse { data: content })).await.unwrap();
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn write_file(
        &self,
        request: Request<tonic::Streaming<WriteFileRequest>>,
    ) -> std::result::Result<Response<WriteFileResponse>, Status> {
        let mut stream = request.into_inner();
        let mut bytes_written = 0;

        while let Some(req) = stream.message().await.map_err(|e| Status::internal(e.to_string()))? {
            self.fs.write_file(&req.path, &req.data)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            bytes_written += req.data.len() as i64;
        }

        Ok(Response::new(WriteFileResponse { bytes_written }))
    }

    async fn list_directory(
        &self,
        request: Request<ListDirectoryRequest>,
    ) -> std::result::Result<Response<ListDirectoryResponse>, Status> {
        let req = request.into_inner();
        let entries = self.fs.list_dir(&req.path)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let node_id = req.node_id.clone();
        let files = entries.into_iter()
            .map(|path| {
                let name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                FileInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    path: path.to_string_lossy().into_owned(),
                    r#type: if path.is_dir() { FileType::Directory as i32 } else { FileType::File as i32 },
                    size: 0, // TODO: Get actual file size
                    created_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    modified_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    accessed_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    owner_node: node_id.clone(),
                    available_nodes: vec![node_id.clone()],
                    attributes: Default::default(),
                }
            })
            .collect();

        Ok(Response::new(ListDirectoryResponse { entries: files }))
    }

    async fn get_file_info(
        &self,
        request: Request<GetFileInfoRequest>,
    ) -> std::result::Result<Response<GetFileInfoResponse>, Status> {
        let req = request.into_inner();
        let path = self.fs.get_path(&req.path);
        
        let info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: req.path,
            r#type: if path.is_dir() { FileType::Directory as i32 } else { FileType::File as i32 },
            size: 0, // TODO: Get actual file size
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            modified_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            accessed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            owner_node: req.node_id.clone(),
            available_nodes: vec![req.node_id],
            attributes: Default::default(),
        };

        Ok(Response::new(GetFileInfoResponse { info: Some(info) }))
    }

    async fn move_file(
        &self,
        _request: Request<MoveFileRequest>,
    ) -> std::result::Result<Response<MoveFileResponse>, Status> {
        // TODO: Implement move file
        Ok(Response::new(MoveFileResponse { success: true }))
    }

    async fn copy_file(
        &self,
        _request: Request<CopyFileRequest>,
    ) -> std::result::Result<Response<CopyFileResponse>, Status> {
        // TODO: Implement copy file
        Ok(Response::new(CopyFileResponse { success: true }))
    }

    type SyncMetadataStream = ReceiverStream<std::result::Result<SyncMetadataResponse, Status>>;

    async fn sync_metadata(
        &self,
        _request: Request<SyncMetadataRequest>,
    ) -> std::result::Result<Response<Self::SyncMetadataStream>, Status> {
        let (tx, rx) = mpsc::channel(4);

        // TODO: Implement metadata sync
        let response = SyncMetadataResponse {
            files: vec![],
            sync_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        tokio::spawn(async move {
            tx.send(Ok(response)).await.unwrap();
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_node_status(
        &self,
        request: Request<GetNodeStatusRequest>,
    ) -> std::result::Result<Response<GetNodeStatusResponse>, Status> {
        let req = request.into_inner();
        
        // 尝试从节点列表中获取节点信息
        let nodes = self.nodes.lock().map_err(|_| {
            Status::internal("Failed to lock nodes map")
        })?;
        
        // 如果找到节点，返回其信息，否则创建一个基本的节点信息
        let node_info = nodes.get(&req.node_id).cloned().unwrap_or_else(|| {
            NodeInfo {
                id: req.node_id.clone(),
                name: "Unknown".to_string(),
                host: "127.0.0.1".to_string(),
                port: 50051,
                status: NodeStatus::NodeUnknown as i32,
                last_seen: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            }
        });

        Ok(Response::new(GetNodeStatusResponse { status: Some(node_info) }))
    }

    type DropFileStream = ReceiverStream<std::result::Result<DropFileResponse, Status>>;

    async fn drop_file(
        &self,
        request: Request<DropFileRequest>,
    ) -> std::result::Result<Response<Self::DropFileStream>, Status> {
        let (tx, rx) = mpsc::channel(4);
        let req = request.into_inner();

        // TODO: Implement file drop
        let response = DropFileResponse {
            response: Some(drop_file_response::Response::FileInfo(FileInfo {
                id: req.file_id,
                name: "".to_string(),
                path: "".to_string(),
                r#type: FileType::File as i32,
                size: 0,
                created_at: 0,
                modified_at: 0,
                accessed_at: 0,
                owner_node: req.source_node,
                available_nodes: vec![req.target_node],
                attributes: Default::default(),
            })),
        };

        tokio::spawn(async move {
            tx.send(Ok(response)).await.unwrap();
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn receive_file(
        &self,
        _request: Request<tonic::Streaming<ReceiveFileRequest>>,
    ) -> std::result::Result<Response<ReceiveFileResponse>, Status> {
        // TODO: Implement file receive
        Ok(Response::new(ReceiveFileResponse { success: true, error: "".to_string() }))
    }

    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let node_info = request.into_inner().node_info.ok_or_else(|| {
            Status::invalid_argument("Node info is required")
        })?;

        // 优先使用集群管理器
        if let Some(cluster_manager) = &self.cluster_manager {
            let manager = cluster_manager.lock().map_err(|_| {
                Status::internal("Failed to lock cluster manager")
            })?;
            
            match manager.register_node(node_info.clone()) {
                Ok(()) => {}
                Err(e) => return Err(Status::internal(e)),
            }
        } else {
            // 回退到简单的内存存储
            let mut nodes = self.nodes.lock().map_err(|_| {
                Status::internal("Failed to lock nodes map")
            })?;
            
            nodes.insert(node_info.id.clone(), node_info.clone());
        }

        Ok(Response::new(RegisterNodeResponse {
            success: true,
            node_info: Some(node_info),
            error: String::new(),
        }))
    }

    async fn get_cluster_info(
        &self,
        _request: Request<GetClusterInfoRequest>,
    ) -> Result<Response<GetClusterInfoResponse>, Status> {
        // 优先使用集群管理器
        if let Some(cluster_manager) = &self.cluster_manager {
            let manager = cluster_manager.lock().map_err(|_| {
                Status::internal("Failed to lock cluster manager")
            })?;
            
            let cluster_info = manager.get_cluster_info();
            
            Ok(Response::new(GetClusterInfoResponse {
                cluster_info: Some(cluster_info),
            }))
        } else {
            // 回退到简单的内存存储
            let nodes = self.nodes.lock().map_err(|_| {
                Status::internal("Failed to lock nodes map")
            })?;

            let cluster_info = ClusterInfo {
                nodes: nodes.values().cloned().collect(),
                last_updated: Utc::now().timestamp(),
            };

            Ok(Response::new(GetClusterInfoResponse {
                cluster_info: Some(cluster_info),
            }))
        }
    }

    async fn update_node_status(
        &self,
        request: Request<UpdateNodeStatusRequest>,
    ) -> Result<Response<UpdateNodeStatusResponse>, Status> {
        let request = request.into_inner();
        
        // 优先使用集群管理器
        if let Some(cluster_manager) = &self.cluster_manager {
            let manager = cluster_manager.lock().map_err(|_| {
                Status::internal("Failed to lock cluster manager")
            })?;
            
            match manager.update_node_status(&request.node_id, 
                                            NodeStatus::NodeUnknown, 
                                            request.last_seen) {
                Ok(()) => {
                    Ok(Response::new(UpdateNodeStatusResponse {
                        success: true,
                        error: String::new(),
                    }))
                }
                Err(e) => {
                    Ok(Response::new(UpdateNodeStatusResponse {
                        success: false,
                        error: e,
                    }))
                }
            }
        } else {
            // 回退到简单的内存存储
            let mut nodes = self.nodes.lock().map_err(|_| {
                Status::internal("Failed to lock nodes map")
            })?;

            if let Some(node_info) = nodes.get_mut(&request.node_id) {
                node_info.status = request.status;
                node_info.last_seen = request.last_seen;

                Ok(Response::new(UpdateNodeStatusResponse {
                    success: true,
                    error: String::new(),
                }))
            } else {
                Ok(Response::new(UpdateNodeStatusResponse {
                    success: false,
                    error: format!("Node {} not found", request.node_id),
                }))
            }
        }
    }
    
    async fn discover_nodes(
        &self,
        request: Request<DiscoverNodesRequest>,
    ) -> Result<Response<DiscoverNodesResponse>, Status> {
        let req = request.into_inner();
        
        // 优先使用集群管理器进行节点发现
        if let Some(cluster_manager) = &self.cluster_manager {
            let manager = cluster_manager.lock().map_err(|_| {
                Status::internal("Failed to lock cluster manager")
            })?;
            
            let nodes = manager.get_all_nodes();
            
            // 过滤掉请求节点自身
            let filtered_nodes: Vec<NodeInfo> = nodes.into_iter()
                .filter(|n| n.id != req.node_id)
                .take(req.max_nodes as usize)
                .collect();
            
            Ok(Response::new(DiscoverNodesResponse {
                nodes: filtered_nodes,
            }))
        } else {
            // 回退到简单内存存储
            let nodes = self.nodes.lock().map_err(|_| {
                Status::internal("Failed to lock nodes map")
            })?;
            
            // 过滤掉请求节点自身
            let filtered_nodes: Vec<NodeInfo> = nodes.values()
                .filter(|n| n.id != req.node_id)
                .take(req.max_nodes as usize)
                .cloned()
                .collect();
            
            Ok(Response::new(DiscoverNodesResponse {
                nodes: filtered_nodes,
            }))
        }
    }
    
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        
        // 更新节点状态
        let node_id = req.node_id.clone();
        let status = NodeStatus::NodeUnknown;
        
        // 优先使用集群管理器
        if let Some(cluster_manager) = &self.cluster_manager {
            let manager = cluster_manager.lock().map_err(|_| {
                Status::internal("Failed to lock cluster manager")
            })?;
            
            // 尝试更新状态，如果节点不存在则忽略错误
            let _ = manager.update_node_status(&node_id, status, req.timestamp);
            
            Ok(Response::new(HeartbeatResponse {
                acknowledged: true,
                server_timestamp: Self::get_current_timestamp(),
                cluster_id: manager.config.id.clone(),
            }))
        } else {
            // 回退到简单内存存储
            let mut nodes = self.nodes.lock().map_err(|_| {
                Status::internal("Failed to lock nodes map")
            })?;
            
            if let Some(node_info) = nodes.get_mut(&node_id) {
                node_info.status = req.status;
                node_info.last_seen = req.timestamp;
            }
            
            Ok(Response::new(HeartbeatResponse {
                acknowledged: true,
                server_timestamp: Self::get_current_timestamp(),
                cluster_id: "default".to_string(),
            }))
        }
    }
    
    async fn join_cluster(
        &self,
        request: Request<JoinClusterRequest>,
    ) -> Result<Response<JoinClusterResponse>, Status> {
        let req = request.into_inner();
        let node_info = req.node_info.ok_or_else(|| {
            Status::invalid_argument("Node info is required")
        })?;
        
        // 必须有集群管理器才能加入集群
        if let Some(cluster_manager) = &self.cluster_manager {
            let manager = cluster_manager.lock().map_err(|_| {
                Status::internal("Failed to lock cluster manager")
            })?;
            
            // 检查 join_token 如果有设置
            if let Some(expected_token) = &manager.config.join_token {
                if req.join_token != *expected_token {
                    return Ok(Response::new(JoinClusterResponse {
                        success: false,
                        error: "Invalid join token".to_string(),
                        cluster_info: None,
                    }));
                }
            }
            
            // 注册节点
            match manager.register_node(node_info) {
                Ok(()) => {
                    Ok(Response::new(JoinClusterResponse {
                        success: true,
                        error: String::new(),
                        cluster_info: Some(manager.get_cluster_info()),
                    }))
                }
                Err(e) => {
                    Ok(Response::new(JoinClusterResponse {
                        success: false,
                        error: e,
                        cluster_info: None,
                    }))
                }
            }
        } else {
            Ok(Response::new(JoinClusterResponse {
                success: false,
                error: "Cluster manager not initialized".to_string(),
                cluster_info: None,
            }))
        }
    }
    
    async fn leave_cluster(
        &self,
        request: Request<LeaveClusterRequest>,
    ) -> Result<Response<LeaveClusterResponse>, Status> {
        let req = request.into_inner();
        
        // 必须有集群管理器才能离开集群
        if let Some(cluster_manager) = &self.cluster_manager {
            let manager = cluster_manager.lock().map_err(|_| {
                Status::internal("Failed to lock cluster manager")
            })?;
            
            // 移除节点
            match manager.remove_node(&req.node_id) {
                Ok(()) => {
                    Ok(Response::new(LeaveClusterResponse {
                        success: true,
                        error: String::new(),
                    }))
                }
                Err(e) => {
                    Ok(Response::new(LeaveClusterResponse {
                        success: false,
                        error: e,
                    }))
                }
            }
        } else {
            // 回退到简单内存存储
            let mut nodes = self.nodes.lock().map_err(|_| {
                Status::internal("Failed to lock nodes map")
            })?;
            
            if nodes.remove(&req.node_id).is_some() {
                Ok(Response::new(LeaveClusterResponse {
                    success: true,
                    error: String::new(),
                }))
            } else {
                Ok(Response::new(LeaveClusterResponse {
                    success: false,
                    error: format!("Node {} not found", req.node_id),
                }))
            }
        }
    }
} 
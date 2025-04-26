use tonic::{Request, Response, Status};
use std::path::Path;
use crate::fs::FileSystem;
use crate::metadata::MetadataStore;
use crate::sync::SyncManager;
use crate::cluster::ClusterManager;
use std::sync::Arc;
use crate::proto::vdfs::vdfs_service_server::VdfsService;
use crate::proto::vdfs::{
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
use chrono::Utc;
use tokio_stream::Stream;
use std::pin::Pin;
use crate::config::{ClusterConfig, NodeConfig};
use tokio::sync::RwLock;

pub struct VDFSServiceImpl {
    fs: Arc<dyn FileSystem>,
    metadata_store: Arc<RwLock<MetadataStore>>,
    sync_manager: Arc<SyncManager>,
    cluster_manager: Arc<ClusterManager>,
}

impl VDFSServiceImpl {
    pub fn new(
        fs: Arc<dyn FileSystem>,
        metadata_store: Arc<RwLock<MetadataStore>>,
        sync_manager: Arc<SyncManager>,
        cluster_manager: Arc<ClusterManager>,
    ) -> Self {
        Self {
            fs,
            metadata_store,
            sync_manager,
            cluster_manager,
        }
    }

    pub async fn with_cluster_manager(fs: Arc<dyn FileSystem>, cluster_manager: Arc<ClusterManager>) -> Result<Self, crate::error::Error> {
        let metadata_store = Arc::new(RwLock::new(MetadataStore::new().await?));
        let sync_manager = Arc::new(SyncManager::new(
            metadata_store.clone(),
            ClusterConfig::default(),
            NodeConfig::default(),
        ));
        Ok(Self::new(
            fs,
            metadata_store,
            sync_manager,
            cluster_manager,
        ))
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
    ) -> Result<Response<CreateFileResponse>, Status> {
        let req = request.into_inner();
        self.fs.write_file(Path::new(&req.path), &[])
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: req.path.split('/').last().unwrap_or("").to_string(),
            path: req.path,
            r#type: req.r#type,
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
        self.fs.create_dir(Path::new(&req.path))
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
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        self.fs.delete_file(Path::new(&req.path))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(DeleteFileResponse { success: true }))
    }

    type ReadFileStream = Pin<Box<dyn Stream<Item = Result<ReadFileResponse, Status>> + Send + 'static>>;

    async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> Result<Response<Self::ReadFileStream>, Status> {
        let req = request.into_inner();
        let content = self.fs.read_file(Path::new(&req.path))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let (tx, rx) = mpsc::channel(4);
        let response_stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));

        tokio::spawn(async move {
            tx.send(Ok(ReadFileResponse { data: content })).await.unwrap();
        });

        Ok(Response::new(response_stream))
    }

    async fn write_file(
        &self,
        request: Request<tonic::Streaming<WriteFileRequest>>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let mut stream = request.into_inner();
        let mut bytes_written = 0;

        while let Some(req) = stream.message().await.map_err(|e| Status::internal(e.to_string()))? {
            self.fs.write_file(Path::new(&req.path), &req.data)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            bytes_written += req.data.len() as i64;
        }

        Ok(Response::new(WriteFileResponse { bytes_written }))
    }

    async fn list_directory(
        &self,
        request: Request<ListDirectoryRequest>,
    ) -> Result<Response<ListDirectoryResponse>, Status> {
        let req = request.into_inner();
        let entries = self.fs.list_dir(Path::new(&req.path))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let node_id = req.node_id.clone();
        let files = entries.into_iter()
            .map(|path| {
                FileInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: path.clone(),
                    path: path,
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
    ) -> Result<Response<GetFileInfoResponse>, Status> {
        let req = request.into_inner();
        let metadata = self.metadata_store.read().await.get_file(&req.path)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("File not found"))?;

        let info = FileInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: metadata.name,
            path: req.path,
            r#type: FileType::File as i32,
            size: metadata.size as i64,
            created_at: metadata.created_at,
            modified_at: metadata.modified_at,
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
        
        let node_info = self.cluster_manager.get_node(&req.node_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .unwrap_or_else(|| {
                NodeInfo {
                    id: req.node_id.clone(),
                    name: "Unknown".to_string(),
                    host: "127.0.0.1".to_string(),
                    port: 50051,
                    status: NodeStatus::NodeUnknown as i32,
                    last_seen: Self::get_current_timestamp(),
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

        self.cluster_manager.add_node(node_info.clone())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

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
        let nodes = self.cluster_manager.list_nodes()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let cluster_info = ClusterInfo {
            nodes,
            last_updated: Utc::now().timestamp(),
        };

        Ok(Response::new(GetClusterInfoResponse {
            cluster_info: Some(cluster_info),
        }))
    }

    async fn update_node_status(
        &self,
        request: Request<UpdateNodeStatusRequest>,
    ) -> Result<Response<UpdateNodeStatusResponse>, Status> {
        let request = request.into_inner();
        
        match self.cluster_manager.update_node_status(&request.node_id, NodeStatus::NodeUnknown).await {
            Ok(()) => {
                Ok(Response::new(UpdateNodeStatusResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                Ok(Response::new(UpdateNodeStatusResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }
    
    async fn discover_nodes(
        &self,
        request: Request<DiscoverNodesRequest>,
    ) -> Result<Response<DiscoverNodesResponse>, Status> {
        let req = request.into_inner();
        
        let nodes = self.cluster_manager.list_nodes()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        // 过滤掉请求节点自身
        let filtered_nodes: Vec<NodeInfo> = nodes.into_iter()
            .filter(|n| n.id != req.node_id)
            .take(req.max_nodes as usize)
            .collect();
        
        Ok(Response::new(DiscoverNodesResponse {
            nodes: filtered_nodes,
        }))
    }
    
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        
        // 更新节点状态
        let _ = self.cluster_manager.update_node_status(&req.node_id, NodeStatus::NodeUnknown).await;
        
        Ok(Response::new(HeartbeatResponse {
            acknowledged: true,
            server_timestamp: Self::get_current_timestamp(),
            cluster_id: "default".to_string(),
        }))
    }
    
    async fn join_cluster(
        &self,
        request: Request<JoinClusterRequest>,
    ) -> Result<Response<JoinClusterResponse>, Status> {
        let req = request.into_inner();
        let node_info = req.node_info.ok_or_else(|| {
            Status::invalid_argument("Node info is required")
        })?;
        
        match self.cluster_manager.add_node(node_info.clone()).await {
            Ok(()) => {
                let nodes = self.cluster_manager.list_nodes()
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                
                let cluster_info = ClusterInfo {
                    nodes,
                    last_updated: Utc::now().timestamp(),
                };
                
                Ok(Response::new(JoinClusterResponse {
                    success: true,
                    error: String::new(),
                    cluster_info: Some(cluster_info),
                }))
            }
            Err(e) => {
                Ok(Response::new(JoinClusterResponse {
                    success: false,
                    error: e.to_string(),
                    cluster_info: None,
                }))
            }
        }
    }
    
    async fn leave_cluster(
        &self,
        request: Request<LeaveClusterRequest>,
    ) -> Result<Response<LeaveClusterResponse>, Status> {
        let req = request.into_inner();
        
        match self.cluster_manager.remove_node(&req.node_id).await {
            Ok(()) => {
                Ok(Response::new(LeaveClusterResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                Ok(Response::new(LeaveClusterResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }
} 
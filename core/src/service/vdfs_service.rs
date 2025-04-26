use crate::fs::FileSystem;
use crate::metadata::{FileMetadata, MetadataStore, NodeStatus};
use crate::proto::vdfs::vdfs_service_server::VdfsService;
use crate::proto::vdfs::{
    CopyFileRequest, CopyFileResponse, CreateFileRequest, CreateFileResponse, DeleteFileRequest,
    DeleteFileResponse, DropFileRequest, DropFileResponse, FileInfo, FileType, GetFileInfoRequest,
    GetFileInfoResponse, GetNodeStatusRequest, GetNodeStatusResponse, ListDirectoryRequest,
    ListDirectoryResponse, MoveFileRequest, MoveFileResponse, ReadFileRequest, ReadFileResponse,
    ReceiveFileRequest, ReceiveFileResponse, SyncMetadataRequest, SyncMetadataResponse,
    WriteFileRequest, WriteFileResponse,
};
use crate::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

pub struct VDFSServiceImpl {
    fs: Arc<dyn FileSystem>,
    metadata: Arc<RwLock<MetadataStore>>,
    sync_manager: Arc<SyncManager>,
}

impl VDFSServiceImpl {
    pub fn new(
        fs: Arc<dyn FileSystem>,
        metadata: Arc<RwLock<MetadataStore>>,
        sync_manager: Arc<SyncManager>,
    ) -> Self {
        Self {
            fs,
            metadata,
            sync_manager,
        }
    }

    pub fn with_cluster_manager(fs: Arc<dyn FileSystem>, cluster_manager: ClusterManager) -> Self {
        let metadata = Arc::new(RwLock::new(MetadataStore::new()));
        let sync_manager = Arc::new(SyncManager::new(metadata.clone()));
        Self {
            fs,
            metadata,
            sync_manager,
        }
    }
}

#[tonic::async_trait]
impl VdfsService for VDFSServiceImpl {
    async fn list_directory(
        &self,
        request: Request<ListDirectoryRequest>,
    ) -> Result<Response<ListDirectoryResponse>, Status> {
        let req = request.get_ref();
        let entries = self.fs.list_directory(&req.path).await?;
        
        Ok(Response::new(ListDirectoryResponse {
            entries: entries.into_iter().map(|info| FileInfo {
                id: info.id,
                name: info.name,
                path: info.path.to_string_lossy().into_owned(),
                r#type: match info.file_type {
                    crate::fs::FileType::File => FileType::File as i32,
                    crate::fs::FileType::Directory => FileType::Directory as i32,
                    crate::fs::FileType::Symlink => FileType::Symlink as i32,
                    crate::fs::FileType::Unknown => FileType::Unknown as i32,
                },
                size: info.size,
                created_at: info.created_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                modified_at: info.modified_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                accessed_at: info.accessed_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                owner_node: req.node_id.clone(),
                available_nodes: vec![req.node_id.clone()],
                attributes: info.attributes,
            }).collect(),
        }))
    }

    async fn get_file_info(
        &self,
        request: Request<GetFileInfoRequest>,
    ) -> Result<Response<GetFileInfoResponse>, Status> {
        let req = request.get_ref();
        let info = self.fs.get_file_info(&req.path).await?;
        
        Ok(Response::new(GetFileInfoResponse {
            info: Some(FileInfo {
                id: info.id,
                name: info.name,
                path: info.path.to_string_lossy().into_owned(),
                r#type: match info.file_type {
                    crate::fs::FileType::File => FileType::File as i32,
                    crate::fs::FileType::Directory => FileType::Directory as i32,
                    crate::fs::FileType::Symlink => FileType::Symlink as i32,
                    crate::fs::FileType::Unknown => FileType::Unknown as i32,
                },
                size: info.size,
                created_at: info.created_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                modified_at: info.modified_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                accessed_at: info.accessed_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                owner_node: req.node_id.clone(),
                available_nodes: vec![req.node_id.clone()],
                attributes: info.attributes,
            }),
        }))
    }

    async fn create_file(
        &self,
        request: Request<CreateFileRequest>,
    ) -> Result<Response<CreateFileResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        
        // 创建文件
        self.fs.create_file(&path).await.map_err(|e| Status::internal(e.to_string()))?;
        
        // 更新元数据
        let mut metadata = self.metadata.write().await;
        metadata.add_file(&path);
        
        // 同步元数据到其他节点
        self.sync_manager.sync_metadata().await;
        
        Ok(Response::new(CreateFileResponse {}))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        
        // 删除文件
        self.fs.delete_file(&path).await.map_err(|e| Status::internal(e.to_string()))?;
        
        // 更新元数据
        let mut metadata = self.metadata.write().await;
        metadata.remove_file(&path);
        
        // 同步元数据到其他节点
        self.sync_manager.sync_metadata().await;
        
        Ok(Response::new(DeleteFileResponse {}))
    }

    async fn move_file(
        &self,
        request: Request<MoveFileRequest>,
    ) -> Result<Response<MoveFileResponse>, Status> {
        let req = request.get_ref();
        self.fs.move_file(&req.source_path, &req.target_path).await?;
        
        Ok(Response::new(MoveFileResponse { success: true }))
    }

    async fn copy_file(
        &self,
        request: Request<CopyFileRequest>,
    ) -> Result<Response<CopyFileResponse>, Status> {
        let req = request.get_ref();
        self.fs.copy_file(&req.source_path, &req.target_path).await?;
        
        Ok(Response::new(CopyFileResponse { success: true }))
    }

    type ReadFileStream = tokio_stream::wrappers::ReceiverStream<Result<ReadFileResponse, Status>>;

    async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> Result<Response<ReadFileResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        
        // 读取文件
        let data = self.fs.read_file(&path).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(ReadFileResponse { data }))
    }

    async fn write_file(
        &self,
        request: Request<WriteFileRequest>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        let data = req.data;
        
        // 写入文件
        self.fs.write_file(&path, &data).await.map_err(|e| Status::internal(e.to_string()))?;
        
        // 更新元数据
        let mut metadata = self.metadata.write().await;
        metadata.update_file(&path);
        
        // 同步元数据到其他节点
        self.sync_manager.sync_metadata().await;
        
        Ok(Response::new(WriteFileResponse {}))
    }

    type SyncMetadataStream = tokio_stream::wrappers::ReceiverStream<Result<SyncMetadataResponse, Status>>;

    async fn sync_metadata(
        &self,
        request: Request<SyncMetadataRequest>,
    ) -> Result<Response<SyncMetadataResponse>, Status> {
        let req = request.into_inner();
        let metadata_update = req.metadata;
        
        // 更新本地元数据
        let mut metadata = self.metadata.write().await;
        metadata.merge(metadata_update);
        
        Ok(Response::new(SyncMetadataResponse {}))
    }

    async fn get_node_status(
        &self,
        request: Request<GetNodeStatusRequest>,
    ) -> Result<Response<GetNodeStatusResponse>, Status> {
        let node_id = request.get_ref().node_id.clone();
        let status = self.metadata.read().await.get_node(&node_id)
            .ok_or_else(|| Status::not_found("Node not found"))?;
        
        Ok(Response::new(GetNodeStatusResponse {
            status: Some(crate::proto::vdfs::NodeStatus {
                id: status.id.clone(),
                name: status.name.clone(),
                host: status.host.clone(),
                port: status.port as i32,
                last_seen: status.last_seen.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                is_online: status.is_online,
            }),
        }))
    }

    type DropFileStream = tokio_stream::wrappers::ReceiverStream<Result<DropFileResponse, Status>>;

    async fn drop_file(
        &self,
        request: Request<DropFileRequest>,
    ) -> Result<Response<Self::DropFileStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        let req = request.get_ref();
        let fs = self.fs.clone();
        let metadata = self.metadata.clone();
        let file_id = req.file_id.clone();
        let target_node = req.target_node.clone();
        
        tokio::spawn(async move {
            if let Some(file) = metadata.read().await.get_file(&file_id) {
                // 发送文件信息
                let _ = tx.send(Ok(DropFileResponse {
                    response: Some(crate::proto::vdfs::drop_file_response::Response::FileInfo(
                        FileInfo {
                            id: file.id.clone(),
                            name: file.name.clone(),
                            path: file.path.clone(),
                            r#type: match file.file_type {
                                crate::fs::FileType::File => FileType::File as i32,
                                crate::fs::FileType::Directory => FileType::Directory as i32,
                                crate::fs::FileType::Symlink => FileType::Symlink as i32,
                                crate::fs::FileType::Unknown => FileType::Unknown as i32,
                            },
                            size: file.size,
                            created_at: file.created_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
                            modified_at: file.modified_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
                            accessed_at: file.accessed_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
                            owner_node: file.owner_node.clone(),
                            available_nodes: file.available_nodes.clone(),
                            attributes: file.attributes.clone(),
                        },
                    )),
                })).await;

                // 分块读取并发送文件内容
                let chunk_size = 1024 * 1024; // 1MB 分块
                let mut offset = 0;
                while offset < file.size {
                    let read_size = std::cmp::min(chunk_size, file.size - offset);
                    match fs.read_file_chunk(&file.path, offset, read_size).await {
                        Ok(chunk) => {
                            let _ = tx.send(Ok(DropFileResponse {
                                response: Some(crate::proto::vdfs::drop_file_response::Response::Chunk(chunk)),
                            })).await;
                            offset += read_size;
                        }
                        Err(e) => {
                            let _ = tx.send(Ok(DropFileResponse {
                                response: Some(crate::proto::vdfs::drop_file_response::Response::Error(e.to_string())),
                            })).await;
                            break;
                        }
                    }
                }
            } else {
                let _ = tx.send(Ok(DropFileResponse {
                    response: Some(crate::proto::vdfs::drop_file_response::Response::Error(
                        "文件不存在".to_string(),
                    )),
                })).await;
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn receive_file(
        &self,
        request: Request<tonic::Streaming<ReceiveFileRequest>>,
    ) -> Result<Response<ReceiveFileResponse>, Status> {
        let mut stream = request.into_inner();
        let mut file_info = None;
        let mut file_path = None;
        let mut offset = 0;

        while let Some(req) = stream.message().await.map_err(|e| Status::internal(e.to_string()))? {
            match req.request {
                Some(crate::proto::vdfs::receive_file_request::Request::FileInfo(info)) => {
                    file_info = Some(info);
                    file_path = Some(format!("{}/{}", self.fs.get_root_dir(), info.name));
                }
                Some(crate::proto::vdfs::receive_file_request::Request::Chunk(chunk)) => {
                    if let Some(path) = &file_path {
                        match self.fs.write_file_chunk(path, offset, &chunk).await {
                            Ok(_) => {
                                offset += chunk.len() as u64;
                            }
                            Err(e) => {
                                return Ok(Response::new(ReceiveFileResponse {
                                    success: false,
                                    error: e.to_string(),
                                }));
                            }
                        }
                    }
                }
                None => {}
            }
        }

        if let Some(info) = file_info {
            // 更新元数据
            let mut metadata = self.metadata.write().await;
            metadata.add_file(FileMetadata {
                id: info.id,
                name: info.name,
                path: info.path,
                size: info.size as u64,
                file_type: match info.r#type {
                    1 => crate::fs::FileType::File,
                    2 => crate::fs::FileType::Directory,
                    3 => crate::fs::FileType::Symlink,
                    _ => crate::fs::FileType::Unknown,
                },
                created_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(info.created_at as u64),
                modified_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(info.modified_at as u64),
                accessed_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(info.accessed_at as u64),
                owner_node: info.owner_node,
                available_nodes: info.available_nodes,
                attributes: info.attributes,
            });
        }

        Ok(Response::new(ReceiveFileResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        
        // 列出目录下的文件
        let files = self.fs.list_files(&path).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(ListFilesResponse { files }))
    }
} 
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
    sync_manager: SyncManager,
}

impl VDFSServiceImpl {
    pub fn new(
        fs: Arc<dyn FileSystem>,
        metadata: Arc<RwLock<MetadataStore>>,
        sync_manager: SyncManager,
    ) -> Self {
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
        let req = request.get_ref();
        let file_type = match req.r#type {
            1 => crate::fs::FileType::File,
            2 => crate::fs::FileType::Directory,
            3 => crate::fs::FileType::Symlink,
            _ => crate::fs::FileType::Unknown,
        };
        
        let info = self.fs.create_file(&req.path, file_type).await?;
        
        Ok(Response::new(CreateFileResponse {
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

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        self.fs.delete_file(&request.get_ref().path).await?;
        
        Ok(Response::new(DeleteFileResponse { success: true }))
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
    ) -> Result<Response<Self::ReadFileStream>, Status> {
        let req = request.get_ref();
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        let fs = self.fs.clone();
        let path = req.path.clone();
        let offset = req.offset;
        let length = req.length as usize;
        
        tokio::spawn(async move {
            match fs.read_file(&path, offset, length).await {
                Ok(data) => {
                    let _ = tx.send(Ok(ReadFileResponse { data })).await;
                }
                Err(e) => {
                    let _ = tx.send(Err(Status::internal(e.to_string()))).await;
                }
            }
        });
        
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn write_file(
        &self,
        request: Request<tonic::Streaming<WriteFileRequest>>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let mut stream = request.into_inner();
        let mut total_bytes = 0;
        
        while let Some(req) = stream.message().await? {
            let bytes_written = self.fs.write_file(&req.path, req.offset, &req.data).await?;
            total_bytes += bytes_written;
        }
        
        Ok(Response::new(WriteFileResponse {
            bytes_written: total_bytes as i64,
        }))
    }

    type SyncMetadataStream = tokio_stream::wrappers::ReceiverStream<Result<SyncMetadataResponse, Status>>;

    async fn sync_metadata(
        &self,
        request: Request<SyncMetadataRequest>,
    ) -> Result<Response<Self::SyncMetadataStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        let metadata = self.metadata.clone();
        let node_id = request.get_ref().node_id.clone();
        
        tokio::spawn(async move {
            let files = metadata.read().await.get_files_by_node(&node_id);
            let response = SyncMetadataResponse {
                files: files.into_iter().map(|f| FileInfo {
                    id: f.id.clone(),
                    name: f.name.clone(),
                    path: f.path.clone(),
                    r#type: match f.file_type {
                        crate::fs::FileType::File => FileType::File as i32,
                        crate::fs::FileType::Directory => FileType::Directory as i32,
                        crate::fs::FileType::Symlink => FileType::Symlink as i32,
                        crate::fs::FileType::Unknown => FileType::Unknown as i32,
                    },
                    size: f.size,
                    created_at: f.created_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
                    modified_at: f.modified_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
                    accessed_at: f.accessed_at.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
                    owner_node: f.owner_node.clone(),
                    available_nodes: f.available_nodes.clone(),
                    attributes: f.attributes.clone(),
                }).collect(),
                sync_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            };
            
            let _ = tx.send(Ok(response)).await;
        });
        
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
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
} 
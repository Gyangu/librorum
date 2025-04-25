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
};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct VDFSServiceImpl {
    fs: LocalFileSystem,
}

impl VDFSServiceImpl {
    pub fn new(fs: LocalFileSystem) -> Self {
        Self { fs }
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
        
        let status = NodeStatus {
            id: req.node_id,
            name: "Unknown".to_string(),
            host: "127.0.0.1".to_string(),
            port: 50051,
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            is_online: true,
        };

        Ok(Response::new(GetNodeStatusResponse { status: Some(status) }))
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
} 
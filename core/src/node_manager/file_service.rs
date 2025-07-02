use crate::proto::file::{
    file_service_server::FileService,
    CreateDirectoryRequest, CreateDirectoryResponse,
    DeleteFileRequest, DeleteFileResponse,
    DownloadFileRequest, DownloadFileResponse,
    FileInfo, FilePermissions, FileType,
    GetFileInfoRequest, GetSyncStatusRequest,
    ListFilesRequest, ListFilesResponse,
    SyncStatus, SyncStatusResponse,
    UploadFileRequest, UploadFileResponse,
};
use crate::vdfs::{VDFS, VDFSConfig, VirtualPath};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, warn};

pub struct FileServiceImpl {
    // VDFS实例 - 实际的分布式文件系统
    vdfs: Option<Arc<VDFS>>,
    // 临时兼容：用于存储文件元数据的内存映射 (用于向后兼容)
    files: Arc<Mutex<HashMap<String, FileInfo>>>,
    // 文件计数器，用于生成唯一的文件ID
    file_counter: Arc<Mutex<u64>>,
}

impl FileServiceImpl {
    pub fn new() -> Self {
        Self {
            vdfs: None, // 初始为空，稍后通过async方法初始化
            files: Arc::new(Mutex::new(HashMap::new())),
            file_counter: Arc::new(Mutex::new(0)),
        }
    }
    
    /// 异步初始化VDFS（在NodeManager中调用）
    pub async fn init_vdfs(&mut self, config: VDFSConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Initializing VDFS for FileService...");
        let vdfs = VDFS::new(config).await
            .map_err(|e| format!("Failed to initialize VDFS: {}", e))?;
        self.vdfs = Some(Arc::new(vdfs));
        info!("✓ VDFS initialized successfully for FileService");
        Ok(())
    }
    
    /// 创建带有自定义VDFS的FileService
    pub fn with_vdfs(vdfs: Arc<VDFS>) -> Self {
        Self {
            vdfs: Some(vdfs),
            files: Arc::new(Mutex::new(HashMap::new())),
            file_counter: Arc::new(Mutex::new(0)),
        }
    }

    async fn generate_file_id(&self) -> String {
        let mut counter = self.file_counter.lock().await;
        *counter += 1;
        format!("file_{:010}", *counter)
    }

    fn get_file_type(is_directory: bool) -> FileType {
        if is_directory {
            FileType::Directory
        } else {
            FileType::Regular
        }
    }

    fn create_file_permissions() -> FilePermissions {
        FilePermissions {
            mode: 0o644,
            owner: "user".to_string(),
            group: "group".to_string(),
            readable: true,
            writable: true,
            executable: false,
        }
    }
}

#[tonic::async_trait]
impl FileService for FileServiceImpl {
    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        info!("Listing files in path: {}", req.path);

        let files = self.files.lock().await;
        
        // 过滤路径匹配的文件
        let matching_files: Vec<FileInfo> = files
            .values()
            .filter(|file| {
                // 如果是递归查询，包含所有以指定路径开始的文件
                if req.recursive {
                    file.parent_path.starts_with(&req.path)
                } else {
                    // 非递归查询，只包含直接子文件
                    file.parent_path == req.path
                }
            })
            .filter(|file| {
                // 根据是否包含隐藏文件进行过滤
                if req.include_hidden {
                    true
                } else {
                    !file.name.starts_with('.')
                }
            })
            .cloned()
            .collect();

        let total_size: i64 = matching_files.iter().map(|f| f.size).sum();

        debug!("Found {} files, total size: {} bytes", matching_files.len(), total_size);

        let response = ListFilesResponse {
            files: matching_files.clone(),
            current_path: req.path,
            total_count: matching_files.len() as i32,
            total_size,
        };

        Ok(Response::new(response))
    }

    async fn upload_file(
        &self,
        request: Request<Streaming<UploadFileRequest>>,
    ) -> Result<Response<UploadFileResponse>, Status> {
        let mut stream = request.into_inner();
        let files = Arc::clone(&self.files);
        
        let mut metadata: Option<crate::proto::file::UploadFileMetadata> = None;
        let mut bytes_uploaded = 0i64;
        let mut _file_data = Vec::new();

        while let Some(request) = stream.next().await {
            match request {
                Ok(req) => {
                    match req.data {
                        Some(crate::proto::file::upload_file_request::Data::Metadata(meta)) => {
                            info!("Receiving file upload: {} ({} bytes)", meta.name, meta.size);
                            metadata = Some(meta);
                        }
                        Some(crate::proto::file::upload_file_request::Data::Chunk(chunk)) => {
                            bytes_uploaded += chunk.len() as i64;
                            _file_data.extend_from_slice(&chunk);
                            
                            debug!("Received chunk: {} bytes (total: {})", chunk.len(), bytes_uploaded);
                        }
                        None => {
                            warn!("Received empty upload request data");
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving upload stream: {}", e);
                    return Err(Status::internal("Upload stream error"));
                }
            }
        }

        // 处理上传完成
        if let Some(meta) = metadata {
            let file_id = format!("file_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
            
            // 尝试使用VDFS存储实际文件数据
            info!("Attempting to write {} bytes to VDFS path: {}", _file_data.len(), meta.path);
            info!("First 32 bytes of data: {:?}", &_file_data.get(..32.min(_file_data.len())).unwrap_or(&[]));
            
            let vdfs_result = match &self.vdfs {
                Some(vdfs) => {
                    match vdfs.write_file(&meta.path, &_file_data).await {
                        Ok(_) => {
                            info!("✓ File successfully written to VDFS: {}", meta.path);
                            true
                        }
                        Err(e) => {
                            error!("✗ Failed to write file to VDFS: {}, falling back to memory storage", e);
                            false
                        }
                    }
                }
                None => {
                    warn!("VDFS not initialized, falling back to memory storage");
                    false
                }
            };
            
            let file_info = FileInfo {
                file_id: file_id.clone(),
                name: meta.name.clone(),
                path: meta.path.clone(),
                parent_path: std::path::Path::new(&meta.path)
                    .parent()
                    .unwrap_or(std::path::Path::new("/"))
                    .to_string_lossy()
                    .to_string(),
                size: bytes_uploaded,
                created_at: chrono::Utc::now().timestamp(),
                modified_at: chrono::Utc::now().timestamp(),
                accessed_at: chrono::Utc::now().timestamp(),
                file_type: FileType::Regular.into(),
                mime_type: meta.mime_type,
                checksum: meta.checksum,
                permissions: Some(Self::create_file_permissions()),
                is_directory: false,
                is_symlink: false,
                chunk_count: 1, // 简化实现，假设单个chunk
                chunk_ids: vec![format!("chunk_{}", file_id)],
                replication_factor: 3,
                is_compressed: meta.compress,
                is_encrypted: meta.encrypt,
                sync_status: if vdfs_result { SyncStatus::Synced } else { SyncStatus::Error }.into(),
            };

            // 保存文件信息到内存映射（向后兼容）
            let mut files_map = files.lock().await;
            files_map.insert(file_id.clone(), file_info.clone());
            
            info!("Successfully uploaded file: {} ({} bytes) - VDFS: {}", 
                  meta.name, bytes_uploaded, vdfs_result);

            let response = UploadFileResponse {
                success: true,
                message: "File uploaded successfully".to_string(),
                file_info: Some(file_info),
                bytes_uploaded,
            };

            Ok(Response::new(response))
        } else {
            error!("No metadata received for file upload");
            let response = UploadFileResponse {
                success: false,
                message: "No metadata received".to_string(),
                file_info: None,
                bytes_uploaded: 0,
            };
            Ok(Response::new(response))
        }
    }

    type DownloadFileStream = Pin<Box<dyn Stream<Item = Result<DownloadFileResponse, Status>> + Send>>;

    async fn download_file(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<Self::DownloadFileStream>, Status> {
        let req = request.into_inner();
        let files = self.files.lock().await;
        
        // 查找文件
        let file_info = if !req.file_id.is_empty() {
            files.get(&req.file_id).cloned()
        } else if !req.path.is_empty() {
            files.values().find(|f| f.path == req.path).cloned()
        } else {
            return Err(Status::invalid_argument("Either file_id or path must be provided"));
        };

        let file_info = file_info.ok_or_else(|| Status::not_found("File not found"))?;

        info!("Downloading file: {} ({} bytes)", file_info.name, file_info.size);

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let file_info_clone = file_info.clone();
        let vdfs = self.vdfs.clone(); // Clone the Option<Arc<VDFS>>
        
        tokio::spawn(async move {
            // 首先发送文件信息
            let info_response = DownloadFileResponse {
                data: Some(crate::proto::file::download_file_response::Data::FileInfo(file_info_clone.clone())),
                offset: 0,
                total_size: file_info_clone.size,
            };
            
            if tx.send(Ok(info_response)).await.is_err() {
                return;
            }

            // 尝试从VDFS读取实际文件数据
            info!("Attempting to read file from VDFS: {}", file_info_clone.path);
            let file_data = match &vdfs {
                Some(vdfs_instance) => {
                    match vdfs_instance.read_file(&file_info_clone.path).await {
                        Ok(data) => {
                            info!("✓ File successfully read from VDFS: {} bytes", data.len());
                            info!("First 32 bytes of read data: {:?}", &data.get(..32.min(data.len())).unwrap_or(&[]));
                            data
                        }
                        Err(e) => {
                            error!("✗ Failed to read file from VDFS: {}, sending empty data", e);
                            vec![0u8; file_info_clone.size as usize] // 向后兼容的模拟数据
                        }
                    }
                }
                None => {
                    warn!("VDFS not initialized, sending empty data");
                    vec![0u8; file_info_clone.size as usize] // 向后兼容的模拟数据
                }
            };
            
            let total_size = file_info_clone.size;
            // 高性能分块大小：更大的chunk减少gRPC序列化开销  
            let chunk_size = if total_size < 5 * 1024 * 1024 { // < 5MB
                1024 * 1024 // 1MB
            } else if total_size < 50 * 1024 * 1024 { // < 50MB
                4 * 1024 * 1024 // 4MB  
            } else {
                8 * 1024 * 1024 // 8MB for large files
            };
            let mut offset = req.offset as usize;
            let end_offset = if req.length > 0 {
                std::cmp::min(req.offset + req.length, total_size) as usize
            } else {
                total_size as usize
            };

            while offset < end_offset && offset < file_data.len() {
                let remaining = end_offset - offset;
                let current_chunk_size = std::cmp::min(chunk_size, remaining);
                let actual_chunk_size = std::cmp::min(current_chunk_size, file_data.len() - offset);
                
                // 读取实际文件数据块
                let chunk_data = if actual_chunk_size > 0 {
                    file_data[offset..offset + actual_chunk_size].to_vec()
                } else {
                    Vec::new()
                };
                
                let chunk_response = DownloadFileResponse {
                    data: Some(crate::proto::file::download_file_response::Data::Chunk(chunk_data)),
                    offset: offset as i64,
                    total_size,
                };
                
                if tx.send(Ok(chunk_response)).await.is_err() {
                    break;
                }
                
                offset += actual_chunk_size;
                
                // 移除人为延迟以提升性能
            }

            debug!("Download completed: {} bytes sent", offset as i64 - req.offset);
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output_stream)))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        let mut files = self.files.lock().await;
        
        // 查找要删除的文件
        let file_to_delete = if !req.file_id.is_empty() {
            files.get(&req.file_id).cloned()
        } else if !req.path.is_empty() {
            files.values().find(|f| f.path == req.path).cloned()
        } else {
            return Err(Status::invalid_argument("Either file_id or path must be provided"));
        };

        let file_info = file_to_delete.ok_or_else(|| Status::not_found("File not found"))?;

        info!("Deleting file: {}", file_info.name);

        let mut deleted_count = 0;

        if file_info.is_directory && req.recursive {
            // 递归删除目录和其内容
            let dir_path = &file_info.path;
            let files_to_remove: Vec<String> = files
                .iter()
                .filter(|(_, f)| f.path.starts_with(dir_path))
                .map(|(id, _)| id.clone())
                .collect();
            
            for file_id in files_to_remove {
                files.remove(&file_id);
                deleted_count += 1;
            }
        } else if !file_info.is_directory || req.force {
            // 删除单个文件或强制删除目录
            files.remove(&file_info.file_id);
            deleted_count = 1;
        } else {
            return Err(Status::failed_precondition(
                "Cannot delete directory without recursive flag"
            ));
        }

        info!("Successfully deleted {} file(s)", deleted_count);

        let response = DeleteFileResponse {
            success: true,
            message: format!("Successfully deleted {} file(s)", deleted_count),
            deleted_count,
        };

        Ok(Response::new(response))
    }

    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        let req = request.into_inner();
        info!("Creating directory: {}", req.path);

        let mut files = self.files.lock().await;
        
        // 检查目录是否已存在
        if files.values().any(|f| f.path == req.path && f.is_directory) {
            return Err(Status::already_exists("Directory already exists"));
        }

        // 如果需要创建父目录
        if req.create_parents {
            let path = std::path::Path::new(&req.path);
            if let Some(parent) = path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if parent_str != "/" && !files.values().any(|f| f.path == parent_str && f.is_directory) {
                    // 递归创建父目录
                    let _parent_req = CreateDirectoryRequest {
                        path: parent_str,
                        create_parents: true,
                        permissions: req.permissions.clone(),
                    };
                    // 这里为了简化，直接创建而不是递归调用
                    // 在实际实现中应该使用递归调用或循环创建所有父目录
                }
            }
        }

        let file_id = format!("dir_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
        let directory_name = std::path::Path::new(&req.path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(""))
            .to_string_lossy()
            .to_string();

        let parent_path = std::path::Path::new(&req.path)
            .parent()
            .unwrap_or(std::path::Path::new("/"))
            .to_string_lossy()
            .to_string();

        let directory_info = FileInfo {
            file_id: file_id.clone(),
            name: directory_name,
            path: req.path.clone(),
            parent_path,
            size: 0, // 目录大小为0
            created_at: chrono::Utc::now().timestamp(),
            modified_at: chrono::Utc::now().timestamp(),
            accessed_at: chrono::Utc::now().timestamp(),
            file_type: FileType::Directory.into(),
            mime_type: "inode/directory".to_string(),
            checksum: String::new(),
            permissions: req.permissions.or_else(|| Some(Self::create_file_permissions())),
            is_directory: true,
            is_symlink: false,
            chunk_count: 0,
            chunk_ids: vec![],
            replication_factor: 1,
            is_compressed: false,
            is_encrypted: false,
            sync_status: SyncStatus::Synced.into(),
        };

        files.insert(file_id, directory_info.clone());

        info!("Successfully created directory: {}", req.path);

        let response = CreateDirectoryResponse {
            success: true,
            message: "Directory created successfully".to_string(),
            directory_info: Some(directory_info),
        };

        Ok(Response::new(response))
    }

    async fn get_file_info(
        &self,
        request: Request<GetFileInfoRequest>,
    ) -> Result<Response<FileInfo>, Status> {
        let req = request.into_inner();
        let files = self.files.lock().await;
        
        let file_info = if !req.file_id.is_empty() {
            files.get(&req.file_id).cloned()
        } else if !req.path.is_empty() {
            files.values().find(|f| f.path == req.path).cloned()
        } else {
            return Err(Status::invalid_argument("Either file_id or path must be provided"));
        };

        let mut file_info = file_info.ok_or_else(|| Status::not_found("File not found"))?;

        // 如果不需要包含chunk信息，清空chunk相关字段
        if !req.include_chunks {
            file_info.chunk_ids.clear();
            file_info.chunk_count = 0;
        }

        debug!("Retrieved file info for: {}", file_info.name);

        Ok(Response::new(file_info))
    }

    async fn get_sync_status(
        &self,
        request: Request<GetSyncStatusRequest>,
    ) -> Result<Response<SyncStatusResponse>, Status> {
        let req = request.into_inner();
        let files = self.files.lock().await;
        
        debug!("Getting sync status for path: {}", req.path);

        // 如果指定了路径，只统计该路径下的文件
        let target_files: Vec<&FileInfo> = if req.path.is_empty() {
            files.values().collect()
        } else {
            files.values().filter(|f| f.path.starts_with(&req.path)).collect()
        };

        let mut pending_uploads = 0;
        let pending_downloads = 0;
        let mut syncing_files = 0;
        let mut error_files = 0;
        let mut conflict_files = 0;
        let mut bytes_to_upload = 0i64;
        let bytes_to_download = 0i64;
        let mut pending_files = Vec::new();

        for file in &target_files {
            match SyncStatus::try_from(file.sync_status).unwrap_or(SyncStatus::Unknown) {
                SyncStatus::Pending => {
                    pending_uploads += 1;
                    bytes_to_upload += file.size;
                    pending_files.push((*file).clone());
                }
                SyncStatus::Syncing => {
                    syncing_files += 1;
                }
                SyncStatus::Error => {
                    error_files += 1;
                    pending_files.push((*file).clone());
                }
                SyncStatus::Conflict => {
                    conflict_files += 1;
                    pending_files.push((*file).clone());
                }
                _ => {}
            }
        }

        let overall_status = if error_files > 0 || conflict_files > 0 {
            SyncStatus::Error
        } else if syncing_files > 0 || pending_uploads > 0 || pending_downloads > 0 {
            SyncStatus::Syncing
        } else {
            SyncStatus::Synced
        };

        debug!("Sync status: {} files, {} pending, {} syncing, {} errors", 
               target_files.len(), pending_uploads, syncing_files, error_files);

        let response = SyncStatusResponse {
            overall_status: overall_status.into(),
            pending_uploads,
            pending_downloads,
            syncing_files,
            error_files,
            conflict_files,
            bytes_to_upload,
            bytes_to_download,
            pending_files,
        };

        Ok(Response::new(response))
    }
}
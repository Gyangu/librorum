//! 简化版Hybrid文件服务
//! 
//! 为了快速解决编译问题，这是一个简化版本的hybrid file service
//! 提供基本的文件服务功能，避免复杂的VDFS集成问题

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
// use futures_util::Stream; // 暂时未使用
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info};
use uuid::Uuid;

use crate::proto::file::{
    file_service_server::FileService,
    CreateDirectoryRequest, CreateDirectoryResponse,
    DeleteFileRequest, DeleteFileResponse,
    DownloadFileRequest, DownloadFileResponse,
    GetFileInfoRequest, GetSyncStatusRequest,
    ListFilesRequest, ListFilesResponse,
    SyncStatusResponse, UploadFileRequest, UploadFileResponse,
    FileInfo, FileType, SyncStatus, FilePermissions,
};

use librorum_shared::transport::hybrid::{
    HybridTransferCoordinator, HybridEvent, TransferType, SessionStatus
};

/// 简化版Hybrid文件服务
pub struct SimpleHybridFileService {
    /// 内存文件存储
    files: Arc<Mutex<HashMap<String, FileInfo>>>,
    /// 文件计数器
    file_counter: Arc<Mutex<u64>>,
    /// Hybrid传输协调器
    hybrid_coordinator: HybridTransferCoordinator,
    /// UTP服务器地址
    utp_server_addr: SocketAddr,
    /// 是否启用hybrid传输
    hybrid_enabled: bool,
}

impl SimpleHybridFileService {
    /// 创建新的简化版Hybrid文件服务
    pub fn new(utp_server_addr: SocketAddr) -> Self {
        let coordinator = HybridTransferCoordinator::new();
        
        // 设置事件处理
        coordinator.add_event_handler(|event| {
            match event {
                HybridEvent::SessionCreated { session_id, transfer_type, .. } => {
                    info!("📋 Hybrid会话创建: {} ({:?})", session_id, transfer_type);
                }
                HybridEvent::TransferProgress { session_id, bytes_transferred, total_bytes, transfer_rate } => {
                    let progress = (bytes_transferred as f64 / total_bytes as f64) * 100.0;
                    debug!("📊 传输进度: {} {:.1}% ({:.2} MB/s)", 
                        session_id, progress, transfer_rate / 1024.0 / 1024.0);
                }
                HybridEvent::TransferCompleted { session_id, success, elapsed_secs, .. } => {
                    if success {
                        info!("✅ 传输完成: {} (耗时: {:.2}s)", session_id, elapsed_secs);
                    } else {
                        info!("❌ 传输失败: {}", session_id);
                    }
                }
                _ => {}
            }
        });
        
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            file_counter: Arc::new(Mutex::new(1)),
            hybrid_coordinator: coordinator,
            utp_server_addr,
            hybrid_enabled: true,
        }
    }
    
    /// 生成新的文件ID
    async fn generate_file_id(&self) -> String {
        let mut counter = self.file_counter.lock().await;
        let id = *counter;
        *counter += 1;
        format!("file_{:06}", id)
    }
    
    /// 判断是否应该使用Hybrid传输
    fn should_use_hybrid(&self, file_size: u64) -> bool {
        self.hybrid_enabled && file_size > 1024 * 1024 // 大于1MB使用UTP
    }
    
    /// 获取传输统计信息
    pub fn get_transfer_stats(&self) -> TransferStats {
        let sessions = self.hybrid_coordinator.get_active_sessions();
        let total_sessions = sessions.len();
        let active_uploads = sessions.iter().filter(|s| s.transfer_type == TransferType::Upload).count();
        let active_downloads = sessions.iter().filter(|s| s.transfer_type == TransferType::Download).count();
        
        TransferStats {
            total_sessions,
            active_uploads,
            active_downloads,
            total_bytes_transferred: 0,
            average_transfer_rate: 0.0,
        }
    }
}

/// 传输统计信息
#[derive(Debug, Clone)]
pub struct TransferStats {
    pub total_sessions: usize,
    pub active_uploads: usize,
    pub active_downloads: usize,
    pub total_bytes_transferred: u64,
    pub average_transfer_rate: f64,
}

#[tonic::async_trait]
impl FileService for SimpleHybridFileService {
    type DownloadFileStream = ReceiverStream<Result<DownloadFileResponse, Status>>;

    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        info!("📂 列出文件: path={}, recursive={}", req.path, req.recursive);
        
        let files = self.files.lock().await;
        let file_list: Vec<FileInfo> = files.values().cloned().collect();
        
        Ok(Response::new(ListFilesResponse {
            files: file_list,
            current_path: req.path,
            total_count: files.len() as i32,
            total_size: files.values().map(|f| f.size).sum(),
        }))
    }

    async fn upload_file(
        &self,
        request: Request<Streaming<UploadFileRequest>>,
    ) -> Result<Response<UploadFileResponse>, Status> {
        let mut stream = request.into_inner();
        let mut file_data = Vec::new();
        let mut file_metadata = None;
        let mut total_size = 0u64;
        
        // 处理流式上传
        while let Some(request) = tokio_stream::StreamExt::next(&mut stream).await {
            let request = request?;
            
            match request.data {
                Some(crate::proto::file::upload_file_request::Data::Metadata(metadata)) => {
                    info!("📋 接收文件信息: {} ({} bytes)", metadata.name, metadata.size);
                    file_metadata = Some(metadata);
                }
                Some(crate::proto::file::upload_file_request::Data::Chunk(chunk)) => {
                    total_size += chunk.len() as u64;
                    file_data.extend_from_slice(&chunk);
                }
                None => {
                    return Err(Status::invalid_argument("无效的上传请求"));
                }
            }
        }
        
        let metadata = file_metadata.ok_or_else(|| Status::invalid_argument("没有接收到文件信息"))?;
        
        if total_size != metadata.size as u64 {
            return Err(Status::invalid_argument(format!(
                "文件大小不匹配: 期望 {} bytes, 实际接收 {} bytes",
                metadata.size, total_size
            )));
        }
        
        // 生成文件ID并存储
        let file_id = self.generate_file_id().await;
        let file_info = FileInfo {
            file_id: file_id.clone(),
            name: metadata.name.clone(),
            path: metadata.path.clone(),
            parent_path: "/".to_string(),
            size: metadata.size,
            created_at: chrono::Utc::now().timestamp(),
            modified_at: chrono::Utc::now().timestamp(),
            accessed_at: chrono::Utc::now().timestamp(),
            file_type: FileType::Regular as i32,
            mime_type: metadata.mime_type.clone(),
            checksum: metadata.checksum.clone(),
            permissions: Some(FilePermissions {
                mode: 0o644,
                owner: "user".to_string(),
                group: "group".to_string(),
                readable: true,
                writable: true,
                executable: false,
            }),
            is_directory: false,
            is_symlink: false,
            chunk_count: 1,
            chunk_ids: vec![file_id.clone()],
            replication_factor: 1,
            is_compressed: metadata.compress,
            is_encrypted: metadata.encrypt,
            sync_status: SyncStatus::Synced as i32,
        };
        
        // 检查是否使用Hybrid传输
        if self.should_use_hybrid(total_size) {
            info!("🚀 使用Hybrid传输: {} ({} bytes)", metadata.name, total_size);
            
            // 这里可以集成实际的UTP传输逻辑
            // 目前先简单存储到内存
        } else {
            info!("📤 使用传统传输: {} ({} bytes)", metadata.name, total_size);
        }
        
        // 存储文件信息
        self.files.lock().await.insert(file_id.clone(), file_info.clone());
        
        info!("✅ 文件上传成功: {} -> {}", metadata.name, file_id);
        
        Ok(Response::new(UploadFileResponse {
            success: true,
            message: format!("文件上传成功: {}", file_id),
            file_info: Some(file_info),
            bytes_uploaded: total_size as i64,
        }))
    }

    async fn download_file(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<Self::DownloadFileStream>, Status> {
        let req = request.into_inner();
        info!("📥 下载文件: file_id={}", req.file_id);
        
        let files = self.files.lock().await;
        let file_info = files.get(&req.file_id)
            .ok_or_else(|| Status::not_found(format!("文件不存在: {}", req.file_id)))?;
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        
        // 发送文件信息
        let file_info_clone = file_info.clone();
        tokio::spawn(async move {
            // 发送文件信息
            if tx.send(Ok(DownloadFileResponse {
                data: Some(crate::proto::file::download_file_response::Data::FileInfo(file_info_clone.clone())),
                offset: 0,
                total_size: file_info_clone.size,
            })).await.is_err() {
                return;
            }
            
            // 模拟文件数据 (在实际实现中这里会从存储中读取)
            let mock_data = vec![0x42u8; file_info_clone.size as usize];
            let chunk_size = 64 * 1024; // 64KB chunks
            
            for (i, chunk) in mock_data.chunks(chunk_size).enumerate() {
                let offset = (i * chunk_size) as i64;
                
                if tx.send(Ok(DownloadFileResponse {
                    data: Some(crate::proto::file::download_file_response::Data::Chunk(chunk.to_vec())),
                    offset,
                    total_size: file_info_clone.size,
                })).await.is_err() {
                    break;
                }
                
                // 小延迟模拟网络传输
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        });
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        info!("🗑️ 删除文件: file_id={}", req.file_id);
        
        let mut files = self.files.lock().await;
        let removed = files.remove(&req.file_id).is_some();
        
        if removed {
            Ok(Response::new(DeleteFileResponse {
                success: true,
                message: format!("文件删除成功: {}", req.file_id),
                deleted_count: 1,
            }))
        } else {
            Err(Status::not_found(format!("文件不存在: {}", req.file_id)))
        }
    }

    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        let req = request.into_inner();
        info!("📁 创建目录: path={}", req.path);
        
        // 简化实现：总是返回成功
        let directory_info = FileInfo {
            file_id: format!("dir_{}", Uuid::new_v4()),
            name: req.path.split('/').last().unwrap_or("").to_string(),
            path: req.path.clone(),
            parent_path: "/".to_string(),
            size: 0,
            created_at: chrono::Utc::now().timestamp(),
            modified_at: chrono::Utc::now().timestamp(),
            accessed_at: chrono::Utc::now().timestamp(),
            file_type: FileType::Directory as i32,
            mime_type: "inode/directory".to_string(),
            checksum: "".to_string(),
            permissions: req.permissions,
            is_directory: true,
            is_symlink: false,
            chunk_count: 0,
            chunk_ids: vec![],
            replication_factor: 1,
            is_compressed: false,
            is_encrypted: false,
            sync_status: SyncStatus::Synced as i32,
        };
        
        Ok(Response::new(CreateDirectoryResponse {
            success: true,
            message: format!("目录创建成功: {}", req.path),
            directory_info: Some(directory_info),
        }))
    }

    async fn get_file_info(
        &self,
        request: Request<GetFileInfoRequest>,
    ) -> Result<Response<FileInfo>, Status> {
        let req = request.into_inner();
        info!("ℹ️ 获取文件信息: file_id={}", req.file_id);
        
        let files = self.files.lock().await;
        let file_info = files.get(&req.file_id)
            .ok_or_else(|| Status::not_found(format!("文件不存在: {}", req.file_id)))?;
        
        Ok(Response::new(file_info.clone()))
    }

    async fn get_sync_status(
        &self,
        request: Request<GetSyncStatusRequest>,
    ) -> Result<Response<SyncStatusResponse>, Status> {
        let req = request.into_inner();
        debug!("🔄 获取同步状态: {}", req.path);
        
        // 检查是否有活跃的传输会话
        let active_sessions = self.hybrid_coordinator.get_active_sessions();
        let relevant_sessions: Vec<_> = active_sessions
            .iter()
            .filter(|session| session.file_info.remote_path == req.path)
            .collect();
        
        let sync_status = if let Some(session) = relevant_sessions.first() {
            match session.status {
                SessionStatus::Transferring => SyncStatus::Syncing,
                SessionStatus::Failed => SyncStatus::Error,
                _ => SyncStatus::Pending,
            }
        } else {
            SyncStatus::Synced
        };
        
        Ok(Response::new(SyncStatusResponse {
            overall_status: sync_status as i32,
            pending_uploads: if sync_status == SyncStatus::Pending { 1 } else { 0 },
            pending_downloads: 0,
            syncing_files: if sync_status == SyncStatus::Syncing { 1 } else { 0 },
            error_files: if sync_status == SyncStatus::Error { 1 } else { 0 },
            conflict_files: 0,
            bytes_to_upload: 0,
            bytes_to_download: 0,
            pending_files: vec![],
        }))
    }
}

impl Default for SimpleHybridFileService {
    fn default() -> Self {
        Self::new("127.0.0.1:9090".parse().unwrap())
    }
}
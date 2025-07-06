//! Hybrid文件服务实现
//! 
//! 结合gRPC控制和UTP数据传输的高性能文件服务

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

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
use librorum_shared::transport::hybrid::{
    HybridTransferCoordinator, HybridEvent, TransferType, SessionStatus
};
use librorum_shared::{UtpServer, UtpConfig, TransportMode};

/// Hybrid文件服务实现
pub struct HybridFileService {
    /// VDFS实例
    vdfs: Option<Arc<VDFS>>,
    /// 传统文件存储 (向后兼容)
    files: Arc<Mutex<HashMap<String, FileInfo>>>,
    /// 文件计数器
    file_counter: Arc<Mutex<u64>>,
    /// Hybrid传输协调器
    hybrid_coordinator: Arc<HybridTransferCoordinator>,
    /// UTP服务器配置
    utp_server_addr: SocketAddr,
    /// 是否启用hybrid模式
    hybrid_enabled: bool,
}

impl HybridFileService {
    /// 创建新的Hybrid文件服务
    pub fn new(utp_server_addr: SocketAddr) -> Self {
        let mut coordinator = HybridTransferCoordinator::new();
        
        // 设置事件处理
        coordinator.add_event_handler(|event| {
            match event {
                HybridEvent::SessionCreated { session_id, transfer_type, file_info } => {
                    info!("📁 Hybrid会话创建: {} ({:?}) - {}", 
                        session_id, transfer_type, file_info.name);
                }
                HybridEvent::GrpcCoordinationComplete { session_id, utp_endpoint } => {
                    info!("🤝 gRPC协调完成: {} -> UTP端点: {}", session_id, utp_endpoint);
                }
                HybridEvent::UtpConnectionEstablished { session_id, transport_mode } => {
                    info!("🔗 UTP连接建立: {} (模式: {:?})", session_id, transport_mode);
                }
                HybridEvent::TransferProgress { session_id, bytes_transferred, total_bytes, transfer_rate } => {
                    let progress = (bytes_transferred as f64 / total_bytes as f64) * 100.0;
                    debug!("📊 传输进度: {} {:.1}% ({:.2} MB/s)", 
                        session_id, progress, transfer_rate / 1024.0 / 1024.0);
                }
                HybridEvent::TransferCompleted { session_id, success, error_message, elapsed_secs } => {
                    if success {
                        info!("✅ 传输完成: {} (耗时: {:.2}s)", session_id, elapsed_secs);
                    } else {
                        error!("❌ 传输失败: {} - {:?}", session_id, error_message);
                    }
                }
                HybridEvent::SessionStatusChanged { session_id, old_status, new_status } => {
                    debug!("🔄 会话状态变化: {} {:?} -> {:?}", session_id, old_status, new_status);
                }
            }
        });
        
        Self {
            vdfs: None,
            files: Arc::new(Mutex::new(HashMap::new())),
            file_counter: Arc::new(Mutex::new(0)),
            hybrid_coordinator: Arc::new(coordinator),
            utp_server_addr,
            hybrid_enabled: true,
        }
    }
    
    /// 初始化VDFS
    pub async fn init_vdfs(&mut self, config: VDFSConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🔧 初始化Hybrid文件服务的VDFS...");
        let vdfs = VDFS::new(config).await
            .map_err(|e| format!("Failed to initialize VDFS: {}", e))?;
        self.vdfs = Some(Arc::new(vdfs));
        info!("✅ VDFS初始化成功");
        Ok(())
    }
    
    /// 启动UTP服务器
    pub async fn start_utp_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.hybrid_enabled {
            return Ok(());
        }
        
        info!("🚀 启动UTP服务器: {}", self.utp_server_addr);
        
        // 获取可变引用来启动服务器
        // 注意：这里需要解决架构问题，因为coordinator在Arc中
        // 暂时使用unsafe方法或重新设计架构
        
        let coordinator = unsafe {
            &mut *(Arc::as_ptr(&self.hybrid_coordinator) as *mut HybridTransferCoordinator)
        };
        
        coordinator.start_utp_server(self.utp_server_addr)
            .map_err(|e| format!("Failed to start UTP server: {}", e))?;
        
        info!("✅ UTP服务器启动成功");
        Ok(())
    }
    
    /// 获取下一个文件ID
    async fn next_file_id(&self) -> String {
        let mut counter = self.file_counter.lock().await;
        *counter += 1;
        format!("file_{}", *counter)
    }
    
    /// 检查是否应该使用hybrid模式
    fn should_use_hybrid(&self, file_size: u64) -> bool {
        self.hybrid_enabled && file_size > 1024 * 1024 // 大于1MB的文件使用hybrid模式
    }
    
    /// 创建hybrid上传会话
    async fn create_hybrid_upload_session(
        &self,
        file_id: String,
        user_id: String,
        file_name: String,
        file_size: u64,
    ) -> Result<String, Status> {
        let local_path = format!("/tmp/librorum_upload_{}", file_id);
        let remote_path = format!("/files/{}/{}", user_id, file_name);
        let grpc_endpoint = "hybrid_file_service".to_string();
        
        let session_id = self.hybrid_coordinator
            .initiate_upload(file_id, user_id, local_path, remote_path, grpc_endpoint)
            .map_err(|e| Status::internal(format!("Failed to create upload session: {}", e)))?;
        
        info!("📤 创建hybrid上传会话: {} (文件: {}, 大小: {} bytes)", 
            session_id, file_name, file_size);
        
        Ok(session_id)
    }
    
    /// 创建hybrid下载会话
    async fn create_hybrid_download_session(
        &self,
        file_id: String,
        user_id: String,
        file_info: &FileInfo,
    ) -> Result<String, Status> {
        let local_path = format!("/tmp/librorum_download_{}", file_id);
        let remote_path = format!("/files/{}/{}", user_id, file_info.name);
        let grpc_endpoint = "hybrid_file_service".to_string();
        
        let session_id = self.hybrid_coordinator
            .initiate_download(
                file_id, 
                user_id, 
                local_path, 
                remote_path, 
                file_info.size, 
                grpc_endpoint
            )
            .map_err(|e| Status::internal(format!("Failed to create download session: {}", e)))?;
        
        info!("📥 创建hybrid下载会话: {} (文件: {}, 大小: {} bytes)", 
            session_id, file_info.name, file_info.size);
        
        Ok(session_id)
    }
    
    /// 处理传统上传 (小文件或禁用hybrid模式)
    async fn handle_traditional_upload(
        &self,
        mut stream: Streaming<UploadFileRequest>,
    ) -> Result<Response<UploadFileResponse>, Status> {
        info!("📤 处理传统上传...");
        
        let mut file_info: Option<FileInfo> = None;
        let mut received_chunks = Vec::new();
        let mut total_size = 0u64;
        
        while let Some(request) = stream.next().await {
            let request = request?;
            
            match request.data {
                Some(crate::proto::file::upload_file_request::Data::FileInfo(info)) => {
                    info!("📋 接收文件信息: {} ({} bytes)", info.name, info.size);
                    file_info = Some(info);
                }
                Some(crate::proto::file::upload_file_request::Data::Chunk(chunk)) => {
                    total_size += chunk.len() as u64;
                    received_chunks.push(chunk);
                    debug!("📦 接收数据块: {} bytes (总计: {} bytes)", chunk.len(), total_size);
                }
                None => {
                    warn!("收到空的上传请求");
                }
            }
        }
        
        let file_info = file_info.ok_or_else(|| Status::invalid_argument("没有接收到文件信息"))?;
        
        if total_size != file_info.size {
            return Err(Status::invalid_argument(format!(
                "文件大小不匹配: 期望 {} bytes, 实际接收 {} bytes",
                file_info.size, total_size
            )));
        }
        
        let file_id = self.next_file_id().await;
        
        // 如果有VDFS，保存到VDFS
        if let Some(vdfs) = &self.vdfs {
            let virtual_path = VirtualPath::new(&format!("/files/{}", file_info.name))?;
            let combined_data: Vec<u8> = received_chunks.into_iter().flatten().collect();
            
            match vdfs.write_file(&virtual_path, &combined_data).await {
                Ok(_) => {
                    info!("✅ 文件成功保存到VDFS: {}", file_info.name);
                }
                Err(e) => {
                    error!("❌ VDFS保存失败: {} - 回退到内存存储", e);
                    // 回退到内存存储
                    let mut files = self.files.lock().await;
                    files.insert(file_id.clone(), file_info.clone());
                }
            }
        } else {
            // 保存到内存存储
            let mut files = self.files.lock().await;
            files.insert(file_id.clone(), file_info.clone());
        }
        
        Ok(Response::new(UploadFileResponse {
            file_id,
            success: true,
            message: "文件上传成功".to_string(),
            upload_session_id: None, // 传统上传不使用会话ID
        }))
    }
    
    /// 处理传统下载 (小文件或禁用hybrid模式)
    async fn handle_traditional_download(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<Self::DownloadFileStream>, Status> {
        let req = request.into_inner();
        info!("📥 处理传统下载: {}", req.file_id);
        
        // 从存储中获取文件信息
        let file_info = if let Some(vdfs) = &self.vdfs {
            // 尝试从VDFS获取
            let virtual_path = VirtualPath::new(&format!("/files/{}", req.file_id))?;
            match vdfs.get_file_info(&virtual_path).await {
                Ok(vdfs_info) => {
                    // 转换VDFS文件信息为gRPC FileInfo
                    FileInfo {
                        name: req.file_id.clone(),
                        size: vdfs_info.size,
                        file_type: FileType::File as i32,
                        permissions: Some(FilePermissions {
                            readable: true,
                            writable: true,
                            executable: false,
                        }),
                        created_at: vdfs_info.created_at.timestamp() as u64,
                        modified_at: vdfs_info.modified_at.timestamp() as u64,
                        hash: vdfs_info.hash,
                        mime_type: "application/octet-stream".to_string(),
                        encryption_info: None,
                        sync_status: SyncStatus::Synced as i32,
                        chunk_count: vdfs_info.chunk_count as u64,
                        chunk_size: vdfs_info.chunk_size,
                        replication_factor: vdfs_info.replication_factor as u32,
                    }
                }
                Err(_) => {
                    // 回退到内存存储
                    let files = self.files.lock().await;
                    files.get(&req.file_id)
                        .ok_or_else(|| Status::not_found(format!("文件未找到: {}", req.file_id)))?
                        .clone()
                }
            }
        } else {
            // 从内存存储获取
            let files = self.files.lock().await;
            files.get(&req.file_id)
                .ok_or_else(|| Status::not_found(format!("文件未找到: {}", req.file_id)))?
                .clone()
        };
        
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        // 发送文件信息
        let info_response = DownloadFileResponse {
            data: Some(crate::proto::file::download_file_response::Data::FileInfo(file_info.clone())),
        };
        
        if tx.send(Ok(info_response)).await.is_err() {
            return Err(Status::internal("无法发送文件信息"));
        }
        
        // 模拟文件数据发送 (实际实现中会从存储读取)
        let chunk_size = 8 * 1024 * 1024; // 8MB chunks
        let total_chunks = (file_info.size + chunk_size - 1) / chunk_size;
        
        tokio::spawn(async move {
            for chunk_index in 0..total_chunks {
                let chunk_start = chunk_index * chunk_size;
                let chunk_end = std::cmp::min(chunk_start + chunk_size, file_info.size);
                let chunk_size = chunk_end - chunk_start;
                
                // 生成模拟数据 (实际实现中会从存储读取)
                let chunk_data = vec![0u8; chunk_size as usize];
                
                let chunk_response = DownloadFileResponse {
                    data: Some(crate::proto::file::download_file_response::Data::Chunk(chunk_data)),
                };
                
                if tx.send(Ok(chunk_response)).await.is_err() {
                    break;
                }
                
                debug!("📦 发送数据块 {}/{} ({} bytes)", chunk_index + 1, total_chunks, chunk_size);
            }
        });
        
        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output_stream) as Self::DownloadFileStream))
    }
}

#[tonic::async_trait]
impl FileService for HybridFileService {
    type DownloadFileStream = Pin<Box<dyn Stream<Item = Result<DownloadFileResponse, Status>> + Send>>;
    
    /// 文件上传 - 支持hybrid模式
    async fn upload_file(
        &self,
        request: Request<Streaming<UploadFileRequest>>,
    ) -> Result<Response<UploadFileResponse>, Status> {
        let mut stream = request.into_inner();
        
        // 先读取第一个请求来获取文件信息
        let first_request = stream.next().await
            .ok_or_else(|| Status::invalid_argument("没有接收到上传请求"))?;
        let first_request = first_request?;
        
        if let Some(crate::proto::file::upload_file_request::Data::FileInfo(file_info)) = &first_request.data {
            // 检查是否应该使用hybrid模式
            if self.should_use_hybrid(file_info.size) {
                info!("🔀 使用hybrid模式上传: {} ({} bytes)", file_info.name, file_info.size);
                
                let file_id = self.next_file_id().await;
                let user_id = "default_user".to_string(); // TODO: 从认证信息获取
                
                let session_id = self.create_hybrid_upload_session(
                    file_id.clone(),
                    user_id,
                    file_info.name.clone(),
                    file_info.size,
                ).await?;
                
                // 启动UTP传输
                let utp_endpoint = format!("{}", self.utp_server_addr);
                self.hybrid_coordinator
                    .start_utp_transfer(&session_id, utp_endpoint)
                    .map_err(|e| Status::internal(format!("Failed to start UTP transfer: {}", e)))?;
                
                return Ok(Response::new(UploadFileResponse {
                    file_id,
                    success: true,
                    message: "Hybrid上传会话已创建，请使用UTP进行数据传输".to_string(),
                    upload_session_id: Some(session_id),
                }));
            }
        }
        
        // 重新创建stream，包含第一个请求
        let new_stream = tokio_stream::iter(vec![Ok(first_request)]).chain(stream);
        let streaming = Streaming::new(Box::pin(new_stream));
        
        // 使用传统上传方式
        self.handle_traditional_upload(streaming).await
    }
    
    /// 文件下载 - 支持hybrid模式
    async fn download_file(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<Self::DownloadFileStream>, Status> {
        let req = request.get_ref();
        info!("📥 下载文件请求: {}", req.file_id);
        
        // 获取文件信息以检查大小
        let file_info = if let Some(vdfs) = &self.vdfs {
            // 从VDFS获取文件信息的逻辑
            None // TODO: 实现VDFS文件信息获取
        } else {
            let files = self.files.lock().await;
            files.get(&req.file_id).cloned()
        };
        
        if let Some(file_info) = &file_info {
            // 检查是否应该使用hybrid模式
            if self.should_use_hybrid(file_info.size) {
                info!("🔀 使用hybrid模式下载: {} ({} bytes)", file_info.name, file_info.size);
                
                let user_id = "default_user".to_string(); // TODO: 从认证信息获取
                
                let session_id = self.create_hybrid_download_session(
                    req.file_id.clone(),
                    user_id,
                    file_info,
                ).await?;
                
                // 启动UTP传输
                let utp_endpoint = format!("{}", self.utp_server_addr);
                self.hybrid_coordinator
                    .start_utp_transfer(&session_id, utp_endpoint)
                    .map_err(|e| Status::internal(format!("Failed to start UTP transfer: {}", e)))?;
                
                // 返回包含会话信息的响应
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                
                let session_info_response = DownloadFileResponse {
                    data: Some(crate::proto::file::download_file_response::Data::FileInfo(
                        FileInfo {
                            name: format!("hybrid_session_{}", session_id),
                            size: 0, // 特殊标记，表示这是hybrid会话信息
                            ..file_info.clone()
                        }
                    )),
                };
                
                tokio::spawn(async move {
                    let _ = tx.send(Ok(session_info_response)).await;
                });
                
                let output_stream = ReceiverStream::new(rx);
                return Ok(Response::new(Box::pin(output_stream) as Self::DownloadFileStream));
            }
        }
        
        // 使用传统下载方式
        self.handle_traditional_download(request).await
    }
    
    /// 列出文件
    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        debug!("📋 列出文件: {}", req.path);
        
        // 优先从VDFS获取
        if let Some(vdfs) = &self.vdfs {
            let virtual_path = VirtualPath::new(&req.path)?;
            match vdfs.list_directory(&virtual_path).await {
                Ok(entries) => {
                    let files: Vec<FileInfo> = entries
                        .into_iter()
                        .map(|entry| FileInfo {
                            name: entry.name,
                            size: entry.size,
                            file_type: if entry.is_directory { FileType::Directory } else { FileType::File } as i32,
                            permissions: Some(FilePermissions {
                                readable: true,
                                writable: true,
                                executable: entry.is_directory,
                            }),
                            created_at: entry.created_at.timestamp() as u64,
                            modified_at: entry.modified_at.timestamp() as u64,
                            hash: entry.hash,
                            mime_type: "application/octet-stream".to_string(),
                            encryption_info: None,
                            sync_status: SyncStatus::Synced as i32,
                            chunk_count: 0,
                            chunk_size: 0,
                            replication_factor: 1,
                        })
                        .collect();
                    
                    return Ok(Response::new(ListFilesResponse { files }));
                }
                Err(e) => {
                    warn!("VDFS列表失败，回退到内存存储: {}", e);
                }
            }
        }
        
        // 回退到内存存储
        let files = self.files.lock().await;
        let file_list: Vec<FileInfo> = files.values().cloned().collect();
        
        Ok(Response::new(ListFilesResponse { files: file_list }))
    }
    
    /// 删除文件
    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        info!("🗑️ 删除文件: {}", req.file_id);
        
        // 优先从VDFS删除
        if let Some(vdfs) = &self.vdfs {
            let virtual_path = VirtualPath::new(&format!("/files/{}", req.file_id))?;
            match vdfs.delete_file(&virtual_path).await {
                Ok(_) => {
                    return Ok(Response::new(DeleteFileResponse {
                        success: true,
                        message: "文件已从VDFS删除".to_string(),
                    }));
                }
                Err(e) => {
                    warn!("VDFS删除失败，尝试内存存储: {}", e);
                }
            }
        }
        
        // 从内存存储删除
        let mut files = self.files.lock().await;
        if files.remove(&req.file_id).is_some() {
            Ok(Response::new(DeleteFileResponse {
                success: true,
                message: "文件已删除".to_string(),
            }))
        } else {
            Err(Status::not_found(format!("文件未找到: {}", req.file_id)))
        }
    }
    
    /// 创建目录
    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        let req = request.into_inner();
        info!("📁 创建目录: {}", req.path);
        
        if let Some(vdfs) = &self.vdfs {
            let virtual_path = VirtualPath::new(&req.path)?;
            match vdfs.create_directory(&virtual_path).await {
                Ok(_) => {
                    return Ok(Response::new(CreateDirectoryResponse {
                        success: true,
                        message: "目录已创建".to_string(),
                    }));
                }
                Err(e) => {
                    return Err(Status::internal(format!("创建目录失败: {}", e)));
                }
            }
        }
        
        // 内存存储不支持目录
        Err(Status::unimplemented("内存存储模式不支持目录创建"))
    }
    
    /// 获取文件信息
    async fn get_file_info(
        &self,
        request: Request<GetFileInfoRequest>,
    ) -> Result<Response<FileInfo>, Status> {
        let req = request.into_inner();
        debug!("ℹ️ 获取文件信息: {}", req.file_id);
        
        // 优先从VDFS获取
        if let Some(vdfs) = &self.vdfs {
            let virtual_path = VirtualPath::new(&format!("/files/{}", req.file_id))?;
            if let Ok(vdfs_info) = vdfs.get_file_info(&virtual_path).await {
                let file_info = FileInfo {
                    name: req.file_id.clone(),
                    size: vdfs_info.size,
                    file_type: FileType::File as i32,
                    permissions: Some(FilePermissions {
                        readable: true,
                        writable: true,
                        executable: false,
                    }),
                    created_at: vdfs_info.created_at.timestamp() as u64,
                    modified_at: vdfs_info.modified_at.timestamp() as u64,
                    hash: vdfs_info.hash,
                    mime_type: "application/octet-stream".to_string(),
                    encryption_info: None,
                    sync_status: SyncStatus::Synced as i32,
                    chunk_count: vdfs_info.chunk_count as u64,
                    chunk_size: vdfs_info.chunk_size,
                    replication_factor: vdfs_info.replication_factor as u32,
                };
                return Ok(Response::new(file_info));
            }
        }
        
        // 从内存存储获取
        let files = self.files.lock().await;
        if let Some(file_info) = files.get(&req.file_id) {
            Ok(Response::new(file_info.clone()))
        } else {
            Err(Status::not_found(format!("文件未找到: {}", req.file_id)))
        }
    }
    
    /// 获取同步状态
    async fn get_sync_status(
        &self,
        request: Request<GetSyncStatusRequest>,
    ) -> Result<Response<SyncStatusResponse>, Status> {
        let req = request.into_inner();
        debug!("🔄 获取同步状态: {}", req.file_id);
        
        // 获取所有活跃的hybrid会话
        let active_sessions = self.hybrid_coordinator.get_active_sessions();
        let hybrid_sessions: Vec<_> = active_sessions
            .into_iter()
            .filter(|session| session.file_id == req.file_id)
            .collect();
        
        let sync_status = if hybrid_sessions.is_empty() {
            SyncStatus::Synced
        } else {
            // 检查会话状态
            match hybrid_sessions[0].status {
                SessionStatus::Transferring => SyncStatus::Syncing,
                SessionStatus::Failed => SyncStatus::Failed,
                _ => SyncStatus::Pending,
            }
        };
        
        Ok(Response::new(SyncStatusResponse {
            sync_status: sync_status as i32,
            last_sync_time: chrono::Utc::now().timestamp() as u64,
            sync_message: format!("Hybrid传输状态: {:?}", sync_status),
        }))
    }
}

impl Default for HybridFileService {
    fn default() -> Self {
        Self::new("127.0.0.1:9090".parse().unwrap())
    }
}
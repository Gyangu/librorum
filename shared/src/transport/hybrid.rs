//! Hybrid Architecture: gRPC控制 + UTP数据传输
//! 
//! 这个模块实现了混合架构，其中：
//! - gRPC: 处理元数据、认证、协调、状态管理
//! - UTP: 处理实际的文件数据传输

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
// use std::time::Instant;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{UtpConfig, TransportMode, UtpResult, UtpError, UtpEvent};
use super::client::UtpClient;
use super::server::UtpServer;
// 暂时注释掉，避免循环依赖
// use crate::proto::file::{FileInfo, UploadFileRequest, DownloadFileResponse};

/// 混合传输会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSession {
    /// 会话ID
    pub session_id: String,
    /// 文件ID (来自gRPC)
    pub file_id: String,
    /// 用户ID
    pub user_id: String,
    /// 传输类型
    pub transfer_type: TransferType,
    /// 文件信息
    pub file_info: HybridFileInfo,
    /// UTP传输配置
    pub utp_config: UtpConfig,
    /// gRPC端点信息
    pub grpc_endpoint: String,
    /// 会话状态
    pub status: SessionStatus,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 传输类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferType {
    /// 上传文件
    Upload,
    /// 下载文件
    Download,
}

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// 初始化中
    Initializing,
    /// 等待gRPC协调
    AwaitingGrpcCoordination,
    /// 等待UTP连接
    AwaitingUtpConnection,
    /// 传输中
    Transferring,
    /// 传输完成
    Completed,
    /// 传输失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 混合文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridFileInfo {
    /// 文件名
    pub name: String,
    /// 文件大小
    pub size: u64,
    /// 文件哈希
    pub hash: String,
    /// MIME类型
    pub mime_type: String,
    /// 本地文件路径
    pub local_path: Option<String>,
    /// 远程文件路径
    pub remote_path: String,
    /// 加密设置
    pub encryption: Option<EncryptionInfo>,
    /// 压缩设置
    pub compression: Option<CompressionInfo>,
}

/// 加密信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionInfo {
    /// 加密算法
    pub algorithm: String,
    /// 密钥ID (不存储实际密钥)
    pub key_id: String,
    /// 初始化向量
    pub iv: Vec<u8>,
}

/// 压缩信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    /// 压缩算法
    pub algorithm: String,
    /// 压缩级别
    pub level: u8,
    /// 压缩前大小
    pub original_size: u64,
    /// 压缩后大小
    pub compressed_size: u64,
}

/// 混合传输协调器
pub struct HybridTransferCoordinator {
    /// 活跃会话
    sessions: Arc<Mutex<HashMap<String, HybridSession>>>,
    /// UTP服务器 (对于接收方)
    utp_server: Option<UtpServer>,
    /// UTP客户端缓存
    utp_clients: Arc<Mutex<HashMap<String, UtpClient>>>,
    /// 事件处理器
    event_handlers: Arc<Mutex<Vec<Box<dyn Fn(HybridEvent) + Send + Sync>>>>,
}

/// 混合传输事件
#[derive(Debug, Clone)]
pub enum HybridEvent {
    /// 会话创建
    SessionCreated {
        session_id: String,
        transfer_type: TransferType,
        file_info: HybridFileInfo,
    },
    /// gRPC协调完成
    GrpcCoordinationComplete {
        session_id: String,
        utp_endpoint: String,
    },
    /// UTP连接建立
    UtpConnectionEstablished {
        session_id: String,
        transport_mode: TransportMode,
    },
    /// 传输进度更新
    TransferProgress {
        session_id: String,
        bytes_transferred: u64,
        total_bytes: u64,
        transfer_rate: f64,
    },
    /// 传输完成
    TransferCompleted {
        session_id: String,
        success: bool,
        error_message: Option<String>,
        elapsed_secs: f64,
    },
    /// 会话状态变化
    SessionStatusChanged {
        session_id: String,
        old_status: SessionStatus,
        new_status: SessionStatus,
    },
}

impl HybridTransferCoordinator {
    /// 创建新的混合传输协调器
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            utp_server: None,
            utp_clients: Arc::new(Mutex::new(HashMap::new())),
            event_handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// 启动UTP服务器
    pub fn start_utp_server(&mut self, bind_addr: SocketAddr) -> UtpResult<()> {
        let mut server = UtpServer::new(bind_addr)?;
        
        // 设置服务器事件处理
        let event_handlers = Arc::clone(&self.event_handlers);
        server.set_event_handler(move |event| {
            let handlers = event_handlers.lock().unwrap();
            for handler in handlers.iter() {
                // 将UTP事件转换为Hybrid事件
                match &event {
                    UtpEvent::TransferStarted { session_id: _, total_size: _ } => {
                        // 这里需要查找会话信息来构造HybridEvent
                    }
                    UtpEvent::TransferProgress { session_id, bytes_transferred, total_size, transfer_rate } => {
                        handler(HybridEvent::TransferProgress {
                            session_id: session_id.clone(),
                            bytes_transferred: *bytes_transferred,
                            total_bytes: *total_size,
                            transfer_rate: *transfer_rate,
                        });
                    }
                    UtpEvent::TransferCompleted { session_id, total_bytes: _, elapsed_secs } => {
                        handler(HybridEvent::TransferCompleted {
                            session_id: session_id.clone(),
                            success: true,
                            error_message: None,
                            elapsed_secs: *elapsed_secs,
                        });
                    }
                    UtpEvent::TransferFailed { session_id, error } => {
                        handler(HybridEvent::TransferCompleted {
                            session_id: session_id.clone(),
                            success: false,
                            error_message: Some(error.clone()),
                            elapsed_secs: 0.0,
                        });
                    }
                    _ => {}
                }
            }
        });
        
        server.start()?;
        self.utp_server = Some(server);
        Ok(())
    }
    
    /// 添加事件处理器
    pub fn add_event_handler<F>(&self, handler: F)
    where
        F: Fn(HybridEvent) + Send + Sync + 'static,
    {
        self.event_handlers.lock().unwrap().push(Box::new(handler));
    }
    
    /// 初始化上传会话
    pub fn initiate_upload(
        &self,
        file_id: String,
        user_id: String,
        local_path: String,
        remote_path: String,
        grpc_endpoint: String,
    ) -> UtpResult<String> {
        let session_id = Uuid::new_v4().to_string();
        
        // 获取文件信息
        let metadata = std::fs::metadata(&local_path)
            .map_err(|e| UtpError::IoError(format!("Failed to get file metadata: {}", e)))?;
        
        let file_size = metadata.len();
        let file_name = std::path::Path::new(&local_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        // 计算文件哈希 (简化实现)
        let file_hash = super::utils::UtpUtils::calculate_file_hash(&local_path)?;
        let mime_type = super::utils::UtpUtils::detect_mime_type(&local_path);
        
        let file_info = HybridFileInfo {
            name: file_name,
            size: file_size,
            hash: file_hash,
            mime_type,
            local_path: Some(local_path),
            remote_path,
            encryption: None,
            compression: None,
        };
        
        let utp_config = UtpConfig {
            mode: TransportMode::Auto,
            bind_addr: None,
            target_addr: None, // 将由gRPC协调确定
            shared_memory_size: Some(64 * 1024 * 1024),
            shared_memory_path: Some(format!("/tmp/librorum_hybrid_{}", session_id)),
            enable_compression: true,
            enable_encryption: false,
            chunk_size: super::utils::UtpUtils::calculate_optimal_chunk_size(file_size),
            timeout_secs: 30,
        };
        
        let session = HybridSession {
            session_id: session_id.clone(),
            file_id,
            user_id,
            transfer_type: TransferType::Upload,
            file_info: file_info.clone(),
            utp_config,
            grpc_endpoint,
            status: SessionStatus::Initializing,
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        };
        
        // 存储会话
        self.sessions.lock().unwrap().insert(session_id.clone(), session);
        
        // 触发事件
        let handlers = self.event_handlers.lock().unwrap();
        for handler in handlers.iter() {
            handler(HybridEvent::SessionCreated {
                session_id: session_id.clone(),
                transfer_type: TransferType::Upload,
                file_info: file_info.clone(),
            });
        }
        
        Ok(session_id)
    }
    
    /// 初始化下载会话
    pub fn initiate_download(
        &self,
        file_id: String,
        user_id: String,
        local_path: String,
        remote_path: String,
        expected_size: u64,
        grpc_endpoint: String,
    ) -> UtpResult<String> {
        let session_id = Uuid::new_v4().to_string();
        
        let file_name = std::path::Path::new(&remote_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let file_info = HybridFileInfo {
            name: file_name,
            size: expected_size,
            hash: "".to_string(), // 将在传输过程中验证
            mime_type: super::utils::UtpUtils::detect_mime_type(&remote_path),
            local_path: Some(local_path),
            remote_path,
            encryption: None,
            compression: None,
        };
        
        let utp_config = UtpConfig {
            mode: TransportMode::Auto,
            bind_addr: None,
            target_addr: None, // 将由gRPC协调确定
            shared_memory_size: Some(64 * 1024 * 1024),
            shared_memory_path: Some(format!("/tmp/librorum_hybrid_{}", session_id)),
            enable_compression: true,
            enable_encryption: false,
            chunk_size: super::utils::UtpUtils::calculate_optimal_chunk_size(expected_size),
            timeout_secs: 30,
        };
        
        let session = HybridSession {
            session_id: session_id.clone(),
            file_id,
            user_id,
            transfer_type: TransferType::Download,
            file_info: file_info.clone(),
            utp_config,
            grpc_endpoint,
            status: SessionStatus::Initializing,
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        };
        
        // 存储会话
        self.sessions.lock().unwrap().insert(session_id.clone(), session);
        
        // 触发事件
        let handlers = self.event_handlers.lock().unwrap();
        for handler in handlers.iter() {
            handler(HybridEvent::SessionCreated {
                session_id: session_id.clone(),
                transfer_type: TransferType::Download,
                file_info: file_info.clone(),
            });
        }
        
        Ok(session_id)
    }
    
    /// gRPC协调完成，开始UTP传输
    pub fn start_utp_transfer(&self, session_id: &str, utp_endpoint: String) -> UtpResult<()> {
        let session = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions.get_mut(session_id)
                .ok_or_else(|| UtpError::ProtocolError(format!("Session not found: {}", session_id)))?;
            
            // 更新会话状态
            self.update_session_status(session, SessionStatus::AwaitingUtpConnection);
            session.clone()
        };
        
        // 触发gRPC协调完成事件
        let handlers = self.event_handlers.lock().unwrap();
        for handler in handlers.iter() {
            handler(HybridEvent::GrpcCoordinationComplete {
                session_id: session_id.to_string(),
                utp_endpoint: utp_endpoint.clone(),
            });
        }
        
        // 解析UTP端点
        let target_addr: SocketAddr = utp_endpoint.parse()
            .map_err(|e| UtpError::NetworkError(format!("Invalid UTP endpoint: {}", e)))?;
        
        // 创建UTP客户端
        let mut utp_client = if super::utils::UtpUtils::is_local_address(&target_addr.ip()) {
            UtpClient::new_local()?
        } else {
            UtpClient::new(target_addr)?
        };
        
        // 设置UTP事件处理
        let session_id_clone = session_id.to_string();
        let event_handlers = Arc::clone(&self.event_handlers);
        utp_client.set_event_handler(move |event| {
            let handlers = event_handlers.lock().unwrap();
            for handler in handlers.iter() {
                match &event {
                    UtpEvent::ConnectionEstablished { mode, .. } => {
                        handler(HybridEvent::UtpConnectionEstablished {
                            session_id: session_id_clone.clone(),
                            transport_mode: *mode,
                        });
                    }
                    UtpEvent::TransferProgress { bytes_transferred, total_size, transfer_rate, .. } => {
                        handler(HybridEvent::TransferProgress {
                            session_id: session_id_clone.clone(),
                            bytes_transferred: *bytes_transferred,
                            total_bytes: *total_size,
                            transfer_rate: *transfer_rate,
                        });
                    }
                    UtpEvent::TransferCompleted { total_bytes: _, elapsed_secs, .. } => {
                        handler(HybridEvent::TransferCompleted {
                            session_id: session_id_clone.clone(),
                            success: true,
                            error_message: None,
                            elapsed_secs: *elapsed_secs,
                        });
                    }
                    UtpEvent::TransferFailed { error, .. } => {
                        handler(HybridEvent::TransferCompleted {
                            session_id: session_id_clone.clone(),
                            success: false,
                            error_message: Some(error.clone()),
                            elapsed_secs: 0.0,
                        });
                    }
                    _ => {}
                }
            }
        });
        
        // 连接并开始传输
        utp_client.connect()?;
        
        match session.transfer_type {
            TransferType::Upload => {
                let local_path = session.file_info.local_path
                    .ok_or_else(|| UtpError::ProtocolError("No local path for upload".to_string()))?;
                utp_client.upload_file(&local_path, &session.file_info.remote_path)?;
            }
            TransferType::Download => {
                let local_path = session.file_info.local_path
                    .ok_or_else(|| UtpError::ProtocolError("No local path for download".to_string()))?;
                utp_client.download_file(&session.file_info.remote_path, &local_path, session.file_info.size)?;
            }
        }
        
        // 缓存客户端
        self.utp_clients.lock().unwrap().insert(session_id.to_string(), utp_client);
        
        Ok(())
    }
    
    /// 更新会话状态
    fn update_session_status(&self, session: &mut HybridSession, new_status: SessionStatus) {
        let old_status = session.status;
        session.status = new_status;
        session.updated_at = chrono::Utc::now().timestamp() as u64;
        
        // 触发状态变化事件
        let handlers = self.event_handlers.lock().unwrap();
        for handler in handlers.iter() {
            handler(HybridEvent::SessionStatusChanged {
                session_id: session.session_id.clone(),
                old_status,
                new_status,
            });
        }
    }
    
    /// 获取会话信息
    pub fn get_session(&self, session_id: &str) -> Option<HybridSession> {
        self.sessions.lock().unwrap().get(session_id).cloned()
    }
    
    /// 取消会话
    pub fn cancel_session(&self, session_id: &str) -> UtpResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            self.update_session_status(session, SessionStatus::Cancelled);
        }
        
        // 关闭UTP客户端
        if let Some(client) = self.utp_clients.lock().unwrap().remove(session_id) {
            client.disconnect()?;
        }
        
        Ok(())
    }
    
    /// 获取所有活跃会话
    pub fn get_active_sessions(&self) -> Vec<HybridSession> {
        self.sessions.lock().unwrap()
            .values()
            .filter(|session| matches!(session.status, 
                SessionStatus::Initializing | 
                SessionStatus::AwaitingGrpcCoordination | 
                SessionStatus::AwaitingUtpConnection | 
                SessionStatus::Transferring
            ))
            .cloned()
            .collect()
    }
    
    /// 清理完成的会话
    pub fn cleanup_completed_sessions(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        let completed_sessions: Vec<String> = sessions.iter()
            .filter(|(_, session)| matches!(session.status, 
                SessionStatus::Completed | 
                SessionStatus::Failed | 
                SessionStatus::Cancelled
            ))
            .map(|(id, _)| id.clone())
            .collect();
        
        for session_id in completed_sessions {
            sessions.remove(&session_id);
            self.utp_clients.lock().unwrap().remove(&session_id);
        }
    }
}

impl Default for HybridTransferCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
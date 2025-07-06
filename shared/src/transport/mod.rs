//! Universal Transport Protocol (UTP) - 高性能文件传输
//! 
//! 这个模块提供了一个封装好的UTP实现，用于librorum的文件传输。
//! 架构设计：
//! - gRPC: 控制平面 (元数据、认证、协调)
//! - UTP: 数据平面 (实际文件数据传输)

pub mod protocol;
pub mod server;
pub mod client;
pub mod shared_memory;
pub mod network;
pub mod utils;
pub mod hybrid;

use std::fmt;
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

/// UTP传输错误类型
#[derive(Debug, Clone)]
pub enum UtpError {
    /// 网络连接错误
    NetworkError(String),
    /// 协议错误
    ProtocolError(String),
    /// IO错误
    IoError(String),
    /// 内存映射错误
    MemoryMapError(String),
    /// 数据校验错误
    ChecksumError(String),
    /// 超时错误
    TimeoutError(String),
}

impl fmt::Display for UtpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UtpError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            UtpError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            UtpError::IoError(msg) => write!(f, "IO error: {}", msg),
            UtpError::MemoryMapError(msg) => write!(f, "Memory map error: {}", msg),
            UtpError::ChecksumError(msg) => write!(f, "Checksum error: {}", msg),
            UtpError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
        }
    }
}

impl std::error::Error for UtpError {}

/// UTP传输结果
pub type UtpResult<T> = Result<T, UtpError>;

/// UTP传输模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportMode {
    /// TCP网络传输 (跨设备)
    Network,
    /// POSIX共享内存 (同设备进程间)
    SharedMemory,
    /// 自动选择 (根据目标地址自动选择)
    Auto,
}

/// UTP传输配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtpConfig {
    /// 传输模式
    pub mode: TransportMode,
    /// 监听地址 (仅网络模式)
    pub bind_addr: Option<SocketAddr>,
    /// 目标地址 (仅网络模式)
    pub target_addr: Option<SocketAddr>,
    /// 共享内存大小 (仅内存模式)
    pub shared_memory_size: Option<usize>,
    /// 共享内存路径 (仅内存模式)
    pub shared_memory_path: Option<String>,
    /// 启用压缩
    pub enable_compression: bool,
    /// 启用加密
    pub enable_encryption: bool,
    /// 块大小 (字节)
    pub chunk_size: usize,
    /// 超时时间 (秒)
    pub timeout_secs: u64,
}

impl Default for UtpConfig {
    fn default() -> Self {
        Self {
            mode: TransportMode::Auto,
            bind_addr: None,
            target_addr: None,
            shared_memory_size: Some(64 * 1024 * 1024), // 64MB
            shared_memory_path: Some("/tmp/librorum_utp".to_string()),
            enable_compression: true,
            enable_encryption: false,
            chunk_size: 8 * 1024 * 1024, // 8MB
            timeout_secs: 30,
        }
    }
}

/// UTP文件传输会话
#[derive(Debug)]
pub struct UtpSession {
    /// 会话ID
    pub session_id: String,
    /// 传输模式
    pub mode: TransportMode,
    /// 总文件大小
    pub total_size: u64,
    /// 已传输字节数
    pub transferred_bytes: u64,
    /// 传输速率 (字节/秒)
    pub transfer_rate: f64,
    /// 开始时间
    pub start_time: std::time::Instant,
}

impl UtpSession {
    /// 创建新的传输会话
    pub fn new(session_id: String, mode: TransportMode, total_size: u64) -> Self {
        Self {
            session_id,
            mode,
            total_size,
            transferred_bytes: 0,
            transfer_rate: 0.0,
            start_time: std::time::Instant::now(),
        }
    }
    
    /// 更新传输进度
    pub fn update_progress(&mut self, bytes_transferred: u64) {
        self.transferred_bytes = bytes_transferred;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.transfer_rate = self.transferred_bytes as f64 / elapsed;
        }
    }
    
    /// 获取传输进度百分比
    pub fn progress_percent(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            (self.transferred_bytes as f64 / self.total_size as f64) * 100.0
        }
    }
    
    /// 获取剩余时间估计 (秒)
    pub fn estimated_time_remaining(&self) -> Option<f64> {
        if self.transfer_rate <= 0.0 {
            return None;
        }
        
        let remaining_bytes = self.total_size.saturating_sub(self.transferred_bytes);
        Some(remaining_bytes as f64 / self.transfer_rate)
    }
}

/// UTP传输事件
#[derive(Debug, Clone)]
pub enum UtpEvent {
    /// 传输开始
    TransferStarted {
        session_id: String,
        total_size: u64,
    },
    /// 传输进度更新
    TransferProgress {
        session_id: String,
        bytes_transferred: u64,
        total_size: u64,
        transfer_rate: f64,
    },
    /// 传输完成
    TransferCompleted {
        session_id: String,
        total_bytes: u64,
        elapsed_secs: f64,
    },
    /// 传输失败
    TransferFailed {
        session_id: String,
        error: String,
    },
    /// 连接建立
    ConnectionEstablished {
        session_id: String,
        mode: TransportMode,
    },
    /// 连接断开
    ConnectionClosed {
        session_id: String,
        reason: String,
    },
}

/// UTP传输统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtpStats {
    /// 总传输会话数
    pub total_sessions: u64,
    /// 成功传输数
    pub successful_transfers: u64,
    /// 失败传输数
    pub failed_transfers: u64,
    /// 总传输字节数
    pub total_bytes_transferred: u64,
    /// 平均传输速率 (字节/秒)
    pub average_transfer_rate: f64,
    /// 最大传输速率 (字节/秒)
    pub max_transfer_rate: f64,
    /// 网络模式使用次数
    pub network_mode_usage: u64,
    /// 共享内存模式使用次数
    pub shared_memory_mode_usage: u64,
}

impl Default for UtpStats {
    fn default() -> Self {
        Self {
            total_sessions: 0,
            successful_transfers: 0,
            failed_transfers: 0,
            total_bytes_transferred: 0,
            average_transfer_rate: 0.0,
            max_transfer_rate: 0.0,
            network_mode_usage: 0,
            shared_memory_mode_usage: 0,
        }
    }
}

/// UTP传输事件回调
pub type UtpEventCallback = Box<dyn Fn(UtpEvent) + Send + Sync>;

/// UTP传输特性
pub trait UtpTransport: Send + Sync {
    /// 发送文件
    fn send_file(&self, file_path: &str, session_id: &str) -> UtpResult<()>;
    
    /// 接收文件
    fn receive_file(&self, file_path: &str, session_id: &str) -> UtpResult<()>;
    
    /// 发送数据块
    fn send_chunk(&self, data: &[u8], session_id: &str) -> UtpResult<()>;
    
    /// 接收数据块
    fn receive_chunk(&self, session_id: &str) -> UtpResult<Vec<u8>>;
    
    /// 设置事件回调
    fn set_event_callback(&self, callback: UtpEventCallback);
    
    /// 获取传输统计
    fn get_stats(&self) -> UtpStats;
    
    /// 关闭连接
    fn close(&self) -> UtpResult<()>;
}

/// UTP传输工厂
pub struct UtpTransportFactory;

impl UtpTransportFactory {
    /// 创建传输实例
    pub fn create(config: UtpConfig) -> UtpResult<Box<dyn UtpTransport>> {
        match config.mode {
            TransportMode::Network => {
                network::NetworkTransport::new(config)
                    .map(|t| Box::new(t) as Box<dyn UtpTransport>)
            }
            TransportMode::SharedMemory => {
                shared_memory::SharedMemoryTransport::new(config)
                    .map(|t| Box::new(t) as Box<dyn UtpTransport>)
            }
            TransportMode::Auto => {
                // 根据配置自动选择
                if config.target_addr.is_some() {
                    network::NetworkTransport::new(config)
                        .map(|t| Box::new(t) as Box<dyn UtpTransport>)
                } else {
                    shared_memory::SharedMemoryTransport::new(config)
                        .map(|t| Box::new(t) as Box<dyn UtpTransport>)
                }
            }
        }
    }
}

/// UTP传输管理器
pub struct UtpManager {
    /// 传输实例
    transport: Box<dyn UtpTransport>,
    /// 活跃会话
    active_sessions: std::collections::HashMap<String, UtpSession>,
    /// 事件回调
    event_callback: Option<UtpEventCallback>,
}

impl UtpManager {
    /// 创建新的UTP管理器
    pub fn new(config: UtpConfig) -> UtpResult<Self> {
        let transport = UtpTransportFactory::create(config)?;
        
        Ok(Self {
            transport,
            active_sessions: std::collections::HashMap::new(),
            event_callback: None,
        })
    }
    
    /// 设置事件回调
    pub fn set_event_callback(&mut self, callback: UtpEventCallback) {
        // 注意：这里有架构问题，暂时只设置内部事件回调
        self.event_callback = Some(callback);
        // TODO: 需要重新设计事件回调架构
        // self.transport.set_event_callback(callback);
    }
    
    /// 开始文件传输
    pub fn start_file_transfer(&mut self, file_path: &str, session_id: &str, total_size: u64) -> UtpResult<()> {
        let session = UtpSession::new(
            session_id.to_string(),
            TransportMode::Auto, // 实际模式由传输实例决定
            total_size,
        );
        
        self.active_sessions.insert(session_id.to_string(), session);
        
        if let Some(callback) = &self.event_callback {
            callback(UtpEvent::TransferStarted {
                session_id: session_id.to_string(),
                total_size,
            });
        }
        
        self.transport.send_file(file_path, session_id)
    }
    
    /// 接收文件传输
    pub fn receive_file_transfer(&mut self, file_path: &str, session_id: &str, total_size: u64) -> UtpResult<()> {
        let session = UtpSession::new(
            session_id.to_string(),
            TransportMode::Auto,
            total_size,
        );
        
        self.active_sessions.insert(session_id.to_string(), session);
        
        if let Some(callback) = &self.event_callback {
            callback(UtpEvent::TransferStarted {
                session_id: session_id.to_string(),
                total_size,
            });
        }
        
        self.transport.receive_file(file_path, session_id)
    }
    
    /// 获取会话状态
    pub fn get_session(&self, session_id: &str) -> Option<&UtpSession> {
        self.active_sessions.get(session_id)
    }
    
    /// 获取传输统计
    pub fn get_stats(&self) -> UtpStats {
        self.transport.get_stats()
    }
    
    /// 关闭管理器
    pub fn close(&self) -> UtpResult<()> {
        self.transport.close()
    }
}
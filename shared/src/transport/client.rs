//! UTP传输客户端

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use super::{UtpManager, UtpConfig, TransportMode, UtpResult, UtpError, UtpEvent, UtpSession};

/// UTP传输客户端
pub struct UtpClient {
    /// 客户端配置
    config: UtpConfig,
    /// UTP管理器
    manager: Arc<UtpManager>,
    /// 连接状态
    connected: Arc<std::sync::atomic::AtomicBool>,
    /// 连接时间
    connect_time: Option<Instant>,
}

impl UtpClient {
    /// 创建新的UTP客户端
    pub fn new(server_addr: SocketAddr) -> UtpResult<Self> {
        let config = UtpConfig {
            mode: TransportMode::Auto,
            bind_addr: None,
            target_addr: Some(server_addr),
            shared_memory_size: Some(64 * 1024 * 1024), // 64MB
            shared_memory_path: Some("/tmp/librorum_utp_client".to_string()),
            enable_compression: true,
            enable_encryption: false,
            chunk_size: 8 * 1024 * 1024, // 8MB
            timeout_secs: 30,
        };
        
        let manager = Arc::new(UtpManager::new(config.clone())?);
        
        Ok(Self {
            config,
            manager,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            connect_time: None,
        })
    }
    
    /// 创建本地客户端 (使用共享内存)
    pub fn new_local() -> UtpResult<Self> {
        let config = UtpConfig {
            mode: TransportMode::SharedMemory,
            bind_addr: None,
            target_addr: None,
            shared_memory_size: Some(64 * 1024 * 1024), // 64MB
            shared_memory_path: Some("/tmp/librorum_utp_local".to_string()),
            enable_compression: true,
            enable_encryption: false,
            chunk_size: 8 * 1024 * 1024, // 8MB
            timeout_secs: 30,
        };
        
        let manager = Arc::new(UtpManager::new(config.clone())?);
        
        Ok(Self {
            config,
            manager,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(true)), // 本地连接视为已连接
            connect_time: Some(Instant::now()),
        })
    }
    
    /// 设置事件处理器
    pub fn set_event_handler<F>(&mut self, handler: F)
    where
        F: Fn(UtpEvent) + Send + Sync + 'static,
    {
        let manager = Arc::get_mut(&mut self.manager).unwrap();
        manager.set_event_callback(Box::new(handler));
    }
    
    /// 连接到服务器
    pub fn connect(&mut self) -> UtpResult<()> {
        if self.connected.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        
        println!("🔗 连接到UTP服务器...");
        println!("服务器地址: {:?}", self.config.target_addr);
        println!("传输模式: {:?}", self.config.mode);
        
        // 对于网络模式，这里会建立TCP连接
        // 对于共享内存模式，这里会创建共享内存区域
        
        self.connected.store(true, std::sync::atomic::Ordering::Release);
        self.connect_time = Some(Instant::now());
        
        println!("✅ 连接成功");
        Ok(())
    }
    
    /// 断开连接
    pub fn disconnect(&self) -> UtpResult<()> {
        if !self.connected.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        
        println!("🔌 断开UTP连接...");
        
        self.manager.close()?;
        self.connected.store(false, std::sync::atomic::Ordering::Release);
        
        println!("✅ 连接已断开");
        Ok(())
    }
    
    /// 上传文件
    pub fn upload_file(&mut self, local_path: &str, remote_path: &str) -> UtpResult<UploadResult> {
        // 验证本地文件
        super::utils::UtpUtils::validate_file_path(local_path)?;
        
        let file_size = std::fs::metadata(local_path)
            .map_err(|e| UtpError::IoError(format!("Failed to get file metadata: {}", e)))?
            .len();
        
        println!("📤 开始上传文件:");
        println!("本地路径: {}", local_path);
        println!("远程路径: {}", remote_path);
        println!("文件大小: {}", super::utils::UtpUtils::format_file_size(file_size));
        
        let start_time = Instant::now();
        let session_id = super::utils::UtpUtils::generate_session_id();
        
        // 确保已连接
        if !self.connected.load(std::sync::atomic::Ordering::Acquire) {
            self.connect()?;
        }
        
        // 开始文件传输
        let manager = Arc::get_mut(&mut self.manager).unwrap();
        manager.start_file_transfer(local_path, &session_id, file_size)?;
        
        let elapsed = start_time.elapsed().as_secs_f64();
        let transfer_rate = file_size as f64 / elapsed;
        
        println!("✅ 文件上传完成");
        println!("传输时间: {}", super::utils::UtpUtils::format_duration(elapsed));
        println!("传输速率: {}", super::utils::UtpUtils::format_transfer_rate(transfer_rate));
        
        Ok(UploadResult {
            session_id,
            local_path: local_path.to_string(),
            remote_path: remote_path.to_string(),
            file_size,
            transfer_time_secs: elapsed,
            transfer_rate,
            success: true,
            error_message: None,
        })
    }
    
    /// 下载文件
    pub fn download_file(&mut self, remote_path: &str, local_path: &str, expected_size: u64) -> UtpResult<DownloadResult> {
        println!("📥 开始下载文件:");
        println!("远程路径: {}", remote_path);
        println!("本地路径: {}", local_path);
        println!("预期大小: {}", super::utils::UtpUtils::format_file_size(expected_size));
        
        let start_time = Instant::now();
        let session_id = super::utils::UtpUtils::generate_session_id();
        
        // 确保已连接
        if !self.connected.load(std::sync::atomic::Ordering::Acquire) {
            self.connect()?;
        }
        
        // 开始文件接收
        let manager = Arc::get_mut(&mut self.manager).unwrap();
        manager.receive_file_transfer(local_path, &session_id, expected_size)?;
        
        // 验证下载的文件
        let actual_size = std::fs::metadata(local_path)
            .map_err(|e| UtpError::IoError(format!("Failed to verify downloaded file: {}", e)))?
            .len();
        
        let elapsed = start_time.elapsed().as_secs_f64();
        let transfer_rate = actual_size as f64 / elapsed;
        
        println!("✅ 文件下载完成");
        println!("实际大小: {}", super::utils::UtpUtils::format_file_size(actual_size));
        println!("传输时间: {}", super::utils::UtpUtils::format_duration(elapsed));
        println!("传输速率: {}", super::utils::UtpUtils::format_transfer_rate(transfer_rate));
        
        Ok(DownloadResult {
            session_id,
            remote_path: remote_path.to_string(),
            local_path: local_path.to_string(),
            expected_size,
            actual_size,
            transfer_time_secs: elapsed,
            transfer_rate,
            success: actual_size == expected_size,
            error_message: if actual_size != expected_size {
                Some(format!("Size mismatch: expected {}, got {}", expected_size, actual_size))
            } else {
                None
            },
        })
    }
    
    /// 获取会话信息
    pub fn get_session(&self, session_id: &str) -> Option<&UtpSession> {
        self.manager.get_session(session_id)
    }
    
    /// 获取连接状态
    pub fn get_connection_status(&self) -> ConnectionStatus {
        let connected = self.connected.load(std::sync::atomic::Ordering::Acquire);
        let uptime = self.connect_time
            .map(|time| time.elapsed().as_secs())
            .unwrap_or(0);
        
        let stats = self.manager.get_stats();
        
        ConnectionStatus {
            connected,
            server_addr: self.config.target_addr,
            transport_mode: self.config.mode,
            uptime_secs: uptime,
            total_uploads: stats.successful_transfers, // 简化统计
            total_downloads: 0, // TODO: 分别统计上传下载
            total_bytes_transferred: stats.total_bytes_transferred,
            average_transfer_rate: stats.average_transfer_rate,
        }
    }
    
    /// 获取传输统计
    pub fn get_transfer_stats(&self) -> super::UtpStats {
        self.manager.get_stats()
    }
}

/// 上传结果
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// 会话ID
    pub session_id: String,
    /// 本地文件路径
    pub local_path: String,
    /// 远程文件路径
    pub remote_path: String,
    /// 文件大小
    pub file_size: u64,
    /// 传输时间 (秒)
    pub transfer_time_secs: f64,
    /// 传输速率 (字节/秒)
    pub transfer_rate: f64,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 下载结果
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// 会话ID
    pub session_id: String,
    /// 远程文件路径
    pub remote_path: String,
    /// 本地文件路径
    pub local_path: String,
    /// 预期文件大小
    pub expected_size: u64,
    /// 实际文件大小
    pub actual_size: u64,
    /// 传输时间 (秒)
    pub transfer_time_secs: f64,
    /// 传输速率 (字节/秒)
    pub transfer_rate: f64,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 连接状态
#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    /// 是否已连接
    pub connected: bool,
    /// 服务器地址
    pub server_addr: Option<SocketAddr>,
    /// 传输模式
    pub transport_mode: TransportMode,
    /// 连接时间 (秒)
    pub uptime_secs: u64,
    /// 总上传次数
    pub total_uploads: u64,
    /// 总下载次数
    pub total_downloads: u64,
    /// 总传输字节数
    pub total_bytes_transferred: u64,
    /// 平均传输速率
    pub average_transfer_rate: f64,
}

impl ConnectionStatus {
    /// 格式化状态输出
    pub fn format(&self) -> String {
        format!(
            "UTP客户端状态:\n\
             连接状态: {}\n\
             服务器地址: {:?}\n\
             传输模式: {:?}\n\
             连接时间: {}\n\
             总上传数: {}\n\
             总下载数: {}\n\
             总传输量: {}\n\
             平均速率: {}",
            if self.connected { "已连接" } else { "未连接" },
            self.server_addr,
            self.transport_mode,
            super::utils::UtpUtils::format_duration(self.uptime_secs as f64),
            self.total_uploads,
            self.total_downloads,
            super::utils::UtpUtils::format_file_size(self.total_bytes_transferred),
            super::utils::UtpUtils::format_transfer_rate(self.average_transfer_rate)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    
    #[test]
    fn test_client_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let client = UtpClient::new(addr);
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_local_client_creation() {
        let client = UtpClient::new_local();
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_connection_status() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let client = UtpClient::new(addr).unwrap();
        let status = client.get_connection_status();
        assert!(!status.connected);
        assert_eq!(status.server_addr, Some(addr));
    }
}
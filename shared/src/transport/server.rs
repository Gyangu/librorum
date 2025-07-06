//! UTP传输服务器

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::{UtpManager, UtpConfig, TransportMode, UtpResult, UtpError, UtpEvent};

/// UTP传输服务器
pub struct UtpServer {
    /// 服务器配置
    config: UtpConfig,
    /// UTP管理器
    manager: Arc<UtpManager>,
    /// 是否运行中
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl UtpServer {
    /// 创建新的UTP服务器
    pub fn new(bind_addr: SocketAddr) -> UtpResult<Self> {
        let config = UtpConfig {
            mode: TransportMode::Auto,
            bind_addr: Some(bind_addr),
            target_addr: None,
            shared_memory_size: Some(64 * 1024 * 1024), // 64MB
            shared_memory_path: Some("/tmp/librorum_utp_server".to_string()),
            enable_compression: true,
            enable_encryption: false,
            chunk_size: 8 * 1024 * 1024, // 8MB
            timeout_secs: 30,
        };
        
        let manager = Arc::new(UtpManager::new(config.clone())?);
        
        Ok(Self {
            config,
            manager,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
    
    /// 启动服务器
    pub fn start(&self) -> UtpResult<()> {
        if self.running.load(std::sync::atomic::Ordering::Acquire) {
            return Err(UtpError::NetworkError("Server already running".to_string()));
        }
        
        self.running.store(true, std::sync::atomic::Ordering::Release);
        
        println!("🚀 UTP服务器启动中...");
        println!("监听地址: {:?}", self.config.bind_addr);
        println!("传输模式: {:?}", self.config.mode);
        
        // 启动服务器监听循环
        let manager = Arc::clone(&self.manager);
        let running = Arc::clone(&self.running);
        
        thread::spawn(move || {
            while running.load(std::sync::atomic::Ordering::Acquire) {
                // 这里可以处理新连接请求
                // 实际实现会根据具体的网络层架构来决定
                
                thread::sleep(Duration::from_millis(100));
            }
        });
        
        println!("✅ UTP服务器启动成功");
        Ok(())
    }
    
    /// 停止服务器
    pub fn stop(&self) -> UtpResult<()> {
        if !self.running.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        
        println!("🛑 UTP服务器停止中...");
        
        self.running.store(false, std::sync::atomic::Ordering::Release);
        self.manager.close()?;
        
        println!("✅ UTP服务器已停止");
        Ok(())
    }
    
    /// 处理文件上传请求
    pub fn handle_upload_request(&self, file_path: &str, session_id: &str, file_size: u64) -> UtpResult<()> {
        println!("📤 处理文件上传请求: {} ({})", file_path, super::utils::UtpUtils::format_file_size(file_size));
        
        // 这里可以添加权限检查、文件验证等逻辑
        
        // 实际的文件接收会通过传输层处理
        // self.manager.receive_file_transfer(file_path, session_id, file_size)
        
        Ok(())
    }
    
    /// 处理文件下载请求
    pub fn handle_download_request(&self, file_path: &str, session_id: &str) -> UtpResult<()> {
        println!("📥 处理文件下载请求: {}", file_path);
        
        // 验证文件是否存在
        super::utils::UtpUtils::validate_file_path(file_path)?;
        
        // 获取文件信息
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| UtpError::IoError(format!("Failed to get file metadata: {}", e)))?
            .len();
        
        println!("文件大小: {}", super::utils::UtpUtils::format_file_size(file_size));
        
        // 实际的文件发送会通过传输层处理
        // self.manager.start_file_transfer(file_path, session_id, file_size)
        
        Ok(())
    }
    
    /// 获取服务器状态
    pub fn get_status(&self) -> ServerStatus {
        let stats = self.manager.get_stats();
        
        ServerStatus {
            running: self.running.load(std::sync::atomic::Ordering::Acquire),
            bind_addr: self.config.bind_addr,
            total_sessions: stats.total_sessions,
            active_transfers: 0, // TODO: 从manager获取活跃传输数
            total_bytes_transferred: stats.total_bytes_transferred,
            average_transfer_rate: stats.average_transfer_rate,
            uptime_secs: 0, // TODO: 记录启动时间
        }
    }
    
    /// 获取传输统计
    pub fn get_transfer_stats(&self) -> super::UtpStats {
        self.manager.get_stats()
    }
}

/// 服务器状态
#[derive(Debug, Clone)]
pub struct ServerStatus {
    /// 是否运行中
    pub running: bool,
    /// 监听地址
    pub bind_addr: Option<SocketAddr>,
    /// 总会话数
    pub total_sessions: u64,
    /// 活跃传输数
    pub active_transfers: u64,
    /// 总传输字节数
    pub total_bytes_transferred: u64,
    /// 平均传输速率
    pub average_transfer_rate: f64,
    /// 运行时间 (秒)
    pub uptime_secs: u64,
}

impl ServerStatus {
    /// 格式化状态输出
    pub fn format(&self) -> String {
        format!(
            "UTP服务器状态:\n\
             运行状态: {}\n\
             监听地址: {:?}\n\
             总会话数: {}\n\
             活跃传输: {}\n\
             总传输量: {}\n\
             平均速率: {}\n\
             运行时间: {}",
            if self.running { "运行中" } else { "已停止" },
            self.bind_addr,
            self.total_sessions,
            self.active_transfers,
            super::utils::UtpUtils::format_file_size(self.total_bytes_transferred),
            super::utils::UtpUtils::format_transfer_rate(self.average_transfer_rate),
            super::utils::UtpUtils::format_duration(self.uptime_secs as f64)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    
    #[test]
    fn test_server_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let server = UtpServer::new(addr);
        assert!(server.is_ok());
    }
    
    #[test]
    fn test_server_status() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let server = UtpServer::new(addr).unwrap();
        let status = server.get_status();
        assert!(!status.running);
        assert_eq!(status.bind_addr, Some(addr));
    }
}
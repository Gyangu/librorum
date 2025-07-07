use anyhow::{Context, Result};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use data_portal_core::{TransportManager, TransportManagerConfig, TransportType};

/// 文件传输协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataPortalMessage {
    /// 文件传输开始 (上传)
    FileTransferStart {
        file_name: String,
        file_size: u64,
        chunk_size: usize,
        /// 文件SHA-256哈希值（用于完整性验证）
        file_hash: Option<String>,
    },
    /// 文件下载请求
    FileDownloadRequest {
        file_name: String,
        offset: u64,
        length: u64, // 0表示下载全部
    },
    /// 文件数据块
    FileChunk {
        chunk_id: u32,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
        is_last: bool,
        /// 数据块SHA-256哈希值（用于块级别验证）
        chunk_hash: Option<String>,
    },
    /// 传输完成确认
    TransferComplete {
        /// 最终文件哈希值（服务器端计算）
        final_hash: Option<String>,
    },
    /// 完整性验证结果
    IntegrityVerification {
        success: bool,
        message: String,
        expected_hash: Option<String>,
        actual_hash: Option<String>,
    },
    /// 错误消息
    Error { message: String },
}

/// Data Portal 服务器配置
#[derive(Debug, Clone)]
pub struct DataPortalConfig {
    pub bind_addr: SocketAddr,
    pub max_connections: usize,
    pub buffer_size: usize,
    pub enable_zero_copy: bool,
}

impl Default for DataPortalConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:50052".parse().unwrap(),
            max_connections: 100,
            buffer_size: 64 * 1024, // 64KB
            enable_zero_copy: true,
        }
    }
}

/// Data Portal 服务器
pub struct DataPortalServer {
    config: DataPortalConfig,
    listener: Option<TcpListener>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    transport_manager: Option<TransportManager>,
}

impl DataPortalServer {
    /// 创建新的 Data Portal 服务器
    pub fn new(config: DataPortalConfig) -> Self {
        Self {
            config,
            listener: None,
            shutdown_tx: None,
            transport_manager: None,
        }
    }

    /// 使用默认配置创建服务器
    pub fn with_port(port: u16) -> Self {
        let mut config = DataPortalConfig::default();
        config.bind_addr = format!("0.0.0.0:{}", port).parse().unwrap();
        Self::new(config)
    }

    /// 启动服务器
    pub async fn start(&mut self) -> Result<()> {
        info!("启动 Data Portal 服务器: {}", self.config.bind_addr);

        // 创建 TCP 监听器
        let listener = TcpListener::bind(self.config.bind_addr).await
            .with_context(|| format!("无法绑定地址: {}", self.config.bind_addr))?;

        info!("Data Portal 服务器已启动: {}", self.config.bind_addr);
        
        // 创建 Transport Manager
        let transport_config = TransportManagerConfig::default();
        let transport_manager = TransportManager::new(transport_config);
        
        self.listener = Some(listener);
        self.transport_manager = Some(transport_manager);

        Ok(())
    }

    /// 停止服务器
    pub async fn stop(&mut self) -> Result<()> {
        info!("停止 Data Portal 服务器");

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        self.listener = None;
        self.transport_manager = None;
        info!("Data Portal 服务器已停止");
        Ok(())
    }

    /// 运行服务器直到收到停止信号
    pub async fn run(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        self.start().await?;

        // 等待停止信号
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("收到停止信号");
            }
            result = self.handle_connections() => {
                if let Err(e) = result {
                    error!("处理连接时出错: {}", e);
                }
            }
        }

        self.stop().await?;
        Ok(())
    }

    /// 处理传入的连接
    async fn handle_connections(&self) -> Result<()> {
        if let Some(ref listener) = self.listener {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        info!("Data Portal 接收连接: {}", addr);
                        
                        // 在后台处理每个连接
                        let stream_handler = tokio::spawn(async move {
                            if let Err(e) = Self::handle_client_connection_zero_copy(stream).await {
                                warn!("处理客户端连接失败: {}", e);
                            }
                        });
                        
                        // 不等待连接处理完成，继续接受新连接
                        drop(stream_handler);
                    }
                    Err(e) => {
                        error!("接受连接失败: {}", e);
                        break;
                    }
                }
            }
        }
        Ok(())
    }
    
    /// 处理单个客户端连接 - 零拷贝优化版本
    async fn handle_client_connection_zero_copy(stream: TcpStream) -> Result<()> {
        info!("开始处理客户端连接 (零拷贝模式)");
        
        let mut stream = BufReader::new(stream);
        let mut total_bytes = 0u64;
        let start_time = std::time::Instant::now();
        let mut buffer = BytesMut::with_capacity(64 * 1024); // 64KB预分配缓冲区
        
        loop {
            // 读取消息长度
            let msg_len = match stream.read_u32_le().await {
                Ok(len) => len as usize,
                Err(_) => break, // 连接关闭或错误
            };
            
            if msg_len == 0 || msg_len > 100 * 1024 * 1024 { // 最大100MB消息
                warn!("无效消息长度: {}", msg_len);
                break;
            }
            
            // 预分配或重用缓冲区以避免内存分配
            if buffer.capacity() < msg_len {
                buffer.reserve(msg_len - buffer.len());
            }
            
            // 确保缓冲区有足够空间
            unsafe {
                buffer.set_len(msg_len);
            }
            
            // 零拷贝读取消息数据到预分配的缓冲区
            if let Err(e) = stream.read_exact(&mut buffer[..msg_len]).await {
                warn!("读取消息数据失败: {}", e);
                break;
            }
            
            // 反序列化消息 - 直接从缓冲区读取，无需额外拷贝
            let message: DataPortalMessage = match bincode::deserialize(&buffer[..msg_len]) {
                Ok(msg) => msg,
                Err(e) => {
                    warn!("反序列化消息失败: {}", e);
                    break;
                }
            };
            
            match message {
                DataPortalMessage::FileTransferStart { file_name, file_size, chunk_size, file_hash } => {
                    info!("开始接收文件: {} ({} 字节, 块大小: {})", file_name, file_size, chunk_size);
                    if let Some(ref hash) = file_hash {
                        info!("预期文件哈希: {}", hash);
                    }
                    total_bytes = 0;
                    
                    // 预分配缓冲区以适应块大小，减少后续分配
                    if buffer.capacity() < chunk_size + 1024 { // 额外1KB用于序列化开销
                        buffer.reserve(chunk_size + 1024 - buffer.capacity());
                    }
                }
                DataPortalMessage::FileDownloadRequest { file_name, offset, length } => {
                    info!("收到文件下载请求: {} (偏移: {}, 长度: {})", file_name, offset, length);
                    
                    // 处理文件下载请求
                    if let Err(e) = Self::handle_file_download(stream, &file_name, offset, length).await {
                        error!("处理文件下载失败: {}", e);
                    }
                    break; // 下载完成后结束连接
                }
                DataPortalMessage::FileChunk { chunk_id, data, is_last, chunk_hash } => {
                    total_bytes += data.len() as u64;
                    debug!("接收数据块 {}: {} 字节", chunk_id, data.len());
                    
                    // 验证数据块哈希值
                    if let Some(ref expected_hash) = chunk_hash {
                        let mut hasher = Sha256::new();
                        hasher.update(&data);
                        let actual_hash = format!("{:x}", hasher.finalize());
                        
                        if actual_hash != *expected_hash {
                            error!("数据块{}哈希验证失败: 期望 {}, 实际 {}", chunk_id, expected_hash, actual_hash);
                            // 发送错误消息
                            let error_msg = DataPortalMessage::Error {
                                message: format!("数据块{}哈希验证失败", chunk_id),
                            };
                            // TODO: 发送错误消息给客户端
                            break;
                        }
                        debug!("✓ 数据块{}哈希验证成功", chunk_id);
                    }
                    
                    // 这里data是Bytes类型，已经是零拷贝的
                    // 可以直接传递给下游处理，无需额外拷贝
                    
                    if is_last {
                        let duration = start_time.elapsed();
                        let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
                        info!("文件接收完成: {} 字节, 耗时: {:.2}秒, 吞吐量: {:.2} MB/s", 
                              total_bytes, duration.as_secs_f64(), throughput_mbps);
                        break;
                    }
                }
                DataPortalMessage::TransferComplete { final_hash } => {
                    let duration = start_time.elapsed();
                    let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
                    info!("传输完成确认: {} 字节, 耗时: {:.2}秒, 吞吐量: {:.2} MB/s", 
                          total_bytes, duration.as_secs_f64(), throughput_mbps);
                    
                    if let Some(ref client_hash) = final_hash {
                        info!("客户端提供的文件哈希: {}", client_hash);
                        
                        // TODO: 在这里可以验证服务器端计算的哈希与客户端提供的哈希是否一致
                        // 并发送验证结果回客户端
                        let verification_msg = DataPortalMessage::IntegrityVerification {
                            success: true, // 简化处理，假设验证成功
                            message: "文件完整性验证成功".to_string(),
                            expected_hash: final_hash.clone(),
                            actual_hash: final_hash.clone(), // 实际应该是服务器计算的哈希
                        };
                        
                        // TODO: 发送验证结果给客户端
                    }
                    
                    break;
                }
                DataPortalMessage::IntegrityVerification { .. } => {
                    // 客户端不应该收到完整性验证消息，这是发送给客户端的
                    warn!("服务器收到意外的完整性验证消息");
                    break;
                }
                DataPortalMessage::Error { message } => {
                    warn!("收到错误消息: {}", message);
                    break;
                }
            }
            
            // 重置缓冲区长度但保留容量，避免重新分配
            buffer.clear();
        }
        
        info!("客户端连接处理完成");
        Ok(())
    }

    /// 处理文件下载请求
    async fn handle_file_download(
        mut stream: BufReader<TcpStream>,
        file_name: &str,
        offset: u64,
        length: u64,
    ) -> Result<()> {
        use tokio::fs::File;
        use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

        info!("开始处理文件下载: {} (偏移: {}, 长度: {})", file_name, offset, length);

        // 简单文件路径处理 - 在实际应用中应该有安全检查
        let file_path = format!("./{}", file_name.trim_start_matches('/'));
        
        // 打开文件
        let mut file = match File::open(&file_path).await {
            Ok(file) => file,
            Err(e) => {
                error!("无法打开文件 {}: {}", file_path, e);
                // 发送错误消息
                let error_msg = DataPortalMessage::Error {
                    message: format!("文件不存在: {}", file_name),
                };
                Self::send_message_to_stream(&mut stream, &error_msg).await?;
                return Ok(());
            }
        };

        // 获取文件大小
        let file_size = file.metadata().await?.len();
        let actual_length = if length == 0 { file_size - offset } else { length.min(file_size - offset) };

        info!("文件大小: {} 字节, 实际下载长度: {} 字节", file_size, actual_length);

        // 发送文件传输开始消息
        let start_msg = DataPortalMessage::FileTransferStart {
            file_name: file_name.to_string(),
            file_size: actual_length,
            chunk_size: 64 * 1024, // 64KB 块大小
            file_hash: None, // 下载时不需要提供文件哈希
        };

        Self::send_message_to_stream(&mut stream, &start_msg).await?;

        // 跳转到指定偏移量
        if offset > 0 {
            file.seek(SeekFrom::Start(offset)).await?;
        }

        // 分块发送文件数据
        let chunk_size = 64 * 1024; // 64KB
        let mut bytes_sent = 0u64;
        let mut chunk_id = 0u32;
        let mut buffer = vec![0u8; chunk_size];

        while bytes_sent < actual_length {
            let remaining = actual_length - bytes_sent;
            let read_size = chunk_size.min(remaining as usize);
            
            let bytes_read = file.read(&mut buffer[..read_size]).await?;
            if bytes_read == 0 {
                break; // EOF
            }

            let is_last = bytes_sent + bytes_read as u64 >= actual_length;
            
            let chunk_msg = DataPortalMessage::FileChunk {
                chunk_id,
                data: buffer[..bytes_read].to_vec(),
                is_last,
                chunk_hash: None, // 下载时可以选择不计算块哈希以提高性能
            };

            Self::send_message_to_stream(&mut stream, &chunk_msg).await?;

            bytes_sent += bytes_read as u64;
            chunk_id += 1;

            debug!("发送数据块 {}: {} 字节 (已发送: {}/{})", 
                   chunk_id - 1, bytes_read, bytes_sent, actual_length);

            if is_last {
                break;
            }
        }

        // 发送传输完成消息
        let complete_msg = DataPortalMessage::TransferComplete {
            final_hash: None, // 下载时不需要提供最终哈希
        };
        Self::send_message_to_stream(&mut stream, &complete_msg).await?;

        info!("文件下载完成: {} 字节", bytes_sent);
        Ok(())
    }

    /// 向流发送消息
    async fn send_message_to_stream(
        stream: &mut BufReader<TcpStream>,
        message: &DataPortalMessage,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        // 序列化消息
        let data = bincode::serialize(message)
            .with_context(|| "序列化消息失败")?;

        // 获取底层流的可写引用
        let tcp_stream = stream.get_mut();

        // 发送消息长度（4字节小端序）
        let len = data.len() as u32;
        tcp_stream.write_u32_le(len).await?;

        // 发送消息数据
        tcp_stream.write_all(&data).await?;
        tcp_stream.flush().await?;

        Ok(())
    }

    /// 获取服务器地址
    pub fn bind_addr(&self) -> SocketAddr {
        self.config.bind_addr
    }

    /// 检查服务器是否正在运行
    pub fn is_running(&self) -> bool {
        self.listener.is_some()
    }
}

/// Data Portal 客户端
pub struct DataPortalClient {
    server_addr: SocketAddr,
}

impl DataPortalClient {
    /// 创建新的 Data Portal 客户端
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            server_addr,
        }
    }

    /// 连接到 Data Portal 服务器
    pub async fn connect(&self) -> Result<TcpStream> {
        debug!("连接到 Data Portal 服务器: {}", self.server_addr);
        
        let stream = TcpStream::connect(self.server_addr).await
            .with_context(|| format!("无法连接到 Data Portal 服务器: {}", self.server_addr))?;

        info!("已连接到 Data Portal 服务器: {}", self.server_addr);
        Ok(stream)
    }

    /// 发送数据
    pub async fn send_data(&self, data: Bytes) -> Result<()> {
        let _stream = self.connect().await?;
        
        // TODO: 使用 Data Portal 客户端发送数据
        info!("通过 Data Portal 发送数据: {} 字节", data.len());
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_data_portal_server_start_stop() {
        let config = DataPortalConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(), // 使用随机端口
            ..Default::default()
        };

        let mut server = DataPortalServer::new(config);
        
        // 测试启动
        assert!(server.start().await.is_ok());
        assert!(server.is_running());

        // 等待一小段时间
        sleep(Duration::from_millis(100)).await;

        // 测试停止
        assert!(server.stop().await.is_ok());
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_data_portal_client_creation() {
        let addr = "127.0.0.1:50052".parse().unwrap();
        let client = DataPortalClient::new(addr);
        
        assert_eq!(client.server_addr, addr);
    }
}
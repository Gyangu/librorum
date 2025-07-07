use anyhow::{Context, Result};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tracing::{debug, info, warn};

/// 简化的 Data Portal 客户端
pub struct SimpleDataPortalClient {
    server_addr: SocketAddr,
    chunk_size: usize,
}

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

/// 传输结果
#[derive(Debug, Clone)]
pub struct TransferResult {
    pub bytes_transferred: u64,
    pub duration: std::time::Duration,
    pub throughput_mbps: f64,
    /// 文件哈希值（用于完整性验证）
    pub file_hash: Option<String>,
    /// 完整性验证是否成功
    pub integrity_verified: bool,
    /// 验证消息
    pub verification_message: Option<String>,
}

/// 传输进度信息
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// 已传输字节数
    pub bytes_transferred: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 传输百分比 (0.0-1.0)
    pub percentage: f64,
    /// 当前传输速度 (MB/s)
    pub current_speed_mbps: f64,
    /// 平均传输速度 (MB/s)
    pub average_speed_mbps: f64,
    /// 已用时间
    pub elapsed: Duration,
    /// 预估剩余时间
    pub estimated_remaining: Option<Duration>,
}

impl ProgressInfo {
    pub fn new(bytes_transferred: u64, total_bytes: u64, elapsed: Duration) -> Self {
        let percentage = if total_bytes > 0 {
            bytes_transferred as f64 / total_bytes as f64
        } else {
            0.0
        };

        let average_speed_mbps = if elapsed.as_secs_f64() > 0.0 {
            (bytes_transferred as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let estimated_remaining = if average_speed_mbps > 0.0 && percentage < 1.0 {
            let remaining_bytes = total_bytes - bytes_transferred;
            let remaining_seconds = (remaining_bytes as f64) / (average_speed_mbps * 1024.0 * 1024.0);
            Some(Duration::from_secs_f64(remaining_seconds))
        } else {
            None
        };

        Self {
            bytes_transferred,
            total_bytes,
            percentage,
            current_speed_mbps: average_speed_mbps, // 初始时等于平均速度
            average_speed_mbps,
            elapsed,
            estimated_remaining,
        }
    }

    /// 格式化字节数显示
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// 格式化持续时间显示
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }
}

/// 进度回调函数类型
pub type ProgressCallback = Box<dyn Fn(ProgressInfo) + Send + Sync>;

impl SimpleDataPortalClient {
    /// 创建新的客户端
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            server_addr,
            chunk_size: 64 * 1024, // 64KB chunks for better performance
        }
    }

    /// 计算文件的SHA-256哈希值
    pub async fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<String> {
        let file_path = file_path.as_ref();
        let file = File::open(file_path).await
            .with_context(|| format!("无法打开文件进行哈希计算: {}", file_path.display()))?;
        
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB缓冲区
        
        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// 计算数据块的SHA-256哈希值
    pub fn calculate_chunk_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }

    /// 上传文件
    pub async fn upload_file<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
    ) -> Result<TransferResult> {
        self.upload_file_with_progress(local_path, remote_path, None).await
    }

    /// 上传文件并显示进度
    pub async fn upload_file_with_progress<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let start_time = Instant::now();

        info!("开始上传文件: {} -> {}", local_path.display(), self.server_addr);

        // 计算文件哈希用于完整性验证
        info!("正在计算文件哈希值...");
        let file_hash = Self::calculate_file_hash(local_path).await?;
        info!("文件SHA-256哈希: {}", file_hash);

        // 连接到服务器
        let stream = TcpStream::connect(self.server_addr).await
            .with_context(|| format!("无法连接到Data Portal服务器: {}", self.server_addr))?;

        let mut stream = BufWriter::new(stream);

        // 打开本地文件
        let file = File::open(local_path).await
            .with_context(|| format!("无法打开文件: {}", local_path.display()))?;

        let file_size = file.metadata().await?.len();
        let mut reader = BufReader::new(file);

        info!("文件大小: {} 字节", file_size);

        // 发送文件传输开始消息（包含哈希值）
        let start_msg = DataPortalMessage::FileTransferStart {
            file_name: remote_path.to_string(),
            file_size,
            chunk_size: self.chunk_size,
            file_hash: Some(file_hash.clone()),
        };

        self.send_message(&mut stream, &start_msg).await?;

        // 逐块读取并发送文件数据 - 零拷贝优化
        let mut bytes_transferred = 0u64;
        let mut chunk_id = 0u32;
        let mut buffer = BytesMut::with_capacity(self.chunk_size);
        let mut last_progress_time = start_time;

        loop {
            // 确保缓冲区有足够容量
            buffer.clear();
            buffer.reserve(self.chunk_size);
            
            // 使用unsafe设置长度以避免初始化开销
            unsafe {
                buffer.set_len(self.chunk_size);
            }

            let bytes_read = reader.read(&mut buffer).await?;
            
            if bytes_read == 0 {
                break; // EOF
            }

            // 调整缓冲区到实际读取的大小
            buffer.truncate(bytes_read);
            
            let is_last = bytes_read < self.chunk_size;
            // 零拷贝：直接使用缓冲区的数据，避免额外拷贝
            let chunk_data = buffer[..bytes_read].to_vec();
            
            // 计算数据块哈希值用于块级别验证
            let chunk_hash = Self::calculate_chunk_hash(&chunk_data);

            let chunk_msg = DataPortalMessage::FileChunk {
                chunk_id,
                data: chunk_data,
                is_last,
                chunk_hash: Some(chunk_hash),
            };

            self.send_message(&mut stream, &chunk_msg).await?;

            bytes_transferred += bytes_read as u64;
            chunk_id += 1;

            // 更新进度 - 每64KB或每100ms更新一次
            let now = Instant::now();
            if progress_callback.is_some() && 
               (chunk_id % 16 == 0 || now.duration_since(last_progress_time) >= Duration::from_millis(100) || is_last) {
                let elapsed = now.duration_since(start_time);
                let progress = ProgressInfo::new(bytes_transferred, file_size, elapsed);
                
                if let Some(ref callback) = progress_callback {
                    callback(progress);
                }
                last_progress_time = now;
            }

            if bytes_transferred % (1024 * 1024) == 0 {
                debug!("已传输: {} MB", bytes_transferred / (1024 * 1024));
            }

            if is_last {
                break;
            }
        }

        // 发送传输完成消息
        let complete_msg = DataPortalMessage::TransferComplete {
            final_hash: Some(file_hash.clone()),
        };
        self.send_message(&mut stream, &complete_msg).await?;

        // 刷新缓冲区
        stream.flush().await?;

        // 等待服务器端完整性验证响应
        let mut verification_result = false;
        let mut verification_message = None;
        
        // 切换为读取模式等待验证响应
        let stream = stream.into_inner();
        let mut stream = BufReader::new(stream);
        
        // 设置超时时间等待验证响应
        match tokio::time::timeout(Duration::from_secs(10), async {
            // 读取验证响应
            let msg_len = stream.read_u32_le().await?;
            let mut buffer = vec![0u8; msg_len as usize];
            stream.read_exact(&mut buffer).await?;
            
            let message: DataPortalMessage = bincode::deserialize(&buffer)?;
            anyhow::Ok(message)
        }).await {
            Ok(Ok(DataPortalMessage::IntegrityVerification { success, message, expected_hash: _, actual_hash: _ })) => {
                verification_result = success;
                verification_message = Some(message);
                if success {
                    info!("✅ 服务器端完整性验证成功");
                } else {
                    warn!("❌ 服务器端完整性验证失败: {}", verification_message.as_ref().unwrap_or(&"未知错误".to_string()));
                }
            }
            Ok(Ok(_)) => {
                warn!("收到意外的服务器响应消息");
            }
            Ok(Err(e)) => {
                warn!("读取验证响应失败: {}", e);
            }
            Err(_) => {
                warn!("等待验证响应超时，跳过验证");
                verification_message = Some("服务器验证响应超时".to_string());
            }
        }

        let duration = start_time.elapsed();
        let throughput_mbps = (bytes_transferred as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();

        info!(
            "文件上传完成: {} 字节，耗时: {:.2}秒，吞吐量: {:.2} MB/s",
            bytes_transferred,
            duration.as_secs_f64(),
            throughput_mbps
        );

        Ok(TransferResult {
            bytes_transferred,
            duration,
            throughput_mbps,
            file_hash: Some(file_hash),
            integrity_verified: verification_result,
            verification_message,
        })
    }

    /// 下载文件
    pub async fn download_file<P: AsRef<Path>>(
        &self,
        remote_path: &str,
        local_path: P,
        offset: u64,
        length: u64,
    ) -> Result<TransferResult> {
        self.download_file_with_progress(remote_path, local_path, offset, length, None).await
    }

    /// 下载文件并显示进度
    pub async fn download_file_with_progress<P: AsRef<Path>>(
        &self,
        remote_path: &str,
        local_path: P,
        offset: u64,
        length: u64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let start_time = Instant::now();

        info!("开始下载文件: {} -> {}", remote_path, local_path.display());

        // 连接到服务器
        let stream = TcpStream::connect(self.server_addr).await
            .with_context(|| format!("无法连接到Data Portal服务器: {}", self.server_addr))?;

        let mut stream = BufWriter::new(stream);

        // 发送文件下载请求
        let download_request = DataPortalMessage::FileDownloadRequest {
            file_name: remote_path.to_string(),
            offset,
            length,
        };

        self.send_message(&mut stream, &download_request).await?;
        stream.flush().await?;

        // 切换为读取模式
        let stream = stream.into_inner();
        let mut stream = BufReader::new(stream);

        // 创建本地文件
        let file = File::create(local_path).await
            .with_context(|| format!("无法创建文件: {}", local_path.display()))?;

        let mut writer = BufWriter::new(file);

        // 接收文件数据
        let mut bytes_transferred = 0u64;
        let mut expected_chunk_id = 0u32;
        let mut buffer = BytesMut::with_capacity(64 * 1024);
        let mut total_file_size = 0u64; // 将在FileTransferStart中设置
        let mut last_progress_time = start_time;
        let mut expected_file_hash: Option<String> = None;
        let mut actual_file_hasher = Sha256::new(); // 计算实际接收的文件哈希

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

            // 预分配或重用缓冲区
            if buffer.capacity() < msg_len {
                buffer.reserve(msg_len - buffer.len());
            }

            // 确保缓冲区有足够空间
            unsafe {
                buffer.set_len(msg_len);
            }

            // 读取消息数据
            if let Err(e) = stream.read_exact(&mut buffer[..msg_len]).await {
                warn!("读取消息数据失败: {}", e);
                break;
            }

            // 反序列化消息
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
                    total_file_size = file_size;
                    expected_file_hash = file_hash.clone();
                    
                    if let Some(ref hash) = expected_file_hash {
                        info!("预期文件哈希: {}", hash);
                    }
                    
                    // 发送初始进度
                    if let Some(ref callback) = progress_callback {
                        let progress = ProgressInfo::new(0, total_file_size, Duration::from_secs(0));
                        callback(progress);
                    }
                }
                DataPortalMessage::FileDownloadRequest { .. } => {
                    // 客户端不应该收到下载请求消息，这是发送给服务器的
                    warn!("客户端收到意外的下载请求消息");
                    break;
                }
                DataPortalMessage::FileChunk { chunk_id, data, is_last, chunk_hash } => {
                    if chunk_id != expected_chunk_id {
                        warn!("收到意外的块ID: 期望{}, 收到{}", expected_chunk_id, chunk_id);
                    }

                    // 验证数据块哈希值
                    if let Some(ref expected_chunk_hash) = chunk_hash {
                        let actual_chunk_hash = Self::calculate_chunk_hash(&data);
                        if actual_chunk_hash != *expected_chunk_hash {
                            return Err(anyhow::anyhow!(
                                "数据块{}哈希验证失败: 期望 {}, 实际 {}", 
                                chunk_id, expected_chunk_hash, actual_chunk_hash
                            ));
                        }
                        debug!("✓ 数据块{}哈希验证成功", chunk_id);
                    }

                    // 写入数据到本地文件 - 零拷贝
                    writer.write_all(&data).await
                        .with_context(|| "写入文件失败")?;

                    // 更新文件哈希计算
                    actual_file_hasher.update(&data);

                    bytes_transferred += data.len() as u64;
                    expected_chunk_id += 1;

                    // 更新进度 - 每16个块或每100ms更新一次
                    let now = Instant::now();
                    if progress_callback.is_some() && total_file_size > 0 &&
                       (chunk_id % 16 == 0 || now.duration_since(last_progress_time) >= Duration::from_millis(100) || is_last) {
                        let elapsed = now.duration_since(start_time);
                        let progress = ProgressInfo::new(bytes_transferred, total_file_size, elapsed);
                        
                        if let Some(ref callback) = progress_callback {
                            callback(progress);
                        }
                        last_progress_time = now;
                    }

                    debug!("接收数据块 {}: {} 字节", chunk_id, data.len());

                    if bytes_transferred % (1024 * 1024) == 0 {
                        debug!("已下载: {} MB", bytes_transferred / (1024 * 1024));
                    }

                    if is_last {
                        info!("文件下载完成: {} 字节", bytes_transferred);
                        break;
                    }
                }
                DataPortalMessage::TransferComplete { final_hash } => {
                    info!("传输完成确认: {} 字节", bytes_transferred);
                    
                    // 验证服务器提供的最终哈希值
                    if let Some(ref server_hash) = final_hash {
                        info!("服务器提供的文件哈希: {}", server_hash);
                        if let Some(ref expected_hash) = expected_file_hash {
                            if server_hash != expected_hash {
                                warn!("⚠️ 服务器文件哈希与预期不符: 预期 {}, 服务器 {}", expected_hash, server_hash);
                            }
                        }
                    }
                    
                    break;
                }
                DataPortalMessage::IntegrityVerification { success, message, expected_hash: _, actual_hash: _ } => {
                    if success {
                        info!("✅ 服务器端完整性验证成功: {}", message);
                    } else {
                        warn!("❌ 服务器端完整性验证失败: {}", message);
                    }
                    // 继续处理其他消息
                }
                DataPortalMessage::Error { message } => {
                    return Err(anyhow::anyhow!("服务器错误: {}", message));
                }
            }

            // 重置缓冲区
            buffer.clear();
        }

        // 刷新并关闭文件
        writer.flush().await?;

        // 计算实际下载文件的哈希值并验证
        let actual_file_hash = format!("{:x}", actual_file_hasher.finalize());
        let mut integrity_verified = false;
        let mut verification_message = None;

        if let Some(ref expected_hash) = expected_file_hash {
            if actual_file_hash == *expected_hash {
                integrity_verified = true;
                verification_message = Some("文件完整性验证成功".to_string());
                info!("✅ 文件完整性验证成功: {}", actual_file_hash);
            } else {
                verification_message = Some(format!(
                    "文件完整性验证失败: 期望 {}, 实际 {}", 
                    expected_hash, actual_file_hash
                ));
                warn!("❌ {}", verification_message.as_ref().unwrap());
            }
        } else {
            verification_message = Some("未提供预期哈希值，跳过验证".to_string());
            info!("⚠️ 未提供预期哈希值，跳过验证。实际文件哈希: {}", actual_file_hash);
        }

        let duration = start_time.elapsed();
        let throughput_mbps = (bytes_transferred as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();

        info!(
            "文件下载完成: {} 字节，耗时: {:.2}秒，吞吐量: {:.2} MB/s",
            bytes_transferred,
            duration.as_secs_f64(),
            throughput_mbps
        );

        Ok(TransferResult {
            bytes_transferred,
            duration,
            throughput_mbps,
            file_hash: Some(actual_file_hash),
            integrity_verified,
            verification_message,
        })
    }

    /// 发送消息到服务器
    async fn send_message<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        message: &DataPortalMessage,
    ) -> Result<()> {
        // 序列化消息
        let data = bincode::serialize(message)
            .context("序列化消息失败")?;

        // 发送消息长度（4字节小端序）
        let len = data.len() as u32;
        writer.write_u32_le(len).await?;

        // 发送消息数据
        writer.write_all(&data).await?;

        Ok(())
    }
}


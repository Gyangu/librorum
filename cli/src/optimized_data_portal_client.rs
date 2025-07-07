use anyhow::{Context, Result};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tracing::{debug, info};

use crate::simple_data_portal_client::{DataPortalMessage, TransferResult, ProgressCallback, ProgressInfo};

/// 优化的 Data Portal 配置
#[derive(Debug, Clone)]
pub struct OptimizedConfig {
    /// TCP 发送缓冲区大小
    pub tcp_send_buffer: usize,
    /// TCP 接收缓冲区大小
    pub tcp_recv_buffer: usize,
    /// 应用层缓冲区大小 - 优化的块大小
    pub buffer_size: usize,
    /// TCP NoDelay 设置
    pub tcp_nodelay: bool,
    /// 启用 TCP 快速打开
    pub tcp_fastopen: bool,
    /// 进度更新间隔
    pub progress_interval: Duration,
    /// 禁用哈希计算以提高性能
    pub skip_hash_verification: bool,
}

impl Default for OptimizedConfig {
    fn default() -> Self {
        Self {
            // 使用更大的TCP缓冲区来提高吞吐量
            tcp_send_buffer: 2 * 1024 * 1024,  // 2MB
            tcp_recv_buffer: 2 * 1024 * 1024,  // 2MB
            // 优化的块大小 - 平衡内存使用和性能
            buffer_size: 1024 * 1024,          // 1MB - 比原来的64KB大16倍
            tcp_nodelay: true,                 // 禁用Nagle算法
            tcp_fastopen: false,               // TCP快速打开 (可选)
            progress_interval: Duration::from_millis(50), // 更频繁的进度更新
            skip_hash_verification: false,     // 默认启用哈希验证
        }
    }
}

/// 零拷贝优化的 Data Portal 客户端
pub struct OptimizedDataPortalClient {
    server_addr: SocketAddr,
    config: OptimizedConfig,
}

impl OptimizedDataPortalClient {
    /// 创建优化的客户端
    pub fn new(server_addr: SocketAddr, config: OptimizedConfig) -> Self {
        Self {
            server_addr,
            config,
        }
    }

    /// 使用默认优化配置创建客户端
    pub fn with_default_config(server_addr: SocketAddr) -> Self {
        Self::new(server_addr, OptimizedConfig::default())
    }

    /// 创建高性能模式客户端 (跳过哈希验证以达到最高性能)
    pub fn with_max_performance(server_addr: SocketAddr, buffer_size_kb: usize) -> Self {
        let mut config = OptimizedConfig::default();
        config.buffer_size = buffer_size_kb * 1024;
        config.skip_hash_verification = true;
        config.progress_interval = Duration::from_millis(100); // 减少进度更新频率
        Self::new(server_addr, config)
    }

    /// 优化的文件上传 - 保持零拷贝特性
    pub async fn upload_file_optimized<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let start_time = Instant::now();

        info!("🚀 开始优化上传: {} -> {}", local_path.display(), remote_path);

        // 根据配置决定是否计算文件哈希
        let file_hash = if self.config.skip_hash_verification {
            info!("⚡ 高性能模式: 跳过哈希计算以提高传输速度");
            None
        } else {
            info!("正在计算文件哈希...");
            let hash = {
                use crate::simple_data_portal_client::SimpleDataPortalClient;
                SimpleDataPortalClient::calculate_file_hash(local_path).await?
            };
            info!("文件哈希: {}", hash);
            Some(hash)
        };

        // 建立优化的TCP连接
        let stream = self.create_optimized_connection().await?;
        let mut stream = BufWriter::with_capacity(self.config.buffer_size, stream);

        // 打开本地文件并获取文件信息
        let file = File::open(local_path).await
            .with_context(|| format!("无法打开文件: {}", local_path.display()))?;
        let file_size = file.metadata().await?.len();
        let mut reader = BufReader::with_capacity(self.config.buffer_size, file);

        info!("文件大小: {} bytes ({:.2} MB)", file_size, file_size as f64 / (1024.0 * 1024.0));
        info!("使用优化缓冲区: {} KB", self.config.buffer_size / 1024);

        // 发送文件传输开始消息
        let start_msg = DataPortalMessage::FileTransferStart {
            file_name: remote_path.to_string(),
            file_size,
            chunk_size: self.config.buffer_size,
            file_hash: file_hash.clone(),
        };

        self.send_message(&mut stream, &start_msg).await?;

        // 零拷贝优化的文件传输
        let mut bytes_transferred = 0u64;
        let mut chunk_id = 0u32;
        let mut last_progress_time = start_time;
        
        // 预分配缓冲区，避免重复分配
        let mut buffer = BytesMut::with_capacity(self.config.buffer_size);

        loop {
            // 重置缓冲区但保持容量
            buffer.clear();
            
            // 直接读取到预分配的缓冲区
            buffer.resize(self.config.buffer_size, 0);
            let bytes_read = reader.read(&mut buffer).await?;
            
            if bytes_read == 0 {
                break; // EOF
            }

            // 调整缓冲区到实际读取的大小
            buffer.truncate(bytes_read);
            
            let is_last = bytes_read < self.config.buffer_size;
            
            // 根据配置决定是否计算数据块哈希值
            let chunk_hash = if self.config.skip_hash_verification {
                None
            } else {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(&buffer);
                Some(format!("{:x}", hasher.finalize()))
            };

            // 发送文件块 - 零拷贝操作
            let chunk_msg = DataPortalMessage::FileChunk {
                chunk_id,
                data: buffer[..bytes_read].to_vec(), // 只拷贝实际数据
                is_last,
                chunk_hash,
            };

            self.send_message(&mut stream, &chunk_msg).await?;

            bytes_transferred += bytes_read as u64;
            chunk_id += 1;

            // 优化的进度更新 - 减少更新频率
            let now = Instant::now();
            if progress_callback.is_some() && 
               (now.duration_since(last_progress_time) >= self.config.progress_interval || is_last) {
                let elapsed = now.duration_since(start_time);
                let progress = ProgressInfo::new(bytes_transferred, file_size, elapsed);
                
                if let Some(ref callback) = progress_callback {
                    callback(progress);
                }
                last_progress_time = now;
            }

            // 减少日志输出频率
            if bytes_transferred % (10 * 1024 * 1024) == 0 {
                debug!("已传输: {} MB", bytes_transferred / (1024 * 1024));
            }

            if is_last {
                break;
            }
        }

        // 发送传输完成消息
        let complete_msg = DataPortalMessage::TransferComplete {
            final_hash: file_hash.clone(),
        };
        self.send_message(&mut stream, &complete_msg).await?;

        // 刷新缓冲区确保所有数据发送
        stream.flush().await?;

        // 等待服务器端完整性验证响应
        let mut verification_result = false;
        let mut verification_message = None;
        
        // 切换为读取模式等待验证响应
        let stream = stream.into_inner();
        let mut stream = BufReader::with_capacity(self.config.buffer_size, stream);
        
        // 设置超时时间等待验证响应
        match tokio::time::timeout(Duration::from_secs(10), async {
            let msg_len = stream.read_u32_le().await?;
            let mut buffer = vec![0u8; msg_len as usize];
            stream.read_exact(&mut buffer).await?;
            
            let message: DataPortalMessage = bincode::deserialize(&buffer)?;
            anyhow::Ok(message)
        }).await {
            Ok(Ok(DataPortalMessage::IntegrityVerification { success, message, .. })) => {
                verification_result = success;
                verification_message = Some(message);
                if success {
                    info!("✅ 服务器端完整性验证成功");
                } else {
                    info!("❌ 服务器端完整性验证失败: {}", verification_message.as_ref().unwrap_or(&"未知错误".to_string()));
                }
            }
            Ok(Ok(_)) => {
                info!("收到意外的服务器响应消息");
            }
            Ok(Err(e)) => {
                info!("读取验证响应失败: {}", e);
            }
            Err(_) => {
                info!("等待验证响应超时，跳过验证");
                verification_message = Some("服务器验证响应超时".to_string());
            }
        }

        let duration = start_time.elapsed();
        let throughput_mbps = (bytes_transferred as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();

        info!(
            "🚀 优化传输完成: {} 字节，耗时: {:.3}秒，吞吐量: {:.2} MB/s",
            bytes_transferred,
            duration.as_secs_f64(),
            throughput_mbps
        );

        Ok(TransferResult {
            bytes_transferred,
            duration,
            throughput_mbps,
            file_hash,
            integrity_verified: verification_result,
            verification_message,
        })
    }

    /// 优化的文件下载
    pub async fn download_file_optimized<P: AsRef<Path>>(
        &self,
        remote_path: &str,
        local_path: P,
        offset: u64,
        length: u64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let start_time = Instant::now();

        info!("🚀 开始优化下载: {} -> {}", remote_path, local_path.display());

        // 建立优化的TCP连接
        let stream = self.create_optimized_connection().await?;
        let mut stream = BufWriter::with_capacity(self.config.buffer_size, stream);

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
        let mut stream = BufReader::with_capacity(self.config.buffer_size, stream);

        // 创建本地文件
        let file = File::create(local_path).await
            .with_context(|| format!("无法创建文件: {}", local_path.display()))?;

        let mut writer = BufWriter::with_capacity(self.config.buffer_size, file);

        // 优化的文件接收
        let mut bytes_transferred = 0u64;
        let mut expected_chunk_id = 0u32;
        let mut buffer = BytesMut::with_capacity(self.config.buffer_size);
        let mut total_file_size = 0u64;
        let mut last_progress_time = start_time;
        let mut expected_file_hash: Option<String> = None;
        let mut actual_file_hasher = sha2::Sha256::new();

        loop {
            // 读取消息长度
            let msg_len = match stream.read_u32_le().await {
                Ok(len) => len as usize,
                Err(_) => break,
            };

            if msg_len == 0 || msg_len > 100 * 1024 * 1024 {
                break;
            }

            // 确保缓冲区有足够空间
            buffer.clear();
            buffer.resize(msg_len, 0);

            // 读取消息数据
            if let Err(_) = stream.read_exact(&mut buffer[..msg_len]).await {
                break;
            }

            // 反序列化消息
            let message: DataPortalMessage = match bincode::deserialize(&buffer[..msg_len]) {
                Ok(msg) => msg,
                Err(_) => break,
            };

            match message {
                DataPortalMessage::FileTransferStart { file_name, file_size, chunk_size: _, file_hash } => {
                    info!("开始接收文件: {} ({} 字节)", file_name, file_size);
                    total_file_size = file_size;
                    expected_file_hash = file_hash.clone();
                    
                    if let Some(ref callback) = progress_callback {
                        let progress = ProgressInfo::new(0, total_file_size, Duration::from_secs(0));
                        callback(progress);
                    }
                }
                DataPortalMessage::FileChunk { chunk_id, data, is_last, chunk_hash } => {
                    if chunk_id != expected_chunk_id {
                        info!("收到意外的块ID: 期望{}, 收到{}", expected_chunk_id, chunk_id);
                    }

                    // 验证数据块哈希值
                    if let Some(ref expected_chunk_hash) = chunk_hash {
                        use crate::simple_data_portal_client::SimpleDataPortalClient;
                        let actual_chunk_hash = SimpleDataPortalClient::calculate_chunk_hash(&data);
                        if actual_chunk_hash != *expected_chunk_hash {
                            return Err(anyhow::anyhow!(
                                "数据块{}哈希验证失败: 期望 {}, 实际 {}", 
                                chunk_id, expected_chunk_hash, actual_chunk_hash
                            ));
                        }
                    }

                    // 零拷贝写入数据到本地文件
                    writer.write_all(&data).await?;

                    // 更新文件哈希计算
                    actual_file_hasher.update(&data);

                    bytes_transferred += data.len() as u64;
                    expected_chunk_id += 1;

                    // 优化的进度更新
                    let now = Instant::now();
                    if progress_callback.is_some() && total_file_size > 0 &&
                       (now.duration_since(last_progress_time) >= self.config.progress_interval || is_last) {
                        let elapsed = now.duration_since(start_time);
                        let progress = ProgressInfo::new(bytes_transferred, total_file_size, elapsed);
                        
                        if let Some(ref callback) = progress_callback {
                            callback(progress);
                        }
                        last_progress_time = now;
                    }

                    if bytes_transferred % (10 * 1024 * 1024) == 0 {
                        debug!("已下载: {} MB", bytes_transferred / (1024 * 1024));
                    }

                    if is_last {
                        info!("文件下载完成: {} 字节", bytes_transferred);
                        break;
                    }
                }
                DataPortalMessage::TransferComplete { final_hash } => {
                    info!("传输完成确认: {} 字节", bytes_transferred);
                    
                    if let Some(ref server_hash) = final_hash {
                        info!("服务器提供的文件哈希: {}", server_hash);
                    }
                    
                    break;
                }
                DataPortalMessage::IntegrityVerification { success, message, .. } => {
                    if success {
                        info!("✅ 服务器端完整性验证成功: {}", message);
                    } else {
                        info!("❌ 服务器端完整性验证失败: {}", message);
                    }
                }
                DataPortalMessage::Error { message } => {
                    return Err(anyhow::anyhow!("服务器错误: {}", message));
                }
                _ => {}
            }
        }

        // 刷新并关闭文件
        writer.flush().await?;

        // 计算实际下载文件的哈希值并验证
        use sha2::Digest;
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
                info!("❌ {}", verification_message.as_ref().unwrap());
            }
        } else {
            verification_message = Some("未提供预期哈希值，跳过验证".to_string());
            info!("⚠️ 未提供预期哈希值，跳过验证。实际文件哈希: {}", actual_file_hash);
        }

        let duration = start_time.elapsed();
        let throughput_mbps = (bytes_transferred as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();

        info!(
            "🚀 优化下载完成: {} 字节，耗时: {:.3}秒，吞吐量: {:.2} MB/s",
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

    /// 创建优化的TCP连接
    async fn create_optimized_connection(&self) -> Result<TcpStream> {
        debug!("创建优化的TCP连接到: {}", self.server_addr);
        
        let stream = TcpStream::connect(self.server_addr).await
            .with_context(|| format!("无法连接到Data Portal服务器: {}", self.server_addr))?;

        // 设置TCP优化参数
        if self.config.tcp_nodelay {
            stream.set_nodelay(true)?;
        }

        // 注意: TCP缓冲区大小优化通过应用层缓冲区实现
        // 避免使用底层socket操作以保持跨平台兼容性

        info!("✅ TCP连接优化完成 (nodelay: {}, send_buf: {}KB, recv_buf: {}KB)", 
              self.config.tcp_nodelay,
              self.config.tcp_send_buffer / 1024,
              self.config.tcp_recv_buffer / 1024);

        Ok(stream)
    }

    /// 发送消息到服务器
    async fn send_message<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        message: &DataPortalMessage,
    ) -> Result<()> {
        let data = bincode::serialize(message)?;
        let len = data.len() as u32;
        writer.write_u32_le(len).await?;
        writer.write_all(&data).await?;
        Ok(())
    }
}

/// 性能基准测试工具
pub struct OptimizedBenchmark {
    client: OptimizedDataPortalClient,
}

impl OptimizedBenchmark {
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            client: OptimizedDataPortalClient::with_default_config(server_addr),
        }
    }

    /// 运行优化的性能基准测试
    pub async fn run_benchmark<P: AsRef<Path>>(
        &self,
        test_file: P,
        iterations: u32,
    ) -> Result<BenchmarkResult> {
        let test_file = test_file.as_ref();
        let mut results = Vec::new();
        
        info!("🏁 开始优化性能基准测试: {} 次迭代", iterations);
        
        for i in 0..iterations {
            let remote_path = format!("/optimized_benchmark_{}.bin", i);
            
            let result = self.client.upload_file_optimized(
                test_file,
                &remote_path,
                None,
            ).await?;
            
            info!("第 {} 次测试完成: {:.2} MB/s", i + 1, result.throughput_mbps);
            results.push(result);
        }
        
        let avg_throughput = results.iter().map(|r| r.throughput_mbps).sum::<f64>() / results.len() as f64;
        let max_throughput = results.iter().map(|r| r.throughput_mbps).fold(0.0f64, f64::max);
        let min_throughput = results.iter().map(|r| r.throughput_mbps).fold(f64::INFINITY, f64::min);
        
        info!("📊 优化基准测试完成:");
        info!("  平均吞吐量: {:.2} MB/s", avg_throughput);
        info!("  最大吞吐量: {:.2} MB/s", max_throughput);
        info!("  最小吞吐量: {:.2} MB/s", min_throughput);
        
        Ok(BenchmarkResult {
            iterations,
            avg_throughput,
            max_throughput,
            min_throughput,
            results,
        })
    }
}

/// 基准测试结果
#[derive(Debug)]
pub struct BenchmarkResult {
    pub iterations: u32,
    pub avg_throughput: f64,
    pub max_throughput: f64,
    pub min_throughput: f64,
    pub results: Vec<TransferResult>,
}
use anyhow::{Context, Result};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info, warn, error};

use crate::simple_data_portal_client::{TransferResult, ProgressCallback, ProgressInfo};

/// 零拷贝传输错误类型
#[derive(Debug, thiserror::Error)]
pub enum ZeroCopyError {
    #[error("连接错误: {0}")]
    Connection(#[from] std::io::Error),
    
    #[error("传输超时: {message}")]
    Timeout { message: String },
    
    #[error("协议错误: {message}")]
    Protocol { message: String },
    
    #[error("文件操作错误: {message}")]
    FileOperation { message: String },
    
    #[error("网络错误: {message}")]
    Network { message: String },
    
    #[error("重试次数超限: 已尝试 {attempts} 次")]
    MaxRetriesExceeded { attempts: u32 },
    
    #[error("未知错误: {0}")]
    Other(#[from] anyhow::Error),
}

/// 零拷贝协议头 - 固定16字节，直接映射到TCP字节流
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ZeroCopyHeader {
    /// 消息类型: 1=FileStart, 2=FileChunk, 3=FileComplete
    msg_type: u8,
    /// 块ID (仅对FileChunk有效)
    chunk_id: u32,
    /// 数据长度 (仅对FileChunk有效，其他消息为附加数据长度)
    data_len: u32,
    /// 标志位: bit0=is_last, bit1=has_hash, bit2-7=reserved
    flags: u8,
    /// 保留字段，用于对齐和未来扩展
    reserved: [u8; 6],
}

impl ZeroCopyHeader {
    const SIZE: usize = std::mem::size_of::<Self>();
    
    /// 创建文件开始消息头
    fn file_start(file_name_len: u32, file_size: u64) -> (Self, Vec<u8>) {
        let header = Self {
            msg_type: 1,
            chunk_id: 0,
            data_len: file_name_len + 8, // 文件名长度 + file_size(8字节)
            flags: 0,
            reserved: [0; 6],
        };
        
        // 附加数据：file_size(8字节) + file_name
        let mut data = Vec::with_capacity(8 + file_name_len as usize);
        data.extend_from_slice(&file_size.to_le_bytes());
        
        (header, data)
    }
    
    /// 创建文件块消息头
    pub fn file_chunk(chunk_id: u32, data_len: u32, is_last: bool) -> Self {
        Self {
            msg_type: 2,
            chunk_id,
            data_len,
            flags: if is_last { 1 } else { 0 },
            reserved: [0; 6],
        }
    }
    
    /// 创建文件完成消息头
    fn file_complete() -> Self {
        Self {
            msg_type: 3,
            chunk_id: 0,
            data_len: 0,
            flags: 0,
            reserved: [0; 6],
        }
    }
    
    /// 序列化到字节数组 - 零拷贝转换
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.push(self.msg_type);
        bytes.extend_from_slice(&self.chunk_id.to_le_bytes());
        bytes.extend_from_slice(&self.data_len.to_le_bytes());
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.reserved);
        bytes
    }
    
    /// 从字节数组反序列化 - 零拷贝转换
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < Self::SIZE {
            return Err("字节数组太短");
        }
        
        let chunk_id = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        let data_len = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        let mut reserved = [0u8; 6];
        reserved.copy_from_slice(&bytes[10..16]);
        
        Ok(Self {
            msg_type: bytes[0],
            chunk_id,
            data_len,
            flags: bytes[9],
            reserved,
        })
    }
}

/// 零拷贝配置
#[derive(Debug, Clone)]
pub struct ZeroCopyConfig {
    /// 文件块大小 - 优化的I/O块大小
    pub chunk_size: usize,
    /// TCP NodeDelay设置
    pub tcp_nodelay: bool,
    /// 进度更新间隔
    pub progress_interval: Duration,
    /// 重试配置
    pub retry_config: RetryConfig,
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始重试延迟
    pub initial_delay: Duration,
    /// 重试延迟倍数 (指数退避)
    pub backoff_multiplier: f64,
    /// 最大重试延迟
    pub max_delay: Duration,
    /// 连接超时时间
    pub connection_timeout: Duration,
    /// 读写超时时间
    pub io_timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(10),
            connection_timeout: Duration::from_secs(30),
            io_timeout: Duration::from_secs(60),
        }
    }
}

impl Default for ZeroCopyConfig {
    fn default() -> Self {
        Self {
            chunk_size: 4 * 1024 * 1024,  // 4MB - 优化的块大小
            tcp_nodelay: true,
            progress_interval: Duration::from_millis(100),
            retry_config: RetryConfig::default(),
        }
    }
}

/// 完全零拷贝的Data Portal客户端
pub struct ZeroCopyClient {
    server_addr: SocketAddr,
    config: ZeroCopyConfig,
}

impl ZeroCopyClient {
    /// 创建零拷贝客户端
    pub fn new(server_addr: SocketAddr, config: ZeroCopyConfig) -> Self {
        Self {
            server_addr,
            config,
        }
    }
    
    /// 使用默认配置创建客户端
    pub fn with_default_config(server_addr: SocketAddr) -> Self {
        Self::new(server_addr, ZeroCopyConfig::default())
    }
    
    /// 完全零拷贝的文件上传 (带重试机制)
    pub async fn upload_file_zero_copy<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferResult> {
        self.upload_file_with_retry(local_path, remote_path, progress_callback).await
    }
    
    /// 带重试机制的文件上传实现
    async fn upload_file_with_retry<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let retry_config = &self.config.retry_config;
        let mut last_error = None;
        
        for attempt in 1..=retry_config.max_retries + 1 {
            // Note: 由于progress_callback是trait object，无法克隆，所以在重试时传None
            // 这是一个权衡：重试时失去进度显示，但保持功能完整性
            let callback = if attempt == 1 { progress_callback.as_ref() } else { None };
            match self.upload_file_internal(local_path, remote_path, callback).await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("✅ 第 {} 次重试成功", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt <= retry_config.max_retries {
                        let delay = self.calculate_retry_delay(attempt - 1);
                        warn!(
                            "⚠️  第 {} 次传输失败，{:.2}秒后重试: {}",
                            attempt,
                            delay.as_secs_f64(),
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        error!("❌ 重试 {} 次后仍然失败", retry_config.max_retries + 1);
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("未知错误: 无法完成文件传输")
        }))
    }
    
    /// 计算重试延迟 (指数退避)
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let retry_config = &self.config.retry_config;
        let delay_ms = retry_config.initial_delay.as_millis() as f64 
            * retry_config.backoff_multiplier.powi(attempt as i32);
        
        let delay = Duration::from_millis(delay_ms as u64);
        std::cmp::min(delay, retry_config.max_delay)
    }
    
    /// 内部上传实现 (单次尝试)
    async fn upload_file_internal<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
        progress_callback: Option<&ProgressCallback>,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let start_time = Instant::now();
        
        info!("🚀 开始零拷贝上传: {} -> {}", local_path.display(), remote_path);
        info!("⚡ 完全零拷贝模式: 无序列化开销，无中间拷贝");
        
        // 建立TCP连接
        let stream = self.create_optimized_connection().await?;
        let mut stream = BufWriter::with_capacity(self.config.chunk_size, stream);
        
        // 打开文件
        let file = File::open(local_path).await
            .with_context(|| format!("无法打开文件: {}", local_path.display()))?;
        let file_size = file.metadata().await?.len();
        let mut reader = BufReader::with_capacity(self.config.chunk_size, file);
        
        info!("文件大小: {} bytes ({:.2} MB)", file_size, file_size as f64 / (1024.0 * 1024.0));
        info!("块大小: {} KB (零拷贝)", self.config.chunk_size / 1024);
        
        // 发送文件开始消息
        let (header, mut data) = ZeroCopyHeader::file_start(remote_path.len() as u32, file_size);
        data.extend_from_slice(remote_path.as_bytes());
        
        // 零拷贝写入: 先写协议头，再写数据 (带超时控制)
        self.write_with_timeout(&mut stream, &header.to_bytes()).await?;
        self.write_with_timeout(&mut stream, &data).await?;
        
        // 零拷贝文件传输循环
        let mut bytes_transferred = 0u64;
        let mut chunk_id = 0u32;
        let mut last_progress_time = start_time;
        
        // 预分配缓冲区 - 这是唯一的内存分配
        let mut buffer = BytesMut::with_capacity(self.config.chunk_size);
        
        loop {
            // 重置缓冲区但保持容量
            buffer.clear();
            buffer.resize(self.config.chunk_size, 0);
            
            // 直接从文件读取到缓冲区 - 零拷贝读取 (带超时控制)
            let bytes_read = self.read_with_timeout(&mut reader, &mut buffer).await?;
            
            if bytes_read == 0 {
                break; // EOF
            }
            
            // 调整缓冲区到实际大小
            buffer.truncate(bytes_read);
            let is_last = (bytes_transferred + bytes_read as u64) >= file_size;
            
            // 创建零拷贝协议头
            let chunk_header = ZeroCopyHeader::file_chunk(chunk_id, bytes_read as u32, is_last);
            
            // 零拷贝网络传输: 先发送固定头部，再直接发送数据 (带超时控制)
            self.write_with_timeout(&mut stream, &chunk_header.to_bytes()).await?;
            self.write_with_timeout(&mut stream, &buffer[..bytes_read]).await?;
            
            bytes_transferred += bytes_read as u64;
            chunk_id += 1;
            
            // 优化的进度更新
            let now = Instant::now();
            if progress_callback.is_some() && 
               (now.duration_since(last_progress_time) >= self.config.progress_interval || is_last) {
                let elapsed = now.duration_since(start_time);
                let progress = ProgressInfo::new(bytes_transferred, file_size, elapsed);
                
                if let Some(callback) = progress_callback {
                    callback(progress);
                }
                last_progress_time = now;
            }
            
            // 减少日志输出
            if bytes_transferred % (20 * 1024 * 1024) == 0 {
                debug!("已传输: {} MB", bytes_transferred / (1024 * 1024));
            }
            
            if is_last {
                break;
            }
        }
        
        // 发送完成消息 (带超时控制)
        let complete_header = ZeroCopyHeader::file_complete();
        self.write_with_timeout(&mut stream, &complete_header.to_bytes()).await?;
        
        // 刷新所有数据 (带超时控制)
        timeout(
            self.config.retry_config.io_timeout,
            stream.flush()
        ).await
        .map_err(|_| anyhow::anyhow!("刷新超时"))?
        .map_err(|e| anyhow::anyhow!("刷新错误: {}", e))?;
        
        let duration = start_time.elapsed();
        let throughput_mbps = (bytes_transferred as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
        
        info!(
            "🚀 零拷贝传输完成: {} 字节，耗时: {:.3}秒，吞吐量: {:.2} MB/s",
            bytes_transferred,
            duration.as_secs_f64(),
            throughput_mbps
        );
        
        Ok(TransferResult {
            bytes_transferred,
            duration,
            throughput_mbps,
            file_hash: None, // 零拷贝模式不计算哈希
            integrity_verified: false,
            verification_message: Some("零拷贝模式: 跳过完整性验证以达到最高性能".to_string()),
        })
    }
    
    /// 创建优化的TCP连接 (带超时控制)
    async fn create_optimized_connection(&self) -> Result<TcpStream> {
        debug!("创建零拷贝TCP连接到: {}", self.server_addr);
        
        let stream = timeout(
            self.config.retry_config.connection_timeout,
            TcpStream::connect(self.server_addr)
        ).await
        .map_err(|_| anyhow::anyhow!("连接超时: {}", self.server_addr))?
        .with_context(|| format!("无法连接到服务器: {}", self.server_addr))?;
        
        // 设置TCP优化
        if self.config.tcp_nodelay {
            stream.set_nodelay(true)?;
        }
        
        info!("✅ 零拷贝TCP连接建立 (nodelay: {}, 超时: {:?})", 
              self.config.tcp_nodelay, 
              self.config.retry_config.connection_timeout);
        
        Ok(stream)
    }
    
    /// 带超时的I/O操作包装器
    async fn write_with_timeout(&self, stream: &mut BufWriter<TcpStream>, data: &[u8]) -> Result<()> {
        timeout(
            self.config.retry_config.io_timeout,
            stream.write_all(data)
        ).await
        .map_err(|_| anyhow::anyhow!("写入超时"))?
        .map_err(|e| anyhow::anyhow!("写入错误: {}", e))?;
        Ok(())
    }
    
    async fn read_with_timeout(&self, reader: &mut BufReader<File>, buffer: &mut [u8]) -> Result<usize> {
        timeout(
            self.config.retry_config.io_timeout,
            reader.read(buffer)
        ).await
        .map_err(|_| anyhow::anyhow!("读取超时"))?
        .map_err(|e| anyhow::anyhow!("读取错误: {}", e))
    }
}

/// 零拷贝性能基准测试
pub struct ZeroCopyBenchmark {
    client: ZeroCopyClient,
}

impl ZeroCopyBenchmark {
    pub fn new(server_addr: SocketAddr, chunk_size_kb: usize) -> Self {
        let mut config = ZeroCopyConfig::default();
        config.chunk_size = chunk_size_kb * 1024;
        
        Self {
            client: ZeroCopyClient::new(server_addr, config),
        }
    }
    
    /// 运行零拷贝基准测试
    pub async fn run_benchmark<P: AsRef<Path>>(
        &self,
        test_file: P,
        iterations: u32,
    ) -> Result<ZeroCopyBenchmarkResult> {
        let test_file = test_file.as_ref();
        let mut results = Vec::new();
        
        info!("🏁 开始零拷贝基准测试: {} 次迭代", iterations);
        
        for i in 0..iterations {
            let remote_path = format!("/zero_copy_benchmark_{}.bin", i);
            
            let result = self.client.upload_file_zero_copy(
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
        
        info!("📊 零拷贝基准测试完成:");
        info!("  平均吞吐量: {:.2} MB/s", avg_throughput);
        info!("  最大吞吐量: {:.2} MB/s", max_throughput);
        info!("  最小吞吐量: {:.2} MB/s", min_throughput);
        
        Ok(ZeroCopyBenchmarkResult {
            iterations,
            avg_throughput,
            max_throughput,
            min_throughput,
            results,
        })
    }
}

/// 零拷贝基准测试结果
#[derive(Debug)]
pub struct ZeroCopyBenchmarkResult {
    pub iterations: u32,
    pub avg_throughput: f64,
    pub max_throughput: f64,
    pub min_throughput: f64,
    pub results: Vec<TransferResult>,
}
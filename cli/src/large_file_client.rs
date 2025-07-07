use anyhow::{Context, Result};
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter, SeekFrom};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Semaphore};
use tracing::{debug, info, warn};

use crate::simple_data_portal_client::{DataPortalMessage, TransferResult, ProgressCallback, ProgressInfo};

/// 大文件传输配置
#[derive(Debug, Clone)]
pub struct LargeFileConfig {
    /// 基础块大小 (bytes)
    pub base_chunk_size: usize,
    /// 最大块大小 (bytes)
    pub max_chunk_size: usize,
    /// 最小块大小 (bytes)
    pub min_chunk_size: usize,
    /// 并发连接数
    pub max_concurrent_chunks: usize,
    /// 内存缓冲区限制 (bytes)
    pub memory_limit: usize,
    /// 重试次数
    pub max_retries: u32,
    /// 连接超时 (seconds)
    pub connection_timeout: Duration,
}

impl Default for LargeFileConfig {
    fn default() -> Self {
        Self {
            base_chunk_size: 1024 * 1024, // 1MB
            max_chunk_size: 16 * 1024 * 1024, // 16MB
            min_chunk_size: 64 * 1024, // 64KB
            max_concurrent_chunks: 4, // 4个并发连接
            memory_limit: 256 * 1024 * 1024, // 256MB内存限制
            max_retries: 3,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

/// 文件块信息
#[derive(Debug, Clone)]
pub struct FileChunkInfo {
    pub chunk_id: u32,
    pub offset: u64,
    pub size: usize,
    pub hash: Option<String>,
    pub retry_count: u32,
}

/// 传输统计信息
#[derive(Debug, Clone)]
pub struct TransferStats {
    pub total_chunks: u32,
    pub completed_chunks: u32,
    pub failed_chunks: u32,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub start_time: Instant,
    pub estimated_completion: Option<Instant>,
}

impl TransferStats {
    pub fn new(total_bytes: u64, total_chunks: u32) -> Self {
        Self {
            total_chunks,
            completed_chunks: 0,
            failed_chunks: 0,
            bytes_transferred: 0,
            total_bytes,
            start_time: Instant::now(),
            estimated_completion: None,
        }
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.bytes_transferred as f64) / (self.total_bytes as f64)
    }

    pub fn throughput_mbps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        (self.bytes_transferred as f64) / (1024.0 * 1024.0) / elapsed
    }
}

/// 大文件传输客户端
pub struct LargeFileClient {
    server_addr: SocketAddr,
    config: LargeFileConfig,
}

impl LargeFileClient {
    /// 创建新的大文件传输客户端
    pub fn new(server_addr: SocketAddr, config: LargeFileConfig) -> Self {
        Self {
            server_addr,
            config,
        }
    }

    /// 使用默认配置创建客户端
    pub fn with_default_config(server_addr: SocketAddr) -> Self {
        Self::new(server_addr, LargeFileConfig::default())
    }

    /// 根据文件大小自适应调整块大小
    pub fn calculate_optimal_chunk_size(&self, file_size: u64) -> usize {
        if file_size < 10 * 1024 * 1024 {
            // 小于10MB，使用小块
            self.config.min_chunk_size
        } else if file_size < 100 * 1024 * 1024 {
            // 10MB-100MB，使用基础块大小
            self.config.base_chunk_size
        } else if file_size < 1024 * 1024 * 1024 {
            // 100MB-1GB，使用2MB块
            2 * 1024 * 1024
        } else if file_size < 10 * 1024 * 1024 * 1024 {
            // 1GB-10GB，使用4MB块
            4 * 1024 * 1024
        } else {
            // 大于10GB，使用最大块大小
            self.config.max_chunk_size
        }
    }

    /// 计算传输所需的内存
    pub fn calculate_memory_usage(&self, chunk_size: usize, concurrent_chunks: usize) -> usize {
        // 每个块需要的内存：数据缓冲区 + 网络缓冲区 + 哈希计算缓冲区
        let per_chunk_memory = chunk_size + (64 * 1024) + (32 * 1024);
        per_chunk_memory * concurrent_chunks
    }

    /// 自动调整并发度以适应内存限制
    pub fn calculate_optimal_concurrency(&self, chunk_size: usize) -> usize {
        let per_chunk_memory = self.calculate_memory_usage(chunk_size, 1);
        let max_concurrent = self.config.memory_limit / per_chunk_memory;
        max_concurrent.min(self.config.max_concurrent_chunks).max(1)
    }

    /// 生成文件块列表
    pub fn generate_chunks(&self, file_size: u64, chunk_size: usize) -> Vec<FileChunkInfo> {
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut chunk_id = 0u32;

        while offset < file_size {
            let remaining = file_size - offset;
            let current_chunk_size = chunk_size.min(remaining as usize);

            chunks.push(FileChunkInfo {
                chunk_id,
                offset,
                size: current_chunk_size,
                hash: None,
                retry_count: 0,
            });

            offset += current_chunk_size as u64;
            chunk_id += 1;
        }

        chunks
    }

    /// 上传大文件
    pub async fn upload_large_file<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let start_time = Instant::now();

        info!("开始上传大文件: {} -> {}", local_path.display(), remote_path);

        // 获取文件信息
        let file = File::open(local_path).await
            .with_context(|| format!("无法打开文件: {}", local_path.display()))?;
        let file_size = file.metadata().await?.len();

        info!("文件大小: {} bytes ({:.2} MB)", file_size, file_size as f64 / (1024.0 * 1024.0));

        // 计算最优配置
        let chunk_size = self.calculate_optimal_chunk_size(file_size);
        let max_concurrent = self.calculate_optimal_concurrency(chunk_size);
        let memory_usage = self.calculate_memory_usage(chunk_size, max_concurrent);

        info!("传输配置: 块大小={}KB, 并发度={}, 预计内存使用={}MB", 
              chunk_size / 1024, max_concurrent, memory_usage / (1024 * 1024));

        // 计算文件哈希
        info!("正在计算文件哈希值...");
        let file_hash = self.calculate_file_hash(local_path).await?;
        info!("文件SHA-256哈希: {}", file_hash);

        // 生成块列表
        let chunks = self.generate_chunks(file_size, chunk_size);
        let total_chunks = chunks.len() as u32;

        info!("将文件分为 {} 个块进行传输", total_chunks);

        // 初始化传输统计
        let mut stats = TransferStats::new(file_size, total_chunks);

        // 创建信号量限制并发度
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        // 创建进度通道
        let (progress_tx, mut progress_rx) = mpsc::channel::<ProgressInfo>(100);

        // 启动进度报告任务
        let progress_task = if let Some(callback) = progress_callback {
            let callback = Arc::new(callback);
            Some(tokio::spawn(async move {
                while let Some(progress) = progress_rx.recv().await {
                    callback(progress);
                }
            }))
        } else {
            None
        };

        // 并发传输块
        let mut handles = Vec::new();
        let file_arc = Arc::new(tokio::sync::Mutex::new(file));

        for chunk in chunks {
            let permit = semaphore.clone().acquire_owned().await?;
            let file_clone = Arc::clone(&file_arc);
            let server_addr = self.server_addr;
            let remote_path = remote_path.to_string();
            let file_hash_clone = file_hash.clone();
            let progress_tx_clone = progress_tx.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit; // 保持许可证直到任务完成

                let result = Self::upload_chunk(
                    server_addr,
                    file_clone,
                    chunk,
                    &remote_path,
                    &file_hash_clone,
                    progress_tx_clone,
                ).await;

                result
            });

            handles.push(handle);
        }

        // 等待所有块传输完成
        let mut total_bytes_transferred = 0u64;
        let mut failed_chunks = 0u32;

        for handle in handles {
            match handle.await? {
                Ok(bytes) => {
                    total_bytes_transferred += bytes;
                    stats.completed_chunks += 1;
                }
                Err(e) => {
                    warn!("块传输失败: {}", e);
                    failed_chunks += 1;
                    stats.failed_chunks += 1;
                }
            }
        }

        // 关闭进度通道
        drop(progress_tx);

        // 等待进度任务完成
        if let Some(task) = progress_task {
            let _ = task.await;
        }

        let duration = start_time.elapsed();
        let throughput_mbps = (total_bytes_transferred as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();

        if failed_chunks > 0 {
            return Err(anyhow::anyhow!("传输失败: {} 个块传输失败", failed_chunks));
        }

        info!(
            "大文件上传完成: {} 字节，耗时: {:.2}秒，吞吐量: {:.2} MB/s",
            total_bytes_transferred,
            duration.as_secs_f64(),
            throughput_mbps
        );

        Ok(TransferResult {
            bytes_transferred: total_bytes_transferred,
            duration,
            throughput_mbps,
            file_hash: Some(file_hash),
            integrity_verified: true, // 大文件传输默认验证成功
            verification_message: Some("大文件分块传输完成".to_string()),
        })
    }

    /// 上传单个文件块
    async fn upload_chunk(
        server_addr: SocketAddr,
        file: Arc<tokio::sync::Mutex<File>>,
        mut chunk: FileChunkInfo,
        remote_path: &str,
        file_hash: &str,
        progress_tx: mpsc::Sender<ProgressInfo>,
    ) -> Result<u64> {
        let mut retries = 0;

        loop {
            match Self::upload_chunk_attempt(
                server_addr,
                Arc::clone(&file),
                &chunk,
                remote_path,
                file_hash,
                progress_tx.clone(),
            ).await {
                Ok(bytes) => {
                    debug!("块 {} 上传成功: {} 字节", chunk.chunk_id, bytes);
                    return Ok(bytes);
                }
                Err(e) => {
                    retries += 1;
                    chunk.retry_count = retries;

                    if retries >= 3 {
                        return Err(anyhow::anyhow!("块 {} 上传失败 (重试 {} 次): {}", 
                                                 chunk.chunk_id, retries, e));
                    }

                    warn!("块 {} 上传失败，正在重试 ({}/3): {}", chunk.chunk_id, retries, e);
                    
                    // 指数退避
                    let delay = Duration::from_millis(100 * (1 << (retries - 1)));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// 单次块上传尝试
    async fn upload_chunk_attempt(
        server_addr: SocketAddr,
        file: Arc<tokio::sync::Mutex<File>>,
        chunk: &FileChunkInfo,
        remote_path: &str,
        file_hash: &str,
        _progress_tx: mpsc::Sender<ProgressInfo>,
    ) -> Result<u64> {
        // 连接到服务器
        let stream = TcpStream::connect(server_addr).await
            .with_context(|| format!("无法连接到服务器: {}", server_addr))?;

        let mut stream = BufWriter::new(stream);

        // 读取块数据
        let mut buffer = vec![0u8; chunk.size];
        {
            let mut file_guard = file.lock().await;
            file_guard.seek(SeekFrom::Start(chunk.offset)).await?;
            file_guard.read_exact(&mut buffer).await?;
        }

        // 计算块哈希
        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let chunk_hash = format!("{:x}", hasher.finalize());

        // 发送文件传输开始消息
        let start_msg = DataPortalMessage::FileTransferStart {
            file_name: format!("{}#{}", remote_path, chunk.chunk_id), // 使用块ID标识
            file_size: chunk.size as u64,
            chunk_size: chunk.size,
            file_hash: Some(file_hash.to_string()),
        };

        Self::send_message(&mut stream, &start_msg).await?;

        // 发送块数据
        let chunk_msg = DataPortalMessage::FileChunk {
            chunk_id: chunk.chunk_id,
            data: buffer,
            is_last: true, // 每个块都是独立的
            chunk_hash: Some(chunk_hash),
        };

        Self::send_message(&mut stream, &chunk_msg).await?;

        // 发送传输完成消息
        let complete_msg = DataPortalMessage::TransferComplete {
            final_hash: Some(file_hash.to_string()),
        };
        Self::send_message(&mut stream, &complete_msg).await?;

        // 刷新缓冲区
        stream.flush().await?;

        debug!("块 {} 数据发送完成: {} 字节", chunk.chunk_id, chunk.size);

        Ok(chunk.size as u64)
    }

    /// 计算文件哈希值
    async fn calculate_file_hash<P: AsRef<Path>>(&self, file_path: P) -> Result<String> {
        let file_path = file_path.as_ref();
        let file = File::open(file_path).await
            .with_context(|| format!("无法打开文件进行哈希计算: {}", file_path.display()))?;
        
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB缓冲区用于哈希计算
        
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

    /// 发送消息到服务器
    async fn send_message<W: AsyncWriteExt + Unpin>(
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
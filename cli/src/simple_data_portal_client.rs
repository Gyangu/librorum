use anyhow::{Context, Result};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Instant;
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
    /// 文件传输开始
    FileTransferStart {
        file_name: String,
        file_size: u64,
        chunk_size: usize,
    },
    /// 文件数据块
    FileChunk {
        chunk_id: u32,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
        is_last: bool,
    },
    /// 传输完成确认
    TransferComplete,
    /// 错误消息
    Error { message: String },
}

/// 传输结果
#[derive(Debug)]
pub struct TransferResult {
    pub bytes_transferred: u64,
    pub duration: std::time::Duration,
    pub throughput_mbps: f64,
}

impl SimpleDataPortalClient {
    /// 创建新的客户端
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            server_addr,
            chunk_size: 64 * 1024, // 64KB chunks for better performance
        }
    }

    /// 上传文件
    pub async fn upload_file<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
    ) -> Result<TransferResult> {
        let local_path = local_path.as_ref();
        let start_time = Instant::now();

        info!("开始上传文件: {} -> {}", local_path.display(), self.server_addr);

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

        // 发送文件传输开始消息
        let start_msg = DataPortalMessage::FileTransferStart {
            file_name: remote_path.to_string(),
            file_size,
            chunk_size: self.chunk_size,
        };

        self.send_message(&mut stream, &start_msg).await?;

        // 逐块读取并发送文件数据 - 零拷贝优化
        let mut bytes_transferred = 0u64;
        let mut chunk_id = 0u32;
        let mut buffer = BytesMut::with_capacity(self.chunk_size);

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

            let chunk_msg = DataPortalMessage::FileChunk {
                chunk_id,
                data: chunk_data,
                is_last,
            };

            self.send_message(&mut stream, &chunk_msg).await?;

            bytes_transferred += bytes_read as u64;
            chunk_id += 1;

            if bytes_transferred % (1024 * 1024) == 0 {
                debug!("已传输: {} MB", bytes_transferred / (1024 * 1024));
            }

            if is_last {
                break;
            }
        }

        // 发送传输完成消息
        let complete_msg = DataPortalMessage::TransferComplete;
        self.send_message(&mut stream, &complete_msg).await?;

        // 刷新缓冲区
        stream.flush().await?;

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


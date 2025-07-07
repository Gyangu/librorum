//! Data Portal 客户端实现
//! 
//! 为CLI客户端提供高性能文件传输功能

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use bytes::{BytesMut, Bytes};
use tracing::info;
use uuid::Uuid;

use data_portal_core::{
    TransportManager, TransportType, NodeInfo, Language,
    manager::TransportManagerConfig,
    DataPortalTransport,
};

/// Data Portal 客户端
pub struct DataPortalClient {
    transport_manager: TransportManager,
    local_node: NodeInfo,
}

/// 文件传输配置
#[derive(Debug, Clone)]
pub struct TransferConfig {
    /// 传输模式
    pub mode: TransportType,
    /// 分块大小
    pub chunk_size: usize,
    /// 是否启用压缩
    pub enable_compression: bool,
    /// 超时时间（秒）
    pub timeout_secs: u64,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            mode: TransportType::DataPortal,
            chunk_size: 1024 * 1024, // 1MB
            enable_compression: false,
            timeout_secs: 300, // 5分钟
        }
    }
}

/// 传输进度回调
pub type ProgressCallback = Box<dyn Fn(u64, u64, f64) + Send + Sync>;

/// 传输统计信息
#[derive(Debug, Clone)]
pub struct TransferMetrics {
    pub session_id: String,
    pub bytes_transferred: u64,
    pub duration_secs: f64,
    pub average_rate_mbps: f64,
    pub transport_mode: TransportType,
    pub compression_enabled: bool,
    pub chunk_count: u32,
}

impl DataPortalClient {
    /// 创建新的Data Portal客户端
    pub fn new() -> Result<Self> {
        let local_node = NodeInfo::local(format!("cli_{}", Uuid::new_v4()), Language::Rust);

        let config = TransportManagerConfig::default();
        let transport_manager = TransportManager::new(config);

        Ok(Self {
            transport_manager,
            local_node,
        })
    }

    /// 上传文件 - 零拷贝优化版本
    pub async fn upload_file(
        &mut self,
        local_path: &Path,
        remote_endpoint: SocketAddr,
        session_id: &str,
        config: TransferConfig,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferMetrics> {
        info!("开始上传文件: {} -> {}", local_path.display(), remote_endpoint);
        
        let start_time = Instant::now();
        let file = File::open(local_path).await
            .with_context(|| format!("无法打开文件: {}", local_path.display()))?;
        
        let file_size = file.metadata().await?.len();
        let mut total_sent = 0u64;
        
        // 使用缓冲读取器提高I/O性能
        let mut reader = BufReader::with_capacity(config.chunk_size * 2, file);
        
        // 预分配字节缓冲区，避免重复分配
        let mut buffer = BytesMut::with_capacity(config.chunk_size);
        
        // 创建目标节点信息
        let destination = NodeInfo::remote(
            format!("server_{}", session_id),
            Language::Rust,
            remote_endpoint.to_string()
        );

        // 发送文件头信息
        let file_header = FileHeader {
            session_id: session_id.to_string(),
            file_name: local_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            file_size,
            chunk_size: config.chunk_size,
            compression: config.enable_compression,
        };
        
        // 使用transport manager发送文件头
        self.transport_manager.send(&file_header, &destination).await
            .with_context(|| "发送文件头失败")?;

        // 分块发送文件数据 - 零拷贝优化
        let mut chunk_id = 0u32;
        loop {
            // 清空buffer但保留容量
            buffer.clear();
            
            // 直接读取到BytesMut，避免额外拷贝
            let bytes_read = reader.read_buf(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            // 冻结buffer为不可变Bytes，零拷贝
            let chunk_data = buffer.split().freeze();
            
            // 根据需要压缩数据
            let final_data = if config.enable_compression {
                // 压缩时需要拷贝，但使用更高效的方式
                compress_data_zerocopy(&chunk_data)?
            } else {
                chunk_data
            };

            let chunk = FileChunk {
                session_id: session_id.to_string(),
                chunk_id,
                data: final_data, // 零拷贝使用 Bytes
                is_final: bytes_read < config.chunk_size,
            };

            // 发送数据块
            self.transport_manager.send(&chunk, &destination).await
                .with_context(|| format!("发送数据块 {} 失败", chunk_id))?;

            total_sent += bytes_read as u64;
            chunk_id += 1;

            // 更新进度
            if let Some(ref callback) = progress_callback {
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = total_sent as f64 / elapsed / 1024.0 / 1024.0; // MB/s
                callback(total_sent, file_size, rate);
            }
        }

        let elapsed = start_time.elapsed();
        let metrics = TransferMetrics {
            session_id: session_id.to_string(),
            bytes_transferred: total_sent,
            duration_secs: elapsed.as_secs_f64(),
            average_rate_mbps: (total_sent as f64 / elapsed.as_secs_f64()) / 1024.0 / 1024.0,
            transport_mode: config.mode,
            compression_enabled: config.enable_compression,
            chunk_count: chunk_id,
        };
        
        info!("文件上传完成: {} bytes in {:.2}s ({:.2} MB/s)", 
              total_sent, elapsed.as_secs_f64(), metrics.average_rate_mbps);

        Ok(metrics)
    }

    /// 下载文件 - 零拷贝优化版本
    pub async fn download_file(
        &mut self,
        remote_endpoint: SocketAddr,
        session_id: &str,
        local_path: &Path,
        config: TransferConfig,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<TransferMetrics> {
        info!("开始下载文件: {} -> {}", remote_endpoint, local_path.display());
        
        let start_time = Instant::now();
        
        // 创建源节点信息
        let source = NodeInfo::remote(
            format!("server_{}", session_id),
            Language::Rust,
            remote_endpoint.to_string()
        );
        
        // 发送下载请求
        let download_request = DownloadRequest {
            session_id: session_id.to_string(),
        };
        
        self.transport_manager.send(&download_request, &source).await?;

        // 接收文件头
        let file_header: FileHeader = self.transport_manager.receive(&source, config.timeout_secs * 1000).await?;
        
        info!("接收文件: {} ({} bytes)", file_header.file_name, file_header.file_size);

        let output_file = File::create(local_path).await
            .with_context(|| format!("无法创建文件: {}", local_path.display()))?;
        
        // 使用缓冲写入器提高I/O性能
        let mut writer = BufWriter::with_capacity(config.chunk_size * 2, output_file);
        
        let mut total_received = 0u64;

        // 接收文件数据块 - 零拷贝优化
        loop {
            let chunk: FileChunk = self.transport_manager.receive(&source, config.timeout_secs * 1000).await?;

            // 零拷贝解压缩
            let final_data = if file_header.compression {
                decompress_data_zerocopy(&chunk.data)?
            } else {
                // 直接使用Bytes，避免拷贝
                chunk.data
            };

            // 高效写入，避免不必要的拷贝
            writer.write_all(&final_data).await?;
            total_received += final_data.len() as u64;

            // 更新进度
            if let Some(ref callback) = progress_callback {
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = total_received as f64 / elapsed / 1024.0 / 1024.0;
                callback(total_received, file_header.file_size, rate);
            }

            if chunk.is_final {
                break;
            }
        }

        writer.flush().await?;
        
        let elapsed = start_time.elapsed();
        let metrics = TransferMetrics {
            session_id: session_id.to_string(),
            bytes_transferred: total_received,
            duration_secs: elapsed.as_secs_f64(),
            average_rate_mbps: (total_received as f64 / elapsed.as_secs_f64()) / 1024.0 / 1024.0,
            transport_mode: config.mode,
            compression_enabled: file_header.compression,
            chunk_count: 0, // 下载时不知道确切的块数
        };
        
        info!("文件下载完成: {} bytes in {:.2}s ({:.2} MB/s)", 
              total_received, elapsed.as_secs_f64(), metrics.average_rate_mbps);

        Ok(metrics)
    }
}

/// 文件头信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FileHeader {
    session_id: String,
    file_name: String,
    file_size: u64,
    chunk_size: usize,
    compression: bool,
}

/// 文件数据块
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FileChunk {
    session_id: String,
    chunk_id: u32,
    #[serde(serialize_with = "serialize_bytes", deserialize_with = "deserialize_bytes")]
    data: Bytes,
    is_final: bool,
}

/// 下载请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DownloadRequest {
    session_id: String,
}

/// 简单的压缩函数（使用zlib）
fn compress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

/// 简单的解压函数
fn decompress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// 零拷贝压缩函数（使用Bytes）
fn compress_data_zerocopy(data: &bytes::Bytes) -> Result<bytes::Bytes> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(bytes::Bytes::from(compressed))
}

/// 零拷贝解压函数（使用Bytes）
fn decompress_data_zerocopy(data: &Bytes) -> Result<bytes::Bytes> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    use std::io::Cursor;
    
    let mut decoder = ZlibDecoder::new(Cursor::new(&data[..]));
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(bytes::Bytes::from(decompressed))
}

/// 自定义 Bytes 序列化
fn serialize_bytes<S>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serde_bytes::serialize(&bytes[..], serializer)
}

/// 自定义 Bytes 反序列化
fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let vec: Vec<u8> = serde_bytes::deserialize(deserializer)?;
    Ok(Bytes::from(vec))
}
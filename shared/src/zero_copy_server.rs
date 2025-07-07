use anyhow::{Context, Result};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// 零拷贝协议头 - 与客户端保持一致
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ZeroCopyHeader {
    /// 消息类型: 1=FileStart, 2=FileChunk, 3=FileComplete
    pub msg_type: u8,
    /// 块ID (仅对FileChunk有效)
    pub chunk_id: u32,
    /// 数据长度 (仅对FileChunk有效，其他消息为附加数据长度)
    pub data_len: u32,
    /// 标志位: bit0=is_last, bit1=has_hash, bit2-7=reserved
    pub flags: u8,
    /// 保留字段，用于对齐和未来扩展
    pub reserved: [u8; 6],
}

impl ZeroCopyHeader {
    pub const SIZE: usize = std::mem::size_of::<Self>();
    
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
    
    /// 检查是否为最后一个块
    pub fn is_last(&self) -> bool {
        (self.flags & 1) != 0
    }
}

/// 零拷贝Data Portal服务器
pub struct ZeroCopyDataPortalServer {
    bind_addr: SocketAddr,
    listener: Option<TcpListener>,
}

impl ZeroCopyDataPortalServer {
    /// 创建零拷贝服务器
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            listener: None,
        }
    }
    
    /// 启动服务器
    pub async fn start(&mut self) -> Result<()> {
        info!("🚀 启动零拷贝Data Portal服务器: {}", self.bind_addr);
        
        let listener = TcpListener::bind(self.bind_addr).await
            .with_context(|| format!("无法绑定地址: {}", self.bind_addr))?;
        
        info!("✅ 零拷贝Data Portal服务器已启动: {}", self.bind_addr);
        self.listener = Some(listener);
        
        Ok(())
    }
    
    /// 运行服务器主循环
    pub async fn run(&mut self) -> Result<()> {
        self.start().await?;
        
        if let Some(ref listener) = self.listener {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        info!("🔗 零拷贝连接来自: {}", addr);
                        
                        // 为每个连接启动异步处理任务
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_zero_copy_connection(stream).await {
                                warn!("处理零拷贝连接失败: {}", e);
                            }
                        });
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
    
    /// 处理单个零拷贝连接 (带错误恢复和超时控制)
    async fn handle_zero_copy_connection(stream: TcpStream) -> Result<()> {
        info!("⚡ 开始处理零拷贝连接");
        
        // 设置TCP选项
        if let Err(e) = stream.set_nodelay(true) {
            warn!("设置TCP nodelay失败: {}", e);
        }
        
        let mut reader = BufReader::with_capacity(4 * 1024 * 1024, stream); // 4MB读缓冲区
        let mut total_bytes = 0u64;
        let start_time = Instant::now();
        let mut current_file: Option<BufWriter<File>> = None;
        let mut current_file_name = String::new();
        let mut expected_file_size = 0u64;
        let mut received_file_size = 0u64;
        
        // 预分配协议头缓冲区
        let mut header_buf = vec![0u8; ZeroCopyHeader::SIZE];
        
        // 超时配置
        let io_timeout = Duration::from_secs(60);
        let connection_timeout = Duration::from_secs(300); // 5分钟连接超时
        let connection_start = Instant::now();
        
        loop {
            // 检查连接超时
            if connection_start.elapsed() > connection_timeout {
                error!("❌ 连接超时，强制关闭");
                break;
            }
            
            // 读取协议头 - 固定16字节 (带超时控制)
            let header_result = timeout(io_timeout, reader.read_exact(&mut header_buf)).await;
            
            match header_result {
                Ok(Ok(_)) => {
                    // 成功读取协议头
                },
                Ok(Err(e)) => {
                    warn!("协议头读取失败: {}", e);
                    break;
                },
                Err(_) => {
                    error!("协议头读取超时");
                    break;
                }
            }
            
            // 解析协议头
            let header = match ZeroCopyHeader::from_bytes(&header_buf) {
                Ok(h) => h,
                Err(e) => {
                    warn!("解析协议头失败: {}", e);
                    break;
                }
            };
            
            let msg_type = header.msg_type;
            let data_len = header.data_len;
            debug!("📦 收到消息类型: {}, 数据长度: {}", msg_type, data_len);
            
            match header.msg_type {
                1 => { // FileStart
                    // 读取附加数据：file_size (8字节) + file_name (带超时控制)
                    let data_len = header.data_len;
                    if data_len < 8 {
                        error!("FileStart消息数据长度无效: {}", data_len);
                        break;
                    }
                    
                    let mut data_buf = vec![0u8; data_len as usize];
                    match timeout(io_timeout, reader.read_exact(&mut data_buf)).await {
                        Ok(Ok(_)) => {},
                        Ok(Err(e)) => {
                            error!("读取FileStart附加数据失败: {}", e);
                            break;
                        },
                        Err(_) => {
                            error!("读取FileStart附加数据超时");
                            break;
                        }
                    }
                    
                    // 解析file_size和file_name
                    let file_size = u64::from_le_bytes([
                        data_buf[0], data_buf[1], data_buf[2], data_buf[3],
                        data_buf[4], data_buf[5], data_buf[6], data_buf[7]
                    ]);
                    
                    let file_name = String::from_utf8_lossy(&data_buf[8..]).to_string();
                    current_file_name = file_name.clone();
                    expected_file_size = file_size;
                    received_file_size = 0;
                    
                    info!("📁 开始接收文件: {} ({} 字节)", file_name, file_size);
                    
                    // 创建输出文件 - 简单的路径处理
                    let safe_path = file_name.trim_start_matches('/');
                    let output_path = format!("./uploads/{}", safe_path);
                    
                    // 确保uploads目录存在 (带错误处理)
                    if let Some(parent) = Path::new(&output_path).parent() {
                        if let Err(e) = tokio::fs::create_dir_all(parent).await {
                            error!("创建上传目录失败: {} - {}", parent.display(), e);
                            break;
                        }
                    }
                    
                    // 关闭当前文件 (如果存在)
                    if let Some(mut old_file) = current_file.take() {
                        if let Err(e) = old_file.flush().await {
                            warn!("关闭上一个文件时刷新失败: {}", e);
                        }
                    }
                    
                    match OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&output_path)
                        .await 
                    {
                        Ok(file) => {
                            current_file = Some(BufWriter::with_capacity(4 * 1024 * 1024, file));
                            total_bytes = 0;
                            info!("✅ 成功创建文件: {}", output_path);
                        },
                        Err(e) => {
                            error!("无法创建文件: {} - {}", output_path, e);
                            break;
                        }
                    }
                }
                
                2 => { // FileChunk
                    let data_len = header.data_len;
                    let chunk_id = header.chunk_id;
                    if data_len == 0 {
                        debug!("收到空数据块，跳过");
                        continue;
                    }
                    
                    // 直接读取数据块到缓冲区并写入文件 - 零拷贝传输
                    if let Some(ref mut file_writer) = current_file {
                        let mut chunk_data = BytesMut::with_capacity(data_len as usize);
                        unsafe {
                            chunk_data.set_len(data_len as usize);
                        }
                        
                        // 零拷贝读取：直接从网络读取到缓冲区 (带超时控制)
                        match timeout(io_timeout, reader.read_exact(&mut chunk_data)).await {
                            Ok(Ok(_)) => {},
                            Ok(Err(e)) => {
                                error!("读取数据块失败: {}", e);
                                break;
                            },
                            Err(_) => {
                                error!("读取数据块超时");
                                break;
                            }
                        }
                        
                        // 零拷贝写入：直接从缓冲区写入文件 (带超时控制)
                        match timeout(io_timeout, file_writer.write_all(&chunk_data)).await {
                            Ok(Ok(_)) => {},
                            Ok(Err(e)) => {
                                error!("写入文件失败: {}", e);
                                break;
                            },
                            Err(_) => {
                                error!("写入文件超时");
                                break;
                            }
                        }
                        
                        total_bytes += data_len as u64;
                        received_file_size += data_len as u64;
                        
                        // 检查文件大小是否超出预期
                        if received_file_size > expected_file_size {
                            error!("❌ 接收的文件大小超出预期: {} > {}", received_file_size, expected_file_size);
                            break;
                        }
                        
                        debug!("✅ 接收数据块 {}: {} 字节 (总计: {}/{})", 
                               chunk_id, data_len, received_file_size, expected_file_size);
                        
                        // 减少日志输出以提高性能
                        if total_bytes % (50 * 1024 * 1024) == 0 { // 每50MB输出一次
                            info!("已接收: {} MB / {} MB ({:.1}%)", 
                                  total_bytes / (1024 * 1024),
                                  expected_file_size / (1024 * 1024),
                                  (received_file_size as f64 / expected_file_size as f64) * 100.0);
                        }
                        
                        if header.is_last() {
                            // 验证文件大小完整性
                            if received_file_size != expected_file_size {
                                error!("❌ 文件大小不匹配: 接收 {} 字节, 预期 {} 字节", 
                                       received_file_size, expected_file_size);
                                break;
                            }
                            
                            // 刷新并关闭文件 (带超时控制)
                            match timeout(io_timeout, file_writer.flush()).await {
                                Ok(Ok(_)) => {
                                    info!("✅ 文件刷新成功");
                                },
                                Ok(Err(e)) => {
                                    error!("刷新文件失败: {}", e);
                                    break;
                                },
                                Err(_) => {
                                    error!("刷新文件超时");
                                    break;
                                }
                            }
                            
                            let duration = start_time.elapsed();
                            let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
                            
                            info!(
                                "🎉 文件接收完成: {} ({} 字节), 耗时: {:.3}秒, 吞吐量: {:.2} MB/s, 完整性: ✅",
                                current_file_name,
                                total_bytes,
                                duration.as_secs_f64(),
                                throughput_mbps
                            );
                            
                            current_file = None;
                        }
                    } else {
                        warn!("收到数据块但没有打开的文件");
                        break;
                    }
                }
                
                3 => { // FileComplete
                    info!("📋 收到传输完成确认");
                    
                    // 确保文件已刷新和关闭
                    if let Some(mut file_writer) = current_file.take() {
                        if let Err(e) = file_writer.flush().await {
                            warn!("最终刷新文件失败: {}", e);
                        }
                    }
                    
                    let duration = start_time.elapsed();
                    let throughput_mbps = (total_bytes as f64) / (1024.0 * 1024.0) / duration.as_secs_f64();
                    
                    info!(
                        "🚀 零拷贝传输会话完成: {} 字节, 耗时: {:.3}秒, 最终吞吐量: {:.2} MB/s",
                        total_bytes,
                        duration.as_secs_f64(),
                        throughput_mbps
                    );
                    
                    break; // 传输完成，关闭连接
                }
                
                _ => {
                    warn!("未知消息类型: {}", header.msg_type);
                    break;
                }
            }
        }
        
        info!("🔚 零拷贝连接处理完成");
        Ok(())
    }
    
    /// 获取绑定地址
    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }
    
    /// 检查服务器是否在运行
    pub fn is_running(&self) -> bool {
        self.listener.is_some()
    }
}
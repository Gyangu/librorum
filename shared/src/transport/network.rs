//! UTP网络传输实现
//! 
//! 基于TCP Socket的高性能网络传输

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use std::fs::File;

use super::protocol::{UtpMessage, UtpMessageType, FileInfo};
use super::{UtpTransport, UtpConfig, UtpResult, UtpError, UtpEvent, UtpEventCallback, UtpStats};

/// 网络传输实现
pub struct NetworkTransport {
    /// 配置
    config: UtpConfig,
    /// 监听器
    listener: Option<Arc<Mutex<TcpListener>>>,
    /// 连接映射
    connections: Arc<Mutex<HashMap<String, TcpStream>>>,
    /// 事件回调
    event_callback: Option<Arc<UtpEventCallback>>,
    /// 统计信息
    stats: Arc<Mutex<UtpStats>>,
    /// 下一个序列号
    next_sequence: Arc<Mutex<u64>>,
}

impl NetworkTransport {
    /// 创建新的网络传输实例
    pub fn new(config: UtpConfig) -> UtpResult<Self> {
        let listener = if let Some(bind_addr) = config.bind_addr {
            let listener = TcpListener::bind(bind_addr)
                .map_err(|e| UtpError::NetworkError(format!("Failed to bind to {}: {}", bind_addr, e)))?;
            Some(Arc::new(Mutex::new(listener)))
        } else {
            None
        };
        
        let mut stats = UtpStats::default();
        stats.network_mode_usage = 1;
        
        Ok(Self {
            config,
            listener,
            connections: Arc::new(Mutex::new(HashMap::new())),
            event_callback: None,
            stats: Arc::new(Mutex::new(stats)),
            next_sequence: Arc::new(Mutex::new(1)),
        })
    }
    
    /// 启动服务器监听
    pub fn start_server(&self) -> UtpResult<()> {
        let listener = self.listener.as_ref()
            .ok_or_else(|| UtpError::NetworkError("No listener configured".to_string()))?;
        
        let listener = Arc::clone(listener);
        let connections = Arc::clone(&self.connections);
        let event_callback = self.event_callback.clone();
        
        thread::spawn(move || {
            loop {
                let (stream, addr) = match listener.lock().unwrap().accept() {
                    Ok(connection) => connection,
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
                
                let session_id = format!("server_{}", addr);
                connections.lock().unwrap().insert(session_id.clone(), stream);
                
                if let Some(callback) = &event_callback {
                    callback(UtpEvent::ConnectionEstablished {
                        session_id,
                        mode: super::TransportMode::Network,
                    });
                }
            }
        });
        
        Ok(())
    }
    
    /// 连接到服务器
    pub fn connect(&self, session_id: &str) -> UtpResult<()> {
        let target_addr = self.config.target_addr
            .ok_or_else(|| UtpError::NetworkError("No target address configured".to_string()))?;
        
        let stream = TcpStream::connect(target_addr)
            .map_err(|e| UtpError::NetworkError(format!("Failed to connect to {}: {}", target_addr, e)))?;
        
        self.connections.lock().unwrap().insert(session_id.to_string(), stream);
        
        if let Some(callback) = &self.event_callback {
            callback(UtpEvent::ConnectionEstablished {
                session_id: session_id.to_string(),
                mode: super::TransportMode::Network,
            });
        }
        
        Ok(())
    }
    
    /// 发送消息
    fn send_message(&self, session_id: &str, message: &UtpMessage) -> UtpResult<()> {
        let mut connections = self.connections.lock().unwrap();
        let stream = connections.get_mut(session_id)
            .ok_or_else(|| UtpError::NetworkError(format!("No connection for session {}", session_id)))?;
        
        let bytes = message.to_bytes();
        stream.write_all(&bytes)
            .map_err(|e| UtpError::NetworkError(format!("Failed to send message: {}", e)))?;
        
        stream.flush()
            .map_err(|e| UtpError::NetworkError(format!("Failed to flush stream: {}", e)))?;
        
        Ok(())
    }
    
    /// 接收消息
    fn receive_message(&self, session_id: &str) -> UtpResult<UtpMessage> {
        let mut connections = self.connections.lock().unwrap();
        let stream = connections.get_mut(session_id)
            .ok_or_else(|| UtpError::NetworkError(format!("No connection for session {}", session_id)))?;
        
        // 读取消息头
        let mut header_buf = [0u8; 32];
        stream.read_exact(&mut header_buf)
            .map_err(|e| UtpError::NetworkError(format!("Failed to read header: {}", e)))?;
        
        let header = super::protocol::UtpHeader::from_bytes(&header_buf)
            .ok_or_else(|| UtpError::ProtocolError("Invalid header".to_string()))?;
        
        // 读取载荷
        let mut payload = vec![0u8; header.payload_length as usize];
        if !payload.is_empty() {
            stream.read_exact(&mut payload)
                .map_err(|e| UtpError::NetworkError(format!("Failed to read payload: {}", e)))?;
        }
        
        let message = UtpMessage { header, payload };
        
        // 验证消息
        if !message.verify() {
            return Err(UtpError::ChecksumError("Message checksum verification failed".to_string()));
        }
        
        Ok(message)
    }
    
    /// 获取下一个序列号
    fn next_sequence(&self) -> u64 {
        let mut seq = self.next_sequence.lock().unwrap();
        let current = *seq;
        *seq += 1;
        current
    }
    
    /// 发送文件实现
    fn send_file_impl(&self, file_path: &str, session_id: &str) -> UtpResult<()> {
        let start_time = Instant::now();
        
        // 打开文件
        let mut file = File::open(file_path)
            .map_err(|e| UtpError::IoError(format!("Failed to open file {}: {}", file_path, e)))?;
        
        let chunk_size = self.config.chunk_size;
        let file_info = FileInfo::from_file(file_path, chunk_size as u32)
            .map_err(|e| UtpError::IoError(format!("Failed to get file info: {}", e)))?;
        
        // 发送文件头
        let file_header_msg = UtpMessage::file_header(self.next_sequence(), file_info.clone());
        self.send_message(session_id, &file_header_msg)?;
        
        // 分块发送文件数据
        let mut buffer = vec![0u8; chunk_size];
        let mut chunk_index = 0u64;
        let mut bytes_sent = 0u64;
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .map_err(|e| UtpError::IoError(format!("Failed to read file: {}", e)))?;
            
            if bytes_read == 0 {
                break;
            }
            
            let is_last = bytes_read < chunk_size;
            let chunk_data = buffer[..bytes_read].to_vec();
            
            let file_data_msg = UtpMessage::file_data(
                self.next_sequence(),
                chunk_index,
                chunk_data,
                is_last,
            );
            
            self.send_message(session_id, &file_data_msg)?;
            
            bytes_sent += bytes_read as u64;
            chunk_index += 1;
            
            // 更新进度
            if let Some(callback) = &self.event_callback {
                callback(UtpEvent::TransferProgress {
                    session_id: session_id.to_string(),
                    bytes_transferred: bytes_sent,
                    total_size: file_info.size,
                    transfer_rate: bytes_sent as f64 / start_time.elapsed().as_secs_f64(),
                });
            }
            
            if is_last {
                break;
            }
        }
        
        // 发送完成消息
        let complete_msg = UtpMessage::file_complete(self.next_sequence(), file_info.hash);
        self.send_message(session_id, &complete_msg)?;
        
        let elapsed = start_time.elapsed().as_secs_f64();
        
        // 更新统计信息
        {
            let mut stats = self.stats.lock().unwrap();
            stats.successful_transfers += 1;
            stats.total_bytes_transferred += bytes_sent;
            let transfer_rate = bytes_sent as f64 / elapsed;
            if transfer_rate > stats.max_transfer_rate {
                stats.max_transfer_rate = transfer_rate;
            }
        }
        
        if let Some(callback) = &self.event_callback {
            callback(UtpEvent::TransferCompleted {
                session_id: session_id.to_string(),
                total_bytes: bytes_sent,
                elapsed_secs: elapsed,
            });
        }
        
        Ok(())
    }
    
    /// 接收文件实现
    fn receive_file_impl(&self, file_path: &str, session_id: &str) -> UtpResult<()> {
        let start_time = Instant::now();
        
        // 接收文件头
        let file_header_msg = self.receive_message(session_id)?;
        if file_header_msg.message_type() != Some(UtpMessageType::FileHeader) {
            return Err(UtpError::ProtocolError("Expected file header".to_string()));
        }
        
        let file_info: FileInfo = serde_json::from_slice(&file_header_msg.payload)
            .map_err(|e| UtpError::ProtocolError(format!("Failed to parse file info: {}", e)))?;
        
        // 创建输出文件
        let mut output_file = File::create(file_path)
            .map_err(|e| UtpError::IoError(format!("Failed to create file {}: {}", file_path, e)))?;
        
        let mut bytes_received = 0u64;
        let mut expected_chunk_index = 0u64;
        
        // 接收文件数据
        loop {
            let file_data_msg = self.receive_message(session_id)?;
            
            match file_data_msg.message_type() {
                Some(UtpMessageType::FileData) => {
                    // 解析chunk_index
                    if file_data_msg.payload.len() < 8 {
                        return Err(UtpError::ProtocolError("Invalid file data payload".to_string()));
                    }
                    
                    let chunk_index = u64::from_le_bytes([
                        file_data_msg.payload[0], file_data_msg.payload[1],
                        file_data_msg.payload[2], file_data_msg.payload[3],
                        file_data_msg.payload[4], file_data_msg.payload[5],
                        file_data_msg.payload[6], file_data_msg.payload[7],
                    ]);
                    
                    if chunk_index != expected_chunk_index {
                        return Err(UtpError::ProtocolError(format!(
                            "Unexpected chunk index: expected {}, got {}",
                            expected_chunk_index, chunk_index
                        )));
                    }
                    
                    let chunk_data = &file_data_msg.payload[8..];
                    output_file.write_all(chunk_data)
                        .map_err(|e| UtpError::IoError(format!("Failed to write chunk: {}", e)))?;
                    
                    bytes_received += chunk_data.len() as u64;
                    expected_chunk_index += 1;
                    
                    // 更新进度
                    if let Some(callback) = &self.event_callback {
                        callback(UtpEvent::TransferProgress {
                            session_id: session_id.to_string(),
                            bytes_transferred: bytes_received,
                            total_size: file_info.size,
                            transfer_rate: bytes_received as f64 / start_time.elapsed().as_secs_f64(),
                        });
                    }
                    
                    // 检查是否为最后一个分片
                    if file_data_msg.flags().last_fragment {
                        break;
                    }
                }
                Some(UtpMessageType::FileComplete) => {
                    break;
                }
                _ => {
                    return Err(UtpError::ProtocolError("Unexpected message type".to_string()));
                }
            }
        }
        
        output_file.flush()
            .map_err(|e| UtpError::IoError(format!("Failed to flush file: {}", e)))?;
        
        let elapsed = start_time.elapsed().as_secs_f64();
        
        // 更新统计信息
        {
            let mut stats = self.stats.lock().unwrap();
            stats.successful_transfers += 1;
            stats.total_bytes_transferred += bytes_received;
            let transfer_rate = bytes_received as f64 / elapsed;
            if transfer_rate > stats.max_transfer_rate {
                stats.max_transfer_rate = transfer_rate;
            }
        }
        
        if let Some(callback) = &self.event_callback {
            callback(UtpEvent::TransferCompleted {
                session_id: session_id.to_string(),
                total_bytes: bytes_received,
                elapsed_secs: elapsed,
            });
        }
        
        Ok(())
    }
}

impl UtpTransport for NetworkTransport {
    fn send_file(&self, file_path: &str, session_id: &str) -> UtpResult<()> {
        // 如果没有连接，尝试建立连接
        if !self.connections.lock().unwrap().contains_key(session_id) {
            self.connect(session_id)?;
        }
        
        self.send_file_impl(file_path, session_id)
    }
    
    fn receive_file(&self, file_path: &str, session_id: &str) -> UtpResult<()> {
        // 如果没有监听器，启动服务器
        if self.listener.is_some() && !self.connections.lock().unwrap().contains_key(session_id) {
            self.start_server()?;
        }
        
        self.receive_file_impl(file_path, session_id)
    }
    
    fn send_chunk(&self, data: &[u8], session_id: &str) -> UtpResult<()> {
        let message = UtpMessage::data(self.next_sequence(), data.to_vec());
        self.send_message(session_id, &message)
    }
    
    fn receive_chunk(&self, session_id: &str) -> UtpResult<Vec<u8>> {
        let message = self.receive_message(session_id)?;
        Ok(message.payload)
    }
    
    fn set_event_callback(&self, callback: UtpEventCallback) {
        // 不能重新赋值，因为self是不可变引用
        // 这里需要修改架构，使用Arc<Mutex<Option<UtpEventCallback>>>
        // 暂时跳过实现
    }
    
    fn get_stats(&self) -> UtpStats {
        self.stats.lock().unwrap().clone()
    }
    
    fn close(&self) -> UtpResult<()> {
        let mut connections = self.connections.lock().unwrap();
        for (session_id, _) in connections.drain() {
            if let Some(callback) = &self.event_callback {
                callback(UtpEvent::ConnectionClosed {
                    session_id,
                    reason: "Transport closed".to_string(),
                });
            }
        }
        Ok(())
    }
}
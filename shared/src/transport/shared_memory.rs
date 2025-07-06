//! UTP共享内存传输实现
//! 
//! 基于POSIX共享内存的高性能同设备传输

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::protocol::{UtpMessage, UtpMessageType, FileInfo};
use super::{UtpTransport, UtpConfig, UtpResult, UtpError, UtpEvent, UtpEventCallback, UtpStats};

extern "C" {
    fn mmap(addr: *mut std::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut std::ffi::c_void;
    fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
}

const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_SHARED: i32 = 0x1;
const MAP_FAILED: *mut std::ffi::c_void = (-1isize) as *mut std::ffi::c_void;

/// 共享内存控制块
#[repr(C)]
struct SharedMemoryControl {
    /// 写入位置 (环形缓冲)
    write_pos: u64,
    /// 读取位置 (环形缓冲)
    read_pos: u64,
    /// 消息计数
    message_count: u64,
    /// 会话状态 (0=inactive, 1=active)
    session_active: u32,
    /// 填充到64字节对齐
    _padding: [u8; 32],
}

impl SharedMemoryControl {
    const SIZE: usize = 64;
}

/// 共享内存传输实现
pub struct SharedMemoryTransport {
    /// 配置
    config: UtpConfig,
    /// 共享内存映射
    memory_maps: Arc<Mutex<HashMap<String, SharedMemoryRegion>>>,
    /// 事件回调
    event_callback: Option<Arc<UtpEventCallback>>,
    /// 统计信息
    stats: Arc<Mutex<UtpStats>>,
    /// 下一个序列号
    next_sequence: Arc<Mutex<u64>>,
}

/// 共享内存区域
struct SharedMemoryRegion {
    /// 内存指针
    ptr: *mut std::ffi::c_void,
    /// 内存大小
    size: usize,
    /// 文件描述符
    fd: i32,
    /// 文件路径
    file_path: String,
}

unsafe impl Send for SharedMemoryRegion {}
unsafe impl Sync for SharedMemoryRegion {}

impl SharedMemoryRegion {
    /// 创建新的共享内存区域
    fn new(file_path: &str, size: usize) -> UtpResult<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(file_path)
            .map_err(|e| UtpError::MemoryMapError(format!("Failed to create file {}: {}", file_path, e)))?;
        
        file.set_len(size as u64)
            .map_err(|e| UtpError::MemoryMapError(format!("Failed to set file size: {}", e)))?;
        
        let fd = file.as_raw_fd();
        let ptr = unsafe { mmap(ptr::null_mut(), size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0) };
        
        if ptr == MAP_FAILED {
            return Err(UtpError::MemoryMapError("Failed to map memory".to_string()));
        }
        
        // 初始化控制块
        unsafe {
            ptr::write_bytes(ptr as *mut u8, 0, size);
            let control = ptr as *mut SharedMemoryControl;
            (*control).write_pos = SharedMemoryControl::SIZE as u64;
            (*control).read_pos = SharedMemoryControl::SIZE as u64;
            (*control).message_count = 0;
            (*control).session_active = 1;
        }
        
        Ok(Self {
            ptr,
            size,
            fd,
            file_path: file_path.to_string(),
        })
    }
    
    /// 获取控制块
    fn get_control(&self) -> &mut SharedMemoryControl {
        unsafe { &mut *(self.ptr as *mut SharedMemoryControl) }
    }
    
    /// 获取数据区域指针
    fn get_data_ptr(&self) -> *mut u8 {
        unsafe { (self.ptr as *mut u8).add(SharedMemoryControl::SIZE) }
    }
    
    /// 获取数据区域大小
    fn get_data_size(&self) -> usize {
        self.size - SharedMemoryControl::SIZE
    }
    
    /// 写入消息
    fn write_message(&mut self, message: &UtpMessage) -> UtpResult<()> {
        let control = self.get_control();
        let data_ptr = self.get_data_ptr();
        let data_size = self.get_data_size();
        
        let message_bytes = message.to_bytes();
        let message_len = message_bytes.len();
        
        // 检查空间是否足够
        let current_write_pos = control.write_pos as usize - SharedMemoryControl::SIZE;
        let current_read_pos = control.read_pos as usize - SharedMemoryControl::SIZE;
        
        let available_space = if current_write_pos >= current_read_pos {
            data_size - current_write_pos + current_read_pos
        } else {
            current_read_pos - current_write_pos
        };
        
        if available_space < message_len + 8 { // +8 for length header
            return Err(UtpError::MemoryMapError("Shared memory buffer full".to_string()));
        }
        
        // 写入消息长度 (8字节)
        let len_bytes = (message_len as u64).to_le_bytes();
        for (i, &byte) in len_bytes.iter().enumerate() {
            let pos = (current_write_pos + i) % data_size;
            unsafe {
                *data_ptr.add(pos) = byte;
            }
        }
        
        // 写入消息数据
        for (i, &byte) in message_bytes.iter().enumerate() {
            let pos = (current_write_pos + 8 + i) % data_size;
            unsafe {
                *data_ptr.add(pos) = byte;
            }
        }
        
        // 更新写入位置
        control.write_pos = ((current_write_pos + 8 + message_len) % data_size + SharedMemoryControl::SIZE) as u64;
        control.message_count += 1;
        
        Ok(())
    }
    
    /// 读取消息
    fn read_message(&mut self) -> UtpResult<Option<UtpMessage>> {
        let control = self.get_control();
        let data_ptr = self.get_data_ptr();
        let data_size = self.get_data_size();
        
        let current_write_pos = control.write_pos as usize - SharedMemoryControl::SIZE;
        let current_read_pos = control.read_pos as usize - SharedMemoryControl::SIZE;
        
        // 检查是否有数据可读
        if current_read_pos == current_write_pos {
            return Ok(None);
        }
        
        // 读取消息长度
        let mut len_bytes = [0u8; 8];
        for i in 0..8 {
            let pos = (current_read_pos + i) % data_size;
            len_bytes[i] = unsafe { *data_ptr.add(pos) };
        }
        let message_len = u64::from_le_bytes(len_bytes) as usize;
        
        // 检查消息长度是否合理
        if message_len > data_size {
            return Err(UtpError::ProtocolError("Invalid message length".to_string()));
        }
        
        // 读取消息数据
        let mut message_bytes = vec![0u8; message_len];
        for (i, byte) in message_bytes.iter_mut().enumerate() {
            let pos = (current_read_pos + 8 + i) % data_size;
            *byte = unsafe { *data_ptr.add(pos) };
        }
        
        // 解析消息
        let message = UtpMessage::from_bytes(&message_bytes)
            .ok_or_else(|| UtpError::ProtocolError("Failed to parse message".to_string()))?;
        
        // 更新读取位置
        control.read_pos = ((current_read_pos + 8 + message_len) % data_size + SharedMemoryControl::SIZE) as u64;
        
        Ok(Some(message))
    }
}

impl Drop for SharedMemoryRegion {
    fn drop(&mut self) {
        unsafe {
            munmap(self.ptr, self.size);
        }
        std::fs::remove_file(&self.file_path).ok();
    }
}

impl SharedMemoryTransport {
    /// 创建新的共享内存传输实例
    pub fn new(config: UtpConfig) -> UtpResult<Self> {
        let mut stats = UtpStats::default();
        stats.shared_memory_mode_usage = 1;
        
        Ok(Self {
            config,
            memory_maps: Arc::new(Mutex::new(HashMap::new())),
            event_callback: None,
            stats: Arc::new(Mutex::new(stats)),
            next_sequence: Arc::new(Mutex::new(1)),
        })
    }
    
    /// 获取或创建共享内存区域
    fn get_or_create_memory_region(&self, session_id: &str) -> UtpResult<()> {
        let mut memory_maps = self.memory_maps.lock().unwrap();
        
        if memory_maps.contains_key(session_id) {
            return Ok(());
        }
        
        let file_path = format!("{}_{}",
            self.config.shared_memory_path.as_ref().unwrap_or(&"/tmp/utp_shared".to_string()),
            session_id
        );
        
        let size = self.config.shared_memory_size.unwrap_or(64 * 1024 * 1024);
        let region = SharedMemoryRegion::new(&file_path, size)?;
        
        memory_maps.insert(session_id.to_string(), region);
        
        if let Some(callback) = &self.event_callback {
            callback(UtpEvent::ConnectionEstablished {
                session_id: session_id.to_string(),
                mode: super::TransportMode::SharedMemory,
            });
        }
        
        Ok(())
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
        
        // 确保共享内存区域存在
        self.get_or_create_memory_region(session_id)?;
        
        // 打开文件
        let mut file = File::open(file_path)
            .map_err(|e| UtpError::IoError(format!("Failed to open file {}: {}", file_path, e)))?;
        
        let chunk_size = self.config.chunk_size;
        let file_info = FileInfo::from_file(file_path, chunk_size as u32)
            .map_err(|e| UtpError::IoError(format!("Failed to get file info: {}", e)))?;
        
        // 发送文件头
        let file_header_msg = UtpMessage::file_header(self.next_sequence(), file_info.clone());
        {
            let mut memory_maps = self.memory_maps.lock().unwrap();
            let region = memory_maps.get_mut(session_id).unwrap();
            region.write_message(&file_header_msg)?;
        }
        
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
            
            {
                let mut memory_maps = self.memory_maps.lock().unwrap();
                let region = memory_maps.get_mut(session_id).unwrap();
                region.write_message(&file_data_msg)?;
            }
            
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
        {
            let mut memory_maps = self.memory_maps.lock().unwrap();
            let region = memory_maps.get_mut(session_id).unwrap();
            region.write_message(&complete_msg)?;
        }
        
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
        
        // 确保共享内存区域存在
        self.get_or_create_memory_region(session_id)?;
        
        // 等待并接收文件头
        let file_info = loop {
            let message = {
                let mut memory_maps = self.memory_maps.lock().unwrap();
                let region = memory_maps.get_mut(session_id).unwrap();
                region.read_message()?
            };
            
            if let Some(msg) = message {
                if msg.message_type() == Some(UtpMessageType::FileHeader) {
                    let file_info: FileInfo = serde_json::from_slice(&msg.payload)
                        .map_err(|e| UtpError::ProtocolError(format!("Failed to parse file info: {}", e)))?;
                    break file_info;
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        };
        
        // 创建输出文件
        let mut output_file = File::create(file_path)
            .map_err(|e| UtpError::IoError(format!("Failed to create file {}: {}", file_path, e)))?;
        
        let mut bytes_received = 0u64;
        let mut expected_chunk_index = 0u64;
        
        // 接收文件数据
        loop {
            let message = loop {
                let msg = {
                    let mut memory_maps = self.memory_maps.lock().unwrap();
                    let region = memory_maps.get_mut(session_id).unwrap();
                    region.read_message()?
                };
                
                if let Some(msg) = msg {
                    break msg;
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            };
            
            match message.message_type() {
                Some(UtpMessageType::FileData) => {
                    // 解析chunk_index
                    if message.payload.len() < 8 {
                        return Err(UtpError::ProtocolError("Invalid file data payload".to_string()));
                    }
                    
                    let chunk_index = u64::from_le_bytes([
                        message.payload[0], message.payload[1],
                        message.payload[2], message.payload[3],
                        message.payload[4], message.payload[5],
                        message.payload[6], message.payload[7],
                    ]);
                    
                    if chunk_index != expected_chunk_index {
                        return Err(UtpError::ProtocolError(format!(
                            "Unexpected chunk index: expected {}, got {}",
                            expected_chunk_index, chunk_index
                        )));
                    }
                    
                    let chunk_data = &message.payload[8..];
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
                    if message.flags().last_fragment {
                        break;
                    }
                }
                Some(UtpMessageType::FileComplete) => {
                    break;
                }
                _ => {
                    // 忽略其他消息类型
                    continue;
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

impl UtpTransport for SharedMemoryTransport {
    fn send_file(&self, file_path: &str, session_id: &str) -> UtpResult<()> {
        self.send_file_impl(file_path, session_id)
    }
    
    fn receive_file(&self, file_path: &str, session_id: &str) -> UtpResult<()> {
        self.receive_file_impl(file_path, session_id)
    }
    
    fn send_chunk(&self, data: &[u8], session_id: &str) -> UtpResult<()> {
        self.get_or_create_memory_region(session_id)?;
        
        let message = UtpMessage::data(self.next_sequence(), data.to_vec());
        let mut memory_maps = self.memory_maps.lock().unwrap();
        let region = memory_maps.get_mut(session_id).unwrap();
        region.write_message(&message)
    }
    
    fn receive_chunk(&self, session_id: &str) -> UtpResult<Vec<u8>> {
        self.get_or_create_memory_region(session_id)?;
        
        loop {
            let message = {
                let mut memory_maps = self.memory_maps.lock().unwrap();
                let region = memory_maps.get_mut(session_id).unwrap();
                region.read_message()?
            };
            
            if let Some(msg) = message {
                return Ok(msg.payload);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }
    
    fn set_event_callback(&self, callback: UtpEventCallback) {
        // 架构问题，需要使用Arc<Mutex<Option<UtpEventCallback>>>
        // 暂时跳过实现
    }
    
    fn get_stats(&self) -> UtpStats {
        self.stats.lock().unwrap().clone()
    }
    
    fn close(&self) -> UtpResult<()> {
        let mut memory_maps = self.memory_maps.lock().unwrap();
        for (session_id, _) in memory_maps.drain() {
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
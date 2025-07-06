//! UTP二进制协议定义
//! 
//! 基于universal-transport的高性能二进制协议实现

use std::time::{SystemTime, UNIX_EPOCH};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};

/// UTP协议魔数
pub const UTP_MAGIC: u32 = 0x55545042; // "UTPB"

/// UTP协议版本
pub const UTP_VERSION: u8 = 1;

/// UTP消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum UtpMessageType {
    /// 数据消息
    Data = 0x01,
    /// 控制消息
    Control = 0x02,
    /// 文件头消息
    FileHeader = 0x03,
    /// 文件数据消息
    FileData = 0x04,
    /// 文件完成消息
    FileComplete = 0x05,
    /// 心跳消息
    Heartbeat = 0x06,
    /// 确认消息
    Ack = 0x07,
    /// 错误消息
    Error = 0x08,
}

/// UTP消息标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtpFlags {
    /// 需要确认
    pub ack_required: bool,
    /// 压缩数据
    pub compressed: bool,
    /// 加密数据
    pub encrypted: bool,
    /// 分片数据
    pub fragmented: bool,
    /// 最后一个分片
    pub last_fragment: bool,
}

impl UtpFlags {
    /// 转换为u16
    pub fn to_u16(&self) -> u16 {
        let mut flags = 0u16;
        if self.ack_required { flags |= 0x01; }
        if self.compressed { flags |= 0x02; }
        if self.encrypted { flags |= 0x04; }
        if self.fragmented { flags |= 0x08; }
        if self.last_fragment { flags |= 0x10; }
        flags
    }
    
    /// 从u16转换
    pub fn from_u16(flags: u16) -> Self {
        Self {
            ack_required: (flags & 0x01) != 0,
            compressed: (flags & 0x02) != 0,
            encrypted: (flags & 0x04) != 0,
            fragmented: (flags & 0x08) != 0,
            last_fragment: (flags & 0x10) != 0,
        }
    }
}

impl Default for UtpFlags {
    fn default() -> Self {
        Self {
            ack_required: false,
            compressed: false,
            encrypted: false,
            fragmented: false,
            last_fragment: false,
        }
    }
}

/// UTP消息头 (32字节固定)
#[derive(Debug, Clone)]
#[repr(C)]
pub struct UtpHeader {
    /// 协议魔数 (4字节)
    pub magic: u32,
    /// 协议版本 (1字节)
    pub version: u8,
    /// 消息类型 (1字节)
    pub message_type: u8,
    /// 标志位 (2字节)
    pub flags: u16,
    /// 载荷长度 (4字节)
    pub payload_length: u32,
    /// 序列号 (8字节)
    pub sequence: u64,
    /// 时间戳 (8字节 - 微秒)
    pub timestamp: u64,
    /// CRC32校验 (4字节)
    pub checksum: u32,
}

impl UtpHeader {
    /// 头部大小
    pub const SIZE: usize = 32;
    
    /// 创建新的消息头
    pub fn new(
        message_type: UtpMessageType,
        flags: UtpFlags,
        payload_length: u32,
        sequence: u64,
    ) -> Self {
        Self {
            magic: UTP_MAGIC,
            version: UTP_VERSION,
            message_type: message_type as u8,
            flags: flags.to_u16(),
            payload_length,
            sequence,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            checksum: 0, // 稍后计算
        }
    }
    
    /// 计算并设置校验和
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        let mut hasher = Hasher::new();
        
        // 头部数据 (除了checksum字段)
        hasher.update(&self.magic.to_le_bytes());
        hasher.update(&[self.version]);
        hasher.update(&[self.message_type]);
        hasher.update(&self.flags.to_le_bytes());
        hasher.update(&self.payload_length.to_le_bytes());
        hasher.update(&self.sequence.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        
        // 载荷数据
        if !payload.is_empty() {
            hasher.update(payload);
        }
        
        self.checksum = hasher.finalize();
    }
    
    /// 验证校验和
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let mut hasher = Hasher::new();
        
        hasher.update(&self.magic.to_le_bytes());
        hasher.update(&[self.version]);
        hasher.update(&[self.message_type]);
        hasher.update(&self.flags.to_le_bytes());
        hasher.update(&self.payload_length.to_le_bytes());
        hasher.update(&self.sequence.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        
        if !payload.is_empty() {
            hasher.update(payload);
        }
        
        hasher.finalize() == self.checksum
    }
    
    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4] = self.version;
        bytes[5] = self.message_type;
        bytes[6..8].copy_from_slice(&self.flags.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.payload_length.to_le_bytes());
        bytes[12..20].copy_from_slice(&self.sequence.to_le_bytes());
        bytes[20..28].copy_from_slice(&self.timestamp.to_le_bytes());
        bytes[28..32].copy_from_slice(&self.checksum.to_le_bytes());
        
        bytes
    }
    
    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != UTP_MAGIC {
            return None;
        }
        
        let version = bytes[4];
        if version != UTP_VERSION {
            return None;
        }
        
        Some(Self {
            magic,
            version,
            message_type: bytes[5],
            flags: u16::from_le_bytes([bytes[6], bytes[7]]),
            payload_length: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            sequence: u64::from_le_bytes([
                bytes[12], bytes[13], bytes[14], bytes[15],
                bytes[16], bytes[17], bytes[18], bytes[19],
            ]),
            timestamp: u64::from_le_bytes([
                bytes[20], bytes[21], bytes[22], bytes[23],
                bytes[24], bytes[25], bytes[26], bytes[27],
            ]),
            checksum: u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
        })
    }
}

/// UTP消息
#[derive(Debug, Clone)]
pub struct UtpMessage {
    /// 消息头
    pub header: UtpHeader,
    /// 消息载荷
    pub payload: Vec<u8>,
}

impl UtpMessage {
    /// 创建新消息
    pub fn new(
        message_type: UtpMessageType,
        flags: UtpFlags,
        sequence: u64,
        payload: Vec<u8>,
    ) -> Self {
        let mut header = UtpHeader::new(
            message_type,
            flags,
            payload.len() as u32,
            sequence,
        );
        
        header.calculate_checksum(&payload);
        
        Self { header, payload }
    }
    
    /// 创建数据消息
    pub fn data(sequence: u64, data: Vec<u8>) -> Self {
        Self::new(UtpMessageType::Data, UtpFlags::default(), sequence, data)
    }
    
    /// 创建文件头消息
    pub fn file_header(sequence: u64, file_info: FileInfo) -> Self {
        let payload = serde_json::to_vec(&file_info).unwrap_or_default();
        Self::new(UtpMessageType::FileHeader, UtpFlags::default(), sequence, payload)
    }
    
    /// 创建文件数据消息
    pub fn file_data(sequence: u64, chunk_index: u64, data: Vec<u8>, is_last: bool) -> Self {
        let mut flags = UtpFlags::default();
        if is_last {
            flags.last_fragment = true;
        }
        
        // 在载荷前添加chunk_index
        let mut payload = chunk_index.to_le_bytes().to_vec();
        payload.extend_from_slice(&data);
        
        Self::new(UtpMessageType::FileData, flags, sequence, payload)
    }
    
    /// 创建文件完成消息
    pub fn file_complete(sequence: u64, file_hash: String) -> Self {
        let payload = file_hash.into_bytes();
        Self::new(UtpMessageType::FileComplete, UtpFlags::default(), sequence, payload)
    }
    
    /// 创建心跳消息
    pub fn heartbeat(sequence: u64) -> Self {
        Self::new(UtpMessageType::Heartbeat, UtpFlags::default(), sequence, vec![])
    }
    
    /// 创建确认消息
    pub fn ack(sequence: u64, ack_sequence: u64) -> Self {
        let payload = ack_sequence.to_le_bytes().to_vec();
        Self::new(UtpMessageType::Ack, UtpFlags::default(), sequence, payload)
    }
    
    /// 创建错误消息
    pub fn error(sequence: u64, error_code: u32, error_message: String) -> Self {
        let mut payload = error_code.to_le_bytes().to_vec();
        payload.extend_from_slice(error_message.as_bytes());
        Self::new(UtpMessageType::Error, UtpFlags::default(), sequence, payload)
    }
    
    /// 验证消息完整性
    pub fn verify(&self) -> bool {
        self.header.verify_checksum(&self.payload)
    }
    
    /// 获取消息类型
    pub fn message_type(&self) -> Option<UtpMessageType> {
        match self.header.message_type {
            0x01 => Some(UtpMessageType::Data),
            0x02 => Some(UtpMessageType::Control),
            0x03 => Some(UtpMessageType::FileHeader),
            0x04 => Some(UtpMessageType::FileData),
            0x05 => Some(UtpMessageType::FileComplete),
            0x06 => Some(UtpMessageType::Heartbeat),
            0x07 => Some(UtpMessageType::Ack),
            0x08 => Some(UtpMessageType::Error),
            _ => None,
        }
    }
    
    /// 获取标志
    pub fn flags(&self) -> UtpFlags {
        UtpFlags::from_u16(self.header.flags)
    }
    
    /// 序列化为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(UtpHeader::SIZE + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }
    
    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < UtpHeader::SIZE {
            return None;
        }
        
        let header = UtpHeader::from_bytes(&bytes[..UtpHeader::SIZE])?;
        let payload_len = header.payload_length as usize;
        
        if bytes.len() < UtpHeader::SIZE + payload_len {
            return None;
        }
        
        let payload = bytes[UtpHeader::SIZE..UtpHeader::SIZE + payload_len].to_vec();
        let message = Self { header, payload };
        
        // 验证校验和
        if !message.verify() {
            return None;
        }
        
        Some(message)
    }
}

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件名
    pub name: String,
    /// 文件大小
    pub size: u64,
    /// 文件哈希 (SHA-256)
    pub hash: String,
    /// MIME类型
    pub mime_type: String,
    /// 创建时间
    pub created_at: u64,
    /// 修改时间
    pub modified_at: u64,
    /// 分块数量
    pub chunk_count: u64,
    /// 分块大小
    pub chunk_size: u32,
    /// 压缩类型
    pub compression: Option<String>,
    /// 加密类型
    pub encryption: Option<String>,
}

impl FileInfo {
    /// 从文件路径创建文件信息
    pub fn from_file(file_path: &str, chunk_size: u32) -> Result<Self, std::io::Error> {
        let metadata = std::fs::metadata(file_path)?;
        let size = metadata.len();
        
        let created_at = metadata.created()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let modified_at = metadata.modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let name = std::path::Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let chunk_count = (size + chunk_size as u64 - 1) / chunk_size as u64;
        
        // TODO: 计算文件哈希
        let hash = "".to_string();
        
        // TODO: 检测MIME类型
        let mime_type = "application/octet-stream".to_string();
        
        Ok(Self {
            name,
            size,
            hash,
            mime_type,
            created_at,
            modified_at,
            chunk_count,
            chunk_size,
            compression: None,
            encryption: None,
        })
    }
}
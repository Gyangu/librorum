//! High-Performance Binary Protocol for Universal Transport
//! 
//! TCP-like fixed-length binary protocol for maximum performance
//! No JSON overhead - direct memory serialization

use std::time::{SystemTime, UNIX_EPOCH};
use bytes::{Bytes, BytesMut, BufMut, Buf};
use crc32fast;

/// Protocol magic number (4 bytes) - "UTPB" (Universal Transport Protocol Binary)
pub const PROTOCOL_MAGIC: u32 = 0x55545042;

/// Protocol version (1 byte)
pub const PROTOCOL_VERSION: u8 = 1;

/// Message header size (32 bytes - cache line aligned)
pub const HEADER_SIZE: usize = 32;

/// Maximum payload size (64MB)
pub const MAX_PAYLOAD_SIZE: u32 = 64 * 1024 * 1024;

/// Message types (1 byte)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Data = 0x01,
    Heartbeat = 0x02,
    Acknowledgment = 0x03,
    Error = 0x04,
    Benchmark = 0x05,
}

impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => MessageType::Data,
            0x02 => MessageType::Heartbeat,
            0x03 => MessageType::Acknowledgment,
            0x04 => MessageType::Error,
            0x05 => MessageType::Benchmark,
            _ => MessageType::Data, // Default fallback
        }
    }
}

/// Binary message header (32 bytes, fixed layout)
/// Layout:
/// 0-3:   Magic number (4 bytes)
/// 4:     Version (1 byte)
/// 5:     Message type (1 byte)
/// 6-7:   Flags (2 bytes)
/// 8-11:  Payload length (4 bytes)
/// 12-19: Sequence number (8 bytes)
/// 20-27: Timestamp (8 bytes, microseconds since epoch)
/// 28-31: CRC32 checksum (4 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BinaryHeader {
    pub magic: u32,
    pub version: u8,
    pub message_type: u8,
    pub flags: u16,
    pub payload_length: u32,
    pub sequence: u64,
    pub timestamp: u64,
    pub checksum: u32,
}

impl BinaryHeader {
    /// Create a new binary header
    pub fn new(message_type: MessageType, payload: &[u8]) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        
        let checksum = Self::calculate_crc32(payload);
        
        Self {
            magic: PROTOCOL_MAGIC,
            version: PROTOCOL_VERSION,
            message_type: message_type as u8,
            flags: 0,
            payload_length: payload.len() as u32,
            sequence: 0, // Set by sender
            timestamp,
            checksum,
        }
    }
    
    /// Set sequence number
    pub fn set_sequence(&mut self, seq: u64) {
        self.sequence = seq;
    }
    
    /// Validate header
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.magic != PROTOCOL_MAGIC {
            return Err(ProtocolError::InvalidMagic(self.magic));
        }
        
        if self.version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(self.version));
        }
        
        if self.payload_length > MAX_PAYLOAD_SIZE {
            return Err(ProtocolError::PayloadTooLarge(self.payload_length));
        }
        
        Ok(())
    }
    
    /// Verify payload checksum
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        self.checksum == Self::calculate_crc32(payload)
    }
    
    /// Calculate CRC32 checksum
    fn calculate_crc32(data: &[u8]) -> u32 {
        crc32fast::hash(data)
    }
    
    /// Serialize header to bytes (little-endian)
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        let mut buf = BytesMut::with_capacity(HEADER_SIZE);
        
        buf.put_u32_le(self.magic);
        buf.put_u8(self.version);
        buf.put_u8(self.message_type);
        buf.put_u16_le(self.flags);
        buf.put_u32_le(self.payload_length);
        buf.put_u64_le(self.sequence);
        buf.put_u64_le(self.timestamp);
        buf.put_u32_le(self.checksum);
        
        bytes.copy_from_slice(&buf[..HEADER_SIZE]);
        bytes
    }
    
    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < HEADER_SIZE {
            return Err(ProtocolError::InsufficientData(bytes.len()));
        }
        
        let mut buf = Bytes::copy_from_slice(&bytes[..HEADER_SIZE]);
        
        let header = Self {
            magic: buf.get_u32_le(),
            version: buf.get_u8(),
            message_type: buf.get_u8(),
            flags: buf.get_u16_le(),
            payload_length: buf.get_u32_le(),
            sequence: buf.get_u64_le(),
            timestamp: buf.get_u64_le(),
            checksum: buf.get_u32_le(),
        };
        
        header.validate()?;
        Ok(header)
    }
}

/// Complete binary message
#[derive(Debug, Clone)]
pub struct BinaryMessage {
    pub header: BinaryHeader,
    pub payload: Bytes,
}

impl BinaryMessage {
    /// Create a new binary message
    pub fn new(message_type: MessageType, payload: Bytes) -> Result<Self, ProtocolError> {
        if payload.len() > MAX_PAYLOAD_SIZE as usize {
            return Err(ProtocolError::PayloadTooLarge(payload.len() as u32));
        }
        
        let header = BinaryHeader::new(message_type, &payload);
        
        Ok(Self { header, payload })
    }
    
    /// Create a benchmark message with specific data
    pub fn benchmark(id: u64, data: Bytes) -> Result<Self, ProtocolError> {
        let mut message = Self::new(MessageType::Benchmark, data)?;
        message.header.set_sequence(id);
        Ok(message)
    }
    
    /// Get total message size
    pub fn total_size(&self) -> usize {
        HEADER_SIZE + self.payload.len()
    }
    
    /// Validate complete message
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.header.validate()?;
        
        if !self.header.verify_checksum(&self.payload) {
            return Err(ProtocolError::ChecksumMismatch);
        }
        
        Ok(())
    }
    
    /// Serialize entire message to bytes
    pub fn to_bytes(&self) -> Bytes {
        let total_size = self.total_size();
        let mut buf = BytesMut::with_capacity(total_size);
        
        // Header
        buf.put_slice(&self.header.to_bytes());
        
        // Payload
        buf.put_slice(&self.payload);
        
        buf.freeze()
    }
    
    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < HEADER_SIZE {
            return Err(ProtocolError::InsufficientData(bytes.len()));
        }
        
        // Parse header
        let header = BinaryHeader::from_bytes(bytes)?;
        
        // Check if we have enough data for payload
        let expected_total = HEADER_SIZE + header.payload_length as usize;
        if bytes.len() < expected_total {
            return Err(ProtocolError::InsufficientData(bytes.len()));
        }
        
        // Extract payload
        let payload = Bytes::copy_from_slice(&bytes[HEADER_SIZE..expected_total]);
        
        let message = Self { header, payload };
        message.validate()?;
        
        Ok(message)
    }
}

/// Benchmark-specific message structure
#[derive(Debug, Clone)]
pub struct BenchmarkMessage {
    pub id: u64,
    pub timestamp: u64,
    pub data: Bytes,
    pub metadata: String,
}

impl BenchmarkMessage {
    /// Create a new benchmark message
    pub fn new(id: u64, data_size: usize) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        
        Self {
            id,
            timestamp,
            data: Bytes::from(vec![0x42; data_size]),
            metadata: format!("benchmark_msg_{}", id),
        }
    }
    
    /// Serialize to binary format (fixed layout)
    /// Layout:
    /// 0-7:    ID (8 bytes)
    /// 8-15:   Timestamp (8 bytes)
    /// 16-19:  Data length (4 bytes)
    /// 20-23:  Metadata length (4 bytes)
    /// 24+:    Data
    /// N+:     Metadata (UTF-8)
    pub fn to_binary(&self) -> Bytes {
        let metadata_bytes = self.metadata.as_bytes();
        let total_size = 24 + self.data.len() + metadata_bytes.len();
        
        let mut buf = BytesMut::with_capacity(total_size);
        
        buf.put_u64_le(self.id);
        buf.put_u64_le(self.timestamp);
        buf.put_u32_le(self.data.len() as u32);
        buf.put_u32_le(metadata_bytes.len() as u32);
        buf.put_slice(&self.data);
        buf.put_slice(metadata_bytes);
        
        buf.freeze()
    }
    
    /// Deserialize from binary format
    pub fn from_binary(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < 24 {
            return Err(ProtocolError::InsufficientData(bytes.len()));
        }
        
        let mut buf = Bytes::copy_from_slice(bytes);
        
        let id = buf.get_u64_le();
        let timestamp = buf.get_u64_le();
        let data_len = buf.get_u32_le() as usize;
        let metadata_len = buf.get_u32_le() as usize;
        
        // Check if we have enough remaining data
        if buf.remaining() < data_len + metadata_len {
            return Err(ProtocolError::InsufficientData(buf.remaining()));
        }
        
        let data = buf.split_to(data_len);
        let metadata_bytes = buf.split_to(metadata_len);
        let metadata = String::from_utf8(metadata_bytes.to_vec())
            .map_err(|_| ProtocolError::InvalidUtf8)?;
        
        Ok(Self {
            id,
            timestamp,
            data,
            metadata,
        })
    }
    
    /// Convert to binary message
    pub fn to_binary_message(&self) -> Result<BinaryMessage, ProtocolError> {
        let payload = self.to_binary();
        BinaryMessage::benchmark(self.id, payload)
    }
    
    /// Create from binary message
    pub fn from_binary_message(msg: &BinaryMessage) -> Result<Self, ProtocolError> {
        Self::from_binary(&msg.payload)
    }
}

/// Protocol errors
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Invalid magic number: 0x{0:x}")]
    InvalidMagic(u32),
    
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u8),
    
    #[error("Payload too large: {0} bytes")]
    PayloadTooLarge(u32),
    
    #[error("Insufficient data: {0} bytes available")]
    InsufficientData(usize),
    
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    
    #[error("Invalid UTF-8 encoding")]
    InvalidUtf8,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_header_serialization() {
        let payload = b"Hello, World!";
        let header = BinaryHeader::new(MessageType::Data, payload);
        
        let bytes = header.to_bytes();
        let decoded = BinaryHeader::from_bytes(&bytes).unwrap();
        
        assert_eq!(header.magic, decoded.magic);
        assert_eq!(header.message_type, decoded.message_type);
        assert_eq!(header.payload_length, decoded.payload_length);
        assert!(decoded.verify_checksum(payload));
    }
    
    #[test]
    fn test_benchmark_message_serialization() {
        let msg = BenchmarkMessage::new(123, 1024);
        let binary = msg.to_binary();
        let decoded = BenchmarkMessage::from_binary(&binary).unwrap();
        
        assert_eq!(msg.id, decoded.id);
        assert_eq!(msg.data.len(), decoded.data.len());
        assert_eq!(msg.metadata, decoded.metadata);
    }
    
    #[test]
    fn test_complete_message_flow() {
        let bench_msg = BenchmarkMessage::new(456, 512);
        let binary_msg = bench_msg.to_binary_message().unwrap();
        let serialized = binary_msg.to_bytes();
        
        let deserialized = BinaryMessage::from_bytes(&serialized).unwrap();
        let recovered_bench = BenchmarkMessage::from_binary_message(&deserialized).unwrap();
        
        assert_eq!(bench_msg.id, recovered_bench.id);
        assert_eq!(bench_msg.data.len(), recovered_bench.data.len());
    }
}
//! Shared memory communication protocol

use crate::{SharedMemoryError, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Shared memory message header (32 bytes, cache-line aligned)
#[repr(C, align(32))]
pub struct MessageHeader {
    /// Protocol magic number
    pub magic: AtomicU32,
    /// Protocol version
    pub version: u8,
    /// Message type
    pub message_type: u8,
    /// Flags
    pub flags: u16,
    /// Message size (excluding header)
    pub size: AtomicU32,
    /// Sequence number
    pub sequence: AtomicU64,
    /// Timestamp (milliseconds since epoch)
    pub timestamp: AtomicU64,
    /// CRC32 checksum of the payload
    pub checksum: AtomicU32,
    /// Reserved for future use
    _reserved: [u8; 4],
}

impl std::fmt::Debug for MessageHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageHeader")
            .field("magic", &self.magic.load(Ordering::Acquire))
            .field("version", &self.version)
            .field("message_type", &self.message_type)
            .field("flags", &self.flags)
            .field("size", &self.size.load(Ordering::Acquire))
            .field("sequence", &self.sequence.load(Ordering::Acquire))
            .field("timestamp", &self.timestamp.load(Ordering::Acquire))
            .field("checksum", &self.checksum.load(Ordering::Acquire))
            .finish()
    }
}

impl Clone for MessageHeader {
    fn clone(&self) -> Self {
        Self {
            magic: AtomicU32::new(self.magic.load(Ordering::Acquire)),
            version: self.version,
            message_type: self.message_type,
            flags: self.flags,
            size: AtomicU32::new(self.size.load(Ordering::Acquire)),
            sequence: AtomicU64::new(self.sequence.load(Ordering::Acquire)),
            timestamp: AtomicU64::new(self.timestamp.load(Ordering::Acquire)),
            checksum: AtomicU32::new(self.checksum.load(Ordering::Acquire)),
            _reserved: self._reserved,
        }
    }
}

impl MessageHeader {
    /// Create a new message header
    pub fn new(message_type: MessageType, payload: &[u8]) -> Self {
        let checksum = crc32fast::hash(payload);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        Self {
            magic: AtomicU32::new(crate::SHARED_MEMORY_MAGIC),
            version: crate::SHARED_MEMORY_VERSION,
            message_type: message_type as u8,
            flags: 0,
            size: AtomicU32::new(payload.len() as u32),
            sequence: AtomicU64::new(0), // Will be set by sender
            timestamp: AtomicU64::new(timestamp),
            checksum: AtomicU32::new(checksum),
            _reserved: [0; 4],
        }
    }
    
    /// Validate the header
    pub fn validate(&self) -> Result<()> {
        if self.magic.load(Ordering::Acquire) != crate::SHARED_MEMORY_MAGIC {
            return Err(SharedMemoryError::Protocol("Invalid magic number".to_string()));
        }
        
        if self.version != crate::SHARED_MEMORY_VERSION {
            return Err(SharedMemoryError::Protocol(
                format!("Unsupported version: {}", self.version)
            ));
        }
        
        Ok(())
    }
    
    /// Get message type
    pub fn get_message_type(&self) -> Result<MessageType> {
        MessageType::try_from(self.message_type)
            .map_err(|_| SharedMemoryError::Protocol(
                format!("Invalid message type: {}", self.message_type)
            ))
    }
    
    /// Verify payload checksum
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let expected = self.checksum.load(Ordering::Acquire);
        let actual = crc32fast::hash(payload);
        expected == actual
    }
}

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Data = 0x01,
    Heartbeat = 0x02,
    Acknowledgment = 0x03,
    Error = 0x04,
}

impl TryFrom<u8> for MessageType {
    type Error = ();
    
    fn try_from(value: u8) -> std::result::Result<Self, <MessageType as TryFrom<u8>>::Error> {
        match value {
            0x01 => Ok(MessageType::Data),
            0x02 => Ok(MessageType::Heartbeat),
            0x03 => Ok(MessageType::Acknowledgment),
            0x04 => Ok(MessageType::Error),
            _ => Err(()),
        }
    }
}

/// Shared memory message
#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: Bytes,
}

impl Message {
    /// Create a new data message
    pub fn new_data(payload: impl Into<Bytes>) -> Self {
        let payload = payload.into();
        let header = MessageHeader::new(MessageType::Data, &payload);
        Self { header, payload }
    }
    
    /// Create a heartbeat message
    pub fn new_heartbeat() -> Self {
        let payload = Bytes::new();
        let header = MessageHeader::new(MessageType::Heartbeat, &payload);
        Self { header, payload }
    }
    
    /// Create an acknowledgment message
    pub fn new_acknowledgment(sequence: u64) -> Self {
        let payload = Bytes::from(sequence.to_le_bytes().to_vec());
        let mut header = MessageHeader::new(MessageType::Acknowledgment, &payload);
        header.sequence.store(sequence, Ordering::Release);
        Self { header, payload }
    }
    
    /// Get total message size (header + payload)
    pub fn total_size(&self) -> usize {
        std::mem::size_of::<MessageHeader>() + self.payload.len()
    }
    
    /// Validate the message
    pub fn validate(&self) -> Result<()> {
        self.header.validate()?;
        
        if !self.header.verify_checksum(&self.payload) {
            return Err(SharedMemoryError::DataCorruption(
                "Checksum mismatch".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Set sequence number
    pub fn set_sequence(&mut self, sequence: u64) {
        self.header.sequence.store(sequence, Ordering::Release);
    }
    
    /// Get sequence number
    pub fn get_sequence(&self) -> u64 {
        self.header.sequence.load(Ordering::Acquire)
    }
}

/// Ring buffer implementation for shared memory communication
#[repr(C)]
pub struct RingBuffer {
    /// Buffer capacity
    pub capacity: AtomicU64,
    /// Write position
    pub write_pos: AtomicU64,
    /// Read position  
    pub read_pos: AtomicU64,
    /// Number of available bytes
    pub available: AtomicU64,
}

impl RingBuffer {
    /// Initialize a new ring buffer
    pub fn new(capacity: u64) -> Self {
        Self {
            capacity: AtomicU64::new(capacity),
            write_pos: AtomicU64::new(0),
            read_pos: AtomicU64::new(0),
            available: AtomicU64::new(0),
        }
    }
    
    /// Get available space for writing
    pub fn available_write_space(&self) -> u64 {
        let capacity = self.capacity.load(Ordering::Acquire);
        let available = self.available.load(Ordering::Acquire);
        capacity - available
    }
    
    /// Get available data for reading
    pub fn available_read_data(&self) -> u64 {
        self.available.load(Ordering::Acquire)
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.available.load(Ordering::Acquire) == 0
    }
    
    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.available.load(Ordering::Acquire) == self.capacity.load(Ordering::Acquire)
    }
    
    /// Calculate next position with wrap-around
    fn next_position(&self, pos: u64, size: u64) -> u64 {
        let capacity = self.capacity.load(Ordering::Acquire);
        (pos + size) % capacity
    }
}

/// Serializable message for cross-language communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableMessage {
    pub message_type: u8,
    pub sequence: u64,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

impl From<&Message> for SerializableMessage {
    fn from(msg: &Message) -> Self {
        Self {
            message_type: msg.header.message_type,
            sequence: msg.get_sequence(),
            timestamp: msg.header.timestamp.load(Ordering::Acquire),
            payload: msg.payload.to_vec(),
        }
    }
}

impl TryFrom<SerializableMessage> for Message {
    type Error = SharedMemoryError;
    
    fn try_from(msg: SerializableMessage) -> Result<Self> {
        let message_type = MessageType::try_from(msg.message_type)
            .map_err(|_| SharedMemoryError::Protocol("Invalid message type".to_string()))?;
        
        let payload = Bytes::from(msg.payload);
        let mut header = MessageHeader::new(message_type, &payload);
        header.sequence.store(msg.sequence, Ordering::Release);
        header.timestamp.store(msg.timestamp, Ordering::Release);
        
        Ok(Message { header, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let data = b"Hello, World!";
        let msg = Message::new_data(Bytes::from_static(data));
        
        assert_eq!(msg.header.get_message_type().unwrap(), MessageType::Data);
        assert_eq!(msg.payload.as_ref(), data);
        assert!(msg.validate().is_ok());
    }

    #[test]
    fn test_message_checksum() {
        let data = b"Test data";
        let msg = Message::new_data(Bytes::from_static(data));
        
        assert!(msg.header.verify_checksum(&msg.payload));
        
        // Test with corrupted data
        let corrupted_data = b"Corrupted";
        assert!(!msg.header.verify_checksum(corrupted_data));
    }

    #[test]
    fn test_ring_buffer() {
        let buffer = RingBuffer::new(1024);
        
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.available_write_space(), 1024);
        assert_eq!(buffer.available_read_data(), 0);
    }

    #[test]
    fn test_serializable_message() {
        let original = Message::new_data(Bytes::from_static(b"test"));
        let serializable = SerializableMessage::from(&original);
        let restored = Message::try_from(serializable).unwrap();
        
        assert_eq!(restored.payload, original.payload);
        assert_eq!(restored.header.message_type, original.header.message_type);
    }
}
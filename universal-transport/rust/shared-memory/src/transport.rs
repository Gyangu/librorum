//! Shared memory transport implementation

use crate::{
    SharedMemoryError, Result, SharedMemoryRegion, SharedMemoryManager,
    Message, MessageType, RingBuffer, PlatformUtils, PlatformOptimizations
};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::time::{Duration, timeout, sleep};
use tracing::{debug, warn, error, instrument};

/// Shared memory transport implementation
pub struct SharedMemoryTransport {
    /// Region manager
    manager: Arc<tokio::sync::Mutex<SharedMemoryManager>>,
    /// Message sequence counter
    sequence_counter: AtomicU64,
    /// Configuration
    config: SharedMemoryConfig,
}

/// Shared memory transport configuration
#[derive(Debug, Clone)]
pub struct SharedMemoryConfig {
    /// Default region size
    pub default_region_size: usize,
    /// Message timeout
    pub message_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Maximum retries
    pub max_retries: u32,
    /// Enable optimizations
    pub enable_optimizations: bool,
}

impl Default for SharedMemoryConfig {
    fn default() -> Self {
        Self {
            default_region_size: crate::DEFAULT_REGION_SIZE,
            message_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(5),
            max_retries: 3,
            enable_optimizations: true,
        }
    }
}

impl SharedMemoryTransport {
    /// Create a new shared memory transport
    pub fn new(config: SharedMemoryConfig) -> Self {
        Self {
            manager: Arc::new(tokio::sync::Mutex::new(SharedMemoryManager::new())),
            sequence_counter: AtomicU64::new(1),
            config,
        }
    }
    
    /// Create with default configuration
    pub fn new_default() -> Self {
        Self::new(SharedMemoryConfig::default())
    }
    
    /// Send a message to a shared memory region
    #[instrument(skip(self, data))]
    pub async fn send_to_region(&self, region_name: &str, data: &[u8]) -> Result<()> {
        let mut manager = self.manager.lock().await;
        let region = manager.get_or_create_region(region_name, self.config.default_region_size)?;
        drop(manager);
        
        // Create message
        let mut message = Message::new_data(Bytes::copy_from_slice(data));
        let sequence = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        message.set_sequence(sequence);
        
        debug!("Sending message {} to region {}", sequence, region_name);
        
        // Write message with timeout
        timeout(self.config.message_timeout, self.write_message_to_region(&region, &message))
            .await
            .map_err(|_| SharedMemoryError::Timeout("Send operation timed out".to_string()))?
    }
    
    /// Receive a message from a shared memory region
    #[instrument(skip(self))]
    pub async fn receive_from_region(&self, region_name: &str, timeout_duration: Duration) -> Result<Bytes> {
        let mut manager = self.manager.lock().await;
        let region = manager.get_or_create_region(region_name, self.config.default_region_size)?;
        drop(manager);
        
        debug!("Receiving message from region {}", region_name);
        
        // Read message with timeout
        let message = timeout(timeout_duration, self.read_message_from_region(&region))
            .await
            .map_err(|_| SharedMemoryError::Timeout("Receive operation timed out".to_string()))?;
        
        Ok(message?.payload)
    }
    
    /// Write a message to a shared memory region
    async fn write_message_to_region(&self, region: &SharedMemoryRegion, message: &Message) -> Result<()> {
        let total_size = message.total_size();
        
        // Retry logic for writing
        for attempt in 0..self.config.max_retries {
            match self.try_write_message(region, message, total_size).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt == self.config.max_retries - 1 => return Err(e),
                Err(e) => {
                    warn!("Write attempt {} failed: {}, retrying...", attempt + 1, e);
                    sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                }
            }
        }
        
        unreachable!()
    }
    
    /// Try to write a message (single attempt)
    async fn try_write_message(&self, region: &SharedMemoryRegion, message: &Message, total_size: usize) -> Result<()> {
        // Get ring buffer (assuming region is initialized)
        let ring_buffer = region.get_ring_buffer()?;
        
        // Check available space
        let available_space = ring_buffer.available_write_space() as usize;
        if available_space < total_size {
            return Err(SharedMemoryError::Platform(
                format!("Insufficient space: need {}, have {}", total_size, available_space)
            ));
        }
        
        // Get data buffer
        let data_buffer = region.get_data_buffer()?;
        let capacity = ring_buffer.capacity.load(Ordering::Acquire) as usize;
        let write_pos = ring_buffer.write_pos.load(Ordering::Acquire) as usize;
        
        // Serialize message header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &message.header as *const _ as *const u8,
                std::mem::size_of_val(&message.header)
            )
        };
        
        // Write header and payload with wrap-around
        self.write_with_wraparound(data_buffer, write_pos, capacity, header_bytes)?;
        let payload_pos = (write_pos + header_bytes.len()) % capacity;
        self.write_with_wraparound(data_buffer, payload_pos, capacity, &message.payload)?;
        
        // Update ring buffer state atomically
        let new_write_pos = (write_pos + total_size) % capacity;
        ring_buffer.write_pos.store(new_write_pos as u64, Ordering::Release);
        ring_buffer.available.fetch_add(total_size as u64, Ordering::SeqCst);
        
        debug!("Successfully wrote {} bytes at position {}", total_size, write_pos);
        Ok(())
    }
    
    /// Read a message from a shared memory region
    async fn read_message_from_region(&self, region: &SharedMemoryRegion) -> Result<Message> {
        // Poll for messages
        loop {
            match self.try_read_message(region)? {
                Some(message) => return Ok(message),
                None => {
                    // No message available, wait a bit
                    sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }
    
    /// Try to read a message (non-blocking)
    fn try_read_message(&self, region: &SharedMemoryRegion) -> Result<Option<Message>> {
        let ring_buffer = region.get_ring_buffer()?;
        
        // Check if data is available
        let available_data = ring_buffer.available_read_data() as usize;
        if available_data < std::mem::size_of::<crate::protocol::MessageHeader>() {
            return Ok(None);
        }
        
        let data_buffer = region.get_data_buffer()?;
        let capacity = ring_buffer.capacity.load(Ordering::Acquire) as usize;
        let read_pos = ring_buffer.read_pos.load(Ordering::Acquire) as usize;
        
        // Read message header first
        let mut header_bytes = vec![0u8; std::mem::size_of::<crate::protocol::MessageHeader>()];
        self.read_with_wraparound(data_buffer, read_pos, capacity, &mut header_bytes)?;
        
        // Deserialize header
        let header = unsafe {
            std::ptr::read(header_bytes.as_ptr() as *const crate::protocol::MessageHeader)
        };
        
        // Validate header
        header.validate()?;
        
        let payload_size = header.size.load(Ordering::Acquire) as usize;
        let total_size = header_bytes.len() + payload_size;
        
        // Check if we have enough data for the complete message
        if available_data < total_size {
            return Ok(None);
        }
        
        // Read payload
        let mut payload_bytes = vec![0u8; payload_size];
        let payload_pos = (read_pos + header_bytes.len()) % capacity;
        self.read_with_wraparound(data_buffer, payload_pos, capacity, &mut payload_bytes)?;
        
        // Create message
        let payload = Bytes::from(payload_bytes);
        let message = Message { header, payload };
        
        // Validate complete message
        message.validate()?;
        
        // Update ring buffer state
        let new_read_pos = (read_pos + total_size) % capacity;
        ring_buffer.read_pos.store(new_read_pos as u64, Ordering::Release);
        ring_buffer.available.fetch_sub(total_size as u64, Ordering::SeqCst);
        
        debug!("Successfully read {} bytes from position {}", total_size, read_pos);
        Ok(Some(message))
    }
    
    /// Write data with wrap-around handling
    fn write_with_wraparound(&self, buffer: &[u8], start_pos: usize, capacity: usize, data: &[u8]) -> Result<()> {
        let buffer_mut = unsafe {
            std::slice::from_raw_parts_mut(buffer.as_ptr() as *mut u8, capacity)
        };
        
        let end_pos = start_pos + data.len();
        
        if end_pos <= capacity {
            // No wrap-around needed
            buffer_mut[start_pos..end_pos].copy_from_slice(data);
        } else {
            // Handle wrap-around
            let first_part_size = capacity - start_pos;
            buffer_mut[start_pos..capacity].copy_from_slice(&data[..first_part_size]);
            buffer_mut[0..(end_pos - capacity)].copy_from_slice(&data[first_part_size..]);
        }
        
        Ok(())
    }
    
    /// Read data with wrap-around handling
    fn read_with_wraparound(&self, buffer: &[u8], start_pos: usize, capacity: usize, data: &mut [u8]) -> Result<()> {
        let end_pos = start_pos + data.len();
        
        if end_pos <= capacity {
            // No wrap-around needed
            data.copy_from_slice(&buffer[start_pos..end_pos]);
        } else {
            // Handle wrap-around
            let first_part_size = capacity - start_pos;
            data[..first_part_size].copy_from_slice(&buffer[start_pos..capacity]);
            data[first_part_size..].copy_from_slice(&buffer[0..(end_pos - capacity)]);
        }
        
        Ok(())
    }
    
    /// Initialize a shared memory region for communication
    pub async fn initialize_region(&self, region_name: &str, buffer_size: Option<usize>) -> Result<()> {
        let buffer_size = buffer_size.unwrap_or_else(|| {
            PlatformUtils::get_optimal_buffer_size()
        });
        
        let mut manager = self.manager.lock().await;
        let mut region = manager.get_or_create_region(region_name, self.config.default_region_size)?;
        
        // Initialize ring buffer
        let region_mut = Arc::get_mut(&mut region)
            .ok_or_else(|| SharedMemoryError::Platform("Cannot get mutable reference to region".to_string()))?;
        
        region_mut.initialize_ring_buffer(buffer_size)?;
        
        // Apply platform optimizations if enabled
        if self.config.enable_optimizations {
            let ptr = region_mut.as_mut_ptr();
            let size = region_mut.size;
            
            if let Err(e) = PlatformOptimizations::optimize_memory_access(ptr, size) {
                warn!("Failed to apply memory optimizations: {}", e);
            }
        }
        
        debug!("Initialized region {} with buffer size {}", region_name, buffer_size);
        Ok(())
    }
    
    /// Check if a region exists and is accessible
    pub async fn region_exists(&self, region_name: &str) -> bool {
        match SharedMemoryRegion::open(region_name) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    
    /// Get region statistics
    pub async fn get_region_stats(&self, region_name: &str) -> Result<RegionStats> {
        let manager = self.manager.lock().await;
        if let Some(region) = manager.get_region(region_name) {
            let ring_buffer = region.get_ring_buffer()?;
            
            Ok(RegionStats {
                region_name: region_name.to_string(),
                total_size: region.size,
                capacity: ring_buffer.capacity.load(Ordering::Acquire) as usize,
                available_data: ring_buffer.available_read_data() as usize,
                available_space: ring_buffer.available_write_space() as usize,
                write_position: ring_buffer.write_pos.load(Ordering::Acquire) as usize,
                read_position: ring_buffer.read_pos.load(Ordering::Acquire) as usize,
            })
        } else {
            Err(SharedMemoryError::RegionNotFound(region_name.to_string()))
        }
    }
}

/// Region statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionStats {
    pub region_name: String,
    pub total_size: usize,
    pub capacity: usize,
    pub available_data: usize,
    pub available_space: usize,
    pub write_position: usize,
    pub read_position: usize,
}

// TODO: Implement Transport trait once core types are stabilized
// This will be implemented after resolving the circular dependency issue

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_shared_memory_transport_creation() {
        let transport = SharedMemoryTransport::new_default();
        assert_eq!(transport.sequence_counter.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn test_region_initialization() {
        let transport = SharedMemoryTransport::new_default();
        let region_name = "test_init_region";
        
        let result = transport.initialize_region(region_name, Some(8192)).await;
        assert!(result.is_ok());
        
        let stats = transport.get_region_stats(region_name).await;
        assert!(stats.is_ok());
        
        let stats = stats.unwrap();
        assert_eq!(stats.region_name, region_name);
        assert_eq!(stats.capacity, 8192);
    }

    #[tokio::test]
    async fn test_send_receive() {
        let transport = SharedMemoryTransport::new_default();
        let region_name = "test_send_receive";
        
        // Initialize region
        transport.initialize_region(region_name, Some(4096)).await.unwrap();
        
        // Send data
        let test_data = b"Hello, Shared Memory!";
        transport.send_to_region(region_name, test_data).await.unwrap();
        
        // Receive data
        let received = transport.receive_from_region(region_name, Duration::from_secs(1)).await.unwrap();
        assert_eq!(received.as_ref(), test_data);
    }

    #[tokio::test]
    async fn test_region_exists() {
        let transport = SharedMemoryTransport::new_default();
        
        assert!(!transport.region_exists("nonexistent_region").await);
        
        transport.initialize_region("existing_region", None).await.unwrap();
        assert!(transport.region_exists("existing_region").await);
    }
}
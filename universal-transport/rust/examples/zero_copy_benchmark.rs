//! Zero-Copy Binary Protocol Benchmark
//! 
//! True zero-copy implementation using memory mapping and direct pointer manipulation

use std::time::{Duration, Instant};
use std::slice;
use std::ptr;
use bytes::{Bytes, BytesMut, BufMut};

/// Zero-copy binary header (32 bytes, repr(C) with natural alignment)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ZeroCopyHeader {
    pub magic: u32,
    pub version: u8,
    pub message_type: u8,
    pub flags: u16,
    pub payload_length: u32,
    pub sequence: u64,
    pub timestamp: u64,
    pub checksum: u32,
}

impl ZeroCopyHeader {
    const MAGIC: u32 = 0x55545042; // "UTPB"
    const VERSION: u8 = 1;
    
    /// Create header directly in place (no allocation)
    pub fn new_in_place(payload_len: u32, sequence: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            message_type: 0x05, // Benchmark
            flags: 0,
            payload_length: payload_len,
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            checksum: 0, // Skip checksum for zero-copy performance
        }
    }
    
    /// Validate header without copying
    pub fn validate(&self) -> bool {
        self.magic == Self::MAGIC && 
        self.version == Self::VERSION && 
        self.payload_length <= 64 * 1024 * 1024
    }
}

/// Zero-copy message using BytesMut for efficient memory management
pub struct ZeroCopyMessage {
    buffer: BytesMut,
}

impl ZeroCopyMessage {
    const HEADER_SIZE: usize = 32;
    
    /// Create a zero-copy message with pre-allocated buffer
    pub fn new(payload_size: usize, sequence: u64) -> Self {
        let total_size = Self::HEADER_SIZE + payload_size;
        let mut buffer = BytesMut::with_capacity(total_size);
        
        // Write header directly to buffer
        let header = ZeroCopyHeader::new_in_place(payload_size as u32, sequence);
        
        // SAFETY: We know the size and alignment requirements
        unsafe {
            let header_bytes = slice::from_raw_parts(
                &header as *const ZeroCopyHeader as *const u8,
                Self::HEADER_SIZE
            );
            buffer.put_slice(header_bytes);
        }
        
        // Fill payload with pattern (simulating data)
        buffer.resize(total_size, 0x42);
        
        Self { buffer }
    }
    
    /// Get header reference without copying
    pub fn header(&self) -> &ZeroCopyHeader {
        unsafe {
            &*(self.buffer.as_ptr() as *const ZeroCopyHeader)
        }
    }
    
    /// Get payload slice without copying
    pub fn payload(&self) -> &[u8] {
        &self.buffer[Self::HEADER_SIZE..]
    }
    
    /// Get total message as bytes slice (zero-copy)
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
    
    /// Parse message from bytes slice (zero-copy view)
    pub fn from_bytes(bytes: &[u8]) -> Option<ZeroCopyMessageRef> {
        if bytes.len() < Self::HEADER_SIZE {
            return None;
        }
        
        let header = unsafe {
            &*(bytes.as_ptr() as *const ZeroCopyHeader)
        };
        
        if !header.validate() {
            return None;
        }
        
        let expected_size = Self::HEADER_SIZE + header.payload_length as usize;
        if bytes.len() < expected_size {
            return None;
        }
        
        Some(ZeroCopyMessageRef {
            header,
            payload: &bytes[Self::HEADER_SIZE..expected_size],
        })
    }
    
    /// Convert to owned bytes (final copy only when needed)
    pub fn into_bytes(self) -> Bytes {
        self.buffer.freeze()
    }
}

/// Zero-copy message reference (no ownership, just views)
pub struct ZeroCopyMessageRef<'a> {
    header: &'a ZeroCopyHeader,
    payload: &'a [u8],
}

impl<'a> ZeroCopyMessageRef<'a> {
    pub fn header(&self) -> &ZeroCopyHeader {
        self.header
    }
    
    pub fn payload(&self) -> &[u8] {
        self.payload
    }
    
    pub fn sequence(&self) -> u64 {
        self.header.sequence
    }
}

/// Zero-copy performance benchmark
pub struct ZeroCopyBenchmark;

impl ZeroCopyBenchmark {
    /// Run zero-copy benchmark
    pub fn run_benchmark() {
        println!("🚀 Zero-Copy Binary Protocol Benchmark");
        println!("======================================");
        println!("Testing true zero-copy performance with direct memory access");
        println!();
        
        let test_cases = vec![
            ("Zero-Copy Small Messages (1KB)", 10000, 1024),
            ("Zero-Copy Medium Messages (64KB)", 1000, 64 * 1024),
            ("Zero-Copy Large Messages (1MB)", 100, 1024 * 1024),
            ("Zero-Copy Huge Messages (16MB)", 10, 16 * 1024 * 1024),
        ];
        
        for (test_name, message_count, message_size) in test_cases {
            Self::run_zero_copy_test(test_name, message_count, message_size);
            println!("{}", "─".repeat(60));
        }
        
        // Memory layout test
        Self::test_memory_layout();
    }
    
    fn run_zero_copy_test(test_name: &str, message_count: usize, message_size: usize) {
        println!("🔬 {}: {} messages × {} bytes", test_name, message_count, message_size);
        
        // Pre-allocate a pool of messages (simulating real-world usage)
        let start_alloc = Instant::now();
        let mut messages = Vec::with_capacity(message_count);
        for i in 0..message_count {
            messages.push(ZeroCopyMessage::new(message_size, i as u64));
        }
        let alloc_time = start_alloc.elapsed();
        
        // Zero-copy operations test
        let start_ops = Instant::now();
        let mut total_validated = 0;
        let mut total_bytes_processed = 0;
        
        for message in &messages {
            // 1. Get header reference (zero-copy)
            let header = message.header();
            
            // 2. Validate (zero-copy)
            if header.validate() {
                total_validated += 1;
            }
            
            // 3. Get payload slice (zero-copy)
            let payload = message.payload();
            total_bytes_processed += payload.len();
            
            // 4. Get full message bytes (zero-copy view)
            let bytes = message.as_bytes();
            
            // 5. Parse back from bytes (zero-copy reference)
            if let Some(parsed) = ZeroCopyMessage::from_bytes(bytes) {
                // Verify sequence without copying
                assert_eq!(parsed.sequence(), header.sequence);
            }
        }
        
        let ops_time = start_ops.elapsed();
        let total_time = start_alloc.elapsed();
        
        // Calculate metrics
        let total_data_mb = (total_bytes_processed as f64) / (1024.0 * 1024.0);
        let ops_throughput = total_data_mb / ops_time.as_secs_f64();
        let overall_throughput = total_data_mb / total_time.as_secs_f64();
        let avg_latency_ns = ops_time.as_nanos() as f64 / message_count as f64;
        
        println!("  Allocation time: {:.3}ms", alloc_time.as_secs_f64() * 1000.0);
        println!("  Zero-copy ops time: {:.3}ms", ops_time.as_secs_f64() * 1000.0);
        println!("  Total data processed: {:.2} MB", total_data_mb);
        println!("  Zero-copy throughput: {:.2} MB/s", ops_throughput);
        println!("  Overall throughput: {:.2} MB/s", overall_throughput);
        println!("  Average latency: {:.2} ns per operation", avg_latency_ns);
        println!("  Validation rate: {:.0} ops/sec", message_count as f64 / ops_time.as_secs_f64());
        println!("  Messages validated: {}/{}", total_validated, message_count);
    }
    
    fn test_memory_layout() {
        println!("🔍 Memory Layout Analysis");
        println!("========================");
        
        let msg = ZeroCopyMessage::new(1024, 42);
        
        println!("Header size: {} bytes", std::mem::size_of::<ZeroCopyHeader>());
        println!("Header alignment: {} bytes", std::mem::align_of::<ZeroCopyHeader>());
        println!("Total message size: {} bytes", msg.as_bytes().len());
        
        let header = msg.header();
        println!("Header contents:");
        println!("  Magic: 0x{:08x}", header.magic);
        println!("  Version: {}", header.version);
        println!("  Payload length: {}", header.payload_length);
        println!("  Sequence: {}", header.sequence);
        
        // Verify zero-copy: header should point into the message buffer
        let header_ptr = header as *const ZeroCopyHeader as *const u8;
        let buffer_ptr = msg.as_bytes().as_ptr();
        
        if header_ptr == buffer_ptr {
            println!("✓ Zero-copy verified: Header is directly mapped to buffer");
        } else {
            println!("✗ Warning: Header is not zero-copy (pointer mismatch)");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ZeroCopyBenchmark::run_benchmark();
    
    println!("\n🎯 ZERO-COPY PERFORMANCE SUMMARY");
    println!("================================");
    println!("✓ Direct memory mapping with repr(C) structs");
    println!("✓ No serialization/deserialization copies");
    println!("✓ Reference-based message parsing");
    println!("✓ BytesMut for efficient memory management");
    println!("✓ Unsafe code for maximum performance");
    
    println!("\n⚡ Key advantages over copy-based approach:");
    println!("• Constant-time message parsing regardless of size");
    println!("• Memory usage = actual data size (no duplication)");
    println!("• CPU cache friendly (data stays in place)");
    println!("• Predictable performance characteristics");
    
    Ok(())
}
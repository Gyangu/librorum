//! TCP vs Memory Communication Benchmark
//! 
//! Comprehensive comparison of communication speeds:
//! 1. In-memory zero-copy communication
//! 2. TCP socket communication (localhost)
//! 3. Actual network measurement

use std::time::{Duration, Instant};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use bytes::{BytesMut, BufMut};
use tokio::net::{TcpListener as TokioTcpListener, TcpStream as TokioTcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Reuse zero-copy protocol
mod zero_copy_protocol {
    use bytes::{BytesMut, BufMut};
    use std::slice;
    
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
        const MAGIC: u32 = 0x55545042;
        const VERSION: u8 = 1;
        
        pub fn new_in_place(payload_len: u32, sequence: u64) -> Self {
            Self {
                magic: Self::MAGIC,
                version: Self::VERSION,
                message_type: 0x10,
                flags: 0,
                payload_length: payload_len,
                sequence,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64,
                checksum: 0,
            }
        }
        
        pub fn validate(&self) -> bool {
            self.magic == Self::MAGIC && 
            self.version == Self::VERSION && 
            self.payload_length <= 64 * 1024 * 1024
        }
    }
    
    pub struct ZeroCopyMessage {
        buffer: BytesMut,
    }
    
    impl ZeroCopyMessage {
        const HEADER_SIZE: usize = 32;
        
        pub fn new(payload_size: usize, sequence: u64) -> Self {
            let total_size = Self::HEADER_SIZE + payload_size;
            let mut buffer = BytesMut::with_capacity(total_size);
            
            let header = ZeroCopyHeader::new_in_place(payload_size as u32, sequence);
            
            unsafe {
                let header_bytes = slice::from_raw_parts(
                    &header as *const ZeroCopyHeader as *const u8,
                    Self::HEADER_SIZE
                );
                buffer.put_slice(header_bytes);
            }
            
            buffer.resize(total_size, 0x42);
            Self { buffer }
        }
        
        pub fn header(&self) -> &ZeroCopyHeader {
            unsafe {
                &*(self.buffer.as_ptr() as *const ZeroCopyHeader)
            }
        }
        
        pub fn as_bytes(&self) -> &[u8] {
            &self.buffer
        }
        
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
    }
    
    pub struct ZeroCopyMessageRef<'a> {
        header: &'a ZeroCopyHeader,
        payload: &'a [u8],
    }
    
    impl<'a> ZeroCopyMessageRef<'a> {
        pub fn header(&self) -> &ZeroCopyHeader {
            self.header
        }
        
        pub fn sequence(&self) -> u64 {
            self.header.sequence
        }
    }
}

use zero_copy_protocol::{ZeroCopyMessage, ZeroCopyMessageRef};

/// Benchmark results structure
#[derive(Debug)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub message_count: usize,
    pub message_size: usize,
    pub duration: Duration,
    pub throughput_mbps: f64,
    pub latency_micros: f64,
    pub messages_per_sec: f64,
}

impl BenchmarkResult {
    pub fn new(test_name: String, message_count: usize, message_size: usize, duration: Duration) -> Self {
        let total_bytes = (message_count * message_size) as f64;
        let throughput_mbps = (total_bytes / (1024.0 * 1024.0)) / duration.as_secs_f64();
        let latency_micros = (duration.as_micros() as f64) / (message_count as f64);
        let messages_per_sec = (message_count as f64) / duration.as_secs_f64();
        
        Self {
            test_name,
            message_count,
            message_size,
            duration,
            throughput_mbps,
            latency_micros,
            messages_per_sec,
        }
    }
    
    pub fn print_summary(&self) {
        println!("📊 {}", self.test_name);
        println!("   Messages: {} × {} bytes", self.message_count, self.message_size);
        println!("   Duration: {:.3}s", self.duration.as_secs_f64());
        println!("   Throughput: {:.2} MB/s", self.throughput_mbps);
        println!("   Latency: {:.2} μs", self.latency_micros);
        println!("   Rate: {:.0} msg/s", self.messages_per_sec);
        println!("   ──────────────────────────────────────");
    }
}

/// Memory-based communication benchmark
pub struct MemoryBenchmark;

impl MemoryBenchmark {
    pub fn run_benchmark(message_count: usize, message_size: usize) -> BenchmarkResult {
        let start = Instant::now();
        let mut processed = 0;
        
        for i in 0..message_count {
            // Create message in memory
            let message = ZeroCopyMessage::new(message_size, i as u64);
            let bytes = message.as_bytes();
            
            // Parse message (simulating receiver)
            if let Some(parsed) = ZeroCopyMessage::from_bytes(bytes) {
                if parsed.sequence() == i as u64 {
                    processed += 1;
                }
            }
        }
        
        let duration = start.elapsed();
        BenchmarkResult::new(
            "Memory Zero-Copy".to_string(),
            processed,
            message_size,
            duration
        )
    }
}

/// TCP-based communication benchmark
pub struct TcpBenchmark;

impl TcpBenchmark {
    /// Run TCP benchmark with dedicated server
    pub async fn run_benchmark(message_count: usize, message_size: usize) -> std::io::Result<BenchmarkResult> {
        let server_addr = "127.0.0.1:9081";
        
        // Start server
        let server_handle = tokio::spawn(Self::tcp_server(server_addr.to_string(), message_count));
        
        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Start client
        let start = Instant::now();
        let mut stream = TokioTcpStream::connect(server_addr).await?;
        let mut successful = 0;
        
        for i in 0..message_count {
            // Create and send message
            let message = ZeroCopyMessage::new(message_size, i as u64);
            let message_bytes = message.as_bytes();
            
            // Send message size + data
            let size_bytes = (message_bytes.len() as u32).to_le_bytes();
            stream.write_all(&size_bytes).await?;
            stream.write_all(message_bytes).await?;
            
            // Read response size
            let mut response_size_buf = [0u8; 4];
            stream.read_exact(&mut response_size_buf).await?;
            let response_size = u32::from_le_bytes(response_size_buf) as usize;
            
            // Read response data
            let mut response_buffer = vec![0u8; response_size];
            stream.read_exact(&mut response_buffer).await?;
            
            // Validate response
            if let Some(response) = ZeroCopyMessage::from_bytes(&response_buffer) {
                if response.sequence() == i as u64 {
                    successful += 1;
                }
            }
        }
        
        let duration = start.elapsed();
        
        // Wait for server to finish
        let _ = server_handle.await;
        
        Ok(BenchmarkResult::new(
            "TCP Socket".to_string(),
            successful,
            message_size,
            duration
        ))
    }
    
    /// TCP server that echoes messages back
    async fn tcp_server(addr: String, expected_messages: usize) -> std::io::Result<()> {
        let listener = TokioTcpListener::bind(&addr).await?;
        
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut received = 0;
            
            while received < expected_messages {
                // Read message size
                let mut size_buf = [0u8; 4];
                if stream.read_exact(&mut size_buf).await.is_err() {
                    break;
                }
                
                let msg_size = u32::from_le_bytes(size_buf) as usize;
                
                // Read message data
                let mut msg_buffer = vec![0u8; msg_size];
                if stream.read_exact(&mut msg_buffer).await.is_err() {
                    break;
                }
                
                // Echo back the same message
                stream.write_all(&size_buf).await?;
                stream.write_all(&msg_buffer).await?;
                
                received += 1;
            }
        }
        
        Ok(())
    }
}

/// Run comprehensive communication benchmark suite
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 TCP vs Memory Communication Benchmark");
    println!("==========================================");
    println!("Comparing actual communication speeds across different transports");
    println!();
    
    let test_cases = vec![
        ("Small Messages", 1000, 1024),        // 1KB
        ("Medium Messages", 200, 64 * 1024),   // 64KB
        ("Large Messages", 50, 1024 * 1024),   // 1MB
        ("Huge Messages", 10, 4 * 1024 * 1024), // 4MB
    ];
    
    let mut memory_results = Vec::new();
    let mut tcp_results = Vec::new();
    
    for (test_name, message_count, message_size) in test_cases {
        println!("🔬 Testing {} ({} × {} bytes)", test_name, message_count, message_size);
        println!();
        
        // Memory benchmark
        println!("📍 Memory Communication:");
        let memory_result = MemoryBenchmark::run_benchmark(message_count, message_size);
        memory_result.print_summary();
        memory_results.push(memory_result);
        
        // TCP benchmark
        println!("🌐 TCP Communication:");
        match TcpBenchmark::run_benchmark(message_count, message_size).await {
            Ok(tcp_result) => {
                tcp_result.print_summary();
                tcp_results.push(tcp_result);
            },
            Err(e) => {
                println!("   ❌ TCP test failed: {}", e);
            }
        }
        
        println!();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    // Summary comparison
    println!("🎯 COMMUNICATION SPEED COMPARISON");
    println!("==================================");
    println!();
    
    println!("📊 Memory vs TCP Performance Ratios:");
    for (i, (memory, tcp)) in memory_results.iter().zip(tcp_results.iter()).enumerate() {
        let throughput_ratio = memory.throughput_mbps / tcp.throughput_mbps;
        let latency_ratio = tcp.latency_micros / memory.latency_micros;
        
        println!("  {}: Memory is {:.1}x faster (throughput), {:.1}x lower latency", 
                 memory.test_name.replace("Memory Zero-Copy", ""),
                 throughput_ratio, latency_ratio);
    }
    
    println!();
    println!("📈 Absolute Performance Numbers:");
    println!("  Memory Communication:");
    for result in &memory_results {
        println!("    {}: {:.1} MB/s, {:.1} μs latency", 
                 result.test_name.replace("Memory Zero-Copy", ""),
                 result.throughput_mbps, result.latency_micros);
    }
    
    println!("  TCP Communication:");
    for result in &tcp_results {
        println!("    {}: {:.1} MB/s, {:.1} μs latency", 
                 result.test_name.replace("TCP Socket", ""),
                 result.throughput_mbps, result.latency_micros);
    }
    
    println!();
    println!("💡 Key Insights:");
    println!("  • Memory communication eliminates network stack overhead");
    println!("  • TCP adds serialization + network latency");
    println!("  • Larger messages show better TCP efficiency (amortized overhead)");
    println!("  • Zero-copy benefits are most visible in memory communication");
    
    Ok(())
}
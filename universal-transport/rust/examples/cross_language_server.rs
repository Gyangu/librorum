//! Cross-Language Communication Server
//! 
//! TCP server that handles both Rust and Swift clients using the zero-copy binary protocol
//! Provides actual cross-language performance testing with real IPC

use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::io::{Read, Write};
use std::thread;
use tokio::sync::RwLock;
use bytes::{BytesMut, BufMut};
use serde::{Deserialize, Serialize};

// Zero-copy binary protocol (embedded for cross-language compatibility)
mod zero_copy_protocol {
    use bytes::{BytesMut, BufMut};
    use std::slice;
    
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
        
        pub fn new_in_place(payload_len: u32, sequence: u64) -> Self {
            Self {
                magic: Self::MAGIC,
                version: Self::VERSION,
                message_type: 0x10, // BenchmarkRequest
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
    
    /// Zero-copy message using BytesMut
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
        
        pub fn payload(&self) -> &[u8] {
            &self.buffer[Self::HEADER_SIZE..]
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
        
        pub fn payload(&self) -> &[u8] {
            self.payload
        }
        
        pub fn sequence(&self) -> u64 {
            self.header.sequence
        }
    }
}

use zero_copy_protocol::{ZeroCopyMessage, ZeroCopyMessageRef};

/// Cross-language message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    BenchmarkRequest = 0x10,
    BenchmarkResponse = 0x11,
    ClientInfo = 0x12,
    ServerStats = 0x13,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_id: String,
    pub language: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Server statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    pub total_messages: u64,
    pub total_bytes: u64,
    pub rust_clients: u32,
    pub swift_clients: u32,
    pub uptime_seconds: u64,
    pub average_latency_micros: f64,
}

/// Cross-language benchmark server
pub struct CrossLanguageServer {
    listener: TcpListener,
    stats: Arc<RwLock<ServerStats>>,
    client_counter: Arc<AtomicU64>,
    start_time: Instant,
}

impl CrossLanguageServer {
    /// Create a new cross-language server
    pub fn new(port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        let stats = Arc::new(RwLock::new(ServerStats {
            total_messages: 0,
            total_bytes: 0,
            rust_clients: 0,
            swift_clients: 0,
            uptime_seconds: 0,
            average_latency_micros: 0.0,
        }));
        
        println!("🚀 Cross-Language Server listening on port {}", port);
        
        Ok(Self {
            listener,
            stats,
            client_counter: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        })
    }
    
    /// Start the server
    pub async fn start(&self) -> std::io::Result<()> {
        println!("📡 Starting cross-language communication server...");
        
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let client_id = self.client_counter.fetch_add(1, Ordering::SeqCst);
                    let stats = Arc::clone(&self.stats);
                    let start_time = self.start_time;
                    
                    thread::spawn(move || {
                        if let Err(e) = Self::handle_client(stream, client_id, stats, start_time) {
                            eprintln!("❌ Client {} error: {}", client_id, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Connection error: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle a client connection
    fn handle_client(
        mut stream: TcpStream,
        client_id: u64,
        stats: Arc<RwLock<ServerStats>>,
        start_time: Instant,
    ) -> std::io::Result<()> {
        println!("🔗 Client {} connected", client_id);
        
        let mut buffer = [0u8; 1024 * 1024]; // 1MB buffer
        let mut client_language = String::from("unknown");
        let mut total_messages = 0u64;
        let mut total_bytes = 0u64;
        let mut latency_sum = 0.0;
        
        loop {
            // Read message size (4 bytes)
            let mut size_buf = [0u8; 4];
            match stream.read_exact(&mut size_buf) {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
            
            let msg_size = u32::from_le_bytes(size_buf) as usize;
            if msg_size > buffer.len() {
                eprintln!("❌ Message too large: {} bytes", msg_size);
                continue;
            }
            
            // Read message data
            let msg_data = &mut buffer[..msg_size];
            stream.read_exact(msg_data)?;
            
            let request_time = Instant::now();
            
            // Parse zero-copy message
            if let Some(message) = ZeroCopyMessage::from_bytes(msg_data) {
                let header = message.header();
                let payload = message.payload();
                
                // Handle different message types
                match header.message_type {
                    0x10 => { // BenchmarkRequest
                        total_messages += 1;
                        total_bytes += payload.len() as u64;
                        
                        // Create response with same payload (echo)
                        let response = ZeroCopyMessage::new(payload.len(), header.sequence);
                        let response_bytes = response.as_bytes();
                        
                        // Send response size and data
                        let size_bytes = (response_bytes.len() as u32).to_le_bytes();
                        stream.write_all(&size_bytes)?;
                        stream.write_all(response_bytes)?;
                        
                        let latency = request_time.elapsed().as_micros() as f64;
                        latency_sum += latency;
                    }
                    0x12 => { // ClientInfo
                        if let Ok(info_str) = std::str::from_utf8(payload) {
                            if let Ok(info) = serde_json::from_str::<ClientInfo>(info_str) {
                                client_language = info.language.clone();
                                println!("🏷️  Client {} identified as: {} ({})", client_id, info.language, info.version);
                                
                                // Update client count
                                let mut stats_guard = stats.try_write().unwrap();
                                match info.language.as_str() {
                                    "rust" => stats_guard.rust_clients += 1,
                                    "swift" => stats_guard.swift_clients += 1,
                                    _ => {}
                                }
                            }
                        }
                    }
                    0x13 => { // ServerStats request
                        let stats_guard = stats.try_read().unwrap();
                        let uptime = start_time.elapsed().as_secs();
                        let avg_latency = if total_messages > 0 { latency_sum / total_messages as f64 } else { 0.0 };
                        
                        let stats_response = ServerStats {
                            total_messages: stats_guard.total_messages + total_messages,
                            total_bytes: stats_guard.total_bytes + total_bytes,
                            rust_clients: stats_guard.rust_clients,
                            swift_clients: stats_guard.swift_clients,
                            uptime_seconds: uptime,
                            average_latency_micros: avg_latency,
                        };
                        
                        let stats_json = serde_json::to_string(&stats_response).unwrap();
                        let response = ZeroCopyMessage::new(stats_json.len(), header.sequence);
                        let response_bytes = response.as_bytes();
                        
                        let size_bytes = (response_bytes.len() as u32).to_le_bytes();
                        stream.write_all(&size_bytes)?;
                        stream.write_all(response_bytes)?;
                    }
                    _ => {
                        eprintln!("⚠️  Unknown message type: 0x{:02x}", header.message_type);
                    }
                }
            } else {
                eprintln!("❌ Invalid message format from client {}", client_id);
            }
        }
        
        println!("📊 Client {} ({}) disconnected. Messages: {}, Bytes: {}, Avg latency: {:.2}μs", 
                 client_id, client_language, total_messages, total_bytes, 
                 if total_messages > 0 { latency_sum / total_messages as f64 } else { 0.0 });
        
        // Update server stats
        let mut stats_guard = stats.try_write().unwrap();
        stats_guard.total_messages += total_messages;
        stats_guard.total_bytes += total_bytes;
        
        Ok(())
    }
}

/// Cross-language benchmark client
pub struct CrossLanguageBenchmarkClient {
    server_addr: String,
    client_info: ClientInfo,
}

impl CrossLanguageBenchmarkClient {
    /// Create a new benchmark client
    pub fn new(server_addr: String, client_id: String) -> Self {
        let client_info = ClientInfo {
            client_id,
            language: "rust".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["zero-copy".to_string(), "binary-protocol".to_string()],
        };
        
        Self {
            server_addr,
            client_info,
        }
    }
    
    /// Connect to server and run benchmark
    pub async fn run_benchmark(&self, message_count: usize, message_size: usize) -> std::io::Result<()> {
        println!("🔌 Connecting to server at {}", self.server_addr);
        
        let mut stream = TcpStream::connect(&self.server_addr)?;
        
        // Send client info
        let info_json = serde_json::to_string(&self.client_info).unwrap();
        let info_message = ZeroCopyMessage::new(info_json.len(), 0);
        let info_bytes = info_message.as_bytes();
        
        let size_bytes = (info_bytes.len() as u32).to_le_bytes();
        stream.write_all(&size_bytes)?;
        stream.write_all(info_bytes)?;
        
        println!("📊 Starting benchmark: {} messages × {} bytes", message_count, message_size);
        
        let mut total_latency = 0.0;
        let mut successful_messages = 0;
        let benchmark_start = Instant::now();
        
        for i in 0..message_count {
            let message_start = Instant::now();
            
            // Create benchmark message
            let message = ZeroCopyMessage::new(message_size, i as u64);
            let message_bytes = message.as_bytes();
            
            // Send message
            let size_bytes = (message_bytes.len() as u32).to_le_bytes();
            stream.write_all(&size_bytes)?;
            stream.write_all(message_bytes)?;
            
            // Read response size
            let mut response_size_buf = [0u8; 4];
            stream.read_exact(&mut response_size_buf)?;
            let response_size = u32::from_le_bytes(response_size_buf) as usize;
            
            // Read response data
            let mut response_buffer = vec![0u8; response_size];
            stream.read_exact(&mut response_buffer)?;
            
            // Validate response
            if let Some(response) = ZeroCopyMessage::from_bytes(&response_buffer) {
                if response.header().sequence == i as u64 {
                    successful_messages += 1;
                    total_latency += message_start.elapsed().as_micros() as f64;
                }
            }
            
            // Progress indicator
            if i % 100 == 0 && i > 0 {
                println!("📈 Progress: {}/{} messages", i, message_count);
            }
        }
        
        let benchmark_duration = benchmark_start.elapsed();
        
        // Calculate metrics
        let total_bytes = successful_messages * message_size;
        let throughput_mbps = (total_bytes as f64 / (1024.0 * 1024.0)) / benchmark_duration.as_secs_f64();
        let avg_latency_micros = total_latency / successful_messages as f64;
        
        println!("🎯 RUST CLIENT BENCHMARK RESULTS");
        println!("================================");
        println!("Messages: {}/{}", successful_messages, message_count);
        println!("Total data: {:.2} MB", total_bytes as f64 / (1024.0 * 1024.0));
        println!("Duration: {:.3}s", benchmark_duration.as_secs_f64());
        println!("Throughput: {:.2} MB/s", throughput_mbps);
        println!("Average latency: {:.2} μs", avg_latency_micros);
        println!("Success rate: {:.2}%", (successful_messages as f64 / message_count as f64) * 100.0);
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: {} <server|client> [options]", args[0]);
        println!("  server: Start cross-language server");
        println!("  client: Run benchmark client");
        return Ok(());
    }
    
    match args[1].as_str() {
        "server" => {
            let server = CrossLanguageServer::new(9080)?;
            server.start().await?;
        }
        "client" => {
            let client = CrossLanguageBenchmarkClient::new(
                "127.0.0.1:9080".to_string(),
                "rust-client-1".to_string(),
            );
            
            // Run different test cases
            let test_cases = vec![
                (500, 1024),        // 500 × 1KB
                (100, 64 * 1024),   // 100 × 64KB
                (50, 1024 * 1024),  // 50 × 1MB
            ];
            
            for (count, size) in test_cases {
                println!("\n🔬 Testing {} messages × {} bytes", count, size);
                client.run_benchmark(count, size).await?;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
        _ => {
            println!("❌ Unknown command: {}", args[1]);
        }
    }
    
    Ok(())
}
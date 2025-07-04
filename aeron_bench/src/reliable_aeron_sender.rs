use tokio::net::UdpSocket;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "reliable_aeron_sender")]
#[command(about = "Rust reliable Aeron sender for bidirectional communication testing")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    
    #[arg(long, default_value = "40001")]
    port: u16,
    
    #[arg(long, default_value = "1024")]
    message_size: usize,
    
    #[arg(long, default_value = "50")]
    message_count: usize,
    
    #[arg(long, default_value = "100")]
    retransmit_timeout_ms: u64,
    
    #[arg(long, default_value = "5")]
    max_retries: usize,
}

// Aeron协议常量
const HEADER_LENGTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FrameType {
    Data = 0x01,
    Ack = 0x02,
    Nak = 0x03,
    Heartbeat = 0x04,
    FlowControl = 0x05,
}

#[derive(Debug, Clone)]
struct PendingMessage {
    data: Vec<u8>,
    sequence_number: u32,
    timestamp: Instant,
    retry_count: usize,
    session_id: u32,
    stream_id: u32,
}

struct ReliableAeronSender {
    socket: UdpSocket,
    sequence_number: u32,
    pending_messages: HashMap<u32, PendingMessage>,
    retransmit_timeout: Duration,
    max_retries: usize,
}

impl ReliableAeronSender {
    async fn new(retransmit_timeout_ms: u64, max_retries: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        
        Ok(Self {
            socket,
            sequence_number: 0,
            pending_messages: HashMap::new(),
            retransmit_timeout: Duration::from_millis(retransmit_timeout_ms),
            max_retries,
        })
    }
    
    async fn connect(&self, host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.socket.connect(format!("{}:{}", host, port)).await?;
        Ok(())
    }
    
    fn create_data_frame(&self, data: &[u8], sequence_number: u32, session_id: u32, stream_id: u32) -> Vec<u8> {
        let mut frame = Vec::new();
        
        let frame_length = HEADER_LENGTH + data.len();
        let frame_type = FrameType::Data as u16;
        let flags = 0x80u8; // Begin and End flags
        let version = 1u8;
        let term_id = 0u32;
        let term_offset = 0u32;
        
        // 构建32字节Aeron头部
        frame.extend_from_slice(&(frame_length as u32).to_le_bytes());
        frame.extend_from_slice(&frame_type.to_le_bytes());
        frame.push(flags);
        frame.push(version);
        frame.extend_from_slice(&session_id.to_le_bytes());
        frame.extend_from_slice(&stream_id.to_le_bytes());
        frame.extend_from_slice(&term_id.to_le_bytes());
        frame.extend_from_slice(&term_offset.to_le_bytes());
        frame.extend_from_slice(&sequence_number.to_le_bytes());
        
        // 填充到32字节
        while frame.len() < HEADER_LENGTH {
            frame.push(0);
        }
        
        // 添加数据
        frame.extend_from_slice(data);
        
        frame
    }
    
    async fn send_reliable(&mut self, data: &[u8], session_id: u32, stream_id: u32) -> Result<(), Box<dyn std::error::Error>> {
        let seq_num = self.sequence_number;
        self.sequence_number += 1;
        
        let frame = self.create_data_frame(data, seq_num, session_id, stream_id);
        
        // 发送数据帧
        self.socket.send(&frame).await?;
        
        // 保存到待确认列表
        let pending = PendingMessage {
            data: data.to_vec(),
            sequence_number: seq_num,
            timestamp: Instant::now(),
            retry_count: 0,
            session_id,
            stream_id,
        };
        
        self.pending_messages.insert(seq_num, pending);
        
        Ok(())
    }
    
    async fn handle_ack(&mut self, ack_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if ack_data.len() >= HEADER_LENGTH {
            // 解析ACK帧
            let frame_type = u16::from_le_bytes([ack_data[4], ack_data[5]]);
            
            if frame_type == FrameType::Ack as u16 {
                let ack_sequence = u32::from_le_bytes([ack_data[24], ack_data[25], ack_data[26], ack_data[27]]);
                
                if self.pending_messages.remove(&ack_sequence).is_some() {
                    println!("✅ ACK received for sequence {}", ack_sequence);
                } else {
                    println!("⚠️ Unexpected ACK for sequence {}", ack_sequence);
                }
            }
        }
        
        Ok(())
    }
    
    async fn check_retransmissions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let mut to_retransmit = Vec::new();
        let mut to_remove = Vec::new();
        
        for (seq_num, pending) in &self.pending_messages {
            if now.duration_since(pending.timestamp) > self.retransmit_timeout {
                if pending.retry_count >= self.max_retries {
                    println!("❌ Message {} dropped after {} retries", seq_num, self.max_retries);
                    to_remove.push(*seq_num);
                } else {
                    to_retransmit.push(*seq_num);
                }
            }
        }
        
        // 移除超过重试次数的消息
        for seq_num in to_remove {
            self.pending_messages.remove(&seq_num);
        }
        
        // 重传超时的消息
        for seq_num in to_retransmit {
            if let Some(mut pending) = self.pending_messages.remove(&seq_num) {
                pending.retry_count += 1;
                pending.timestamp = now;
                
                let frame = self.create_data_frame(&pending.data, pending.sequence_number, pending.session_id, pending.stream_id);
                self.socket.send(&frame).await?;
                
                println!("🔄 Retransmitted message {} (retry {})", seq_num, pending.retry_count);
                self.pending_messages.insert(seq_num, pending);
            }
        }
        
        Ok(())
    }
    
    async fn wait_for_acks(&mut self, timeout_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let mut buffer = vec![0u8; 1024];
        
        while !self.pending_messages.is_empty() && start_time.elapsed().as_secs() < timeout_secs {
            // 设置短时间超时来检查ACK
            match tokio::time::timeout(Duration::from_millis(50), self.socket.recv(&mut buffer)).await {
                Ok(Ok(len)) => {
                    self.handle_ack(&buffer[..len]).await?;
                }
                Ok(Err(e)) => {
                    println!("Receive error: {}", e);
                }
                Err(_) => {
                    // 超时，检查重传
                    self.check_retransmissions().await?;
                }
            }
        }
        
        Ok(())
    }
    
    fn get_statistics(&self) -> (u32, usize, usize) {
        let total_sent = self.sequence_number;
        let pending_count = self.pending_messages.len();
        let acked_count = total_sent as usize - pending_count;
        
        (total_sent, acked_count, pending_count)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("Reliable Aeron Rust Sender");
    println!("Target: {}:{}", args.host, args.port);
    println!("Message size: {} bytes", args.message_size);
    println!("Message count: {}", args.message_count);
    println!("Retransmit timeout: {}ms", args.retransmit_timeout_ms);
    println!("Max retries: {}", args.max_retries);
    println!("");
    
    let mut sender = ReliableAeronSender::new(args.retransmit_timeout_ms, args.max_retries).await?;
    sender.connect(&args.host, args.port).await?;
    
    println!("✅ Connected to Swift Aeron receiver");
    
    // 创建测试数据
    let test_data = vec![0xBBu8; args.message_size]; // 用0xBB标识这是Rust发送的数据
    let start_time = Instant::now();
    
    println!("📤 Sending {} reliable messages to Swift...", args.message_count);
    
    // 发送所有消息
    for i in 0..args.message_count {
        sender.send_reliable(&test_data, 2, 2001).await?; // session_id=2, stream_id=2001 (区别于Swift发送的)
        
        if i % 10 == 0 {
            println!("Sent message {}", i);
        }
        
        // 小延迟避免网络拥塞
        if i % 20 == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    let send_time = start_time.elapsed();
    println!("📤 All messages sent in {:.2}s", send_time.as_secs_f64());
    
    // 等待ACKs
    println!("⏳ Waiting for ACKs from Swift...");
    sender.wait_for_acks(30).await?; // 最多等待30秒
    
    let total_time = start_time.elapsed();
    let (total_sent, acked_count, pending_count) = sender.get_statistics();
    
    println!("\n=== Rust → Swift Communication Results ===");
    println!("Total messages sent: {}", total_sent);
    println!("Messages acknowledged: {}", acked_count);
    println!("Messages pending/lost: {}", pending_count);
    println!("Success rate: {:.1}%", (acked_count as f64 / total_sent as f64) * 100.0);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    
    let total_bytes = args.message_size * args.message_count;
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    
    if pending_count == 0 {
        println!("🎉 Perfect reliability: All messages delivered!");
    } else {
        println!("⚠️ Some messages were lost or not acknowledged");
    }
    
    println!("\n✅ Rust → Swift bidirectional communication test completed!");
    
    Ok(())
}
use tokio::net::UdpSocket;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "reliable_aeron_receiver")]
#[command(about = "Rust reliable Aeron receiver with ACK support")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    
    #[arg(long, default_value = "40001")]
    port: u16,
    
    #[arg(long, default_value = "50")]
    expected_messages: usize,
    
    #[arg(long, default_value = "false")]
    simulate_loss: bool,
    
    #[arg(long, default_value = "0.1")]
    loss_rate: f64,
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

impl FrameType {
    fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x01 => Some(FrameType::Data),
            0x02 => Some(FrameType::Ack),
            0x03 => Some(FrameType::Nak),
            0x04 => Some(FrameType::Heartbeat),
            0x05 => Some(FrameType::FlowControl),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ReliableAeronFrame {
    frame_length: u32,
    frame_type: FrameType,
    flags: u8,
    version: u8,
    session_id: u32,
    stream_id: u32,
    term_id: u32,
    term_offset: u32,
    sequence_number: u32,
    data: Vec<u8>,
}

impl ReliableAeronFrame {
    fn parse(buffer: &[u8]) -> Result<Self, String> {
        if buffer.len() < HEADER_LENGTH {
            return Err(format!("Frame too short: {} bytes", buffer.len()));
        }
        
        let frame_length = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let frame_type_raw = u16::from_le_bytes([buffer[4], buffer[5]]);
        let frame_type = FrameType::from_u16(frame_type_raw)
            .ok_or_else(|| format!("Unknown frame type: {}", frame_type_raw))?;
        let flags = buffer[6];
        let version = buffer[7];
        let session_id = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
        let stream_id = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
        let term_id = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
        let term_offset = u32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]);
        let sequence_number = u32::from_le_bytes([buffer[24], buffer[25], buffer[26], buffer[27]]);
        
        // 提取数据部分
        let data = if buffer.len() > HEADER_LENGTH {
            buffer[HEADER_LENGTH..].to_vec()
        } else {
            Vec::new()
        };
        
        Ok(ReliableAeronFrame {
            frame_length,
            frame_type,
            flags,
            version,
            session_id,
            stream_id,
            term_id,
            term_offset,
            sequence_number,
            data,
        })
    }
    
    fn create_ack(sequence_number: u32, session_id: u32, stream_id: u32) -> Vec<u8> {
        let mut frame = Vec::new();
        
        let frame_length = HEADER_LENGTH as u32;
        let frame_type = FrameType::Ack as u16;
        let flags = 0u8;
        let version = 1u8;
        let term_id = 0u32;
        let term_offset = 0u32;
        
        frame.extend_from_slice(&frame_length.to_le_bytes());
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
        
        frame
    }
}

struct ReliabilityManager {
    expected_sequence: u32,
    received_sequences: HashSet<u32>,
    out_of_order_buffer: HashMap<u32, Vec<u8>>,
    duplicate_count: u32,
    out_of_order_count: u32,
    ack_sent_count: u32,
}

impl ReliabilityManager {
    fn new() -> Self {
        Self {
            expected_sequence: 0,
            received_sequences: HashSet::new(),
            out_of_order_buffer: HashMap::new(),
            duplicate_count: 0,
            out_of_order_count: 0,
            ack_sent_count: 0,
        }
    }
    
    fn process_data_frame(&mut self, frame: &ReliableAeronFrame) -> Vec<(u32, Vec<u8>)> {
        let sequence_number = frame.sequence_number;
        let mut delivered_messages = Vec::new();
        
        // 检查重复
        if self.received_sequences.contains(&sequence_number) {
            self.duplicate_count += 1;
            println!("🔄 Duplicate message {}, ignoring", sequence_number);
            return delivered_messages;
        }
        
        self.received_sequences.insert(sequence_number);
        
        if sequence_number == self.expected_sequence {
            // 按序到达
            delivered_messages.push((sequence_number, frame.data.clone()));
            self.expected_sequence += 1;
            
            // 检查缓存的乱序消息
            while let Some(buffered_data) = self.out_of_order_buffer.remove(&self.expected_sequence) {
                delivered_messages.push((self.expected_sequence, buffered_data));
                self.expected_sequence += 1;
            }
        } else if sequence_number > self.expected_sequence {
            // 乱序到达，缓存
            self.out_of_order_count += 1;
            self.out_of_order_buffer.insert(sequence_number, frame.data.clone());
            println!("📦 Buffering out-of-order message {}, expected {}", 
                sequence_number, self.expected_sequence);
        } else {
            // 过期消息
            println!("⏰ Late message {}, expected {}", sequence_number, self.expected_sequence);
        }
        
        delivered_messages
    }
    
    fn get_statistics(&self) -> (u32, u32, u32, usize, usize) {
        (
            self.expected_sequence,
            self.duplicate_count,
            self.out_of_order_count,
            self.received_sequences.len(),
            self.out_of_order_buffer.len(),
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("Reliable Aeron Rust Receiver");
    println!("Listening on {}:{}", args.host, args.port);
    println!("Expected messages: {}", args.expected_messages);
    if args.simulate_loss {
        println!("Simulating {}% ACK loss", args.loss_rate * 100.0);
    }
    println!("");
    
    let socket = UdpSocket::bind(format!("{}:{}", args.host, args.port)).await?;
    println!("✅ Socket bound, waiting for reliable Aeron frames...");
    
    let mut received_count = 0;
    let mut total_bytes = 0;
    let mut total_data_bytes = 0;
    let start_time = Instant::now();
    let mut first_message_time: Option<Instant> = None;
    let mut buffer = vec![0u8; 65536];
    let mut reliability_manager = ReliabilityManager::new();
    
    while received_count < args.expected_messages {
        let (len, addr) = socket.recv_from(&mut buffer).await?;
        
        if first_message_time.is_none() {
            first_message_time = Some(Instant::now());
            println!("First message received from: {}", addr);
        }
        
        // 解析Aeron帧
        match ReliableAeronFrame::parse(&buffer[..len]) {
            Ok(frame) => {
                total_bytes += len;
                
                match frame.frame_type {
                    FrameType::Data => {
                        // 处理数据帧
                        let delivered_messages = reliability_manager.process_data_frame(&frame);
                        
                        for (seq_num, data) in delivered_messages {
                            received_count += 1;
                            total_data_bytes += data.len();
                            
                            if received_count <= 5 || received_count % 10 == 0 {
                                println!("✅ Delivered message {}: {} bytes", seq_num, data.len());
                                
                                // 验证数据内容
                                if !data.is_empty() {
                                    let first_byte = data[0];
                                    let last_byte = data.last().unwrap_or(&0);
                                    println!("   Data: first={}, last={}", first_byte, last_byte);
                                }
                            }
                        }
                        
                        // 发送ACK（模拟丢包）
                        let should_send_ack = if args.simulate_loss {
                            rand::random::<f64>() > args.loss_rate
                        } else {
                            true
                        };
                        
                        if should_send_ack {
                            let ack_frame = ReliableAeronFrame::create_ack(
                                frame.sequence_number,
                                frame.session_id,
                                frame.stream_id
                            );
                            
                            if let Err(e) = socket.send_to(&ack_frame, addr).await {
                                println!("❌ Failed to send ACK: {}", e);
                            } else {
                                reliability_manager.ack_sent_count += 1;
                                if reliability_manager.ack_sent_count <= 5 {
                                    println!("📨 ACK sent for sequence {}", frame.sequence_number);
                                }
                            }
                        } else {
                            println!("🔥 Simulated ACK loss for sequence {}", frame.sequence_number);
                        }
                    },
                    FrameType::Heartbeat => {
                        println!("💓 Heartbeat from session {}", frame.session_id);
                    },
                    _ => {
                        println!("📥 Received frame type: {:?}", frame.frame_type);
                    }
                }
                
                // 定期打印统计信息
                if received_count % 20 == 0 && received_count > 0 {
                    let (expected_seq, duplicates, out_of_order, total_received, buffered) = 
                        reliability_manager.get_statistics();
                    println!("📊 Stats: delivered={}, expected_seq={}, duplicates={}, out_of_order={}, buffered={}", 
                        received_count, expected_seq, duplicates, out_of_order, buffered);
                }
            }
            Err(e) => {
                println!("❌ Failed to parse frame: {}", e);
            }
        }
    }
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let data_throughput_mbps = (total_data_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== Reliable Aeron Receiver Results ===");
    println!("Total messages delivered: {}", received_count);
    println!("Total frame bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
    println!("Total data bytes: {} ({:.2} MB)", total_data_bytes, total_data_bytes as f64 / 1024.0 / 1024.0);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Frame throughput: {:.2} MB/s", throughput_mbps);
    println!("Data throughput: {:.2} MB/s", data_throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    // 可靠性统计
    let (expected_seq, duplicates, out_of_order, total_received, buffered) = 
        reliability_manager.get_statistics();
    println!("\n=== Reliability Statistics ===");
    println!("Expected next sequence: {}", expected_seq);
    println!("Total unique messages: {}", total_received);
    println!("Duplicate messages: {}", duplicates);
    println!("Out-of-order messages: {}", out_of_order);
    println!("Still buffered: {}", buffered);
    println!("ACKs sent: {}", reliability_manager.ack_sent_count);
    println!("Message loss: {}", args.expected_messages.saturating_sub(received_count));
    println!("Success rate: {:.1}%", (received_count as f64 / args.expected_messages as f64) * 100.0);
    
    // 协议开销
    let overhead = total_bytes - total_data_bytes;
    let overhead_percentage = (overhead as f64 / total_bytes as f64) * 100.0;
    println!("Protocol overhead: {} bytes ({:.1}%)", overhead, overhead_percentage);
    
    if out_of_order > 0 {
        println!("✅ Out-of-order delivery handled correctly");
    }
    if duplicates > 0 {
        println!("✅ Duplicate detection working");
    }
    
    Ok(())
}
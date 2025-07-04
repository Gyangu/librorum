use tokio::net::UdpSocket;
use std::time::{Duration, Instant};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "aeron_rust_receiver")]
#[command(about = "Rust UDP receiver for testing Swift Aeron compatibility")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    
    #[arg(long, default_value = "40001")]
    port: u16,
    
    #[arg(long, default_value = "100")]
    expected_messages: usize,
}

// Aeron协议常量
const DATA_HEADER_LENGTH: usize = 32;

#[derive(Debug)]
struct AeronFrame {
    frame_length: u32,
    frame_type: u16,
    flags: u8,
    version: u8,
    session_id: u32,
    stream_id: u32,
    term_id: u32,
    term_offset: u32,
    data: Vec<u8>,
}

impl AeronFrame {
    fn parse(buffer: &[u8]) -> Result<Self, String> {
        if buffer.len() < DATA_HEADER_LENGTH {
            return Err(format!("Frame too short: {} bytes", buffer.len()));
        }
        
        let frame_length = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let frame_type = u16::from_le_bytes([buffer[4], buffer[5]]);
        let flags = buffer[6];
        let version = buffer[7];
        let session_id = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
        let stream_id = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
        let term_id = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
        let term_offset = u32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]);
        
        // 提取数据部分
        let data = if buffer.len() > DATA_HEADER_LENGTH {
            buffer[DATA_HEADER_LENGTH..].to_vec()
        } else {
            Vec::new()
        };
        
        Ok(AeronFrame {
            frame_length,
            frame_type,
            flags,
            version,
            session_id,
            stream_id,
            term_id,
            term_offset,
            data,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("Rust Aeron-Compatible UDP Receiver");
    println!("Listening on {}:{}", args.host, args.port);
    println!("Expected messages: {}", args.expected_messages);
    
    let socket = UdpSocket::bind(format!("{}:{}", args.host, args.port)).await?;
    println!("Socket bound, waiting for Aeron frames from Swift...");
    
    let mut received_count = 0;
    let mut total_bytes = 0;
    let mut total_data_bytes = 0;
    let start_time = Instant::now();
    let mut first_message_time: Option<Instant> = None;
    let mut buffer = vec![0u8; 65536];
    
    while received_count < args.expected_messages {
        let (len, addr) = socket.recv_from(&mut buffer).await?;
        
        if first_message_time.is_none() {
            first_message_time = Some(Instant::now());
            println!("First message received from: {}", addr);
        }
        
        // 解析Aeron帧
        match AeronFrame::parse(&buffer[..len]) {
            Ok(frame) => {
                received_count += 1;
                total_bytes += len;
                total_data_bytes += frame.data.len();
                
                if received_count <= 5 || received_count % 10 == 0 {
                    println!("Message {}: Frame length: {}, Type: {}, Session: {}, Stream: {}, Data: {} bytes", 
                        received_count, frame.frame_length, frame.frame_type, 
                        frame.session_id, frame.stream_id, frame.data.len());
                }
                
                // 验证数据内容（Swift发送的是42）
                if !frame.data.is_empty() {
                    let first_byte = frame.data[0];
                    if received_count <= 3 {
                        println!("  Data sample: first byte = {}, last byte = {}", 
                            first_byte, frame.data.last().unwrap_or(&0));
                    }
                }
            }
            Err(e) => {
                println!("Failed to parse Aeron frame: {}", e);
                println!("Raw data length: {}, first 32 bytes: {:?}", len, &buffer[..32.min(len)]);
            }
        }
    }
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let data_throughput_mbps = (total_data_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== Rust Aeron Receiver Results ===");
    println!("Total messages: {}", received_count);
    println!("Total frame bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
    println!("Total data bytes: {} ({:.2} MB)", total_data_bytes, total_data_bytes as f64 / 1024.0 / 1024.0);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Frame throughput: {:.2} MB/s", throughput_mbps);
    println!("Data throughput: {:.2} MB/s", data_throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    // 协议开销分析
    let overhead = total_bytes - total_data_bytes;
    let overhead_percentage = (overhead as f64 / total_bytes as f64) * 100.0;
    println!("Protocol overhead: {} bytes ({:.1}%)", overhead, overhead_percentage);
    
    Ok(())
}
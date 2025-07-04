use std::os::unix::net::UnixStream;
use std::io::Write;
use std::time::Instant;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rust_ipc_sender")]
#[command(about = "Rust IPC Aeron sender using Unix Domain Socket")]
struct Args {
    #[arg(long, default_value = "/tmp/aeron_ipc.sock")]
    socket_path: String,
    
    #[arg(long, default_value = "1001")]
    stream_id: u32,
    
    #[arg(long, default_value = "1")]
    session_id: u32,
    
    #[arg(long, default_value = "1024")]
    message_size: usize,
    
    #[arg(long, default_value = "10000")]
    message_count: usize,
}

fn create_setup_frame(stream_id: u32, session_id: u32, term_id: u32) -> Vec<u8> {
    let mut frame = vec![0u8; 40];
    
    // Frame header
    frame[0..4].copy_from_slice(&40u32.to_le_bytes());           // length
    frame[4] = 0x01;                                             // version
    frame[5] = 0x00;                                             // flags
    frame[6..8].copy_from_slice(&0x05u16.to_le_bytes());         // setup type
    frame[8..12].copy_from_slice(&0u32.to_le_bytes());           // term offset
    frame[12..16].copy_from_slice(&session_id.to_le_bytes());    // session ID
    frame[16..20].copy_from_slice(&stream_id.to_le_bytes());     // stream ID
    frame[20..24].copy_from_slice(&term_id.to_le_bytes());       // term ID
    
    frame
}

fn create_data_frame(payload: &[u8], stream_id: u32, session_id: u32, term_id: u32, term_offset: u32) -> Vec<u8> {
    let frame_length = 32 + payload.len();
    let mut frame = vec![0u8; frame_length];
    
    // Frame header
    frame[0..4].copy_from_slice(&(frame_length as u32).to_le_bytes()); // length
    frame[4] = 0x01;                                                    // version
    frame[5] = 0x00;                                                    // flags
    frame[6..8].copy_from_slice(&0x01u16.to_le_bytes());                // data type
    frame[8..12].copy_from_slice(&term_offset.to_le_bytes());           // term offset
    frame[12..16].copy_from_slice(&session_id.to_le_bytes());           // session ID
    frame[16..20].copy_from_slice(&stream_id.to_le_bytes());            // stream ID
    frame[20..24].copy_from_slice(&term_id.to_le_bytes());              // term ID
    
    // Payload
    frame[32..].copy_from_slice(payload);
    
    frame
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🚀 Rust IPC Aeron发送器");
    println!("Socket路径: {}", args.socket_path);
    println!("流ID: {}, 会话ID: {}", args.stream_id, args.session_id);
    println!("消息大小: {} bytes, 数量: {}", args.message_size, args.message_count);
    
    // 等待服务器启动
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    println!("🔗 连接到Rust IPC服务器...");
    let mut stream = UnixStream::connect(&args.socket_path)?;
    println!("✅ 连接成功");
    
    let term_id = rand::random::<u32>();
    
    // 发送Setup帧
    let setup_frame = create_setup_frame(args.stream_id, args.session_id, term_id);
    stream.write_all(&setup_frame)?;
    println!("📤 发送Setup帧 ({} bytes)", setup_frame.len());
    
    // 预热
    println!("🔥 开始预热...");
    let warmup_data = vec![0x41u8; 64];
    for _ in 0..100 {
        let warmup_frame = create_data_frame(&warmup_data, args.stream_id, args.session_id, term_id, 0);
        stream.write_all(&warmup_frame)?;
    }
    
    // 短暂延迟
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // 主要性能测试
    println!("📊 开始Rust IPC性能测试...");
    let test_data = vec![0x42u8; args.message_size];
    let start_time = Instant::now();
    let mut term_offset = 0u32;
    let mut sent_messages = 0;
    let mut total_bytes = 0;
    
    for i in 0..args.message_count {
        let data_frame = create_data_frame(&test_data, args.stream_id, args.session_id, term_id, term_offset);
        
        match stream.write_all(&data_frame) {
            Ok(()) => {
                sent_messages += 1;
                total_bytes += test_data.len();
                term_offset += data_frame.len() as u32;
            }
            Err(e) => {
                println!("❌ 发送失败 at {}: {}", i, e);
                break;
            }
        }
        
        if i % (args.message_count / 10) == 0 {
            println!("发送进度: {}/{}", i, args.message_count);
        }
        
        // 小批量延迟
        if i % 1000 == 0 && i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    
    // 确保所有数据发送完成
    stream.flush()?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = sent_messages as f64 / total_time.as_secs_f64();
    
    println!("\n=== Rust IPC Aeron发送结果 ===");
    println!("发送消息: {}", sent_messages);
    println!("总字节数: {:.2} MB", total_bytes as f64 / 1024.0 / 1024.0);
    println!("持续时间: {:.3}s", total_time.as_secs_f64());
    println!("吞吐量: {:.2} MB/s", throughput_mbps);
    println!("消息速率: {:.0} 消息/秒", messages_per_sec);
    
    // 与网络Aeron对比
    let network_baseline = 8.95; // 网络Aeron基准
    let improvement = throughput_mbps / network_baseline;
    println!("相对网络Aeron性能: {:.1}倍", improvement);
    
    println!("🎉 Rust IPC发送完成!");
    
    Ok(())
}
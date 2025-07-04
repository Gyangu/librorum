use std::net::UdpSocket;
use std::time::{Duration, Instant};
use clap::Parser;
use std::thread;

#[derive(Parser)]
#[command(name = "simple_udp_aeron_sender")]
#[command(about = "Simple UDP sender to test Aeron protocol performance")]
struct Args {
    /// Target host
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Target port
    #[arg(short, long, default_value = "40401")]
    port: u16,

    /// Stream ID
    #[arg(short, long, default_value = "1001")]
    stream_id: u32,

    /// Session ID
    #[arg(short = 'S', long, default_value = "1")]
    session_id: u32,

    /// Message size in bytes
    #[arg(short, long, default_value = "1024")]
    message_size: usize,

    /// Message count
    #[arg(short, long, default_value = "10000")]
    count: i32,

    /// Warmup messages
    #[arg(short, long, default_value = "1000")]
    warmup: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🚀 启动Rust UDP Aeron发送器");
    println!("目标: {}:{}", args.host, args.port);
    println!("流ID: {}, 会话ID: {}", args.stream_id, args.session_id);
    println!("消息大小: {} bytes, 数量: {}", args.message_size, args.count);
    println!("预热消息: {}", args.warmup);
    
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let target_addr = format!("{}:{}", args.host, args.port);
    
    println!("✅ 连接到 {}", target_addr);
    
    // 等待接收器启动
    thread::sleep(Duration::from_millis(100));
    
    // 发送Setup帧
    let initial_term_id = rand::random::<u32>();
    send_setup_frame(&socket, &target_addr, args.stream_id, args.session_id, initial_term_id)?;
    println!("📋 发送Setup帧: 流{}, 会话{}, 初始术语{}", args.stream_id, args.session_id, initial_term_id);
    
    // 等待状态消息
    thread::sleep(Duration::from_millis(10));
    
    // 创建测试数据
    let payload_size = if args.message_size > 32 { args.message_size - 32 } else { args.message_size };
    let test_data = vec![0x42u8; payload_size];
    
    // 预热阶段
    if args.warmup > 0 {
        println!("🔥 开始预热 ({} 消息)...", args.warmup);
        let mut term_offset = 0u32;
        
        for i in 0..args.warmup {
            let frame = create_data_frame(
                &test_data,
                args.stream_id,
                args.session_id,
                initial_term_id,
                term_offset
            );
            
            socket.send_to(&frame, &target_addr)?;
            term_offset += align_to_32(frame.len() as u32);
            
            if i % 1000 == 0 {
                thread::sleep(Duration::from_millis(1));
            }
        }
        
        thread::sleep(Duration::from_millis(100));
        println!("✅ 预热完成");
    }
    
    // 性能测试 - 多种消息大小
    let test_sizes = vec![64, 256, 1024, 4096];
    
    for &size in &test_sizes {
        if size <= args.message_size {
            run_performance_test(&socket, &target_addr, &args, size, initial_term_id)?;
        }
    }
    
    // 主要性能测试
    println!("\n🎯 开始主要性能测试...");
    run_performance_test(&socket, &target_addr, &args, args.message_size, initial_term_id)?;
    
    Ok(())
}

fn run_performance_test(
    socket: &UdpSocket,
    target_addr: &str,
    args: &Args,
    message_size: usize,
    initial_term_id: u32
) -> Result<(), Box<dyn std::error::Error>> {
    
    let payload_size = if message_size > 32 { message_size - 32 } else { message_size };
    let test_data = vec![0x42u8; payload_size];
    let mut term_offset = 0u32;
    
    println!("\n--- 消息大小: {} bytes ---", message_size);
    
    let start_time = Instant::now();
    let mut bytes_sent = 0;
    
    for i in 0..args.count {
        let frame = create_data_frame(
            &test_data,
            args.stream_id,
            args.session_id,
            initial_term_id,
            term_offset
        );
        
        socket.send_to(&frame, target_addr)?;
        bytes_sent += frame.len();
        term_offset += align_to_32(frame.len() as u32);
        
        if i % 1000 == 0 && i > 0 {
            print!("已发送: {}/{} 消息\r", i + 1, args.count);
            std::io::Write::flush(&mut std::io::stdout())?;
        }
        
        // 适度的发送间隔，避免缓冲区溢出
        if i % 100 == 0 {
            thread::sleep(Duration::from_nanos(100));
        }
    }
    
    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f64();
    let throughput_mbps = (bytes_sent as f64) / 1024.0 / 1024.0 / duration_secs;
    let msg_rate = args.count as f64 / duration_secs;
    let avg_latency_ms = duration_secs * 1000.0 / args.count as f64;
    
    println!("  持续时间: {:.3}s", duration_secs);
    println!("  吞吐量: {:.2} MB/s", throughput_mbps);
    println!("  消息速率: {:.0} 消息/秒", msg_rate);
    println!("  平均延迟: {:.3} ms/消息", avg_latency_ms);
    println!("  总字节数: {} bytes", bytes_sent);
    
    Ok(())
}

fn send_setup_frame(
    socket: &UdpSocket,
    target_addr: &str,
    stream_id: u32,
    session_id: u32,
    initial_term_id: u32
) -> Result<(), Box<dyn std::error::Error>> {
    let mut setup_frame = Vec::with_capacity(40);
    
    // Frame Length (4 bytes)
    setup_frame.extend_from_slice(&40u32.to_le_bytes());
    
    // Version (1 byte)
    setup_frame.push(0x01);
    
    // Flags (1 byte)
    setup_frame.push(0x00);
    
    // Type (2 bytes) - Setup Frame
    setup_frame.extend_from_slice(&0x05u16.to_le_bytes());
    
    // Term Offset (4 bytes)
    setup_frame.extend_from_slice(&0u32.to_le_bytes());
    
    // Session ID (4 bytes)
    setup_frame.extend_from_slice(&session_id.to_le_bytes());
    
    // Stream ID (4 bytes)
    setup_frame.extend_from_slice(&stream_id.to_le_bytes());
    
    // Initial Term ID (4 bytes)
    setup_frame.extend_from_slice(&initial_term_id.to_le_bytes());
    
    // Active Term ID (4 bytes)
    setup_frame.extend_from_slice(&initial_term_id.to_le_bytes());
    
    // Term Length (4 bytes) - 16MB默认
    setup_frame.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
    
    // MTU Length (4 bytes)
    setup_frame.extend_from_slice(&1408u32.to_le_bytes());
    
    // TTL (4 bytes)
    setup_frame.extend_from_slice(&0u32.to_le_bytes());
    
    socket.send_to(&setup_frame, target_addr)?;
    Ok(())
}

fn create_data_frame(
    data: &[u8],
    stream_id: u32,
    session_id: u32,
    term_id: u32,
    term_offset: u32
) -> Vec<u8> {
    let frame_length = 32 + data.len(); // 32字节头部 + 数据
    let aligned_length = align_to_32(frame_length as u32) as usize;
    
    let mut frame = Vec::with_capacity(aligned_length);
    
    // Frame Length (4 bytes)
    frame.extend_from_slice(&(frame_length as u32).to_le_bytes());
    
    // Version (1 byte)
    frame.push(0x01);
    
    // Flags (1 byte) - BEGIN_FLAG | END_FLAG
    frame.push(0xC0);
    
    // Type (2 bytes) - Data Frame
    frame.extend_from_slice(&0x01u16.to_le_bytes());
    
    // Term Offset (4 bytes)
    frame.extend_from_slice(&term_offset.to_le_bytes());
    
    // Session ID (4 bytes)
    frame.extend_from_slice(&session_id.to_le_bytes());
    
    // Stream ID (4 bytes)
    frame.extend_from_slice(&stream_id.to_le_bytes());
    
    // Term ID (4 bytes)
    frame.extend_from_slice(&term_id.to_le_bytes());
    
    // Reserved Value (8 bytes)
    frame.extend_from_slice(&0u64.to_le_bytes());
    
    // Data payload
    frame.extend_from_slice(data);
    
    // Padding to 32-byte alignment
    while frame.len() < aligned_length {
        frame.push(0);
    }
    
    frame
}

fn align_to_32(length: u32) -> u32 {
    (length + 31) & !31
}
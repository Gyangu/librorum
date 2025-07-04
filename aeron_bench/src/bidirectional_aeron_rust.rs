use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::thread;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use clap::Parser;

#[derive(Parser)]
#[command(name = "bidirectional_aeron_rust")]
#[command(about = "Rust双向Aeron性能基准测试")]
struct Args {
    /// 发布主机
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    publish_host: String,

    /// 发布端口
    #[arg(short = 'p', long, default_value = "40001")]
    publish_port: u16,

    /// 订阅端口
    #[arg(short = 's', long, default_value = "40002")]
    subscribe_port: u16,

    /// 流ID
    #[arg(long, default_value = "1001")]
    stream_id: u32,

    /// 会话ID
    #[arg(long, default_value = "1")]
    session_id: u32,

    /// 消息大小
    #[arg(short, long, default_value = "1024")]
    message_size: usize,

    /// 消息数量
    #[arg(short, long, default_value = "5000")]
    count: i32,

    /// 测试模式 (concurrent 或 echo)
    #[arg(short = 'M', long, default_value = "concurrent")]
    mode: String,
}

struct BidirectionalStats {
    sent_messages: u64,
    received_messages: u64,
    sent_bytes: u64,
    received_bytes: u64,
    duration: f64,
}

impl BidirectionalStats {
    fn print_stats(&self) {
        println!("\n{}", "=".repeat(60));
        println!("🔄 Rust双向Aeron性能统计");
        println!("{}", "=".repeat(60));
        
        println!("📤 发送性能:");
        println!("  消息数: {}", self.sent_messages);
        println!("  字节数: {:.2} MB", self.sent_bytes as f64 / 1024.0 / 1024.0);
        if self.duration > 0.0 {
            let send_throughput = (self.sent_bytes as f64) / 1024.0 / 1024.0 / self.duration;
            let send_rate = self.sent_messages as f64 / self.duration;
            println!("  吞吐量: {:.2} MB/s", send_throughput);
            println!("  消息速率: {:.0} 消息/秒", send_rate);
        }
        
        println!("\n📥 接收性能:");
        println!("  消息数: {}", self.received_messages);
        println!("  字节数: {:.2} MB", self.received_bytes as f64 / 1024.0 / 1024.0);
        if self.duration > 0.0 {
            let recv_throughput = (self.received_bytes as f64) / 1024.0 / 1024.0 / self.duration;
            let recv_rate = self.received_messages as f64 / self.duration;
            println!("  吞吐量: {:.2} MB/s", recv_throughput);
            println!("  消息速率: {:.0} 消息/秒", recv_rate);
        }
        
        println!("\n🔄 双向性能:");
        if self.duration > 0.0 {
            let total_throughput = (self.sent_bytes + self.received_bytes) as f64 / 1024.0 / 1024.0 / self.duration;
            println!("  总吞吐量: {:.2} MB/s", total_throughput);
        }
        let integrity = if self.sent_messages > 0 {
            (self.received_messages as f64 / self.sent_messages as f64) * 100.0
        } else { 0.0 };
        println!("  数据完整性: {:.1}%", integrity);
        println!("  测试时长: {:.3}s", self.duration);
        
        println!("{}", "=".repeat(60));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🔄 Rust双向Aeron性能测试");
    println!("发布目标: {}:{}", args.publish_host, args.publish_port);
    println!("订阅端口: {}", args.subscribe_port);
    println!("流ID: {}, 会话ID: {}", args.stream_id, args.session_id);
    println!("消息大小: {} bytes, 数量: {}", args.message_size, args.count);
    println!("测试模式: {}\n", args.mode);
    
    match args.mode.as_str() {
        "concurrent" => run_concurrent_test(&args),
        "echo" => run_echo_test(&args),
        _ => {
            println!("❌ 未知测试模式: {}", args.mode);
            Ok(())
        }
    }
}

fn run_concurrent_test(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 开始并发双向测试...");
    
    // 共享统计
    let sent_messages = Arc::new(AtomicU64::new(0));
    let received_messages = Arc::new(AtomicU64::new(0));
    let sent_bytes = Arc::new(AtomicU64::new(0));
    let received_bytes = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));
    
    let start_time = Instant::now();
    
    // 接收器线程
    let recv_socket = UdpSocket::bind(format!("0.0.0.0:{}", args.subscribe_port))?;
    recv_socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    
    let received_messages_clone = received_messages.clone();
    let received_bytes_clone = received_bytes.clone();
    let running_clone = running.clone();
    
    let receiver_handle = thread::spawn(move || {
        let mut buffer = vec![0u8; 65536];
        
        while running_clone.load(Ordering::Relaxed) {
            match recv_socket.recv_from(&mut buffer) {
                Ok((size, _)) => {
                    // 解析Aeron帧
                    if size >= 32 {
                        let frame_type = u16::from_le_bytes([buffer[6], buffer[7]]);
                        if frame_type == 0x01 { // 数据帧
                            received_messages_clone.fetch_add(1, Ordering::Relaxed);
                            received_bytes_clone.fetch_add((size - 32) as u64, Ordering::Relaxed);
                        }
                    }
                }
                Err(_) => {
                    // 超时，继续循环
                }
            }
        }
    });
    
    // 发送器
    let send_socket = UdpSocket::bind("0.0.0.0:0")?;
    let target_addr = format!("{}:{}", args.publish_host, args.publish_port);
    
    // 发送Setup帧
    let initial_term_id = rand::random::<u32>();
    send_setup_frame(&send_socket, &target_addr, args.stream_id, args.session_id, initial_term_id)?;
    println!("📋 发送Setup帧");
    
    thread::sleep(Duration::from_millis(100));
    
    // 创建测试数据
    let payload_size = if args.message_size > 32 { args.message_size - 32 } else { 0 };
    let test_data = vec![0x42u8; payload_size];
    
    // 发送数据
    let mut term_offset = 0u32;
    for i in 0..args.count {
        let frame = create_data_frame(
            &test_data,
            args.stream_id,
            args.session_id,
            initial_term_id,
            term_offset
        );
        
        send_socket.send_to(&frame, &target_addr)?;
        sent_messages.fetch_add(1, Ordering::Relaxed);
        sent_bytes.fetch_add(test_data.len() as u64, Ordering::Relaxed);
        
        term_offset += align_to_32(frame.len() as u32);
        
        if i % 1000 == 0 {
            println!("发送进度: {}/{}", i + 1, args.count);
        }
        
        // 适度延迟
        if i % 100 == 0 {
            thread::sleep(Duration::from_micros(100));
        }
    }
    
    // 等待接收完成
    thread::sleep(Duration::from_secs(2));
    running.store(false, Ordering::Relaxed);
    receiver_handle.join().unwrap();
    
    let duration = start_time.elapsed().as_secs_f64();
    
    let stats = BidirectionalStats {
        sent_messages: sent_messages.load(Ordering::Relaxed),
        received_messages: received_messages.load(Ordering::Relaxed),
        sent_bytes: sent_bytes.load(Ordering::Relaxed),
        received_bytes: received_bytes.load(Ordering::Relaxed),
        duration,
    };
    
    stats.print_stats();
    
    println!("\n🎯 Rust双向性能分析:");
    if stats.duration > 0.0 {
        let total_throughput = (stats.sent_bytes + stats.received_bytes) as f64 / 1024.0 / 1024.0 / stats.duration;
        if total_throughput > 20.0 {
            println!("✅ Rust双向性能优秀！");
        } else if total_throughput > 10.0 {
            println!("🔄 Rust双向性能良好");
        } else {
            println!("❌ Rust双向性能需要优化");
        }
    }
    
    Ok(())
}

fn run_echo_test(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 开始回声双向测试...");
    
    let sent_messages = Arc::new(AtomicU64::new(0));
    let received_messages = Arc::new(AtomicU64::new(0));
    let sent_bytes = Arc::new(AtomicU64::new(0));
    let received_bytes = Arc::new(AtomicU64::new(0));
    
    let start_time = Instant::now();
    
    // 创建发送和接收socket
    let send_socket = UdpSocket::bind("0.0.0.0:0")?;
    let recv_socket = UdpSocket::bind(format!("0.0.0.0:{}", args.subscribe_port))?;
    recv_socket.set_read_timeout(Some(Duration::from_millis(10)))?;
    
    let target_addr = format!("{}:{}", args.publish_host, args.publish_port);
    
    // 发送Setup帧
    let initial_term_id = rand::random::<u32>();
    send_setup_frame(&send_socket, &target_addr, args.stream_id, args.session_id, initial_term_id)?;
    println!("📋 发送Setup帧");
    
    thread::sleep(Duration::from_millis(100));
    
    // 回声测试
    let payload_size = if args.message_size > 32 { args.message_size - 32 } else { 0 };
    let test_data = vec![0x42u8; payload_size];
    let mut term_offset = 0u32;
    let mut buffer = vec![0u8; 65536];
    
    for i in 0..args.count {
        // 发送消息
        let frame = create_data_frame(
            &test_data,
            args.stream_id,
            args.session_id,
            initial_term_id,
            term_offset
        );
        
        send_socket.send_to(&frame, &target_addr)?;
        sent_messages.fetch_add(1, Ordering::Relaxed);
        sent_bytes.fetch_add(test_data.len() as u64, Ordering::Relaxed);
        
        term_offset += align_to_32(frame.len() as u32);
        
        // 尝试接收回声
        let timeout = Instant::now() + Duration::from_millis(1);
        while Instant::now() < timeout {
            if let Ok((size, _)) = recv_socket.recv_from(&mut buffer) {
                if size >= 32 {
                    let frame_type = u16::from_le_bytes([buffer[6], buffer[7]]);
                    if frame_type == 0x01 { // 数据帧
                        received_messages.fetch_add(1, Ordering::Relaxed);
                        received_bytes.fetch_add((size - 32) as u64, Ordering::Relaxed);
                        break;
                    }
                }
            }
        }
        
        if i % 1000 == 0 {
            println!("回声进度: {}/{}", i + 1, args.count);
        }
    }
    
    let duration = start_time.elapsed().as_secs_f64();
    
    let stats = BidirectionalStats {
        sent_messages: sent_messages.load(Ordering::Relaxed),
        received_messages: received_messages.load(Ordering::Relaxed),
        sent_bytes: sent_bytes.load(Ordering::Relaxed),
        received_bytes: received_bytes.load(Ordering::Relaxed),
        duration,
    };
    
    stats.print_stats();
    
    let avg_latency = if stats.received_messages > 0 {
        (duration * 1000.0) / stats.received_messages as f64
    } else { 0.0 };
    println!("\n🎯 回声延迟: {:.3} ms", avg_latency);
    
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
    
    setup_frame.extend_from_slice(&40u32.to_le_bytes());
    setup_frame.push(0x01); // version
    setup_frame.push(0x00); // flags
    setup_frame.extend_from_slice(&0x05u16.to_le_bytes()); // setup type
    setup_frame.extend_from_slice(&0u32.to_le_bytes()); // term offset
    setup_frame.extend_from_slice(&session_id.to_le_bytes());
    setup_frame.extend_from_slice(&stream_id.to_le_bytes());
    setup_frame.extend_from_slice(&initial_term_id.to_le_bytes());
    setup_frame.extend_from_slice(&initial_term_id.to_le_bytes());
    setup_frame.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
    setup_frame.extend_from_slice(&1408u32.to_le_bytes());
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
    let frame_length = 32 + data.len();
    let aligned_length = align_to_32(frame_length as u32) as usize;
    
    let mut frame = Vec::with_capacity(aligned_length);
    
    frame.extend_from_slice(&(frame_length as u32).to_le_bytes());
    frame.push(0x01); // version
    frame.push(0xC0); // flags
    frame.extend_from_slice(&0x01u16.to_le_bytes()); // data type
    frame.extend_from_slice(&term_offset.to_le_bytes());
    frame.extend_from_slice(&session_id.to_le_bytes());
    frame.extend_from_slice(&stream_id.to_le_bytes());
    frame.extend_from_slice(&term_id.to_le_bytes());
    frame.extend_from_slice(&0u64.to_le_bytes()); // reserved
    frame.extend_from_slice(data);
    
    while frame.len() < aligned_length {
        frame.push(0);
    }
    
    frame
}

fn align_to_32(length: u32) -> u32 {
    (length + 31) & !31
}
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use clap::Parser;

#[derive(Parser)]
#[command(name = "simple_udp_aeron_receiver")]
#[command(about = "Simple UDP receiver to test Aeron protocol compatibility")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "40401")]
    port: u16,

    /// Expected message count
    #[arg(short, long, default_value = "1000")]
    count: i32,

    /// Timeout in seconds
    #[arg(short, long, default_value = "60")]
    timeout: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🎯 启动简单UDP Aeron接收器");
    println!("端口: {}", args.port);
    println!("期望消息数: {}", args.count);
    println!("超时: {}秒", args.timeout);
    
    let socket = UdpSocket::bind(format!("127.0.0.1:{}", args.port))?;
    socket.set_read_timeout(Some(Duration::from_secs(args.timeout)))?;
    
    println!("✅ 开始监听 127.0.0.1:{}", args.port);
    
    let mut buffer = [0; 65536];
    let mut message_count = 0;
    let mut total_bytes = 0;
    let mut setup_received = false;
    let start_time = Instant::now();
    let mut first_data_time: Option<Instant> = None;
    
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, addr)) => {
                total_bytes += size;
                
                if size >= 8 {
                    let frame_length = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                    let frame_type = u16::from_le_bytes([buffer[6], buffer[7]]);
                    
                    match frame_type {
                        0x05 => { // Setup Frame
                            setup_received = true;
                            let session_id = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
                            let stream_id = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
                            println!("📋 收到Setup帧: 流{}, 会话{}, 长度{}, 来自{}", 
                                stream_id, session_id, frame_length, addr);
                            
                            // 发送状态消息响应
                            send_status_message(&socket, addr, session_id, stream_id)?;
                        },
                        0x01 => { // Data Frame
                            if first_data_time.is_none() {
                                first_data_time = Some(Instant::now());
                            }
                            
                            message_count += 1;
                            let session_id = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
                            let stream_id = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
                            let term_id = u32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]);
                            let term_offset = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
                            
                            if message_count % 100 == 0 || message_count <= 10 {
                                println!("📊 数据帧 #{}: 流{}, 会话{}, 术语{}, 偏移{}, 长度{}", 
                                    message_count, stream_id, session_id, term_id, term_offset, frame_length);
                            }
                            
                            // 定期发送状态消息
                            if message_count % 50 == 0 {
                                send_status_message(&socket, addr, session_id, stream_id)?;
                            }
                        },
                        _ => {
                            println!("🔍 未知帧类型: 0x{:02x}, 长度: {}", frame_type, frame_length);
                        }
                    }
                }
                
                if message_count >= args.count {
                    break;
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut {
                    println!("⏰ 接收超时");
                    break;
                } else {
                    return Err(e.into());
                }
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    
    println!("\n=== 接收结果 ===");
    println!("Setup帧: {}", if setup_received { "✅ 已接收" } else { "❌ 未接收" });
    println!("数据消息: {}/{}", message_count, args.count);
    println!("总字节数: {}", total_bytes);
    println!("总持续时间: {:.2}秒", total_duration.as_secs_f64());
    
    if let Some(first_time) = first_data_time {
        let data_duration = first_time.elapsed();
        if data_duration.as_secs_f64() > 0.0 {
            let throughput_mbps = (total_bytes as f64) / 1024.0 / 1024.0 / data_duration.as_secs_f64();
            let msg_rate = message_count as f64 / data_duration.as_secs_f64();
            
            println!("数据持续时间: {:.2}秒", data_duration.as_secs_f64());
            println!("吞吐量: {:.2} MB/s", throughput_mbps);
            println!("消息速率: {:.0} 消息/秒", msg_rate);
        }
    }
    
    let success = setup_received && message_count > 0;
    println!("协议兼容性: {}", if success { "✅ 成功" } else { "❌ 失败" });
    
    if success {
        println!("\n🎉 Swift Aeron协议兼容性验证成功!");
        std::process::exit(0);
    } else {
        println!("\n❌ 协议兼容性测试失败");
        std::process::exit(1);
    }
}

fn send_status_message(socket: &UdpSocket, addr: std::net::SocketAddr, session_id: u32, stream_id: u32) -> Result<(), Box<dyn std::error::Error>> {
    // 构建状态消息 (28字节)
    let mut status_msg = Vec::with_capacity(28);
    
    // Frame Length (4 bytes)
    status_msg.extend_from_slice(&28u32.to_le_bytes());
    
    // Version (1 byte)
    status_msg.push(0x01);
    
    // Flags (1 byte)
    status_msg.push(0x00);
    
    // Type (2 bytes) - Status Message
    status_msg.extend_from_slice(&0x03u16.to_le_bytes());
    
    // Session ID (4 bytes)
    status_msg.extend_from_slice(&session_id.to_le_bytes());
    
    // Stream ID (4 bytes)
    status_msg.extend_from_slice(&stream_id.to_le_bytes());
    
    // Consumption Term ID (4 bytes)
    status_msg.extend_from_slice(&1u32.to_le_bytes());
    
    // Consumption Term Offset (4 bytes)
    status_msg.extend_from_slice(&0u32.to_le_bytes());
    
    // Receiver Window Length (4 bytes) - 16MB窗口
    status_msg.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
    
    socket.send_to(&status_msg, addr)?;
    println!("📤 发送状态消息到 {}: 会话{}, 流{}", addr, session_id, stream_id);
    
    Ok(())
}
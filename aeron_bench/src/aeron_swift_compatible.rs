use aeron_rs::{
    concurrent::strategies::BusySpinIdleStrategy,
    context::Context,
    subscription::Subscription,
    publication::Publication,
    fragment_assembler::FragmentAssembler,
    concurrent::AtomicBuffer,
};
use clap::Parser;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "aeron_swift_compatible")]
#[command(about = "Aeron-rs compatible test for Swift Aeron implementation")]
struct Args {
    #[arg(long, default_value = "publisher")]
    mode: String,
    
    #[arg(long, default_value = "aeron:udp?endpoint=127.0.0.1:40001")]
    channel: String,
    
    #[arg(long, default_value = "1001")]
    stream_id: i32,
    
    #[arg(long, default_value = "1024")]
    message_size: usize,
    
    #[arg(long, default_value = "10000")]
    message_count: usize,
    
    #[arg(long, default_value = "60")]
    timeout_seconds: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🔧 Aeron-rs Swift兼容性测试");
    println!("模式: {}", args.mode);
    println!("通道: {}", args.channel);
    println!("流ID: {}", args.stream_id);
    println!("消息大小: {} bytes", args.message_size);
    println!("消息数量: {}", args.message_count);
    println!("");
    
    match args.mode.as_str() {
        "publisher" => run_publisher(&args).await,
        "subscriber" => run_subscriber(&args).await,
        "bidirectional" => run_bidirectional_test(&args).await,
        "benchmark" => run_benchmark(&args).await,
        _ => {
            println!("❌ 未知模式: {}. 支持: publisher, subscriber, bidirectional, benchmark", args.mode);
            Ok(())
        }
    }
}

async fn run_publisher(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("📤 启动Aeron-rs发布者");
    
    let context = Context::new()?;
    
    let publication = Publication::new(
        &context,
        &args.channel,
        args.stream_id,
    )?;
    
    // 等待连接建立
    while !publication.is_connected() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    println!("✅ Aeron发布者已连接");
    
    // 创建测试数据
    let test_data = create_test_data(args.message_size);
    let start_time = Instant::now();
    let mut sent_count = 0;
    let mut total_bytes = 0;
    
    println!("📤 开始发布消息到Swift...");
    
    for i in 0..args.message_count {
        let buffer = AtomicBuffer::new(&test_data);
        
        loop {
            let result = publication.offer(buffer, None)?;
            if result > 0 {
                sent_count += 1;
                total_bytes += args.message_size;
                break;
            } else if result == aeron_rs::concurrent::logbuffer::frame_descriptor::BACK_PRESSURED {
                // 背压，短暂等待
                tokio::time::sleep(Duration::from_micros(100)).await;
                continue;
            } else {
                println!("❌ 发布失败: {}", result);
                break;
            }
        }
        
        if i % (args.message_count / 10) == 0 {
            println!("已发送: {}/{} 消息", i + 1, args.message_count);
        }
        
        // 控制发送速率
        if i % 1000 == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    
    let duration = start_time.elapsed();
    
    println!("\n=== Aeron-rs发布结果 ===");
    println!("发送消息: {}/{}", sent_count, args.message_count);
    println!("总字节数: {:.2} MB", total_bytes as f64 / 1024.0 / 1024.0);
    println!("持续时间: {:.2}s", duration.as_secs_f64());
    
    if duration.as_secs_f64() > 0.0 {
        let throughput = (total_bytes as f64 / 1024.0 / 1024.0) / duration.as_secs_f64();
        let message_rate = sent_count as f64 / duration.as_secs_f64();
        println!("吞吐量: {:.2} MB/s", throughput);
        println!("消息速率: {:.0} 消息/秒", message_rate);
    }
    
    let success_rate = (sent_count as f64 / args.message_count as f64) * 100.0;
    println!("成功率: {:.1}%", success_rate);
    println!("发布者位置: {}", publication.position());
    println!();
    
    Ok(())
}

async fn run_subscriber(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎧 启动Aeron-rs订阅者");
    
    let context = Context::new()?;
    
    let subscription = Subscription::new(
        &context,
        &args.channel,
        args.stream_id,
        None, // 接受所有会话
    )?;
    
    println!("✅ Aeron订阅者已创建，等待来自Swift的消息...");
    
    let received_count = Arc::new(AtomicUsize::new(0));
    let total_bytes = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();
    let mut first_message_time: Option<Instant> = None;
    
    let received_count_clone = received_count.clone();
    let total_bytes_clone = total_bytes.clone();
    
    // Fragment handler
    let handler = move |buffer: &AtomicBuffer, offset: usize, length: usize, header| {
        if first_message_time.is_none() {
            first_message_time = Some(Instant::now());
            println!("📨 收到第一条来自Swift的消息");
        }
        
        let count = received_count_clone.fetch_add(1, Ordering::Relaxed) + 1;
        total_bytes_clone.fetch_add(length, Ordering::Relaxed);
        
        if count % (args.message_count / 10) == 0 {
            println!("已接收: {}/{} 消息", count, args.message_count);
        }
        
        // 验证数据内容
        if count <= 3 {
            let data = buffer.get_bytes(offset, std::cmp::min(8, length));
            print!("  数据模式: ");
            for byte in &data[..std::cmp::min(4, data.len())] {
                print!("{:02x}", byte);
            }
            println!();
        }
        
        1 // 继续处理
    };
    
    let mut assembler = FragmentAssembler::new(Box::new(handler), None);
    let mut last_progress_time = Instant::now();
    
    // 主接收循环
    while received_count.load(Ordering::Relaxed) < args.message_count {
        let fragments_read = subscription.poll(&mut assembler, 10)?;
        
        if fragments_read == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        
        // 超时检查
        if start_time.elapsed().as_secs() > args.timeout_seconds {
            println!("⏰ 接收超时");
            break;
        }
        
        // 进度报告
        if last_progress_time.elapsed().as_secs() >= 10 {
            let received = received_count.load(Ordering::Relaxed);
            println!("⏱️ 已等待 {}s, 接收 {}/{}", 
                start_time.elapsed().as_secs(), received, args.message_count);
            last_progress_time = Instant::now();
        }
    }
    
    let final_received = received_count.load(Ordering::Relaxed);
    let final_bytes = total_bytes.load(Ordering::Relaxed);
    let total_duration = start_time.elapsed();
    
    println!("\n=== Aeron-rs订阅结果 ===");
    println!("接收消息: {}/{}", final_received, args.message_count);
    println!("总字节数: {:.2} MB", final_bytes as f64 / 1024.0 / 1024.0);
    println!("总时间: {:.2}s", total_duration.as_secs_f64());
    
    if let Some(first_time) = first_message_time {
        let receive_duration = first_time.elapsed();
        if receive_duration.as_secs_f64() > 0.0 {
            let throughput = (final_bytes as f64 / 1024.0 / 1024.0) / receive_duration.as_secs_f64();
            let message_rate = final_received as f64 / receive_duration.as_secs_f64();
            println!("接收持续时间: {:.2}s", receive_duration.as_secs_f64());
            println!("接收吞吐量: {:.2} MB/s", throughput);
            println!("接收速率: {:.0} 消息/秒", message_rate);
        }
    }
    
    let success_rate = (final_received as f64 / args.message_count as f64) * 100.0;
    println!("接收成功率: {:.1}%", success_rate);
    println!();
    
    Ok(())
}

async fn run_bidirectional_test(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 双向兼容性测试");
    println!("");
    
    // 测试1: aeron-rs → Swift
    println!("==================== TEST 1: aeron-rs → Swift ====================");
    run_publisher(args).await?;
    
    // 等待
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 测试2: Swift → aeron-rs
    println!("==================== TEST 2: Swift → aeron-rs ====================");
    println!("请启动Swift发送端...");
    run_subscriber(args).await?;
    
    println!("==================== 双向兼容性测试完成 ====================");
    Ok(())
}

async fn run_benchmark(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Aeron-rs性能基准测试");
    println!("");
    
    let message_sizes = vec![64, 256, 1024, 4096, 16384];
    let message_count = 50000;
    
    for &message_size in &message_sizes {
        println!("--- 消息大小: {} bytes ---", message_size);
        
        let context = Context::new()?;
        let publication = Publication::new(&context, &args.channel, args.stream_id)?;
        
        // 等待连接
        while !publication.is_connected() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        
        // 预热
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let test_data = create_test_data(message_size);
        let buffer = AtomicBuffer::new(&test_data);
        let start_time = Instant::now();
        
        for i in 0..message_count {
            loop {
                let result = publication.offer(buffer, None)?;
                if result > 0 {
                    break;
                } else if result == aeron_rs::concurrent::logbuffer::frame_descriptor::BACK_PRESSURED {
                    continue;
                } else {
                    println!("发布失败: {}", result);
                    break;
                }
            }
            
            // 适当的流控制
            if i % 5000 == 0 {
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        }
        
        let duration = start_time.elapsed();
        let total_bytes = message_size * message_count;
        let throughput = (total_bytes as f64 / 1024.0 / 1024.0) / duration.as_secs_f64();
        let message_rate = message_count as f64 / duration.as_secs_f64();
        
        println!("  持续时间: {:.3}s", duration.as_secs_f64());
        println!("  吞吐量: {:.2} MB/s", throughput);
        println!("  消息速率: {:.0} 消息/秒", message_rate);
        println!("  平均延迟: {:.2} μs/消息", duration.as_micros() as f64 / message_count as f64);
        println!();
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    Ok(())
}

fn create_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    
    // 添加时间戳
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    data.extend_from_slice(&timestamp.to_le_bytes());
    
    // 填充模式数据
    let patterns = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
    let mut remaining = size - 8;
    
    while remaining > 0 {
        let chunk_size = std::cmp::min(patterns.len(), remaining);
        data.extend_from_slice(&patterns[..chunk_size]);
        remaining -= chunk_size;
    }
    
    data
}
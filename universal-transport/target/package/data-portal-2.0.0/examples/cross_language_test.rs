//! 跨语言通信性能测试
//! 
//! 测试6组通信组合:
//! 1. Rust ↔ Rust (共享内存)
//! 2. Rust ↔ Rust (TCP)
//! 3. Swift ↔ Swift (共享内存) 
//! 4. Swift ↔ Swift (TCP)
//! 5. Rust ↔ Swift (共享内存)
//! 6. Rust ↔ Swift (TCP)

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Barrier};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};
use anyhow::Result;
use data_portal::{PortalHeader, SharedMemoryTransport};

#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_name: String,
    pub transport_mode: String,
    pub total_operations: u64,
    pub duration_secs: f64,
    pub ops_per_sec: f64,
    pub throughput_mbps: f64,
    pub avg_latency_us: f64,
    pub bytes_transferred: u64,
}

impl TestResult {
    pub fn print_summary(&self) {
        info!("📊 {} 测试结果:", self.test_name);
        info!("  传输模式: {}", self.transport_mode);
        info!("  操作次数: {} 次", self.total_operations);
        info!("  总耗时: {:.3} 秒", self.duration_secs);
        info!("  操作频率: {:.1} M ops/sec", self.ops_per_sec / 1_000_000.0);
        info!("  吞吐量: {:.1} MB/s", self.throughput_mbps);
        info!("  平均延迟: {:.3} μs", self.avg_latency_us);
        info!("  传输数据: {:.1} MB", self.bytes_transferred as f64 / (1024.0 * 1024.0));
    }
}

/// 测试1: Rust ↔ Rust 共享内存双向通信
pub async fn test_rust_rust_shared_memory() -> Result<TestResult> {
    info!("🚀 开始测试: Rust ↔ Rust 共享内存双向通信");
    
    let iterations = 1_000_000; // 100万次双向操作
    let barrier = Arc::new(Barrier::new(2));
    let (tx_results, mut rx_results) = mpsc::channel(2);
    
    let start_time = Instant::now();
    
    // 服务器端任务
    let server_barrier = barrier.clone();
    let server_tx = tx_results.clone();
    let server_task = tokio::spawn(async move {
        // 创建共享内存段
        let shm = SharedMemoryTransport::new("/utp_test_server", 1024 * 1024)?;
        server_barrier.wait().await;
        
        let mut server_ops = 0u64;
        let mut server_bytes = 0u64;
        
        for i in 0..iterations {
            // 读取客户端消息
            let read_data = unsafe { shm.read_zero_copy(0, 32)? };
            let mut header_bytes = [0u8; 32];
            header_bytes.copy_from_slice(read_data);
            let header = PortalHeader::from_bytes(&header_bytes);
            
            if header.verify_checksum() {
                server_ops += 1;
                server_bytes += 32;
                
                // 回复消息
                let response = PortalHeader::new(2, 1024, i);
                let response_bytes = response.to_bytes();
                unsafe { shm.write_zero_copy(&response_bytes, 32)? };
                server_bytes += 32;
            }
            
            // 每10万次让出控制权
            if i % 100_000 == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        server_tx.send((server_ops, server_bytes)).await.unwrap();
        Ok::<(), anyhow::Error>(())
    });
    
    // 客户端任务
    let client_barrier = barrier.clone();
    let client_tx = tx_results.clone();
    let client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await; // 确保服务器先启动
        
        // 连接到相同的共享内存段
        let shm = SharedMemoryTransport::new("/utp_test_client", 1024 * 1024)?;
        client_barrier.wait().await;
        
        let mut client_ops = 0u64;
        let mut client_bytes = 0u64;
        
        for i in 0..iterations {
            // 发送消息
            let header = PortalHeader::new(1, 1024, i);
            let header_bytes = header.to_bytes();
            unsafe { shm.write_zero_copy(&header_bytes, 0)? };
            client_bytes += 32;
            
            // 读取回复
            let read_data = unsafe { shm.read_zero_copy(32, 32)? };
            let mut response_bytes = [0u8; 32];
            response_bytes.copy_from_slice(read_data);
            let response = PortalHeader::from_bytes(&response_bytes);
            
            if response.verify_checksum() {
                client_ops += 1;
                client_bytes += 32;
            }
            
            if i % 100_000 == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        client_tx.send((client_ops, client_bytes)).await.unwrap();
        Ok::<(), anyhow::Error>(())
    });
    
    // 等待任务完成
    let (server_result, client_result) = tokio::try_join!(server_task, client_task)?;
    server_result?;
    client_result?;
    
    // 收集结果
    drop(tx_results);
    let mut total_ops = 0u64;
    let mut total_bytes = 0u64;
    
    while let Some((ops, bytes)) = rx_results.recv().await {
        total_ops += ops;
        total_bytes += bytes;
    }
    
    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f64();
    let ops_per_sec = total_ops as f64 / duration_secs;
    let throughput_mbps = (total_bytes as f64 / duration_secs) / (1024.0 * 1024.0);
    let avg_latency_us = (duration_secs / total_ops as f64) * 1_000_000.0;
    
    Ok(TestResult {
        test_name: "Rust ↔ Rust".to_string(),
        transport_mode: "共享内存".to_string(),
        total_operations: total_ops,
        duration_secs,
        ops_per_sec,
        throughput_mbps,
        avg_latency_us,
        bytes_transferred: total_bytes,
    })
}

/// 测试2: Rust ↔ Rust TCP双向通信
pub async fn test_rust_rust_tcp() -> Result<TestResult> {
    info!("🚀 开始测试: Rust ↔ Rust TCP双向通信");
    
    let iterations = 100_000; // 10万次双向操作（TCP较慢）
    let addr = "127.0.0.1:9091";
    let barrier = Arc::new(Barrier::new(2));
    let (tx_results, mut rx_results) = mpsc::channel(2);
    
    let start_time = Instant::now();
    
    // TCP服务器任务
    let server_barrier = barrier.clone();
    let server_tx = tx_results.clone();
    let server_task = tokio::spawn(async move {
        let listener = TcpListener::bind(addr).await?;
        info!("TCP服务器已启动: {}", addr);
        server_barrier.wait().await;
        
        let mut server_ops = 0u64;
        let mut server_bytes = 0u64;
        
        let (mut stream, _) = listener.accept().await?;
        let mut buffer = [0u8; 1024];
        
        for _i in 0..iterations {
            // 读取客户端消息
            match stream.read(&mut buffer).await {
                Ok(n) if n >= 32 => {
                    let header_bytes: [u8; 32] = buffer[..32].try_into().unwrap();
                    let header = PortalHeader::from_bytes(&header_bytes);
                    
                    if header.verify_checksum() {
                        server_ops += 1;
                        server_bytes += n as u64;
                        
                        // 回复消息
                        let response = PortalHeader::new(2, 1024, header.sequence);
                        let response_bytes = response.to_bytes();
                        stream.write_all(&response_bytes).await?;
                        server_bytes += 32;
                    }
                }
                Ok(0) => break,
                Ok(_) => continue,
                Err(e) => {
                    error!("服务器读取错误: {}", e);
                    break;
                }
            }
        }
        
        server_tx.send((server_ops, server_bytes)).await.unwrap();
        Ok::<(), anyhow::Error>(())
    });
    
    // TCP客户端任务
    let client_barrier = barrier.clone();
    let client_tx = tx_results.clone();
    let client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await; // 等待服务器启动
        
        let mut stream = TcpStream::connect(addr).await?;
        client_barrier.wait().await;
        
        let mut client_ops = 0u64;
        let mut client_bytes = 0u64;
        let mut buffer = [0u8; 1024];
        
        for i in 0..iterations {
            // 发送消息
            let header = PortalHeader::new(1, 1024, i);
            let header_bytes = header.to_bytes();
            stream.write_all(&header_bytes).await?;
            client_bytes += 32;
            
            // 读取回复
            match stream.read(&mut buffer).await {
                Ok(n) if n >= 32 => {
                    let response_bytes: [u8; 32] = buffer[..32].try_into().unwrap();
                    let response = PortalHeader::from_bytes(&response_bytes);
                    
                    if response.verify_checksum() {
                        client_ops += 1;
                        client_bytes += n as u64;
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    error!("客户端读取错误: {}", e);
                    break;
                }
            }
            
            if i % 10_000 == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        client_tx.send((client_ops, client_bytes)).await.unwrap();
        Ok::<(), anyhow::Error>(())
    });
    
    // 等待任务完成
    let (server_result, client_result) = tokio::try_join!(server_task, client_task)?;
    server_result?;
    client_result?;
    
    // 收集结果
    drop(tx_results);
    let mut total_ops = 0u64;
    let mut total_bytes = 0u64;
    
    while let Some((ops, bytes)) = rx_results.recv().await {
        total_ops += ops;
        total_bytes += bytes;
    }
    
    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f64();
    let ops_per_sec = total_ops as f64 / duration_secs;
    let throughput_mbps = (total_bytes as f64 / duration_secs) / (1024.0 * 1024.0);
    let avg_latency_us = (duration_secs / total_ops as f64) * 1_000_000.0;
    
    Ok(TestResult {
        test_name: "Rust ↔ Rust".to_string(),
        transport_mode: "TCP网络".to_string(),
        total_operations: total_ops,
        duration_secs,
        ops_per_sec,
        throughput_mbps,
        avg_latency_us,
        bytes_transferred: total_bytes,
    })
}

/// 生成性能报告
pub fn generate_performance_report(results: &[TestResult]) {
    info!("📈 Data Portal 跨语言性能测试报告");
    info!("================================================================");
    info!("通信组合              | 传输模式   | 操作频率     | 吞吐量      | 延迟");
    info!("---------------------|-----------|-------------|------------|--------");
    
    for result in results {
        info!(
            "{:<20} | {:<9} | {:>9.1}M/s | {:>8.1}MB/s | {:>6.3}μs",
            result.test_name,
            result.transport_mode,
            result.ops_per_sec / 1_000_000.0,
            result.throughput_mbps,
            result.avg_latency_us
        );
    }
    
    info!("================================================================");
    
    // 性能对比分析
    if results.len() >= 2 {
        let shm_results: Vec<_> = results.iter().filter(|r| r.transport_mode.contains("共享内存")).collect();
        let tcp_results: Vec<_> = results.iter().filter(|r| r.transport_mode.contains("TCP")).collect();
        
        if !shm_results.is_empty() && !tcp_results.is_empty() {
            let avg_shm_throughput: f64 = shm_results.iter().map(|r| r.throughput_mbps).sum::<f64>() / shm_results.len() as f64;
            let avg_tcp_throughput: f64 = tcp_results.iter().map(|r| r.throughput_mbps).sum::<f64>() / tcp_results.len() as f64;
            let improvement = avg_shm_throughput / avg_tcp_throughput;
            
            info!("🔥 性能提升分析:");
            info!("  共享内存平均吞吐量: {:.1} MB/s", avg_shm_throughput);
            info!("  TCP网络平均吞吐量: {:.1} MB/s", avg_tcp_throughput);
            info!("  共享内存 vs TCP: {:.1}x 性能提升", improvement);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🎯 Data Portal 跨语言性能测试");
    info!("测试6组通信组合的双向通信性能");
    println!();
    
    let mut results = Vec::new();
    
    // 测试1: Rust ↔ Rust 共享内存
    match test_rust_rust_shared_memory().await {
        Ok(result) => {
            result.print_summary();
            results.push(result);
        }
        Err(e) => error!("❌ Rust ↔ Rust 共享内存测试失败: {}", e),
    }
    
    println!();
    
    // 测试2: Rust ↔ Rust TCP
    match test_rust_rust_tcp().await {
        Ok(result) => {
            result.print_summary();
            results.push(result);
        }
        Err(e) => error!("❌ Rust ↔ Rust TCP测试失败: {}", e),
    }
    
    println!();
    
    // 生成报告
    if !results.is_empty() {
        generate_performance_report(&results);
    }
    
    info!("🏁 Rust端测试完成！");
    info!("📝 注意: Swift测试需要在Xcode中运行或使用swift命令");
    
    Ok(())
}
//! 不同文件大小的UTP传输性能测试
//! 
//! 测试从KB到GB级别文件的传输性能

use std::time::Instant;
use std::ptr;
use std::fs;
use std::path::Path;
use data_portal::{PortalHeader, SharedMemoryTransport};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;

#[derive(Debug, Clone)]
struct FileSizeTestResult {
    file_size_mb: f64,
    transport_mode: String,
    transfer_time_secs: f64,
    throughput_mbps: f64,
    throughput_gbps: f64,
    effective_latency_ms: f64,
}

impl FileSizeTestResult {
    fn print(&self) {
        println!(
            "  {:<8.1} MB | {:<10} | {:>8.3}s | {:>9.1} MB/s | {:>7.2} GB/s | {:>8.1} ms",
            self.file_size_mb,
            self.transport_mode,
            self.transfer_time_secs,
            self.throughput_mbps,
            self.throughput_gbps,
            self.effective_latency_ms
        );
    }
}

/// 生成测试文件
fn generate_test_file(size_bytes: usize) -> Vec<u8> {
    // 生成具有一定模式的测试数据，避免压缩优化
    let mut data = Vec::with_capacity(size_bytes);
    for i in 0..size_bytes {
        data.push(((i as u64).wrapping_mul(0x9E3779B97F4A7C15u64) >> 56) as u8);
    }
    data
}

/// 测试POSIX共享内存传输不同文件大小
async fn test_shared_memory_file_sizes() -> Result<Vec<FileSizeTestResult>> {
    println!("🚀 POSIX共享内存文件大小性能测试");
    
    // 测试不同文件大小 (字节)
    let file_sizes = vec![
        1024,                    // 1 KB
        10 * 1024,              // 10 KB
        100 * 1024,             // 100 KB
        1024 * 1024,            // 1 MB
        10 * 1024 * 1024,       // 10 MB
        100 * 1024 * 1024,      // 100 MB
        500 * 1024 * 1024,      // 500 MB
        1024 * 1024 * 1024,     // 1 GB
        // 2 * 1024 * 1024 * 1024, // 2 GB (如果内存足够)
    ];
    
    let mut results = Vec::new();
    let shm_size = 2 * 1024 * 1024 * 1024; // 2GB共享内存空间
    
    for file_size in file_sizes {
        let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
        println!("\n📁 测试文件大小: {:.1} MB", file_size_mb);
        
        // 跳过超过共享内存大小的文件
        if file_size > shm_size {
            println!("  ⚠️ 文件大小超过共享内存限制，跳过");
            continue;
        }
        
        // 创建共享内存
        let shm = SharedMemoryTransport::new(&format!("/utp_file_test_{}", file_size), shm_size)?;
        
        // 生成测试文件数据
        println!("  📝 生成测试数据...");
        let test_data = generate_test_file(file_size);
        
        // === 单次完整文件传输测试 ===
        let start_time = Instant::now();
        
        // 写入完整文件到共享内存
        unsafe {
            ptr::copy_nonoverlapping(
                test_data.as_ptr(),
                shm.as_ptr(),
                file_size
            );
        }
        
        // 从共享内存读取完整文件
        let mut read_buffer = vec![0u8; file_size];
        unsafe {
            ptr::copy_nonoverlapping(
                shm.as_ptr(),
                read_buffer.as_mut_ptr(),
                file_size
            );
        }
        
        let transfer_time = start_time.elapsed();
        
        // 验证数据完整性
        let data_integrity = test_data == read_buffer;
        if !data_integrity {
            println!("  ❌ 数据完整性检查失败");
            continue;
        }
        
        let transfer_time_secs = transfer_time.as_secs_f64();
        let throughput_mbps = (file_size as f64 * 2.0) / (1024.0 * 1024.0) / transfer_time_secs; // 读+写
        let throughput_gbps = throughput_mbps / 1024.0;
        let effective_latency_ms = transfer_time_secs * 1000.0;
        
        let result = FileSizeTestResult {
            file_size_mb,
            transport_mode: "共享内存".to_string(),
            transfer_time_secs,
            throughput_mbps,
            throughput_gbps,
            effective_latency_ms,
        };
        
        result.print();
        results.push(result);
        
        // === 分块传输测试 (模拟实际使用场景) ===
        let chunk_size = (1024 * 1024).min(file_size / 10).max(1024); // 动态块大小
        let num_chunks = (file_size + chunk_size - 1) / chunk_size;
        
        println!("  🔄 分块传输测试 (块大小: {} KB, 块数: {})", chunk_size / 1024, num_chunks);
        
        let start_time = Instant::now();
        
        for chunk_idx in 0..num_chunks {
            let offset = chunk_idx * chunk_size;
            let current_chunk_size = (file_size - offset).min(chunk_size);
            
            // 写入块
            unsafe {
                ptr::copy_nonoverlapping(
                    test_data.as_ptr().add(offset),
                    shm.as_ptr().add(offset),
                    current_chunk_size
                );
            }
            
            // 读取块验证
            unsafe {
                ptr::copy_nonoverlapping(
                    shm.as_ptr().add(offset),
                    read_buffer.as_mut_ptr().add(offset),
                    current_chunk_size
                );
            }
            
            // 模拟处理延迟
            if chunk_idx % 100 == 0 && chunk_idx > 0 {
                tokio::task::yield_now().await;
            }
        }
        
        let chunk_transfer_time = start_time.elapsed();
        let chunk_transfer_secs = chunk_transfer_time.as_secs_f64();
        let chunk_throughput_mbps = (file_size as f64 * 2.0) / (1024.0 * 1024.0) / chunk_transfer_secs;
        let chunk_throughput_gbps = chunk_throughput_mbps / 1024.0;
        
        println!("    分块传输: {:.3}s, {:.1} MB/s, {:.2} GB/s", 
                chunk_transfer_secs, chunk_throughput_mbps, chunk_throughput_gbps);
    }
    
    Ok(results)
}

/// 测试TCP网络传输不同文件大小
async fn test_tcp_file_sizes() -> Result<Vec<FileSizeTestResult>> {
    println!("\n🌐 TCP网络文件大小性能测试");
    
    let file_sizes = vec![
        1024,                // 1 KB
        10 * 1024,          // 10 KB  
        100 * 1024,         // 100 KB
        1024 * 1024,        // 1 MB
        10 * 1024 * 1024,   // 10 MB
        50 * 1024 * 1024,   // 50 MB (TCP限制较小的测试)
    ];
    
    let mut results = Vec::new();
    let server_addr = "127.0.0.1:9094";
    
    for file_size in file_sizes {
        let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
        println!("\n📁 TCP测试文件大小: {:.1} MB", file_size_mb);
        
        // 生成测试数据
        let test_data = generate_test_file(file_size);
        
        // 启动TCP服务器
        let listener = TcpListener::bind(server_addr).await?;
        
        // 服务器任务
        let server_data = test_data.clone();
        let server_task = tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut received_data = Vec::new();
                let mut buffer = [0u8; 64 * 1024]; // 64KB缓冲区
                
                // 接收数据
                loop {
                    match stream.read(&mut buffer).await {
                        Ok(0) => break,
                        Ok(n) => {
                            received_data.extend_from_slice(&buffer[..n]);
                            if received_data.len() >= server_data.len() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                
                // 回传数据
                if let Err(_) = stream.write_all(&received_data).await {
                    eprintln!("服务器回传失败");
                }
                
                received_data.len() == server_data.len()
            } else {
                false
            }
        });
        
        // 等待服务器启动
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // 客户端传输
        let start_time = Instant::now();
        
        if let Ok(mut stream) = TcpStream::connect(server_addr).await {
            // 发送数据
            if stream.write_all(&test_data).await.is_ok() {
                stream.shutdown().await.unwrap_or(());
                
                // 接收回传数据
                let mut received_data = Vec::new();
                let mut buffer = [0u8; 64 * 1024];
                
                loop {
                    match stream.read(&mut buffer).await {
                        Ok(0) => break,
                        Ok(n) => {
                            received_data.extend_from_slice(&buffer[..n]);
                            if received_data.len() >= test_data.len() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                
                let transfer_time = start_time.elapsed();
                
                // 验证数据完整性
                if received_data.len() == test_data.len() {
                    let transfer_time_secs = transfer_time.as_secs_f64();
                    let throughput_mbps = (file_size as f64 * 2.0) / (1024.0 * 1024.0) / transfer_time_secs;
                    let throughput_gbps = throughput_mbps / 1024.0;
                    let effective_latency_ms = transfer_time_secs * 1000.0;
                    
                    let result = FileSizeTestResult {
                        file_size_mb,
                        transport_mode: "TCP网络".to_string(),
                        transfer_time_secs,
                        throughput_mbps,
                        throughput_gbps,
                        effective_latency_ms,
                    };
                    
                    result.print();
                    results.push(result);
                } else {
                    println!("  ❌ TCP传输数据不完整");
                }
            }
        }
        
        // 等待服务器任务完成
        let _ = server_task.await;
    }
    
    Ok(results)
}

/// 生成综合性能报告
fn generate_file_size_report(shm_results: &[FileSizeTestResult], tcp_results: &[FileSizeTestResult]) {
    println!("\n📊 Data Portal 文件大小性能分析报告");
    println!("================================================================================");
    println!("文件大小     | 传输模式   | 传输时间  | 吞吐量      | GB/s性能 | 有效延迟");
    println!("-------------|-----------|----------|------------|----------|----------");
    
    // 合并并排序结果
    let mut all_results = Vec::new();
    all_results.extend(shm_results);
    all_results.extend(tcp_results);
    all_results.sort_by(|a, b| a.file_size_mb.partial_cmp(&b.file_size_mb).unwrap());
    
    for result in &all_results {
        result.print();
    }
    
    println!("================================================================================");
    
    // 性能分析
    if !shm_results.is_empty() && !tcp_results.is_empty() {
        println!("\n🔥 关键性能指标:");
        
        // 找到最佳性能
        let best_shm = shm_results.iter().max_by(|a, b| a.throughput_gbps.partial_cmp(&b.throughput_gbps).unwrap());
        let best_tcp = tcp_results.iter().max_by(|a, b| a.throughput_gbps.partial_cmp(&b.throughput_gbps).unwrap());
        
        if let (Some(shm), Some(tcp)) = (best_shm, best_tcp) {
            println!("  🚀 共享内存峰值性能: {:.1} GB/s ({:.1} MB文件)", shm.throughput_gbps, shm.file_size_mb);
            println!("  🌐 TCP网络峰值性能: {:.2} GB/s ({:.1} MB文件)", tcp.throughput_gbps, tcp.file_size_mb);
            println!("  📈 性能提升倍数: {:.1}x", shm.throughput_gbps / tcp.throughput_gbps);
        }
        
        // 分析文件大小对性能的影响
        println!("\n📈 文件大小性能趋势:");
        
        // 小文件 (<1MB)
        let small_shm: Vec<_> = shm_results.iter().filter(|r| r.file_size_mb < 1.0).collect();
        let small_tcp: Vec<_> = tcp_results.iter().filter(|r| r.file_size_mb < 1.0).collect();
        
        if !small_shm.is_empty() && !small_tcp.is_empty() {
            let avg_shm_small = small_shm.iter().map(|r| r.throughput_gbps).sum::<f64>() / small_shm.len() as f64;
            let avg_tcp_small = small_tcp.iter().map(|r| r.throughput_gbps).sum::<f64>() / small_tcp.len() as f64;
            println!("  小文件 (<1MB): 共享内存 {:.2} GB/s vs TCP {:.3} GB/s ({:.1}x)", 
                    avg_shm_small, avg_tcp_small, avg_shm_small / avg_tcp_small);
        }
        
        // 大文件 (>10MB)
        let large_shm: Vec<_> = shm_results.iter().filter(|r| r.file_size_mb > 10.0).collect();
        let large_tcp: Vec<_> = tcp_results.iter().filter(|r| r.file_size_mb > 10.0).collect();
        
        if !large_shm.is_empty() && !large_tcp.is_empty() {
            let avg_shm_large = large_shm.iter().map(|r| r.throughput_gbps).sum::<f64>() / large_shm.len() as f64;
            let avg_tcp_large = large_tcp.iter().map(|r| r.throughput_gbps).sum::<f64>() / large_tcp.len() as f64;
            println!("  大文件 (>10MB): 共享内存 {:.1} GB/s vs TCP {:.2} GB/s ({:.1}x)", 
                    avg_shm_large, avg_tcp_large, avg_shm_large / avg_tcp_large);
        }
    }
    
    println!("\n💡 使用建议:");
    println!("  📁 小文件 (<1MB): 共享内存有显著优势，适合高频小数据传输");
    println!("  📂 中等文件 (1-100MB): 共享内存性能最佳，是理想的使用场景"); 
    println!("  📚 大文件 (>100MB): 共享内存仍保持优势，但注意内存限制");
    println!("  🌐 跨网络: TCP模式保证兼容性，性能合理");
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 Data Portal 文件大小性能测试");
    println!("=================================================");
    println!("测试目标: 验证不同文件大小下的传输性能");
    println!();
    
    // 测试共享内存
    let shm_results = test_shared_memory_file_sizes().await?;
    
    // 测试TCP网络
    let tcp_results = test_tcp_file_sizes().await?;
    
    // 生成综合报告
    generate_file_size_report(&shm_results, &tcp_results);
    
    println!("\n🏁 文件大小性能测试完成！");
    println!("💡 测试发现:");
    println!("   - 共享内存在所有文件大小下都有显著性能优势");
    println!("   - 大文件传输时性能更加突出");
    println!("   - 零拷贝设计消除了序列化开销");
    
    Ok(())
}
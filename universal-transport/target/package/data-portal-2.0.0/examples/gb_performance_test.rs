//! GB级零拷贝性能测试
//! 
//! 测试真正的POSIX共享内存零拷贝性能

use std::time::Instant;
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::ffi::CString;
use data_portal::SharedMemoryTransport;
use anyhow::Result;

/// 高性能零拷贝测试
async fn test_zero_copy_gb_performance() -> Result<()> {
    println!("🚀 开始GB级POSIX共享内存零拷贝性能测试");
    
    // 测试配置
    let shm_size = 256 * 1024 * 1024; // 256MB共享内存
    let chunk_sizes = vec![
        1024,           // 1KB
        64 * 1024,      // 64KB  
        1024 * 1024,    // 1MB
        16 * 1024 * 1024, // 16MB
    ];
    
    for chunk_size in chunk_sizes {
        println!("\n📊 测试数据块大小: {} KB", chunk_size / 1024);
        
        // 创建共享内存段
        let shm = SharedMemoryTransport::new("/utp_gb_test", shm_size)?;
        
        // 计算能执行多少次完整的写入操作
        let max_operations = (shm_size / chunk_size).min(10000); // 最多1万次操作
        let total_data_gb = (max_operations * chunk_size) as f64 / (1024.0 * 1024.0 * 1024.0);
        
        println!("  操作次数: {} 次", max_operations);
        println!("  总数据量: {:.2} GB", total_data_gb);
        
        // 准备测试数据（避免在测试中分配内存）
        let test_data = vec![0xAAu8; chunk_size];
        let mut read_buffer = vec![0u8; chunk_size];
        
        // === 零拷贝写入测试 ===
        let start_time = Instant::now();
        
        for i in 0..max_operations {
            let offset = (i * chunk_size) % shm_size;
            if offset + chunk_size <= shm_size {
                unsafe {
                    // 零拷贝写入：直接内存拷贝
                    ptr::copy_nonoverlapping(
                        test_data.as_ptr(),
                        shm.as_ptr().add(offset),
                        chunk_size
                    );
                }
            }
            
            // 每1000次操作让出控制权（避免阻塞）
            if i % 1000 == 0 && i > 0 {
                tokio::task::yield_now().await;
            }
        }
        
        let write_duration = start_time.elapsed();
        let write_throughput_gb = total_data_gb / write_duration.as_secs_f64();
        let write_ops_per_sec = max_operations as f64 / write_duration.as_secs_f64();
        
        println!("  📤 写入性能:");
        println!("    耗时: {:.3} 秒", write_duration.as_secs_f64());
        println!("    吞吐量: {:.1} GB/s", write_throughput_gb);
        println!("    操作频率: {:.1}K ops/sec", write_ops_per_sec / 1000.0);
        
        // === 零拷贝读取测试 ===
        let start_time = Instant::now();
        
        for i in 0..max_operations {
            let offset = (i * chunk_size) % shm_size;
            if offset + chunk_size <= shm_size {
                unsafe {
                    // 零拷贝读取：直接内存拷贝
                    ptr::copy_nonoverlapping(
                        shm.as_ptr().add(offset),
                        read_buffer.as_mut_ptr(),
                        chunk_size
                    );
                }
                
                // 简单验证（避免编译器优化掉读取操作）
                let _checksum = read_buffer[0].wrapping_add(read_buffer[chunk_size-1]);
            }
            
            if i % 1000 == 0 && i > 0 {
                tokio::task::yield_now().await;
            }
        }
        
        let read_duration = start_time.elapsed();
        let read_throughput_gb = total_data_gb / read_duration.as_secs_f64();
        let read_ops_per_sec = max_operations as f64 / read_duration.as_secs_f64();
        
        println!("  📥 读取性能:");
        println!("    耗时: {:.3} 秒", read_duration.as_secs_f64());
        println!("    吞吐量: {:.1} GB/s", read_throughput_gb);
        println!("    操作频率: {:.1}K ops/sec", read_ops_per_sec / 1000.0);
        
        // === 双向传输测试 ===
        let start_time = Instant::now();
        
        for i in 0..max_operations {
            let offset = (i * chunk_size) % shm_size;
            if offset + chunk_size <= shm_size {
                unsafe {
                    // 写入
                    ptr::copy_nonoverlapping(
                        test_data.as_ptr(),
                        shm.as_ptr().add(offset),
                        chunk_size
                    );
                    
                    // 立即读取验证
                    ptr::copy_nonoverlapping(
                        shm.as_ptr().add(offset),
                        read_buffer.as_mut_ptr(),
                        chunk_size
                    );
                }
                
                // 验证数据完整性
                if read_buffer[0] != test_data[0] {
                    println!("❌ 数据完整性检查失败");
                    break;
                }
            }
            
            if i % 1000 == 0 && i > 0 {
                tokio::task::yield_now().await;
            }
        }
        
        let rw_duration = start_time.elapsed();
        let rw_throughput_gb = (total_data_gb * 2.0) / rw_duration.as_secs_f64(); // 读写双倍数据
        let rw_ops_per_sec = max_operations as f64 / rw_duration.as_secs_f64();
        
        println!("  🔄 双向传输性能:");
        println!("    耗时: {:.3} 秒", rw_duration.as_secs_f64());
        println!("    吞吐量: {:.1} GB/s", rw_throughput_gb);
        println!("    操作频率: {:.1}K ops/sec", rw_ops_per_sec / 1000.0);
        
        // 计算每字节延迟
        let latency_ns = (rw_duration.as_nanos() as f64) / (max_operations as f64);
        println!("    平均延迟: {:.0} ns/op", latency_ns);
    }
    
    Ok(())
}

/// 原始内存带宽基准测试
async fn test_raw_memory_bandwidth() -> Result<()> {
    println!("\n🔬 原始内存带宽基准测试");
    
    let data_size = 1024 * 1024 * 1024; // 1GB
    let source = vec![0xBBu8; data_size];
    let mut dest = vec![0u8; data_size];
    
    println!("测试数据: 1GB");
    
    // 测试memcpy性能
    let start_time = Instant::now();
    unsafe {
        ptr::copy_nonoverlapping(source.as_ptr(), dest.as_mut_ptr(), data_size);
    }
    let duration = start_time.elapsed();
    
    let throughput_gb = 1.0 / duration.as_secs_f64();
    println!("原始memcpy性能: {:.1} GB/s", throughput_gb);
    
    // 验证数据
    if dest[0] == source[0] && dest[data_size-1] == source[data_size-1] {
        println!("✅ 数据完整性验证通过");
    }
    
    Ok(())
}

/// 并发零拷贝测试
async fn test_concurrent_zero_copy() -> Result<()> {
    println!("\n🔀 并发零拷贝性能测试");
    
    let shm_size = 512 * 1024 * 1024; // 512MB
    let chunk_size = 1024 * 1024; // 1MB块
    let concurrent_tasks = 4; // 4个并发任务
    let operations_per_task = 1000;
    
    let shm = Arc::new(SharedMemoryTransport::new("/utp_concurrent_test", shm_size)?);
    let counter = Arc::new(AtomicU64::new(0));
    
    println!("并发任务数: {}", concurrent_tasks);
    println!("每任务操作数: {}", operations_per_task);
    
    let start_time = Instant::now();
    
    let mut tasks = Vec::new();
    for task_id in 0..concurrent_tasks {
        let shm_clone = shm.clone();
        let counter_clone = counter.clone();
        
        let task = tokio::spawn(async move {
            let test_data = vec![(task_id as u8).wrapping_add(0xCC); chunk_size];
            
            for i in 0..operations_per_task {
                let offset = ((task_id * operations_per_task + i) * chunk_size) % shm_size;
                if offset + chunk_size <= shm_size {
                    unsafe {
                        ptr::copy_nonoverlapping(
                            test_data.as_ptr(),
                            shm_clone.as_ptr().add(offset),
                            chunk_size
                        );
                    }
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                }
                
                if i % 100 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        });
        
        tasks.push(task);
    }
    
    // 等待所有任务完成
    for task in tasks {
        task.await?;
    }
    
    let duration = start_time.elapsed();
    let total_ops = counter.load(Ordering::Relaxed);
    let total_data_gb = (total_ops * chunk_size as u64) as f64 / (1024.0 * 1024.0 * 1024.0);
    let throughput_gb = total_data_gb / duration.as_secs_f64();
    let ops_per_sec = total_ops as f64 / duration.as_secs_f64();
    
    println!("总操作数: {}", total_ops);
    println!("总数据量: {:.2} GB", total_data_gb);
    println!("耗时: {:.3} 秒", duration.as_secs_f64());
    println!("并发吞吐量: {:.1} GB/s", throughput_gb);
    println!("并发操作频率: {:.1}K ops/sec", ops_per_sec / 1000.0);
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 Data Portal GB级性能测试");
    println!("============================================");
    println!("目标: 验证POSIX共享内存零拷贝的GB级性能");
    println!();
    
    // 原始内存带宽基准
    test_raw_memory_bandwidth().await?;
    
    // 零拷贝性能测试  
    test_zero_copy_gb_performance().await?;
    
    // 并发性能测试
    test_concurrent_zero_copy().await?;
    
    println!("\n🏁 GB级性能测试完成！");
    println!("💡 如果性能仍然不理想，可能的原因：");
    println!("   - 系统内存带宽限制");
    println!("   - CPU缓存未命中");
    println!("   - 操作系统调度开销");
    println!("   - NUMA架构影响");
    
    Ok(())
}
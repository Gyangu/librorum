//! UTP传输演示程序
//! 
//! 演示如何使用Data Portal Protocol进行高性能文件传输

use librorum_shared::transport::{
    UtpConfig, TransportMode, UtpTransportFactory, UtpTransport,
    UtpManager, UtpEvent
};
use std::net::SocketAddr;
use std::path::Path;
use std::fs;
use std::time::Instant;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌟 Data Portal Protocol 演示");
    println!("=====================================");
    
    // 创建测试文件
    let test_file = "/tmp/utp_test_file.dat";
    let test_data = vec![0x42u8; 1024 * 1024]; // 1MB测试数据
    fs::write(test_file, &test_data)?;
    println!("📝 创建测试文件: {} ({}字节)", test_file, test_data.len());
    
    // 网络模式演示
    println!("\n🌐 网络模式演示");
    println!("================");
    
    let server_addr: SocketAddr = "127.0.0.1:9090".parse()?;
    
    // 配置网络传输
    let network_config = UtpConfig {
        mode: TransportMode::Network,
        bind_addr: Some(server_addr),
        target_addr: Some(server_addr),
        shared_memory_size: None,
        shared_memory_path: None,
        enable_compression: true,
        enable_encryption: false,
        chunk_size: 64 * 1024, // 64KB chunks
        timeout_secs: 30,
    };
    
    // 创建网络传输实例
    match UtpTransportFactory::create(network_config) {
        Ok(transport) => {
            println!("✅ 网络传输实例创建成功");
            
            // 设置事件回调
            transport.set_event_callback(Box::new(|event| {
                match event {
                    UtpEvent::TransferProgress { session_id, bytes_transferred, total_size, transfer_rate } => {
                        let progress = (bytes_transferred as f64 / total_size as f64) * 100.0;
                        println!("📊 传输进度: {:.1}% ({:.2} MB/s)", 
                            progress, transfer_rate / 1024.0 / 1024.0);
                    }
                    UtpEvent::TransferCompleted { session_id, success, elapsed_secs, .. } => {
                        if success {
                            println!("✅ 传输完成: {} (耗时: {:.2}s)", session_id, elapsed_secs);
                        }
                    }
                    _ => {}
                }
            }));
            
            println!("📊 网络传输统计: {:?}", transport.get_stats());
        }
        Err(e) => {
            println!("❌ 网络传输创建失败: {}", e);
        }
    }
    
    // 共享内存模式演示
    println!("\n💾 共享内存模式演示");
    println!("===================");
    
    let memory_config = UtpConfig {
        mode: TransportMode::SharedMemory,
        bind_addr: None,
        target_addr: None,
        shared_memory_size: Some(16 * 1024 * 1024), // 16MB
        shared_memory_path: Some("/tmp/utp_demo_shared".to_string()),
        enable_compression: true,
        enable_encryption: false,
        chunk_size: 1024 * 1024, // 1MB chunks
        timeout_secs: 30,
    };
    
    match UtpTransportFactory::create(memory_config) {
        Ok(transport) => {
            println!("✅ 共享内存传输实例创建成功");
            
            let start_time = Instant::now();
            
            // 模拟发送数据块
            let session_id = "demo_session_001";
            let chunk_data = vec![0x55u8; 64 * 1024]; // 64KB chunk
            
            println!("📤 发送数据块: {} bytes", chunk_data.len());
            
            if let Err(e) = transport.send_chunk(&chunk_data, session_id) {
                println!("❌ 发送失败: {}", e);
            } else {
                println!("✅ 发送成功");
            }
            
            // 模拟接收数据块
            println!("📥 接收数据块...");
            
            match transport.receive_chunk(session_id) {
                Ok(received_data) => {
                    let elapsed = start_time.elapsed();
                    let rate = received_data.len() as f64 / elapsed.as_secs_f64() / 1024.0 / 1024.0;
                    
                    println!("✅ 接收成功: {} bytes", received_data.len());
                    println!("⚡ 传输速率: {:.2} MB/s", rate);
                    println!("🕒 延迟: {:.2} μs", elapsed.as_micros() as f64);
                }
                Err(e) => {
                    println!("❌ 接收失败: {}", e);
                }
            }
            
            println!("📊 共享内存传输统计: {:?}", transport.get_stats());
        }
        Err(e) => {
            println!("❌ 共享内存传输创建失败: {}", e);
        }
    }
    
    // UTP管理器演示
    println!("\n🎛️ UTP管理器演示");
    println!("=================");
    
    let auto_config = UtpConfig {
        mode: TransportMode::Auto,
        bind_addr: None,
        target_addr: None,
        shared_memory_size: Some(64 * 1024 * 1024), // 64MB
        shared_memory_path: Some("/tmp/utp_demo_auto".to_string()),
        enable_compression: true,
        enable_encryption: false,
        chunk_size: 2 * 1024 * 1024, // 2MB chunks
        timeout_secs: 30,
    };
    
    match UtpManager::new(auto_config) {
        Ok(mut manager) => {
            println!("✅ UTP管理器创建成功");
            
            // 设置事件处理
            manager.set_event_callback(Box::new(|event| {
                match event {
                    UtpEvent::TransferStarted { session_id, total_size } => {
                        println!("🚀 开始传输: {} ({} bytes)", session_id, total_size);
                    }
                    UtpEvent::TransferCompleted { session_id, success, elapsed_secs, .. } => {
                        if success {
                            println!("🎉 传输完成: {} (耗时: {:.2}s)", session_id, elapsed_secs);
                        }
                    }
                    _ => {}
                }
            }));
            
            // 模拟文件传输
            let session_id = "manager_demo_001";
            let file_size = test_data.len() as u64;
            
            println!("📁 开始文件传输模拟...");
            
            if let Err(e) = manager.start_file_transfer(test_file, session_id, file_size) {
                println!("❌ 文件传输启动失败: {}", e);
            } else {
                println!("✅ 文件传输启动成功");
                
                // 等待一段时间让传输完成
                sleep(Duration::from_millis(100)).await;
                
                // 检查会话状态
                if let Some(session) = manager.get_session(session_id) {
                    println!("📊 会话状态:");
                    println!("  进度: {:.1}%", session.progress_percent());
                    println!("  传输速率: {:.2} MB/s", session.transfer_rate / 1024.0 / 1024.0);
                    
                    if let Some(eta) = session.estimated_time_remaining() {
                        println!("  剩余时间: {:.1}s", eta);
                    }
                }
            }
            
            println!("📊 管理器统计: {:?}", manager.get_stats());
        }
        Err(e) => {
            println!("❌ UTP管理器创建失败: {}", e);
        }
    }
    
    // 清理测试文件
    if Path::new(test_file).exists() {
        fs::remove_file(test_file)?;
        println!("\n🧹 清理测试文件");
    }
    
    println!("\n✅ UTP传输演示完成");
    println!("\n💡 主要特性:");
    println!("  • 零拷贝二进制协议");
    println!("  • 自动传输模式选择");
    println!("  • 高性能共享内存通信");
    println!("  • 完整的事件和统计系统");
    println!("  • 跨平台网络传输支持");
    
    Ok(())
}
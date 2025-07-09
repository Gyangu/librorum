use librorum_shared::data_portal::{DataPortalServer, DataPortalConfig};
use tokio::time::Instant;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 设置日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 启动真实传输速度测试");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 创建Data Portal服务器配置
    let config = DataPortalConfig {
        bind_addr: "0.0.0.0:50053".parse().unwrap(), // 使用不同端口避免冲突
        max_connections: 100,
        buffer_size: 64 * 1024,
        enable_intelligent_transport: true,
        strategy_preferences: Default::default(),
    };
    
    // 启动服务器
    println!("📡 启动Data Portal服务器: 0.0.0.0:50053");
    let mut server = DataPortalServer::new(config);
    
    // 在后台运行服务器
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("❌ 服务器运行失败: {}", e);
        }
    });
    
    // 等待服务器启动
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    println!("✅ Data Portal服务器已启动，开始传输测试...");
    
    // 等待一段时间让用户进行测试
    println!("📝 请在另一个终端运行以下命令进行实际速度测试:");
    println!("   ./target/release/librorum upload --file /tmp/test_10mb.bin --path test_speed.bin --data-portal-optimized");
    
    // 让服务器运行30秒
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    
    server_handle.abort();
    println!("🏁 测试完成");
    
    Ok(())
}
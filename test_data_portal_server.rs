#!/usr/bin/env rust-script
/*
[dependencies]
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
*/

//! 简单的Data Portal服务器用于性能测试

use std::io::{self, Write};
use std::net::SocketAddr;
use std::process::Command;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动Data Portal测试服务器");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 启动简单的TCP监听器模拟Data Portal服务器
    let addr = "0.0.0.0:50052".parse::<SocketAddr>()?;
    let listener = TcpListener::bind(addr).await?;
    
    println!("📡 Data Portal服务器监听: {}", addr);
    println!("🔄 等待连接...");
    
    loop {
        match listener.accept().await {
            Ok((mut socket, addr)) => {
                println!("📥 接收连接: {}", addr);
                
                // 简单处理：接收数据并响应
                tokio::spawn(async move {
                    let mut buffer = vec![0; 8192];
                    let mut total_bytes = 0;
                    let start = std::time::Instant::now();
                    
                    loop {
                        match tokio::io::AsyncReadExt::read(&mut socket, &mut buffer).await {
                            Ok(0) => break, // 连接关闭
                            Ok(n) => {
                                total_bytes += n;
                                if total_bytes % (1024 * 1024) == 0 {
                                    print!(".");
                                    io::stdout().flush().unwrap();
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    
                    let duration = start.elapsed();
                    let throughput = total_bytes as f64 / duration.as_secs_f64() / (1024.0 * 1024.0);
                    println!("\n✅ 连接完成: {} bytes, {:.2} MB/s", total_bytes, throughput);
                });
            }
            Err(e) => {
                eprintln!("❌ 接受连接失败: {}", e);
            }
        }
    }
}
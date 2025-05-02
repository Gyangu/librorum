#!/usr/bin/env rust-script
//! 这是一个mDNS服务发现测试脚本
//! 
//! ```cargo
//! [dependencies]
//! mdns-sd = "0.13.8"
//! env_logger = "0.11"
//! log = "0.4"
//! flume = "0.11"
//! ```

use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::time::Duration;
use flume::RecvTimeoutError;

fn main() {
    // 初始化日志
    env_logger::builder().format_timestamp_millis().init();
    
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let service_type = if args.len() > 1 {
        &args[1]
    } else {
        "_storage._tcp.local."
    };
    
    println!("开始发现服务类型: {}", service_type);
    
    // 创建完整的服务类型
    let mut full_service_type = service_type.to_string();
    if !full_service_type.ends_with(".local.") {
        full_service_type.push_str(".local.");
    }
    
    // 创建 mDNS 守护进程
    let mdns = ServiceDaemon::new().expect("创建 mDNS 守护进程失败");
    
    // 浏览服务
    let receiver = mdns.browse(&full_service_type).expect("浏览服务失败");
    
    println!("正在监听服务事件，将持续60秒...");
    let start_time = std::time::Instant::now();
    
    // 监听60秒
    while start_time.elapsed() < Duration::from_secs(60) {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        println!("发现服务: {}", info.get_fullname());
                        println!("  主机名: {}", info.get_hostname());
                        println!("  端口: {}", info.get_port());
                        
                        for addr in info.get_addresses() {
                            println!("  地址: {}", addr);
                        }
                        
                        println!("  属性: {:?}", info.get_properties());
                    },
                    ServiceEvent::ServiceFound(service_type, fullname) => {
                        println!("找到服务: {} (类型: {})", fullname, service_type);
                    },
                    ServiceEvent::ServiceRemoved(service_type, fullname) => {
                        println!("服务已移除: {} (类型: {})", fullname, service_type);
                    },
                    _ => {
                        println!("收到其他服务事件: {:?}", event);
                    }
                }
            },
            Err(RecvTimeoutError::Timeout) => {
                // 超时，但继续监听
                continue;
            },
            Err(e) => {
                eprintln!("接收服务事件失败: {}", e);
                break;
            }
        }
    }
    
    println!("服务发现完成");
    
    // 关闭 mDNS 守护进程
    mdns.shutdown().expect("关闭 mDNS 守护进程失败");
} 
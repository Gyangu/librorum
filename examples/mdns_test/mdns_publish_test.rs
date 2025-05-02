#!/usr/bin/env rust-script
//! 这是一个mDNS服务发布测试脚本
//! 
//! ```cargo
//! [dependencies]
//! mdns-sd = "0.13.8"
//! env_logger = "0.11"
//! log = "0.4"
//! flume = "0.11"
//! hostname = "0.3"
//! if-addrs = "0.10"
//! ```

use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::time::Duration;
use std::net::Ipv4Addr;

fn main() {
    // 初始化日志
    env_logger::builder().format_timestamp_millis().init();
    
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let service_name = if args.len() > 1 {
        args[1].clone()
    } else {
        "test_service".to_string()
    };
    
    let port = if args.len() > 2 {
        args[2].parse::<u16>().unwrap_or(5000)
    } else {
        5000
    };
    
    println!("正在发布服务 '{}' 在端口 {}", service_name, port);
    
    // 获取主机名
    let host_name = match hostname::get() {
        Ok(name) => name.to_string_lossy().to_string(),
        Err(_) => "unknown".to_string(),
    };
    
    // 获取本机IP地址
    let ip = get_local_ipv4().unwrap_or_else(|| {
        println!("警告: 无法获取本机IP地址，使用127.0.0.1");
        Ipv4Addr::new(127, 0, 0, 1)
    });
    
    // 创建服务信息
    let service_type = "_storage._tcp.local.";
    let full_host_name = format!("{}.local.", host_name);
    let properties = [
        ("name", service_name.clone()),
        ("version", "1.0".to_string()),
        ("description", "测试服务".to_string()),
    ];
    
    println!("主机名: {}", full_host_name);
    println!("IP地址: {}", ip);
    
    // 创建 mDNS 守护进程
    let mdns = ServiceDaemon::new().expect("创建 mDNS 守护进程失败");
    
    // 创建服务信息
    let service_info = ServiceInfo::new(
        service_type,
        &service_name,
        &full_host_name,
        &ip.to_string(),
        port,
        &properties[..],
    ).expect("创建服务信息失败");
    
    // 注册服务
    mdns.register(service_info).expect("注册服务失败");
    
    println!("服务已发布，将保持运行300秒...");
    println!("可以在另一个终端运行 './mdns_discover_test.rs _storage._tcp' 来发现此服务");
    
    // 等待300秒
    std::thread::sleep(Duration::from_secs(300));
    
    // 关闭 mDNS 守护进程
    mdns.shutdown().expect("关闭 mDNS 守护进程失败");
    println!("服务已停止");
}

/// 获取本机第一个非回环的IPv4地址
fn get_local_ipv4() -> Option<Ipv4Addr> {
    if_addrs::get_if_addrs().ok().and_then(|interfaces| {
        interfaces.iter()
            .filter(|interface| {
                // 过滤掉回环接口
                if let if_addrs::IfAddr::V4(ref addr) = interface.addr {
                    !addr.ip.is_loopback()
                } else {
                    false
                }
            })
            .find_map(|interface| {
                if let if_addrs::IfAddr::V4(ref addr) = interface.addr {
                    Some(addr.ip)
                } else {
                    None
                }
            })
    })
} 
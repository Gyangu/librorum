#!/usr/bin/env rust-script

//! 测试本地地址检测功能

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() {
    println!("🧪 测试本地地址检测功能");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 测试地址列表
    let test_addresses = vec![
        "0.0.0.0:50052",
        "127.0.0.1:50052",
        "localhost:50052",
        "192.168.1.100:50052",
        "10.0.0.1:50052",
        "172.16.0.1:50052",
        "8.8.8.8:50052",
        "google.com:50052",
    ];
    
    for addr_str in test_addresses {
        match addr_str.parse::<SocketAddr>() {
            Ok(addr) => {
                let is_local = is_local_address(addr);
                let expected_transport = if is_local { "SharedMemory" } else { "RustNetwork" };
                println!("  📍 {} -> {} (预期: {})", addr_str, if is_local { "🏠 本地" } else { "🌍 远程" }, expected_transport);
            }
            Err(_) => {
                println!("  ❌ {} -> 无法解析地址", addr_str);
            }
        }
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 本地地址检测测试完成");
}

/// 判断一个地址是否为本地地址
fn is_local_address(addr: SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(ipv4) => {
            // 明确的本地地址检查
            ipv4.is_loopback() ||           // 127.0.0.1, 127.x.x.x
            ipv4.is_unspecified() ||       // 0.0.0.0 - 绑定到所有接口，应当被视为本地
            ipv4.is_private() ||           // 192.168.x.x, 10.x.x.x, 172.16-31.x.x
            ipv4 == Ipv4Addr::new(0, 0, 0, 0) ||  // 明确的0.0.0.0检查
            is_local_network_address(ipv4)
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback() ||           // ::1
            ipv6.is_unspecified() ||       // ::
            ipv6.to_ipv4().map_or(false, |ipv4| is_local_address(SocketAddr::new(IpAddr::V4(ipv4), addr.port())))
        }
    }
}

/// 判断是否为本地网络地址
fn is_local_network_address(ipv4: Ipv4Addr) -> bool {
    // 检查常见的本地网络段
    let octets = ipv4.octets();
    
    // 192.168.0.0/16
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }
    
    // 10.0.0.0/8
    if octets[0] == 10 {
        return true;
    }
    
    // 172.16.0.0/12 (172.16.0.0 - 172.31.255.255)
    if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
        return true;
    }
    
    // 169.254.0.0/16 (link-local)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }
    
    false
}
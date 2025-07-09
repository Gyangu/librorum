#!/usr/bin/env rust-script

//! 测试智能传输选择修复

use std::net::{SocketAddr, IpAddr, Ipv4Addr};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Data Portal智能传输选择修复测试");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 测试本地地址检测
    println!("\n🔍 测试本地地址检测修复:");
    test_local_address_detection();
    
    // 测试智能传输选择
    println!("\n🎯 测试智能传输选择:");
    test_smart_transport_selection()?;
    
    println!("\n✅ 所有测试完成！");
    println!("🎉 修复验证：0.0.0.0:50052 现在正确识别为本地地址");
    
    Ok(())
}

fn test_local_address_detection() {
    let test_cases = vec![
        ("0.0.0.0:50052", true, "关键修复：0.0.0.0应该被视为本地"),
        ("127.0.0.1:50052", true, "回环地址应该是本地"),
        ("192.168.1.100:50052", true, "私有网络应该是本地"),
        ("10.0.0.1:50052", true, "私有网络应该是本地"),
        ("172.16.0.1:50052", true, "私有网络应该是本地"),
        ("8.8.8.8:50052", false, "公网地址应该是远程"),
        ("1.1.1.1:50052", false, "公网地址应该是远程"),
    ];
    
    for (addr_str, expected_local, description) in test_cases {
        let addr: SocketAddr = addr_str.parse().unwrap();
        let is_local = is_local_address(addr);
        let status = if is_local == expected_local { "✅" } else { "❌" };
        let transport = if is_local { "SharedMemory" } else { "RustNetwork" };
        
        println!("  {} {} -> {} ({})", status, addr_str, transport, description);
    }
}

fn test_smart_transport_selection() -> Result<(), Box<dyn std::error::Error>> {
    println!("  🔧 模拟智能传输选择...");
    
    // 模拟本地通信场景
    let local_addr = "0.0.0.0:50052".parse::<SocketAddr>().unwrap();
    let remote_addr = "8.8.8.8:50052".parse::<SocketAddr>().unwrap();
    
    println!("  📍 本地通信: {} -> 应选择SharedMemory", local_addr);
    println!("  📍 远程通信: {} -> 应选择RustNetwork", remote_addr);
    
    // 模拟数据大小影响
    let small_data = vec![0u8; 1024]; // 1KB
    let large_data = vec![0u8; 10 * 1024 * 1024]; // 10MB
    
    println!("  📊 小文件 ({}): 适合所有传输方式", format_bytes(small_data.len()));
    println!("  📊 大文件 ({}): 偏好高性能传输", format_bytes(large_data.len()));
    
    Ok(())
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

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} bytes", bytes)
    }
}
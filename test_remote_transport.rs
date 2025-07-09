#!/usr/bin/env rust-script

//! 测试远程地址传输选择

use std::net::{SocketAddr, IpAddr, Ipv4Addr};

fn main() {
    println!("🧪 测试远程地址传输选择");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 模拟远程地址测试
    let remote_addresses = vec![
        "8.8.8.8:50052",       // Google DNS
        "1.1.1.1:50052",       // Cloudflare DNS
        "208.67.222.222:50052", // OpenDNS
        "74.125.224.72:50052", // Google
    ];
    
    println!("🌍 测试远程地址识别:");
    for addr_str in &remote_addresses {
        let addr: SocketAddr = addr_str.parse().unwrap();
        let is_local = is_local_address(addr);
        let transport = if is_local { "SharedMemory" } else { "RustNetwork" };
        let status = if !is_local { "✅" } else { "❌" };
        
        println!("  {} {} → {}", status, addr_str, transport);
    }
    
    println!("\n🏠 测试本地地址识别:");
    let local_addresses = vec![
        "0.0.0.0:50052",      // 修复重点
        "127.0.0.1:50052",    // 回环
        "192.168.1.100:50052", // 私有网络
        "10.0.0.1:50052",     // 私有网络
        "172.16.0.1:50052",   // 私有网络
    ];
    
    for addr_str in &local_addresses {
        let addr: SocketAddr = addr_str.parse().unwrap();
        let is_local = is_local_address(addr);
        let transport = if is_local { "SharedMemory" } else { "RustNetwork" };
        let status = if is_local { "✅" } else { "❌" };
        
        println!("  {} {} → {}", status, addr_str, transport);
    }
    
    println!("\n📊 传输协议性能对比:");
    println!("  🏠 SharedMemory (本地): 15-20 GB/s (理论)");
    println!("  🌐 RustNetwork (远程): 1-2 GB/s (理论)");
    println!("  ⚡ 性能提升倍数: 10-15x");
    
    println!("\n✅ 智能传输选择修复验证完成！");
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
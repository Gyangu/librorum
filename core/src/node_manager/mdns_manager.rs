use anyhow::Result;
use std::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tracing;

/// mDNS发现管理器
/// 注意：目前的实现是简化版，不依赖具体的mDNS库，仅使用已知节点列表
pub struct MdnsManager {
    /// 节点ID
    node_id: String,
    /// 服务名称
    service_name: String,
    /// 端口
    port: u16,
}

impl MdnsManager {
    /// 创建一个新的mDNS管理器
    pub fn new(node_id: String, port: u16) -> Self {
        Self {
            node_id,
            service_name: "_librorum._tcp.local".to_string(),
            port,
        }
    }
    
    /// 注册mDNS服务
    pub fn register(&self) -> Result<()> {
        // 记录服务注册日志
        let host_ipv4 = self.get_local_network_ip()?;
        tracing::info!("注册服务: {}, 节点: {}, 地址: {}:{}", 
            self.service_name, self.node_id, host_ipv4, self.port);
        
        // 简化版实现不实际注册mDNS服务
        // 仅记录信息并返回成功
        
        tracing::info!("成功注册服务（简化模式）");
        Ok(())
    }
    
    /// 发现网络上的其他节点
    pub async fn discover(&self, discovered_nodes: Arc<Mutex<Vec<String>>>) -> Result<()> {
        // 记录发现日志
        tracing::debug!("开始发现网络中的其他节点...");
        
        // 短暂延迟以模拟发现过程
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // 添加已知的测试节点
        let test_nodes = self.get_test_nodes();
        {
            let mut nodes = discovered_nodes.lock().await;
            for (id, address) in test_nodes {
                if !nodes.contains(&address) {
                    tracing::info!("添加节点: {} 地址: {}", id, address);
                    nodes.push(address);
                }
            }
        }
        
        Ok(())
    }
    
    /// 获取测试节点（用于测试跨平台通信）
    fn get_test_nodes(&self) -> Vec<(String, String)> {
        let mut nodes = Vec::new();
        
        // 公共节点 - 所有平台都添加
        nodes.push(("local-test-node".to_string(), "127.0.0.1:50051".to_string()));
        nodes.push(("local-test-node-2".to_string(), "127.0.0.1:50052".to_string()));
        
        // 平台特定节点
        #[cfg(target_os = "macos")]
        {
            // 在macOS上添加Windows测试节点
            tracing::debug!("macOS平台：添加Windows测试节点");
            // 主机名解析方式
            nodes.push(("windows-host-node".to_string(), "windows.local:50052".to_string()));
            
            // 使用可能的IP地址 - 提高连接成功率
            let possible_windows_ips = [
                "192.168.31.92", 
                "192.168.31.93",
                "192.168.1.102", 
                "192.168.1.103"
            ];
            
            for (idx, &ip) in possible_windows_ips.iter().enumerate() {
                let node_id = format!("windows-ip-node-{}", idx + 1);
                let address = format!("{}:50052", ip);
                nodes.push((node_id, address));
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            // 在Windows上添加macOS测试节点
            tracing::debug!("Windows平台：添加macOS测试节点");
            // 主机名解析方式
            nodes.push(("macos-host-node".to_string(), "gy.local:50051".to_string()));
            
            // 使用可能的IP地址 - 提高连接成功率
            let possible_mac_ips = [
                "192.168.31.90",
                "192.168.31.91",
                "192.168.1.100",
                "192.168.1.101"
            ];
            
            for (idx, &ip) in possible_mac_ips.iter().enumerate() {
                let node_id = format!("macos-ip-node-{}", idx + 1);
                let address = format!("{}:50051", ip);
                nodes.push((node_id, address));
            }
        }
        
        tracing::debug!("生成的测试节点列表: {:?}", nodes);
        nodes
    }
    
    /// 获取本地网络IP
    fn get_local_network_ip(&self) -> Result<String> {
        // 这个技巧用于获取本地网络接口的IP
        // 连接一个公网IP（但不实际发送数据），系统会选择合适的网络接口
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect("8.8.8.8:80")?;
        let local_addr = socket.local_addr()?;
        let ip = local_addr.ip().to_string();
        
        tracing::debug!("检测到本地网络IP: {}", ip);
        Ok(ip)
    }
} 
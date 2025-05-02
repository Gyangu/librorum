use crate::proto::node::{HeartbeatRequest, HeartbeatResponse};
use crate::proto::node::node_service_client::NodeServiceClient;
use anyhow::{Result, Context, anyhow};
use chrono::Utc;
use tonic::transport::Channel;
use tokio::time::timeout;
use std::time::Duration as StdDuration;
use std::net::{ToSocketAddrs, IpAddr};

// 配置常量
const CONNECT_TIMEOUT_SECS: u64 = 5;     // 连接超时时间
const MAX_RETRY_COUNT: usize = 3;        // 最大重试次数
const RETRY_DELAY_MS: u64 = 1000;        // 重试间隔时间（毫秒）
const DNS_RETRY_COUNT: usize = 3;        // DNS解析重试次数增加到3次，提高成功率

/// 节点客户端
#[derive(Debug)]
pub struct NodeClient {
    node_id: String,
    address: String,
    system_info: String,
}

impl NodeClient {
    /// 创建新的节点客户端
    pub fn new(node_id: String, address: String, system_info: String) -> Self {
        Self {
            node_id,
            address,
            system_info,
        }
    }
    
    /// 连接到远程节点并发送心跳包，带有重试功能
    pub async fn send_heartbeat(&self, remote_addr: &str) -> Result<HeartbeatResponse> {
        // 初始化重试计数器
        let mut retry_count = 0;
        let mut last_error = None;
        
        // 重试循环
        while retry_count < MAX_RETRY_COUNT {
            match self.try_send_heartbeat(remote_addr).await {
                Ok(response) => {
                    // 如果成功，记录重试次数（如果有重试）
                    if retry_count > 0 {
                        tracing::info!("发送心跳成功，在第 {} 次尝试后", retry_count + 1);
                    }
                    return Ok(response);
                },
                Err(err) => {
                    // 记录错误并重试
                    tracing::warn!("尝试发送心跳失败: {}", err);
                    last_error = Some(err);
                    retry_count += 1;
                    
                    if retry_count < MAX_RETRY_COUNT {
                        tracing::debug!(
                            "发送心跳失败，正在重试 ({}/{})...", 
                            retry_count, 
                            MAX_RETRY_COUNT
                        );
                        // 等待一段时间再重试
                        tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }
        
        // 如果所有尝试都失败，返回最后一个错误
        Err(last_error.unwrap_or_else(|| anyhow!("无法发送心跳包到节点 {}", remote_addr)))
    }
    
    /// 尝试单次发送心跳包
    async fn try_send_heartbeat(&self, remote_addr: &str) -> Result<HeartbeatResponse> {
        // 连接到远程节点
        let mut client = self.connect_with_timeout(remote_addr).await?;
            
        // 构造心跳请求
        let request = HeartbeatRequest {
            node_id: self.node_id.clone(),
            address: self.address.clone(),
            system_info: self.system_info.clone(),
            timestamp: Utc::now().timestamp(),
        };
        
        // 发送心跳请求
        let response = client.heartbeat(request)
            .await
            .with_context(|| format!("发送心跳包到节点失败: {}", remote_addr))?;
            
        tracing::debug!("心跳发送成功到节点: {}", remote_addr);
        Ok(response.into_inner())
    }
    
    /// 获取已建立连接的客户端，带超时处理
    async fn connect_with_timeout(&self, remote_addr: &str) -> Result<NodeServiceClient<Channel>> {
        // 获取需要连接的地址
        let mut dns_retry = 0;
        let addrs = self.resolve_addr(remote_addr, dns_retry).await?;
        
        let mut last_error = None;
        
        // 依次尝试各个地址
        for (idx, addr) in addrs.iter().enumerate() {
            let endpoint = format!("http://{}", addr);
            tracing::debug!("尝试连接到节点 [{}/{}]: {}", idx+1, addrs.len(), endpoint);
            
            // 带超时的连接
            let connect_future = NodeServiceClient::connect(endpoint.clone());
            let timeout_duration = StdDuration::from_secs(CONNECT_TIMEOUT_SECS);
            
            match timeout(timeout_duration, connect_future).await {
                Ok(result) => {
                    match result {
                        Ok(client) => {
                            tracing::info!("成功连接到节点: {}", addr);
                            return Ok(client);
                        },
                        Err(err) => {
                            tracing::debug!("连接到节点失败: {} - 错误: {}", addr, err);
                            last_error = Some(anyhow!("连接到节点失败: {} - 错误: {}", addr, err));
                        }
                    }
                },
                Err(_) => {
                    tracing::debug!("连接到节点超时: {}", addr);
                    last_error = Some(anyhow!("连接到节点超时: {}", addr));
                }
            }
            
            // 短暂延迟后尝试下一个地址
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        // 如果DNS解析出的地址都连不上，我们可以尝试增加重试次数
        if dns_retry < DNS_RETRY_COUNT {
            dns_retry += 1;
            tracing::debug!("DNS解析重试 ({}/{})", dns_retry, DNS_RETRY_COUNT);
            
            // 再次解析，可能会得到不同的地址
            let addrs = self.resolve_addr(remote_addr, dns_retry).await?;
            
            for (idx, addr) in addrs.iter().enumerate() {
                let endpoint = format!("http://{}", addr);
                tracing::debug!("重试连接到节点 [{}/{}]: {}", idx+1, addrs.len(), endpoint);
                
                let connect_future = NodeServiceClient::connect(endpoint.clone());
                let timeout_duration = StdDuration::from_secs(CONNECT_TIMEOUT_SECS);
                
                match timeout(timeout_duration, connect_future).await {
                    Ok(result) => {
                        match result {
                            Ok(client) => {
                                tracing::info!("重试成功连接到节点: {}", addr);
                                return Ok(client);
                            },
                            Err(err) => {
                                tracing::debug!("重试连接到节点失败: {} - 错误: {}", addr, err);
                                last_error = Some(anyhow!("连接到节点失败: {} - 错误: {}", addr, err));
                            }
                        }
                    },
                    Err(_) => {
                        tracing::debug!("重试连接到节点超时: {}", addr);
                        last_error = Some(anyhow!("连接到节点超时: {}", addr));
                    }
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
        
        // 如果所有地址都失败，返回最后一个错误
        Err(last_error.unwrap_or_else(|| anyhow!("无法连接到节点: {}", remote_addr)))
    }
    
    /// 解析远程地址
    async fn resolve_addr(&self, remote_addr: &str, _retry: usize) -> Result<Vec<String>> {
        let mut addrs = Vec::new();
        
        // 1. 尝试直接使用地址（支持IP:端口和主机名:端口）
        // 只有不像IPv6地址才直接添加，防止IPv6导致的连接问题
        if !remote_addr.contains('[') {
            addrs.push(remote_addr.to_string());
        }
        
        // 2. 尝试DNS解析
        let clean_addr = if remote_addr.starts_with("http://") || remote_addr.starts_with("https://") {
            remote_addr.replace("http://", "").replace("https://", "")
        } else {
            remote_addr.to_string()
        };
        
        // 尝试解析地址
        match clean_addr.to_socket_addrs() {
            Ok(socket_addrs) => {
                let socket_addrs: Vec<_> = socket_addrs.collect();
                tracing::debug!("DNS解析结果 {} -> {:?}", clean_addr, socket_addrs);
                
                // 优先使用IPv4地址
                for addr in socket_addrs {
                    match addr.ip() {
                        IpAddr::V4(ipv4) => {
                            let resolved = format!("{}:{}", ipv4, addr.port());
                            if !addrs.contains(&resolved) {
                                tracing::debug!("使用IPv4地址: {} -> {}", clean_addr, resolved);
                                addrs.push(resolved);
                            }
                        },
                        IpAddr::V6(_) => {
                            // 跳过IPv6地址，它们可能导致连接问题
                            tracing::debug!("跳过IPv6地址: {}", addr);
                        }
                    }
                }
            },
            Err(e) => {
                tracing::debug!("DNS解析失败 {}: {}", clean_addr, e);
                
                // 3. 尝试分离主机名和端口，手动构建SocketAddr
                if let Some((host, port_str)) = clean_addr.split_once(':') {
                    if let Ok(port) = port_str.parse::<u16>() {
                        // 特殊处理跨平台域名解析
                        if host.ends_with(".local") {
                            tracing::info!("检测到.local域名: {}，尝试平台适配处理", host);
                            
                            // 处理Mac节点连接
                            if host.contains("gy") || host == "gy.local" {
                                // 添加已知可能的IP地址
                                let ips = [
                                    // 局域网 IP 地址
                                    "192.168.31.90", 
                                    "192.168.31.91", 
                                    "192.168.1.100", 
                                    "192.168.1.101",
                                    // 备用地址范围
                                    "192.168.0.100",
                                    "192.168.0.101",
                                    "10.0.0.100",
                                    "10.0.0.101",
                                    "172.16.0.100",
                                    "172.16.0.101",
                                    // VPN可能使用的地址
                                    "172.17.0.100",
                                    "10.8.0.100"
                                ];
                                for &ip in &ips {
                                    let fallback = format!("{}:{}", ip, port);
                                    if !addrs.contains(&fallback) {
                                        tracing::debug!("跨平台适配，尝试Mac IP: {} -> {}", clean_addr, fallback);
                                        addrs.push(fallback);
                                    }
                                }
                            }
                            
                            // 处理Windows节点连接
                            if host.contains("windows") || host == "windows.local" {
                                // 添加已知可能的IP地址
                                let ips = [
                                    // 局域网 IP 地址
                                    "192.168.31.92", 
                                    "192.168.31.93",
                                    "192.168.1.102",
                                    "192.168.1.103",
                                    // 备用地址范围
                                    "192.168.0.102",
                                    "192.168.0.103",
                                    "10.0.0.102",
                                    "10.0.0.103",
                                    "172.16.0.102",
                                    "172.16.0.103",
                                    // VPN可能使用的地址
                                    "172.17.0.102",
                                    "10.8.0.102"
                                ];
                                for &ip in &ips {
                                    let fallback = format!("{}:{}", ip, port);
                                    if !addrs.contains(&fallback) {
                                        tracing::debug!("跨平台适配，尝试Windows IP: {} -> {}", clean_addr, fallback);
                                        addrs.push(fallback);
                                    }
                                }
                            }
                        }
                        
                        // 尝试常见的局域网地址
                        if addrs.len() <= 1 { // 如果只有原始地址
                            // 使用主机名的最后部分作为设备标识符
                            let host_parts: Vec<&str> = host.split('.').collect();
                            let device_id = host_parts.first().unwrap_or(&host);
                            
                            // 尝试常见的局域网IP段
                            let common_networks = [
                                "192.168.31.", "192.168.1.", "192.168.0.", 
                                "10.0.0.", "172.16.0.", "172.17.0."
                            ];
                            
                            for &network in &common_networks {
                                // 根据设备标识符生成可能的IP地址
                                let possible_hosts = if device_id.contains("win") || device_id.contains("windows") {
                                    // Windows设备可能的IP尾数
                                    vec!["102", "103", "104", "105", "112"]
                                } else if device_id.contains("mac") || device_id.contains("gy") {
                                    // Mac设备可能的IP尾数
                                    vec!["100", "101", "110", "111", "90", "91"]
                                } else {
                                    // 通用设备 - 常见IP范围
                                    vec!["100", "101", "102", "103", "104", "105", "110", "111", "112"]
                                };
                                
                                for suffix in &possible_hosts {
                                    let ip = format!("{}{}", network, suffix);
                                    let addr_str = format!("{}:{}", ip, port);
                                    if !addrs.contains(&addr_str) {
                                        tracing::debug!("智能猜测IP: {} -> {}", device_id, addr_str);
                                        addrs.push(addr_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 确保至少有一个地址，如果没有解析到合适的地址，添加原始地址作为兜底
        if addrs.is_empty() {
            tracing::warn!("无法解析地址: {}，使用原始地址作为兜底", remote_addr);
            addrs.push(remote_addr.to_string());
        }
        
        // 对地址进行智能排序，优先尝试更可能成功的地址
        self.sort_addresses(&mut addrs).await;
        
        tracing::debug!("最终解析的地址列表 ({} 个): {:?}", addrs.len(), addrs);
        Ok(addrs)
    }
    
    /// 智能排序地址列表，优先尝试更可能成功的地址
    async fn sort_addresses(&self, addrs: &mut Vec<String>) {
        // 本地系统信息
        let is_windows = self.system_info.to_lowercase().contains("windows");
        let is_macos = self.system_info.to_lowercase().contains("macos");
        
        // 根据系统类型对地址进行排序
        addrs.sort_by(|a, b| {
            let a_score = self.calculate_address_priority(a, is_windows, is_macos);
            let b_score = self.calculate_address_priority(b, is_windows, is_macos);
            // 降序排序 - 分数高的排在前面
            b_score.cmp(&a_score)
        });
    }
    
    /// 计算地址优先级分数
    fn calculate_address_priority(&self, addr: &str, is_windows: bool, is_macos: bool) -> u32 {
        let mut score = 0;
        
        // 本地地址优先级最高
        if addr.starts_with("127.0.0.1:") || addr.starts_with("localhost:") {
            score += 1000;
        }
        
        // 完全匹配的域名优先
        if addr.contains(".local:") {
            score += 800;
        }
        
        // 特定平台优先级
        if is_windows {
            // Windows上优先尝试Mac的地址
            if addr.contains("192.168.31.90:") || addr.contains("192.168.31.91:") {
                score += 500;
            } else if addr.contains("192.168.1.100:") || addr.contains("192.168.1.101:") {
                score += 400;
            } else if addr.contains("gy.local:") {
                score += 300;
            } 
        } else if is_macos {
            // Mac上优先尝试Windows的地址
            if addr.contains("192.168.31.92:") || addr.contains("192.168.31.93:") {
                score += 500;
            } else if addr.contains("192.168.1.102:") || addr.contains("192.168.1.103:") {
                score += 400;
            } else if addr.contains("windows.local:") {
                score += 300;
            }
        }
        
        // 特定网段优先级
        if addr.starts_with("192.168.31.") {
            score += 50;
        } else if addr.starts_with("192.168.1.") {
            score += 40;
        } else if addr.starts_with("192.168.0.") {
            score += 30;
        } else if addr.starts_with("10.0.0.") {
            score += 20;
        } else if addr.starts_with("172.16.0.") || addr.starts_with("172.17.0.") {
            score += 10;
        }
        
        score
    }
} 
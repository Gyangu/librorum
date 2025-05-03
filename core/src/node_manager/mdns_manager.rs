use anyhow::{Context, Result};
use if_addrs::get_if_addrs;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task;
use tracing::{info, warn, error, debug};

/// mDNS服务类型
const SERVICE_TYPE: &str = "_librorum._tcp.local.";

/// mDNS管理器，用于服务发布和发现
pub struct MdnsManager {
    /// 节点ID
    node_id: String,
    /// 服务端口
    port: u16,
    /// mDNS守护进程
    mdns: Option<Arc<Mutex<ServiceDaemon>>>,
    /// 是否正在运行服务发现
    discovery_running: Arc<Mutex<bool>>,
}

impl MdnsManager {
    /// 创建新的mDNS管理器
    pub fn new(node_id: String, port: u16) -> Self {
        Self {
            node_id,
            port,
            mdns: None,
            discovery_running: Arc::new(Mutex::new(false)),
        }
    }

    /// 获取本机第一个非回环的IPv4地址
    fn get_local_ipv4(&self) -> Option<Ipv4Addr> {
        get_if_addrs().ok().and_then(|interfaces| {
            interfaces
                .iter()
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

    /// 注册mDNS服务
    pub fn register(&self) -> Result<()> {
        // 获取主机名
        let host_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        // 获取本机IP地址
        let ip = self.get_local_ipv4().unwrap_or_else(|| {
            warn!("警告: 无法获取本机IP地址，使用127.0.0.1");
            Ipv4Addr::new(127, 0, 0, 1)
        });

        // 创建mDNS守护进程
        let mdns = ServiceDaemon::new().with_context(|| "创建mDNS守护进程失败")?;

        // 服务属性
        let full_host_name = format!("{}.local.", host_name);
        let properties = [
            ("node_id", self.node_id.clone()),
            ("version", env!("CARGO_PKG_VERSION").to_string()),
            ("system", self.get_system_info()),
        ];

        info!("注册mDNS服务 '{}' 在端口 {}", self.node_id, self.port);
        debug!("主机名: {}", full_host_name);
        debug!("IP地址: {}", ip);

        // 创建服务信息
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &self.node_id,
            &full_host_name,
            &ip.to_string(),
            self.port,
            &properties[..],
        )
        .with_context(|| "创建服务信息失败")?;

        // 注册服务
        mdns.register(service_info)
            .with_context(|| "注册mDNS服务失败")?;

        // 保存mDNS守护进程实例
        let mdns_arc = Arc::new(Mutex::new(mdns));
        let self_mdns = self.mdns.clone();
        match self_mdns {
            Some(existing_mdns) => {
                // 如果已存在，替换它
                let mut locked = existing_mdns.lock().unwrap();
                *locked = mdns_arc.lock().unwrap().clone();
            }
            None => {
                // 第一次初始化
                unsafe {
                    let self_mut = self as *const Self as *mut Self;
                    (*self_mut).mdns = Some(mdns_arc);
                }
            }
        }

        info!("mDNS服务注册成功");
        Ok(())
    }

    /// 启动服务发现
    pub async fn start_discovery(
        &self,
        discovered_callback: impl Fn(String, String, u16) + Send + Sync + 'static,
        removed_callback: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<()> {
        // 检查是否已经在运行
        {
            let mut running = self.discovery_running.lock().unwrap();
            if *running {
                info!("mDNS服务发现已经在运行中");
                return Ok(());
            }
            *running = true;
        }

        // 创建mDNS守护进程
        let mdns = ServiceDaemon::new().with_context(|| "创建mDNS服务发现守护进程失败")?;

        // 浏览服务
        let receiver = mdns
            .browse(SERVICE_TYPE)
            .with_context(|| "浏览mDNS服务失败")?;

        info!("开始服务发现: {}", SERVICE_TYPE);

        // 打印当前网络接口信息，帮助调试
        if let Ok(interfaces) = get_if_addrs() {
            info!("当前网络接口信息:");
            for interface in interfaces {
                info!("  接口: {}, 地址: {:?}", interface.name, interface.addr);
            }
        }

        // 不使用通道，直接在处理事件时调用回调函数
        let discovery_running = self.discovery_running.clone();
        let own_node_id = self.node_id.clone();
        let discovered_cb = Arc::new(discovered_callback);
        let removed_cb = Arc::new(removed_callback);
        
        task::spawn(async move {
            info!("启动mDNS事件监听任务");
            
            while *discovery_running.lock().unwrap() {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(event) => {
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                let fullname = info.get_fullname().to_string();
                                let hostname = info.get_hostname().to_string();
                                let port = info.get_port();

                                // 获取节点ID
                                let node_id = match info.get_property_val_str("node_id") {
                                    Some(id) => id.to_string(),
                                    None => fullname.clone(),
                                };

                                // 过滤掉当前节点自身
                                if node_id == own_node_id {
                                    debug!("忽略自身节点: {}", node_id);
                                    continue;
                                }

                                // 获取IP地址 - 只处理IPv4地址
                                if let Some(addr) = info.get_addresses().iter().find(|addr| {
                                    // 检查地址是否为IPv4格式（不包含冒号）
                                    !addr.to_string().contains(':')
                                }) {
                                    let ip_str = addr.to_string();
                                    let service_addr = format!("{}:{}", ip_str, port);

                                    // 打印所有可用地址，便于调试
                                    let all_addresses: Vec<String> = info.get_addresses().iter()
                                        .map(|a| a.to_string())
                                        .collect();
                                    debug!(
                                        "发现节点: {} ({} - {}) 全部地址: {:?}",
                                        node_id,
                                        service_addr,
                                        hostname,
                                        all_addresses
                                    );

                                    let discovery_cb_clone = discovered_cb.clone();
                                    let node_id_clone = node_id.clone();
                                    let addr_clone = service_addr.clone();
                                    
                                    // 捕获并记录回调执行过程中的任何panic
                                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        discovery_cb_clone(node_id_clone, addr_clone, port);
                                    }));
                                    
                                    if result.is_err() {
                                        error!("发现回调执行过程中发生panic: 节点ID={}, 地址={}", node_id, service_addr);
                                    }
                                } else {
                                    debug!("节点 {} 无可用的IPv4地址，忽略", node_id);
                                }
                            }
                            ServiceEvent::ServiceRemoved(_, fullname) => {
                                let node_id = fullname.to_string();
                                info!("节点离线: {}", node_id);
                                
                                // 直接调用移除回调
                                info!("直接调用移除回调: 节点ID={}", node_id);
                                let remove_cb_clone = removed_cb.clone();
                                let node_id_clone = node_id.clone();
                                
                                // 捕获并记录回调执行过程中的任何panic
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    remove_cb_clone(node_id_clone);
                                }));
                                
                                if result.is_err() {
                                    error!("移除回调执行过程中发生panic: 节点ID={}", node_id);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {
                        // 超时继续
                        continue;
                    }
                }
            }

            info!("mDNS事件监听任务结束");
            
            // 关闭mDNS守护进程
            if let Err(e) = mdns.shutdown() {
                error!("关闭mDNS服务发现失败: {}", e);
            }
            
            info!("mDNS服务发现已完全停止");
        });

        Ok(())
    }

    /// 获取系统信息
    fn get_system_info(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            "Windows".to_string()
        }

        #[cfg(target_os = "macos")]
        {
            "macOS".to_string()
        }

        #[cfg(target_os = "linux")]
        {
            "Linux".to_string()
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            "Unknown".to_string()
        }
    }
}
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::Duration;
use tracing::{debug, info, warn};

/// 网络配置，用于配置节点网络行为
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// 绑定IP地址
    #[serde(default = "default_bind_ip")]
    pub bind_ip: String,

    /// 绑定端口
    #[serde(default = "default_bind_port")]
    pub bind_port: u16,

    /// 心跳间隔时间(秒)
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,

    /// 心跳超时时间(秒)
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_secs: i64,

    /// 连接超时时间(秒)
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// 最大重试次数
    #[serde(default = "default_max_retry_count")]
    pub max_retry_count: usize,

    /// 重试间隔时间(毫秒)
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,

    /// 是否自动发现节点
    #[serde(default = "default_auto_discovery")]
    pub auto_discovery: bool,

    /// 是否禁用IPv6
    #[serde(default = "default_disable_ipv6")]
    pub disable_ipv6: bool,

    /// 静态节点列表
    #[serde(default)]
    pub static_nodes: Vec<String>,
}

// 默认值函数
fn default_bind_ip() -> String {
    "0.0.0.0".to_string()
}

fn default_bind_port() -> u16 {
    50051
}

fn default_heartbeat_interval() -> u64 {
    30
}

fn default_heartbeat_timeout() -> i64 {
    60
}

fn default_connection_timeout() -> u64 {
    5
}

fn default_max_retry_count() -> usize {
    3
}

fn default_retry_delay() -> u64 {
    1000
}

fn default_auto_discovery() -> bool {
    true
}

fn default_disable_ipv6() -> bool {
    true
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_ip: default_bind_ip(),
            bind_port: default_bind_port(),
            heartbeat_interval_secs: default_heartbeat_interval(),
            heartbeat_timeout_secs: default_heartbeat_timeout(),
            connection_timeout_secs: default_connection_timeout(),
            max_retry_count: default_max_retry_count(),
            retry_delay_ms: default_retry_delay(),
            auto_discovery: default_auto_discovery(),
            disable_ipv6: default_disable_ipv6(),
            static_nodes: Vec::new(),
        }
    }
}

impl NetworkConfig {
    /// 创建新的网络配置
    pub fn new() -> Self {
        Default::default()
    }

    /// 设置绑定IP地址
    pub fn with_bind_ip(mut self, ip: &str) -> Self {
        self.bind_ip = ip.to_string();
        self
    }

    /// 设置绑定端口
    pub fn with_bind_port(mut self, port: u16) -> Self {
        self.bind_port = port;
        self
    }

    /// 设置心跳间隔时间
    pub fn with_heartbeat_interval(mut self, seconds: u64) -> Self {
        self.heartbeat_interval_secs = seconds;
        self
    }

    /// 设置心跳超时时间
    pub fn with_heartbeat_timeout(mut self, seconds: i64) -> Self {
        self.heartbeat_timeout_secs = seconds;
        self
    }

    /// 设置连接超时时间
    pub fn with_connection_timeout(mut self, seconds: u64) -> Self {
        self.connection_timeout_secs = seconds;
        self
    }

    /// 设置最大重试次数
    pub fn with_max_retry_count(mut self, count: usize) -> Self {
        self.max_retry_count = count;
        self
    }

    /// 设置重试间隔时间
    pub fn with_retry_delay(mut self, milliseconds: u64) -> Self {
        self.retry_delay_ms = milliseconds;
        self
    }

    /// 设置是否自动发现节点
    pub fn with_auto_discovery(mut self, auto_discovery: bool) -> Self {
        self.auto_discovery = auto_discovery;
        self
    }

    /// 设置是否禁用IPv6
    pub fn with_disable_ipv6(mut self, disable_ipv6: bool) -> Self {
        self.disable_ipv6 = disable_ipv6;
        self
    }

    /// 添加静态节点
    pub fn add_static_node(mut self, address: &str) -> Self {
        self.static_nodes.push(address.to_string());
        self
    }

    /// 获取心跳间隔时间
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval_secs)
    }

    /// 获取心跳超时时间
    pub fn heartbeat_timeout(&self) -> Duration {
        Duration::from_secs(self.heartbeat_timeout_secs as u64)
    }

    /// 获取连接超时时间
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// 获取重试间隔时间
    pub fn retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry_delay_ms)
    }

    /// 获取绑定地址
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_ip, self.bind_port)
    }

    /// 验证配置的合法性
    pub fn validate(&self) -> Result<()> {
        // 验证IP地址格式
        self.bind_ip
            .parse::<IpAddr>()
            .with_context(|| format!("无效的绑定IP地址: {}", self.bind_ip))?;

        // 验证端口范围
        if self.bind_port == 0 {
            warn!("绑定端口为0，将使用系统分配的随机端口");
        }

        // 验证心跳参数
        if self.heartbeat_interval_secs == 0 {
            warn!("心跳间隔为0，将禁用心跳机制");
        }

        if self.heartbeat_timeout_secs <= 0 {
            warn!("心跳超时时间必须大于0，使用默认值60秒");
        }

        // 验证连接参数
        if self.connection_timeout_secs == 0 {
            warn!("连接超时时间为0，可能导致连接无法完成");
        }

        // 验证重试参数
        if self.max_retry_count == 0 {
            warn!("最大重试次数为0，将不会重试失败的连接");
        }

        // 验证静态节点格式
        for node in &self.static_nodes {
            if !node.contains(':') {
                warn!("静态节点地址 '{}' 可能缺少端口号", node);
            }
        }

        Ok(())
    }

    /// 从文件加载配置
    pub fn from_file(path: &str) -> Result<Self> {
        let config_str =
            std::fs::read_to_string(path).with_context(|| format!("无法读取配置文件: {}", path))?;

        let config: Self = serde_json::from_str(&config_str)
            .with_context(|| format!("无法解析配置文件: {}", path))?;

        config.validate()?;

        info!("从文件加载网络配置: {}", path);
        debug!("配置详情: {:?}", config);

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let config_str = serde_json::to_string_pretty(self).with_context(|| "无法序列化配置")?;

        std::fs::write(path, config_str).with_context(|| format!("无法写入配置文件: {}", path))?;

        info!("网络配置已保存到文件: {}", path);
        Ok(())
    }
}

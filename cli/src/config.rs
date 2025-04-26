use anyhow::Result;
use librorum_core::config::{ClusterConfig, NodeConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

/// CLI 配置
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CliConfig {
    /// 默认节点 ID
    pub default_node: Option<String>,
    /// 已知节点映射
    pub log_level: Option<String>,
    /// 输出格式
    pub output_format: Option<String>,
    pub nodes: Vec<NodeAddress>,
}

/// 节点地址信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeAddress {
    pub id: String,
    pub host: String,
    pub port: u16,
}

impl CliConfig {
    /// 获取配置文件路径
    pub fn get_config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".librorum").join("config.toml")
    }

    /// 加载配置文件
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path();
        
        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        
        let contents = fs::read_to_string(config_path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    /// 保存配置文件
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        let toml = toml::to_string_pretty(self)?;
        fs::write(config_path, toml)?;
        Ok(())
    }

    /// 获取节点地址
    pub fn get_node_address(&self, node_id: &str) -> Option<&NodeAddress> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// 添加或更新节点
    pub fn set_node(&mut self, id: String, host: String, port: u16) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.host = host;
            node.port = port;
        } else {
            self.nodes.push(NodeAddress { id, host, port });
        }
    }

    /// 获取默认节点
    pub fn get_default_node(&self) -> Option<&str> {
        self.default_node.as_deref()
    }

    /// 设置默认节点
    pub fn set_default_node(&mut self, node_id: String) {
        self.default_node = Some(node_id);
    }

    /// 设置日志级别
    pub fn set_log_level(&mut self, level: String) {
        self.log_level = Some(level);
    }

    /// 设置输出格式
    pub fn set_output_format(&mut self, format: String) {
        self.output_format = Some(format);
    }
}

/// 加载节点配置文件
pub fn load_node_config(path: &Path) -> Result<NodeConfig> {
    let config_str = fs::read_to_string(path)?;
    let config = toml::from_str(&config_str)?;
    Ok(config)
}

/// 加载集群配置文件
pub fn load_cluster_config(path: &Path) -> Result<ClusterConfig> {
    let config_str = fs::read_to_string(path)?;
    let config = toml::from_str(&config_str)?;
    Ok(config)
}

/// 全局 CLI 配置单例
pub fn get_cli_config() -> Result<CliConfig> {
    CliConfig::load()
}

/// 保存全局 CLI 配置
pub fn save_cli_config(config: &CliConfig) -> Result<()> {
    config.save()
} 
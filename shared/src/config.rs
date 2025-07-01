use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// 节点标识前缀，会与设备名结合生成节点ID
    #[serde(default = "default_node_prefix")]
    pub node_prefix: String,

    /// 节点监听地址，默认为 0.0.0.0
    #[serde(default = "default_bind_host")]
    pub bind_host: String,

    /// 节点监听端口，默认为 50051
    #[serde(default = "default_bind_port")]
    pub bind_port: u16,

    /// 日志级别
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// 数据目录
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// 心跳间隔，单位为秒，默认为 5 秒
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,

    /// 发现间隔，单位为秒，默认为 10 秒
    #[serde(default = "default_discovery_interval")]
    pub discovery_interval: u64,

    /// 已知节点列表
    #[serde(default = "default_known_nodes")]
    pub known_nodes: Vec<String>,
}

// 默认配置值函数
fn default_node_prefix() -> String {
    "node".to_string()
}

fn default_bind_host() -> String {
    "0.0.0.0".to_string()
}

fn default_bind_port() -> u16 {
    50051
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_data_dir() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("librorum")
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("librorum")
    }
}

fn default_heartbeat_interval() -> u64 {
    5
}

fn default_discovery_interval() -> u64 {
    10
}

fn default_known_nodes() -> Vec<String> {
    Vec::new()
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_prefix: default_node_prefix(),
            bind_host: default_bind_host(),
            bind_port: default_bind_port(),
            log_level: default_log_level(),
            data_dir: default_data_dir(),
            heartbeat_interval: default_heartbeat_interval(),
            discovery_interval: default_discovery_interval(),
            known_nodes: default_known_nodes(),
        }
    }
}

impl NodeConfig {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("无法读取配置文件: {:?}", path.as_ref()))?;

        let config: NodeConfig = toml::from_str(&content)
            .with_context(|| format!("无法解析配置文件: {:?}", path.as_ref()))?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("无法创建目录: {:?}", parent))?;
        }

        // 序列化并保存
        let content = toml::to_string_pretty(self).with_context(|| "无法序列化配置")?;

        std::fs::write(&path, content)
            .with_context(|| format!("无法写入配置文件: {:?}", path.as_ref()))?;

        Ok(())
    }

    /// 获取绑定地址
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.bind_host, self.bind_port)
    }

    /// 创建数据目录
    pub fn create_data_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)
            .with_context(|| format!("无法创建数据目录: {:?}", self.data_dir))?;
        Ok(())
    }

    /// 尝试查找配置文件
    pub fn find_config_file() -> Option<PathBuf> {
        // 1. 当前目录下的librorum.toml
        let current_dir = Path::new("librorum.toml");
        if current_dir.exists() {
            info!("使用自动检测的配置文件: {}", current_dir.display());
            return Some(current_dir.to_path_buf());
        }

        // 2. 当前目录下的windows专用配置
        #[cfg(windows)]
        {
            let windows_config = Path::new("librorum-windows.toml");
            if windows_config.exists() {
                info!("使用Windows专用配置文件: {}", windows_config.display());
                return Some(windows_config.to_path_buf());
            }
        }

        // 3. 当前目录下的mac专用配置
        #[cfg(target_os = "macos")]
        {
            let mac_config = Path::new("librorum-mac.toml");
            if mac_config.exists() {
                info!("使用macOS专用配置文件: {}", mac_config.display());
                return Some(mac_config.to_path_buf());
            }
        }

        // 4. 用户配置目录
        if let Some(config_dir) = dirs::config_dir() {
            let user_config = config_dir.join("librorum").join("config.toml");
            if user_config.exists() {
                info!("使用用户配置目录配置文件: {}", user_config.display());
                return Some(user_config);
            }
        }

        // 5. 系统配置目录
        #[cfg(not(windows))]
        {
            let system_config = Path::new("/etc/librorum/config.toml");
            if system_config.exists() {
                info!("使用系统配置目录配置文件: {}", system_config.display());
                return Some(system_config.to_path_buf());
            }
        }

        #[cfg(windows)]
        {
            // Windows系统目录
            let system_config = Path::new("C:\\ProgramData\\librorum\\config.toml");
            if system_config.exists() {
                info!("使用Windows系统配置目录配置文件: {}", system_config.display());
                return Some(system_config.to_path_buf());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.node_prefix, "node");
        assert_eq!(config.bind_host, "0.0.0.0");
        assert_eq!(config.bind_port, 50051);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.heartbeat_interval, 5);
        assert_eq!(config.discovery_interval, 10);
        assert!(config.known_nodes.is_empty());
    }

    #[test]
    fn test_bind_address() {
        let config = NodeConfig {
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8080,
            ..Default::default()
        };
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
    }

    #[test]
    fn test_save_and_load_config() -> anyhow::Result<()> {
        let config = NodeConfig {
            node_prefix: "test_node".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 9999,
            log_level: "debug".to_string(),
            heartbeat_interval: 3,
            discovery_interval: 15,
            known_nodes: vec!["node1.local".to_string(), "node2.local".to_string()],
            ..Default::default()
        };

        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_path_buf();

        config.save_to_file(&temp_path)?;

        let loaded_config = NodeConfig::from_file(&temp_path)?;

        assert_eq!(loaded_config.node_prefix, config.node_prefix);
        assert_eq!(loaded_config.bind_host, config.bind_host);
        assert_eq!(loaded_config.bind_port, config.bind_port);
        assert_eq!(loaded_config.log_level, config.log_level);
        assert_eq!(loaded_config.heartbeat_interval, config.heartbeat_interval);
        assert_eq!(loaded_config.discovery_interval, config.discovery_interval);
        assert_eq!(loaded_config.known_nodes, config.known_nodes);

        Ok(())
    }

    #[test]
    fn test_load_invalid_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml content [[[").unwrap();
        let temp_path = temp_file.path();

        let result = NodeConfig::from_file(temp_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_config() {
        let result = NodeConfig::from_file("/nonexistent/file.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_partial_config() -> anyhow::Result<()> {
        let toml_content = r#"
            node_prefix = "custom_node"
            bind_port = 8888
        "#;

        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "{}", toml_content)?;
        let temp_path = temp_file.path();

        let config = NodeConfig::from_file(temp_path)?;

        assert_eq!(config.node_prefix, "custom_node");
        assert_eq!(config.bind_host, "0.0.0.0"); // 应使用默认值
        assert_eq!(config.bind_port, 8888);
        assert_eq!(config.log_level, "info"); // 应使用默认值

        Ok(())
    }

    #[test]
    fn test_create_data_dir() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let data_path = temp_dir.path().join("test_data");
        
        let config = NodeConfig {
            data_dir: data_path.clone(),
            ..Default::default()
        };

        config.create_data_dir()?;
        assert!(data_path.exists());
        assert!(data_path.is_dir());

        Ok(())
    }

    #[test]
    fn test_find_config_file_nonexistent() {
        // 在临时目录中测试，确保没有config文件
        let temp_dir = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        
        std::env::set_current_dir(temp_dir.path()).unwrap();
        let result = NodeConfig::find_config_file();
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_none());
    }

    #[test]
    fn test_find_config_file_exists() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let config_path = temp_dir.path().join("librorum.toml");
        
        // 创建一个测试配置文件
        let config = NodeConfig::default();
        config.save_to_file(&config_path)?;
        
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(temp_dir.path())?;
        
        let result = NodeConfig::find_config_file();
        std::env::set_current_dir(original_dir)?;
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().file_name().unwrap(), "librorum.toml");

        Ok(())
    }

    #[test]
    fn test_config_serialization_roundtrip() -> anyhow::Result<()> {
        let original_config = NodeConfig {
            node_prefix: "roundtrip_test".to_string(),
            bind_host: "192.168.1.100".to_string(),
            bind_port: 12345,
            log_level: "trace".to_string(),
            heartbeat_interval: 1,
            discovery_interval: 2,
            known_nodes: vec![
                "node1.example.com".to_string(),
                "node2.example.com".to_string(),
                "192.168.1.50:50051".to_string(),
            ],
            data_dir: PathBuf::from("/custom/data/path"),
        };

        // 序列化到字符串
        let toml_string = toml::to_string(&original_config)?;
        
        // 从字符串反序列化
        let deserialized_config: NodeConfig = toml::from_str(&toml_string)?;
        
        // 验证所有字段都正确
        assert_eq!(original_config.node_prefix, deserialized_config.node_prefix);
        assert_eq!(original_config.bind_host, deserialized_config.bind_host);
        assert_eq!(original_config.bind_port, deserialized_config.bind_port);
        assert_eq!(original_config.log_level, deserialized_config.log_level);
        assert_eq!(original_config.heartbeat_interval, deserialized_config.heartbeat_interval);
        assert_eq!(original_config.discovery_interval, deserialized_config.discovery_interval);
        assert_eq!(original_config.known_nodes, deserialized_config.known_nodes);
        assert_eq!(original_config.data_dir, deserialized_config.data_dir);

        Ok(())
    }
}
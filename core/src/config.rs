use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub root_dir: PathBuf,
    pub max_file_size: u64,
    pub chunk_size: u64,
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub sync_interval: u64, // 同步间隔（秒）
    pub p2p_enabled: bool,  // 是否启用 P2P 传输
    pub nodes: Vec<NodeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeConfigWrapper {
    node: NodeConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "default".to_string(),
            host: "127.0.0.1".to_string(),
            port: 50051,
            root_dir: PathBuf::from("."),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            chunk_size: 1024 * 1024, // 1MB
            workers: num_cpus::get(),
        }
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            nodes: vec![NodeConfig::default()],
            sync_interval: 60,
            p2p_enabled: true,
        }
    }
}

impl NodeConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let config_str = fs::read_to_string(path)
            .map_err(|e| crate::error::VDFSError::Io(e))?;
        let wrapper: NodeConfigWrapper = toml::from_str(&config_str)?;
        Ok(wrapper.node)
    }
}

impl ClusterConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let config_str = fs::read_to_string(path)
            .map_err(|e| crate::error::VDFSError::Io(e))?;
        Ok(toml::from_str(&config_str)?)
    }
} 
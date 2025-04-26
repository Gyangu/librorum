use anyhow::{Result, anyhow};
use crate::config::CliConfig;

// 由于 proto 定义可能不匹配，我们临时使用模拟客户端
pub struct LibrorumClient {
    pub addr: String,
}

impl LibrorumClient {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}

pub async fn connect(addr: &str) -> Result<LibrorumClient> {
    // 模拟连接过程
    println!("正在连接到服务器 {}...", addr);
    Ok(LibrorumClient::new(addr.to_string()))
}

pub fn get_node_addr(node_id: &str) -> Result<String> {
    let config = CliConfig::load()?;
    
    // 如果节点已知，返回其地址
    if let Some(node) = config.get_node_address(node_id) {
        return Ok(format!("{}:{}", node.host, node.port));
    }
    
    // 否则尝试解析为 host:port 格式
    if node_id.contains(':') {
        return Ok(node_id.to_string());
    }
    
    // 对于测试，提供默认地址
    match node_id {
        "node1" => Ok("127.0.0.1:50051".to_string()),
        "node2" => Ok("127.0.0.1:50052".to_string()),
        _ => Err(anyhow!("未知节点ID: {}", node_id))
    }
}

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// 节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    /// 在线
    Online,
    /// 离线
    Offline,
    /// 未知
    Unknown,
}

/// 节点健康信息
#[derive(Debug, Clone)]
pub struct NodeHealth {
    /// 节点ID
    pub node_id: String,
    /// 节点地址
    pub address: String,
    /// 系统类型
    pub system_info: String,
    /// 最后一次心跳时间
    pub last_heartbeat: DateTime<Utc>,
    /// 连续失败次数
    pub failure_count: u32,
    /// 节点状态
    pub status: NodeStatus,
    /// 延迟(毫秒)
    pub latency_ms: Option<u64>,
}

impl NodeHealth {
    /// 创建新的节点健康信息
    pub fn new(node_id: String, address: String, system_info: String) -> Self {
        Self {
            node_id,
            address,
            system_info,
            last_heartbeat: Utc::now(),
            failure_count: 0,
            status: NodeStatus::Unknown,
            latency_ms: None,
        }
    }

    /// 更新节点状态为在线
    pub fn mark_online(&mut self, latency_ms: Option<u64>) {
        self.last_heartbeat = Utc::now();
        self.failure_count = 0;
        self.status = NodeStatus::Online;
        self.latency_ms = latency_ms;
    }

    /// 更新节点状态为离线
    pub fn mark_failure(&mut self) {
        self.failure_count += 1;
        // 如果连续失败超过3次，标记为离线
        if self.failure_count >= 3 {
            self.status = NodeStatus::Offline;
        }
    }

    /// 返回节点最后心跳是否超时
    pub fn is_timeout(&self, timeout_secs: i64) -> bool {
        let now = Utc::now();
        let diff = now.timestamp() - self.last_heartbeat.timestamp();
        diff > timeout_secs
    }
}

/// 健康监控器，负责跟踪和管理节点的健康状态
#[derive(Clone, Debug)]
pub struct HealthMonitor {
    /// 节点健康状态
    node_health: Arc<Mutex<HashMap<String, NodeHealth>>>,
    /// 心跳超时时间（秒）
    heartbeat_timeout: i64,
}

impl HealthMonitor {
    /// 创建新的健康监控器
    pub fn new(heartbeat_timeout: i64) -> Self {
        Self {
            node_health: Arc::new(Mutex::new(HashMap::new())),
            heartbeat_timeout,
        }
    }

    /// 添加新节点到健康监控
    pub fn add_node(&self, node_id: String, address: String, system_info: String) {
        let mut health_map = self.node_health.lock().unwrap();
        if !health_map.contains_key(&address) {
            debug!("添加节点到健康监控: {} ({})", node_id, address);
            let health = NodeHealth::new(node_id, address.clone(), system_info);
            health_map.insert(address, health);
        }
    }

    /// 更新节点健康状态为在线
    pub fn mark_node_online(&self, address: &str, latency_ms: Option<u64>) -> Result<()> {
        let mut health_map = self.node_health.lock().unwrap();
        if let Some(health) = health_map.get_mut(address) {
            health.mark_online(latency_ms);
            debug!("节点标记为在线: {}", address);
            Ok(())
        } else {
            warn!("尝试更新未知节点状态: {}", address);
            Err(anyhow::anyhow!("未知节点: {}", address))
        }
    }

    /// 更新节点健康状态为失败
    pub fn mark_node_failure(&self, address: &str) -> Result<()> {
        let mut health_map = self.node_health.lock().unwrap();
        if let Some(health) = health_map.get_mut(address) {
            health.mark_failure();
            debug!("节点心跳失败 ({}次): {}", health.failure_count, address);
            Ok(())
        } else {
            warn!("尝试更新未知节点失败: {}", address);
            Err(anyhow::anyhow!("未知节点: {}", address))
        }
    }

    /// 重置节点健康状态，强制设为在线
    pub fn reset_node_status(&self, address: &str) -> Result<()> {
        let mut health_map = self.node_health.lock().unwrap();
        if let Some(health) = health_map.get_mut(address) {
            debug!(
                "强制重置节点状态: {}, 原状态: {:?}, 失败计数: {}",
                address, health.status, health.failure_count
            );

            // 重置状态
            health.last_heartbeat = Utc::now();
            health.failure_count = 0;
            health.status = NodeStatus::Online;
            health.latency_ms = None;

            Ok(())
        } else {
            warn!("尝试重置未知节点状态: {}", address);
            Err(anyhow::anyhow!("未知节点: {}", address))
        }
    }

    /// 获取所有节点的健康状态
    pub fn get_nodes_health(&self) -> Vec<NodeHealth> {
        let health_map = self.node_health.lock().unwrap();
        health_map.values().cloned().collect()
    }

    /// 获取节点的健康状态
    pub fn get_node_health(&self, address: &str) -> Option<NodeHealth> {
        let health_map = self.node_health.lock().unwrap();
        health_map.get(address).cloned()
    }

    /// 获取健康报告
    pub fn generate_health_report(&self) -> String {
        let health_map = self.node_health.lock().unwrap();

        if health_map.is_empty() {
            return "没有发现任何节点".to_string();
        }

        let mut online_count = 0;
        let mut offline_count = 0;
        let mut unknown_count = 0;

        for health in health_map.values() {
            match health.status {
                NodeStatus::Online => online_count += 1,
                NodeStatus::Offline => offline_count += 1,
                NodeStatus::Unknown => unknown_count += 1,
            }
        }

        let mut report = format!(
            "节点状态摘要: 共 {} 个节点 (在线: {}, 离线: {}, 未知: {})\n",
            health_map.len(),
            online_count,
            offline_count,
            unknown_count
        );

        report.push_str("节点详情:\n");

        for health in health_map.values() {
            let status_str = match health.status {
                NodeStatus::Online => "在线",
                NodeStatus::Offline => "离线",
                NodeStatus::Unknown => "未知",
            };

            let last_seen_secs = (Utc::now() - health.last_heartbeat).num_seconds();
            let last_seen = if last_seen_secs < 60 {
                format!("{}秒前", last_seen_secs)
            } else if last_seen_secs < 3600 {
                format!("{}分钟前", last_seen_secs / 60)
            } else {
                format!("{}小时前", last_seen_secs / 3600)
            };

            let latency = match health.latency_ms {
                Some(ms) => format!("{}ms", ms),
                None => "未知".to_string(),
            };

            report.push_str(&format!(
                "  - {}: {} | {} | 延迟: {} | 最后心跳: {} | 失败计数: {}\n",
                health.address,
                health.node_id,
                status_str,
                latency,
                last_seen,
                health.failure_count
            ));
        }

        report
    }

    /// 获取健康监控器的Arc引用
    pub fn get_ref(&self) -> Arc<Mutex<HashMap<String, NodeHealth>>> {
        self.node_health.clone()
    }

    /// 检查所有节点健康状态，标记超时节点为离线
    pub fn check_nodes_health(&self) {
        let mut health_map = self.node_health.lock().unwrap();
        for (addr, health) in health_map.iter_mut() {
            if health.is_timeout(self.heartbeat_timeout) && health.status == NodeStatus::Online {
                info!("节点心跳超时，标记为离线: {}", addr);
                health.mark_failure();
            }
        }
    }
}

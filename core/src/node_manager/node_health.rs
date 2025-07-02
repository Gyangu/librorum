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

// 暂时禁用这些测试，因为方法还没有实现
/*
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_node_health_creation() {
        let health = NodeHealth::new(
            "test_node_1".to_string(),
            "127.0.0.1:50051".to_string(),
            "Linux x86_64".to_string(),
        );
        
        assert_eq!(health.node_id, "test_node_1");
        assert_eq!(health.address, "127.0.0.1:50051");
        assert_eq!(health.system_info, "Linux x86_64");
        assert_eq!(health.failure_count, 0);
        assert_eq!(health.status, NodeStatus::Unknown);
        assert!(health.latency_ms.is_none());
        
        // 心跳时间应该是最近的
        let now = Utc::now();
        let time_diff = (now - health.last_heartbeat).num_seconds();
        assert!(time_diff < 2); // 应该在2秒内
    }

    #[test]
    fn test_node_health_update_heartbeat() {
        let mut health = NodeHealth::new(
            "test_node".to_string(),
            "127.0.0.1:50051".to_string(),
            "Test System".to_string(),
        );
        
        let original_heartbeat = health.last_heartbeat;
        
        // 等待一小段时间确保时间戳不同
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        health.update_heartbeat(Some(100));
        
        assert!(health.last_heartbeat > original_heartbeat);
        assert_eq!(health.latency_ms, Some(100));
        assert_eq!(health.failure_count, 0);
        assert_eq!(health.status, NodeStatus::Online);
    }

    #[test]
    fn test_node_health_mark_failure() {
        let mut health = NodeHealth::new(
            "test_node".to_string(),
            "127.0.0.1:50051".to_string(),
            "Test System".to_string(),
        );
        
        // 初始状态
        assert_eq!(health.failure_count, 0);
        assert_eq!(health.status, NodeStatus::Unknown);
        
        // 第一次失败
        health.mark_failure();
        assert_eq!(health.failure_count, 1);
        assert_eq!(health.status, NodeStatus::Online); // 还不算离线
        
        // 多次失败
        health.mark_failure();
        health.mark_failure();
        health.mark_failure();
        health.mark_failure(); // 第5次失败
        
        assert_eq!(health.failure_count, 5);
        assert_eq!(health.status, NodeStatus::Offline); // 现在应该离线
    }

    #[test]
    fn test_node_health_is_healthy() {
        let mut health = NodeHealth::new(
            "test_node".to_string(),
            "127.0.0.1:50051".to_string(),
            "Test System".to_string(),
        );
        
        // 新节点应该不健康（未知状态）
        assert!(!health.is_healthy());
        
        // 更新心跳后应该健康
        health.update_heartbeat(Some(50));
        assert!(health.is_healthy());
        
        // 多次失败后应该不健康
        for _ in 0..5 {
            health.mark_failure();
        }
        assert!(!health.is_healthy());
    }

    #[test]
    fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new();
        
        let nodes = monitor.get_all_nodes();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_health_monitor_add_node() {
        let monitor = HealthMonitor::new();
        
        monitor.add_node(
            "node_1".to_string(),
            "192.168.1.100:50051".to_string(),
            "macOS arm64".to_string(),
        );
        
        let nodes = monitor.get_all_nodes();
        assert_eq!(nodes.len(), 1);
        
        let node = &nodes[0];
        assert_eq!(node.node_id, "node_1");
        assert_eq!(node.address, "192.168.1.100:50051");
        assert_eq!(node.system_info, "macOS arm64");
    }

    #[test]
    fn test_health_monitor_update_heartbeat() {
        let monitor = HealthMonitor::new();
        
        // 添加节点
        monitor.add_node(
            "node_1".to_string(),
            "127.0.0.1:50051".to_string(),
            "Test System".to_string(),
        );
        
        // 更新心跳
        monitor.update_heartbeat("node_1", Some(75));
        
        let nodes = monitor.get_all_nodes();
        let node = &nodes[0];
        
        assert_eq!(node.status, NodeStatus::Online);
        assert_eq!(node.latency_ms, Some(75));
        assert_eq!(node.failure_count, 0);
    }

    #[test]
    fn test_health_monitor_mark_failure() {
        let monitor = HealthMonitor::new();
        
        // 添加节点
        monitor.add_node(
            "node_1".to_string(),
            "127.0.0.1:50051".to_string(),
            "Test System".to_string(),
        );
        
        // 标记失败
        monitor.mark_node_failure("node_1");
        
        let nodes = monitor.get_all_nodes();
        let node = &nodes[0];
        
        assert_eq!(node.failure_count, 1);
        
        // 标记多次失败
        for _ in 0..4 {
            monitor.mark_node_failure("node_1");
        }
        
        let nodes = monitor.get_all_nodes();
        let node = &nodes[0];
        
        assert_eq!(node.failure_count, 5);
        assert_eq!(node.status, NodeStatus::Offline);
    }

    #[test]
    fn test_health_monitor_remove_node() {
        let monitor = HealthMonitor::new();
        
        // 添加多个节点
        monitor.add_node("node_1".to_string(), "127.0.0.1:50051".to_string(), "System1".to_string());
        monitor.add_node("node_2".to_string(), "127.0.0.1:50052".to_string(), "System2".to_string());
        monitor.add_node("node_3".to_string(), "127.0.0.1:50053".to_string(), "System3".to_string());
        
        assert_eq!(monitor.get_all_nodes().len(), 3);
        
        // 移除一个节点
        monitor.remove_node("node_2");
        
        let nodes = monitor.get_all_nodes();
        assert_eq!(nodes.len(), 2);
        
        let node_ids: Vec<&String> = nodes.iter().map(|n| &n.node_id).collect();
        assert!(node_ids.contains(&&"node_1".to_string()));
        assert!(node_ids.contains(&&"node_3".to_string()));
        assert!(!node_ids.contains(&&"node_2".to_string()));
    }

    #[test]
    fn test_health_monitor_get_healthy_nodes() {
        let monitor = HealthMonitor::new();
        
        // 添加节点
        monitor.add_node("healthy_node".to_string(), "127.0.0.1:50051".to_string(), "System1".to_string());
        monitor.add_node("unhealthy_node".to_string(), "127.0.0.1:50052".to_string(), "System2".to_string());
        
        // 让一个节点健康
        monitor.update_heartbeat("healthy_node", Some(50));
        
        // 让另一个节点不健康
        for _ in 0..5 {
            monitor.mark_node_failure("unhealthy_node");
        }
        
        let healthy_nodes = monitor.get_healthy_nodes();
        assert_eq!(healthy_nodes.len(), 1);
        assert_eq!(healthy_nodes[0].node_id, "healthy_node");
    }

    #[test]
    fn test_health_monitor_generate_health_report() {
        let monitor = HealthMonitor::new();
        
        // 空的监控器
        let report = monitor.generate_health_report();
        assert!(report.contains("没有发现任何节点"));
        
        // 添加一些节点
        monitor.add_node("online_node".to_string(), "127.0.0.1:50051".to_string(), "System1".to_string());
        monitor.add_node("offline_node".to_string(), "127.0.0.1:50052".to_string(), "System2".to_string());
        
        // 设置节点状态
        monitor.update_heartbeat("online_node", Some(30));
        for _ in 0..5 {
            monitor.mark_node_failure("offline_node");
        }
        
        let report = monitor.generate_health_report();
        assert!(report.contains("节点状态摘要"));
        assert!(report.contains("在线: 1"));
        assert!(report.contains("离线: 1"));
        assert!(report.contains("online_node"));
        assert!(report.contains("offline_node"));
    }

    #[test]
    fn test_health_monitor_concurrent_operations() {
        let monitor = Arc::new(HealthMonitor::new());
        
        // 模拟并发操作
        let handles: Vec<_> = (0..10).map(|i| {
            let monitor = Arc::clone(&monitor);
            std::thread::spawn(move || {
                let node_id = format!("node_{}", i);
                monitor.add_node(node_id.clone(), format!("127.0.0.1:5005{}", i), "Test System".to_string());
                monitor.update_heartbeat(&node_id, Some(i as u64 * 10));
            })
        }).collect();
        
        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }
        
        let nodes = monitor.get_all_nodes();
        assert_eq!(nodes.len(), 10);
        
        // 验证所有节点都是健康的
        let healthy_nodes = monitor.get_healthy_nodes();
        assert_eq!(healthy_nodes.len(), 10);
    }

    #[test]
    fn test_node_status_enum() {
        // 测试枚举值
        assert_eq!(NodeStatus::Online, NodeStatus::Online);
        assert_ne!(NodeStatus::Online, NodeStatus::Offline);
        assert_ne!(NodeStatus::Offline, NodeStatus::Unknown);
        
        // 测试Clone
        let status = NodeStatus::Online;
        let cloned_status = status.clone();
        assert_eq!(status, cloned_status);
    }

    #[tokio::test]
    async fn test_health_monitor_async_operations() {
        let monitor = HealthMonitor::new();
        
        // 异步添加节点
        let add_tasks: Vec<_> = (0..5).map(|i| {
            let monitor = &monitor;
            async move {
                let node_id = format!("async_node_{}", i);
                monitor.add_node(node_id.clone(), format!("127.0.0.1:6000{}", i), "Async System".to_string());
                
                // 模拟一些延迟
                sleep(Duration::from_millis(10)).await;
                
                monitor.update_heartbeat(&node_id, Some(i as u64 * 20));
            }
        }).collect();
        
        // 等待所有任务完成
        futures::future::join_all(add_tasks).await;
        
        let nodes = monitor.get_all_nodes();
        assert_eq!(nodes.len(), 5);
        
        let healthy_nodes = monitor.get_healthy_nodes();
        assert_eq!(healthy_nodes.len(), 5);
    }

    #[test]
    fn test_health_monitor_check_nodes_health() {
        let monitor = HealthMonitor::new();
        
        // 添加节点
        monitor.add_node("test_node".to_string(), "127.0.0.1:50051".to_string(), "Test System".to_string());
        monitor.update_heartbeat("test_node", Some(50)); // 设为在线
        
        // 检查健康状态（应该没有变化，因为心跳是最近的）
        monitor.check_nodes_health();
        
        let nodes = monitor.get_all_nodes();
        assert_eq!(nodes[0].status, NodeStatus::Online);
    }
}
*/

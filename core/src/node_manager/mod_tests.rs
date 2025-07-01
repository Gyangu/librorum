// 节点管理器测试模块
#[cfg(test)]
mod tests {
    use super::*;
    use librorum_shared::NodeConfig;
    use std::time::Duration;
    use tokio::time::timeout;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_node_manager_creation() {
        let config = NodeConfig::default();
        let node_manager = NodeManager::with_config(config);
        
        // 验证节点管理器基本属性
        assert!(!node_manager.node_id().is_empty());
        assert!(node_manager.bind_address().contains("50051"));
        assert!(!node_manager.system_info().is_empty());
    }

    #[tokio::test]
    async fn test_node_manager_with_custom_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = NodeConfig {
            node_prefix: "test_node".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8888,
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let node_manager = NodeManager::with_config(config);
        
        assert!(node_manager.node_id().starts_with("test_node"));
        assert_eq!(node_manager.bind_address(), "127.0.0.1:8888");
    }

    #[tokio::test]
    async fn test_node_manager_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let config = NodeConfig {
            bind_port: 0, // 使用动态端口避免冲突
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let node_manager = NodeManager::with_config(config);
        
        // 测试启动（由于是后台服务，我们只测试是否会panic）
        let start_future = node_manager.start();
        
        // 使用超时来避免测试挂起
        let result = timeout(Duration::from_millis(100), start_future).await;
        
        // 应该超时，说明服务正在运行
        assert!(result.is_err()); // 超时错误
    }

    #[test]
    fn test_node_id_generation() {
        let config1 = NodeConfig::default();
        let config2 = NodeConfig::default();
        
        let manager1 = NodeManager::with_config(config1);
        let manager2 = NodeManager::with_config(config2);
        
        // 不同的管理器应该有不同的节点ID
        assert_ne!(manager1.node_id(), manager2.node_id());
    }

    #[test]
    fn test_node_id_prefix() {
        let config = NodeConfig {
            node_prefix: "custom_prefix".to_string(),
            ..Default::default()
        };
        
        let manager = NodeManager::with_config(config);
        assert!(manager.node_id().starts_with("custom_prefix"));
    }

    #[test]
    fn test_system_info() {
        let config = NodeConfig::default();
        let manager = NodeManager::with_config(config);
        
        let system_info = manager.system_info();
        
        // 系统信息应该包含一些基本信息
        assert!(!system_info.is_empty());
        // 可能包含操作系统、架构等信息
        assert!(system_info.len() > 10);
    }

    #[test]
    fn test_bind_address_formats() {
        let test_cases = vec![
            ("0.0.0.0", 50051, "0.0.0.0:50051"),
            ("127.0.0.1", 8080, "127.0.0.1:8080"),
            ("192.168.1.100", 9999, "192.168.1.100:9999"),
        ];
        
        for (host, port, expected) in test_cases {
            let config = NodeConfig {
                bind_host: host.to_string(),
                bind_port: port,
                ..Default::default()
            };
            
            let manager = NodeManager::with_config(config);
            assert_eq!(manager.bind_address(), expected);
        }
    }
}
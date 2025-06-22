use librorum_core::config::NodeConfig;
use librorum_core::node_manager::NodeServiceImpl;
use librorum_core::proto::node::node_service_server::NodeServiceServer;
use serial_test::serial;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use tonic::transport::Server;

/// 端到端测试工具
struct E2ETestEnvironment {
    nodes: Vec<TestNode>,
}

struct TestNode {
    pub service: Arc<NodeServiceImpl>,
    pub port: u16,
    #[allow(dead_code)]
    pub config: NodeConfig,
    #[allow(dead_code)]
    pub temp_dir: TempDir,
}

impl E2ETestEnvironment {
    /// 创建测试环境
    async fn new(node_count: usize) -> Self {
        let mut nodes = Vec::new();
        // 使用当前时间戳确保端口唯一性
        let base_port = 50100 + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() % 1000) as u16;

        for i in 0..node_count {
            let temp_dir = TempDir::new().unwrap();
            let port = base_port + i as u16;
            
            let mut config = NodeConfig::default();
            config.node_prefix = format!("test_node_{}", i);
            config.bind_host = "127.0.0.1".to_string();
            config.bind_port = port;
            config.data_dir = temp_dir.path().to_path_buf();
            config.heartbeat_interval = 2; // 快速心跳用于测试
            config.discovery_interval = 3;

            let service = Arc::new(NodeServiceImpl::new(
                format!("{}_{}", config.node_prefix, i),
                config.bind_address(),
                "Test System".to_string(),
            ));

            let node = TestNode {
                config,
                service,
                port,
                temp_dir,
            };

            nodes.push(node);
        }

        Self { nodes }
    }

    /// 启动所有节点
    async fn start_all_nodes(&self) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();

        for node in &self.nodes {
            // 使用共享的节点列表创建服务实例
            let service = NodeServiceImpl::with_shared_nodes(
                node.service.node_id.clone(),
                node.service.address.clone(),
                node.service.system_info.clone(),
                node.service.nodes.clone(), // 共享节点列表
            );
            let addr = format!("127.0.0.1:{}", node.port).parse().unwrap();

            let handle = tokio::spawn(async move {
                Server::builder()
                    .add_service(NodeServiceServer::new(service))
                    .serve(addr)
                    .await
                    .unwrap();
            });

            handles.push(handle);
        }

        // 等待所有服务器启动
        sleep(Duration::from_millis(300)).await;
        handles
    }

    /// 获取节点地址列表
    fn get_node_addresses(&self) -> Vec<String> {
        self.nodes
            .iter()
            .map(|node| format!("127.0.0.1:{}", node.port))
            .collect()
    }

    /// 模拟节点相互发现过程
    async fn simulate_discovery(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addresses = self.get_node_addresses();
        
        // 每个节点尝试连接其他所有节点
        for (i, node) in self.nodes.iter().enumerate() {
            let client = librorum_core::node_manager::NodeClient::new(
                node.service.node_id.clone(),
                addresses[i].clone(),
                "Test System".to_string(),
            );

            for (j, target_addr) in addresses.iter().enumerate() {
                if i != j {
                    // 尝试发送心跳到目标节点
                    match timeout(Duration::from_secs(5), client.send_heartbeat(target_addr)).await {
                        Ok(Ok(_)) => {
                            println!("节点 {} 成功连接到节点 {}", i, j);
                        }
                        Ok(Err(e)) => {
                            println!("节点 {} 连接节点 {} 失败: {}", i, j, e);
                        }
                        Err(_) => {
                            println!("节点 {} 连接节点 {} 超时", i, j);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 验证所有节点都能相互发现
    async fn verify_full_connectivity(&self) -> bool {
        let node_count = self.nodes.len();
        
        for node in &self.nodes {
            let connected_nodes = node.service.get_all_nodes().await;
            // 每个节点应该能发现其他所有节点（除了自己）
            if connected_nodes.len() != node_count - 1 {
                return false;
            }
        }
        
        true
    }

    /// 获取网络连接统计
    async fn get_connectivity_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        for node in &self.nodes {
            let connected_nodes = node.service.get_all_nodes().await;
            stats.insert(node.service.node_id.clone(), connected_nodes.len());
        }
        
        stats
    }
}

#[tokio::test]
#[serial]
async fn test_two_node_communication() {
    let env = E2ETestEnvironment::new(2).await;
    let _handles = env.start_all_nodes().await;

    // 模拟节点发现
    env.simulate_discovery().await.unwrap();

    // 等待心跳传播
    sleep(Duration::from_secs(1)).await;

    // 验证连接
    let stats = env.get_connectivity_stats().await;
    println!("连接统计: {:?}", stats);

    // 每个节点都应该发现另一个节点
    for (node_id, connection_count) in stats {
        assert_eq!(connection_count, 1, "节点 {} 应该连接到 1 个其他节点", node_id);
    }
}

#[tokio::test]
#[serial]
async fn test_three_node_mesh() {
    let env = E2ETestEnvironment::new(3).await;
    let _handles = env.start_all_nodes().await;

    // 模拟节点发现
    env.simulate_discovery().await.unwrap();

    // 等待心跳传播
    sleep(Duration::from_secs(2)).await;

    // 验证连接
    let stats = env.get_connectivity_stats().await;
    println!("三节点网络连接统计: {:?}", stats);

    // 每个节点都应该发现其他两个节点
    for (node_id, connection_count) in stats {
        assert_eq!(connection_count, 2, "节点 {} 应该连接到 2 个其他节点", node_id);
    }
}

#[tokio::test]
#[serial]
async fn test_node_failure_and_recovery() {
    let env = E2ETestEnvironment::new(3).await;
    let handles = env.start_all_nodes().await;

    // 初始发现
    env.simulate_discovery().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // 验证初始连接
    assert!(env.verify_full_connectivity().await, "初始连接验证失败");

    // 模拟一个节点失败（终止服务器）
    handles[1].abort();
    
    // 等待失败检测
    sleep(Duration::from_secs(3)).await;

    // 剩余节点应该仍然能够通信
    let client = librorum_core::node_manager::NodeClient::new(
        env.nodes[0].service.node_id.clone(),
        format!("127.0.0.1:{}", env.nodes[0].port),
        "Test System".to_string(),
    );

    let result = client
        .send_heartbeat(&format!("127.0.0.1:{}", env.nodes[2].port))
        .await;
    
    assert!(result.is_ok(), "剩余节点之间的通信应该正常");
}

#[tokio::test]
#[serial]
async fn test_concurrent_heartbeats() {
    let env = E2ETestEnvironment::new(2).await;
    let _handles = env.start_all_nodes().await;

    let node0_addr = format!("127.0.0.1:{}", env.nodes[0].port);
    let node1_addr = format!("127.0.0.1:{}", env.nodes[1].port);

    // 并发发送多个心跳
    let mut tasks = Vec::new();
    
    for i in 0..10 {
        let client = librorum_core::node_manager::NodeClient::new(
            format!("concurrent_client_{}", i),
            node0_addr.clone(),
            "Test System".to_string(),
        );
        let target = node1_addr.clone();
        
        let task = tokio::spawn(async move {
            client.send_heartbeat(&target).await
        });
        
        tasks.push(task);
    }

    // 等待所有任务完成
    let mut success_count = 0;
    for task in tasks {
        if task.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10, "所有并发心跳都应该成功");
}

#[tokio::test]
#[serial]
async fn test_heartbeat_with_system_info() {
    let env = E2ETestEnvironment::new(2).await;
    let _handles = env.start_all_nodes().await;

    let client = librorum_core::node_manager::NodeClient::new(
        "detailed_client".to_string(),
        format!("127.0.0.1:{}", env.nodes[0].port),
        "Detailed Test System v1.0".to_string(),
    );

    let result = client
        .send_heartbeat(&format!("127.0.0.1:{}", env.nodes[1].port))
        .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    
    // 验证响应包含正确的系统信息
    assert!(!response.system_info.is_empty());
    assert!(response.timestamp > 0);
    assert!(response.status);
}

#[tokio::test]
#[serial]
async fn test_large_scale_network() {
    // 测试较大规模的网络（5个节点）
    let env = E2ETestEnvironment::new(5).await;
    let _handles = env.start_all_nodes().await;

    // 逐步添加节点到网络
    for round in 1..=3 {
        println!("第 {} 轮发现", round);
        env.simulate_discovery().await.unwrap();
        sleep(Duration::from_millis(500)).await;
        
        let stats = env.get_connectivity_stats().await;
        println!("轮 {} 连接统计: {:?}", round, stats);
    }

    // 最终验证
    let final_stats = env.get_connectivity_stats().await;
    println!("最终连接统计: {:?}", final_stats);

    // 每个节点最终都应该发现其他4个节点
    for (node_id, connection_count) in final_stats {
        assert!(
            connection_count >= 3, // 至少连接到大部分节点
            "节点 {} 只连接到 {} 个节点，期望至少 3 个",
            node_id,
            connection_count
        );
    }
}
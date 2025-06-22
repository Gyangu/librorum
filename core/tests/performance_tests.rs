mod test_utils;

use test_utils::{PerformanceMonitor, TestUtils};
use librorum_core::node_manager::NodeClient;
use librorum_core::node_manager::NodeServiceImpl;
use librorum_core::proto::node::node_service_server::NodeServiceServer;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{Duration, Instant, sleep};
use tonic::transport::Server;

async fn start_simple_test_server(port: u16) -> std::net::SocketAddr {
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let service = NodeServiceImpl::new(
        "test_server".to_string(),
        format!("127.0.0.1:{}", port),
        "Test System".to_string(),
    );

    tokio::spawn(async move {
        Server::builder()
            .add_service(NodeServiceServer::new(service))
            .serve(addr)
            .await
            .unwrap();
    });

    sleep(Duration::from_millis(100)).await;
    addr
}

#[tokio::test]
async fn test_heartbeat_latency() {
    let addr = start_simple_test_server(TestUtils::find_available_port()).await;
    
    let client = NodeClient::new(
        "perf_client".to_string(),
        "127.0.0.1:40000".to_string(),
        "Performance Test System".to_string(),
    );

    let mut monitor = PerformanceMonitor::new();

    // 执行多次心跳测试
    for _ in 0..10 {
        monitor.start_measurement();
        
        let result = client.send_heartbeat(&addr.to_string()).await;
        
        monitor.end_measurement();
        
        assert!(result.is_ok(), "心跳应该成功");
    }

    monitor.print_stats();
    
    // 验证平均延迟在合理范围内（< 100ms）
    let avg_latency = monitor.get_average_duration();
    assert!(
        avg_latency < Duration::from_millis(100),
        "平均延迟过高: {:?}",
        avg_latency
    );
}

#[tokio::test]
async fn test_concurrent_heartbeat_throughput() {
    let addr = start_simple_test_server(TestUtils::find_available_port()).await;
    let server_addr = addr.to_string();
    let concurrent_clients = 10;
    let requests_per_client = 5;

    let semaphore = Arc::new(Semaphore::new(concurrent_clients));
    let mut handles = Vec::new();

    let start_time = Instant::now();

    for i in 0..concurrent_clients {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let addr = server_addr.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let client = NodeClient::new(
                format!("perf_client_{}", i),
                format!("127.0.0.1:{}", 40000 + i),
                "Performance Test System".to_string(),
            );

            let mut success_count = 0;
            for _ in 0..requests_per_client {
                if client.send_heartbeat(&addr).await.is_ok() {
                    success_count += 1;
                }
            }
            success_count
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    let mut total_success = 0;
    for handle in handles {
        total_success += handle.await.unwrap();
    }

    let total_time = start_time.elapsed();
    let total_requests = concurrent_clients * requests_per_client;
    let throughput = total_success as f64 / total_time.as_secs_f64();

    println!("吞吐量测试结果:");
    println!("  总请求数: {}", total_requests);
    println!("  成功请求数: {}", total_success);
    println!("  总时间: {:?}", total_time);
    println!("  吞吐量: {:.2} requests/sec", throughput);

    // 验证成功率
    let success_rate = total_success as f64 / total_requests as f64;
    assert!(
        success_rate > 0.8,
        "成功率过低: {:.2}%",
        success_rate * 100.0
    );
}

#[tokio::test]
async fn test_large_payload_performance() {
    let addr = start_simple_test_server(TestUtils::find_available_port()).await;
    let server_addr = addr.to_string();
    
    // 创建带有大系统信息的客户端
    let large_system_info = "A".repeat(1024); // 1KB 系统信息
    let client = NodeClient::new(
        "large_payload_client".to_string(),
        "127.0.0.1:47000".to_string(),
        large_system_info,
    );

    let mut monitor = PerformanceMonitor::new();

    // 测试大负载性能
    for _ in 0..10 {
        monitor.start_measurement();
        
        let result = client.send_heartbeat(&server_addr).await;
        
        monitor.end_measurement();
        
        assert!(result.is_ok(), "大负载心跳应该成功");
    }

    monitor.print_stats();

    // 验证大负载不会显著影响性能
    let avg_duration = monitor.get_average_duration();
    assert!(
        avg_duration < Duration::from_millis(200),
        "大负载性能过低，平均时间: {:?}",
        avg_duration
    );
}
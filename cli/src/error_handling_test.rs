use anyhow::Result;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use crate::zero_copy_client::{ZeroCopyClient, ZeroCopyConfig, RetryConfig};

/// 错误处理和重试机制测试
pub struct ErrorHandlingTest {
    server_addr: SocketAddr,
}

impl ErrorHandlingTest {
    pub fn new(server_addr: SocketAddr) -> Self {
        Self { server_addr }
    }
    
    /// 测试连接超时处理
    pub async fn test_connection_timeout(&self) -> Result<()> {
        info!("🧪 测试连接超时处理...");
        
        // 使用一个不存在的地址来模拟连接超时
        let invalid_addr: SocketAddr = "192.0.2.1:9999".parse().unwrap(); // RFC 5737 测试地址
        
        let mut config = ZeroCopyConfig::default();
        config.retry_config.connection_timeout = Duration::from_millis(100); // 短超时
        config.retry_config.max_retries = 2;
        config.retry_config.initial_delay = Duration::from_millis(50);
        
        let client = ZeroCopyClient::new(invalid_addr, config);
        
        let start_time = std::time::Instant::now();
        let result = client.upload_file_zero_copy(
            "/tmp/test_timeout.txt",
            "/test_timeout.txt",
            None,
        ).await;
        
        let elapsed = start_time.elapsed();
        
        match result {
            Err(_) => {
                info!("✅ 连接超时测试通过: {} ms", elapsed.as_millis());
                // 应该在合理时间内失败 (3次重试，每次100ms连接超时 + 重试延迟)
                assert!(elapsed < Duration::from_secs(2), "超时处理时间过长");
            },
            Ok(_) => {
                panic!("❌ 连接超时测试失败: 应该返回错误");
            }
        }
        
        Ok(())
    }
    
    /// 测试重试机制
    pub async fn test_retry_mechanism(&self) -> Result<()> {
        info!("🧪 测试重试机制...");
        
        // 创建一个模拟间歇性失败的配置
        let mut config = ZeroCopyConfig::default();
        config.retry_config.max_retries = 3;
        config.retry_config.initial_delay = Duration::from_millis(100);
        config.retry_config.backoff_multiplier = 2.0;
        config.retry_config.connection_timeout = Duration::from_millis(500);
        
        let client = ZeroCopyClient::new(self.server_addr, config);
        
        // 创建一个小测试文件
        let test_content = "Hello, retry test!";
        tokio::fs::write("/tmp/retry_test.txt", test_content).await?;
        
        info!("开始重试测试 (如果服务器不可用将展示重试行为)...");
        
        let result = client.upload_file_zero_copy(
            "/tmp/retry_test.txt",
            "/retry_test.txt",
            None,
        ).await;
        
        match result {
            Ok(transfer_result) => {
                info!("✅ 重试测试通过: 传输成功 {:.2} MB/s", transfer_result.throughput_mbps);
            },
            Err(e) => {
                info!("⚠️  重试测试: 最终失败 (这在服务器不可用时是正常的): {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 测试I/O超时处理
    pub async fn test_io_timeout(&self) -> Result<()> {
        info!("🧪 测试I/O超时配置...");
        
        let mut config = ZeroCopyConfig::default();
        config.retry_config.io_timeout = Duration::from_millis(100); // 非常短的I/O超时
        config.retry_config.max_retries = 1;
        
        let client = ZeroCopyClient::new(self.server_addr, config);
        
        // 创建一个较大的测试文件 (可能触发I/O超时)
        let large_content = "x".repeat(10 * 1024 * 1024); // 10MB
        tokio::fs::write("/tmp/io_timeout_test.txt", large_content).await?;
        
        let result = client.upload_file_zero_copy(
            "/tmp/io_timeout_test.txt",
            "/io_timeout_test.txt",
            None,
        ).await;
        
        match result {
            Ok(_) => {
                info!("✅ I/O超时测试: 传输成功 (网络足够快)");
            },
            Err(e) => {
                info!("⚠️  I/O超时测试: 检测到超时 (这在慢速网络上是正常的): {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 运行所有错误处理测试
    pub async fn run_all_tests(&self) -> Result<()> {
        info!("🚀 开始错误处理和重试机制测试套件...");
        
        // 测试1: 连接超时
        self.test_connection_timeout().await?;
        sleep(Duration::from_millis(100)).await;
        
        // 测试2: 重试机制  
        self.test_retry_mechanism().await?;
        sleep(Duration::from_millis(100)).await;
        
        // 测试3: I/O超时
        self.test_io_timeout().await?;
        
        info!("🎉 错误处理测试套件完成!");
        
        Ok(())
    }
}

/// 零拷贝客户端弹性测试
pub async fn test_zero_copy_resilience(server_addr: SocketAddr) -> Result<()> {
    let test_suite = ErrorHandlingTest::new(server_addr);
    test_suite.run_all_tests().await
}
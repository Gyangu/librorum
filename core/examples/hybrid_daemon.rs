//! Hybrid Daemon示例
//! 
//! 演示如何使用hybrid架构运行librorum daemon

use anyhow::{Context, Result};
use clap::Parser;
use librorum_core::logger;
use librorum_core::node_manager::HybridNodeManager;
use librorum_shared::NodeConfig;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// librorum hybrid daemon示例
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct HybridDaemonCli {
    /// 配置文件路径
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// gRPC服务端口
    #[clap(long, default_value = "50051")]
    grpc_port: u16,

    /// UTP服务端口
    #[clap(long, default_value = "9090")]
    utp_port: u16,

    /// 日志级别 (trace, debug, info, warn, error)
    #[clap(short, long, default_value = "info")]
    log_level: String,
    
    /// 启用调试日志
    #[clap(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let mut cli = HybridDaemonCli::parse();
    
    // 如果指定了verbose参数，设置日志级别为debug
    if cli.verbose {
        cli.log_level = "debug".to_string();
    }

    // 配置日志
    logger::init_with_level(&cli.log_level)
        .context("Failed to initialize logger")?;

    info!("🚀 启动librorum hybrid daemon示例");
    info!("gRPC端口: {}", cli.grpc_port);
    info!("UTP端口: {}", cli.utp_port);

    // 加载配置
    let config = if let Some(config_path) = &cli.config {
        info!("📋 加载配置文件: {:?}", config_path);
        NodeConfig::from_file(config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?
    } else {
        info!("📋 使用默认配置");
        NodeConfig::default()
    };

    // 创建并启动Hybrid节点管理器
    let mut node_manager = if cli.config.is_some() {
        HybridNodeManager::with_config(config, cli.utp_port)
    } else {
        HybridNodeManager::new(cli.grpc_port, cli.utp_port)
    };

    // 启动节点管理器
    if let Err(e) = node_manager.start().await {
        error!("❌ 节点管理器启动失败: {}", e);
        return Err(e);
    }

    info!("✅ Hybrid daemon启动完成");
    info!("🔗 节点ID: {}", node_manager.node_id());
    info!("🌐 gRPC地址: {}", node_manager.grpc_bind_address());
    info!("⚡ UTP地址: {}", node_manager.utp_bind_address());

    // 设置Ctrl+C处理
    tokio::spawn(async {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        info!("🛑 收到Ctrl+C信号，退出...");
        std::process::exit(0);
    });

    // 主循环 - 定期输出统计信息
    let mut stats_interval = tokio::time::interval(std::time::Duration::from_secs(60));

    loop {
        stats_interval.tick().await;
        
        // 输出统计信息
        let discovered_nodes = node_manager.get_discovered_nodes();
        let known_nodes = node_manager.get_known_nodes().await;
        
        info!("📊 节点统计: 发现{}, 已知{}", discovered_nodes.len(), known_nodes.len());
        
        // 输出UTP统计
        if let Some(utp_stats) = node_manager.get_utp_stats() {
            info!("📊 UTP统计: 会话{}, 成功{}, 失败{}, 传输{:.1}MB", 
                utp_stats.total_sessions,
                utp_stats.successful_transfers,
                utp_stats.failed_transfers,
                utp_stats.total_bytes_transferred as f64 / 1024.0 / 1024.0
            );
        }
    }
}
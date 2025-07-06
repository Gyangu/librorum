//! Hybrid Core Daemon
//! 
//! 支持UTP传输的高性能core daemon

use anyhow::{Context, Result};
use clap::Parser;
use librorum_core::logger;
use librorum_core::node_manager::HybridNodeManager;
use librorum_shared::NodeConfig;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// librorum hybrid核心守护进程
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct HybridCli {
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
    
    /// 启用调试日志（相当于 --log-level=debug）
    #[clap(short, long)]
    verbose: bool,

    /// 启用hybrid模式
    #[clap(long, default_value = "true")]
    hybrid_enabled: bool,

    /// 作为守护进程运行
    #[clap(long)]
    daemon: bool,

    /// 显示版本信息
    #[clap(long)]
    version_info: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 在Windows平台上设置控制台代码页为UTF-8以支持中文显示
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("powershell")
            .args(&["-Command", "chcp 65001"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    // 解析命令行参数
    let mut cli = HybridCli::parse();
    
    // 显示版本信息
    if cli.version_info {
        println!("librorum-hybrid-core {}", env!("CARGO_PKG_VERSION"));
        println!("Hybrid架构支持: gRPC控制 + UTP数据传输");
        println!("编译时间: {}", env!("BUILD_TIME"));
        println!("Git版本: {}", env!("GIT_HASH"));
        return Ok(());
    }
    
    // 如果指定了verbose参数，设置日志级别为debug
    if cli.verbose {
        cli.log_level = "debug".to_string();
    }

    // 配置日志
    logger::init_with_level(&cli.log_level)
        .context("Failed to initialize logger")?;

    info!("🚀 启动librorum hybrid核心守护进程");
    info!("版本: {}", env!("CARGO_PKG_VERSION"));
    info!("gRPC端口: {}", cli.grpc_port);
    info!("UTP端口: {}", cli.utp_port);
    info!("Hybrid模式: {}", cli.hybrid_enabled);

    // 加载配置
    let config = if let Some(config_path) = &cli.config {
        info!("📋 加载配置文件: {:?}", config_path);
        NodeConfig::from_file(config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?
    } else {
        info!("📋 使用默认配置");
        NodeConfig::default()
    };

    // 如果需要作为守护进程运行
    if cli.daemon {
        #[cfg(unix)]
        {
            use daemonize::Daemonize;
            
            info!("🔄 转换为守护进程模式...");
            
            let daemonize = Daemonize::new()
                .pid_file("/tmp/librorum-hybrid-core.pid")
                .chown_pid_file(true)
                .working_directory("/tmp")
                .umask(0o027);

            match daemonize.start() {
                Ok(_) => info!("✅ 守护进程启动成功"),
                Err(e) => {
                    error!("❌ 守护进程启动失败: {}", e);
                    return Err(anyhow::anyhow!("守护进程启动失败: {}", e));
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            warn!("⚠️ 当前平台不支持守护进程模式，继续以前台模式运行");
        }
    }

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

    info!("✅ Hybrid核心守护进程启动完成");
    info!("🔗 节点ID: {}", node_manager.node_id());
    info!("🌐 gRPC地址: {}", node_manager.grpc_bind_address());
    info!("⚡ UTP地址: {}", node_manager.utp_bind_address());

    // 设置信号处理
    let node_manager_clone = std::sync::Arc::new(tokio::sync::Mutex::new(node_manager));
    
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        
        let manager_for_signals = node_manager_clone.clone();
        
        tokio::spawn(async move {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("🛑 收到SIGTERM信号，开始优雅关闭...");
                }
                _ = sigint.recv() => {
                    info!("🛑 收到SIGINT信号，开始优雅关闭...");
                }
            }
            
            // 优雅关闭
            let manager = manager_for_signals.lock().await;
            if let Err(e) = manager.stop().await {
                error!("❌ 节点管理器停止失败: {}", e);
            } else {
                info!("✅ 节点管理器已优雅停止");
            }
            
            std::process::exit(0);
        });
    }
    
    #[cfg(windows)]
    {
        use tokio::signal;
        
        let manager_for_signals = node_manager_clone.clone();
        
        tokio::spawn(async move {
            signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
            info!("🛑 收到Ctrl+C信号，开始优雅关闭...");
            
            // 优雅关闭
            let manager = manager_for_signals.lock().await;
            if let Err(e) = manager.stop().await {
                error!("❌ 节点管理器停止失败: {}", e);
            } else {
                info!("✅ 节点管理器已优雅停止");
            }
            
            std::process::exit(0);
        });
    }

    // 主循环 - 定期输出统计信息
    let mut stats_interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 每5分钟
    let mut cleanup_interval = tokio::time::interval(std::time::Duration::from_secs(60)); // 每分钟清理

    loop {
        tokio::select! {
            _ = stats_interval.tick() => {
                let manager = node_manager_clone.lock().await;
                
                // 输出统计信息
                let discovered_nodes = manager.get_discovered_nodes();
                let known_nodes = manager.get_known_nodes().await;
                
                info!("📊 节点统计:");
                info!("  已发现节点: {}", discovered_nodes.len());
                info!("  已知节点: {}", known_nodes.len());
                
                // 输出UTP统计
                if let Some(utp_stats) = manager.get_utp_stats() {
                    info!("📊 UTP传输统计:");
                    info!("  总会话数: {}", utp_stats.total_sessions);
                    info!("  成功传输: {}", utp_stats.successful_transfers);
                    info!("  失败传输: {}", utp_stats.failed_transfers);
                    info!("  总传输量: {:.2} MB", utp_stats.total_bytes_transferred as f64 / 1024.0 / 1024.0);
                    info!("  平均速率: {:.2} MB/s", utp_stats.average_transfer_rate / 1024.0 / 1024.0);
                    info!("  最大速率: {:.2} MB/s", utp_stats.max_transfer_rate / 1024.0 / 1024.0);
                    info!("  网络模式使用: {}", utp_stats.network_mode_usage);
                    info!("  共享内存模式使用: {}", utp_stats.shared_memory_mode_usage);
                }
                
                // 输出健康状态
                let all_health = manager.get_all_node_health().await;
                let online_count = all_health.iter().filter(|h| h.status == librorum_core::node_manager::NodeStatus::Online).count();
                info!("💗 节点健康: {}/{} 在线", online_count, all_health.len());
            }
            
            _ = cleanup_interval.tick() => {
                let manager = node_manager_clone.lock().await;
                
                // 清理完成的UTP会话
                manager.cleanup_utp_sessions().await;
            }
        }
    }
}

// 在编译时获取构建信息
fn main() -> Result<()> {
    // 设置编译时环境变量
    println!("cargo:rustc-env=BUILD_TIME={}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    
    // 获取Git哈希
    if let Ok(output) = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let git_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        } else {
            println!("cargo:rustc-env=GIT_HASH=unknown");
        }
    } else {
        println!("cargo:rustc-env=GIT_HASH=unknown");
    }
    
    // 调用实际的main函数
    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(async_main()),
        Err(e) => Err(anyhow::anyhow!("Failed to create tokio runtime: {}", e)),
    }
}

// 重命名原来的main函数
async fn async_main() -> Result<()> {
    // 在Windows平台上设置控制台代码页为UTF-8以支持中文显示
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("powershell")
            .args(&["-Command", "chcp 65001"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    // 解析命令行参数
    let mut cli = HybridCli::parse();
    
    // 显示版本信息
    if cli.version_info {
        println!("librorum-hybrid-core {}", env!("CARGO_PKG_VERSION"));
        println!("Hybrid架构支持: gRPC控制 + UTP数据传输");
        if let Ok(build_time) = std::env::var("BUILD_TIME") {
            println!("编译时间: {}", build_time);
        }
        if let Ok(git_hash) = std::env::var("GIT_HASH") {
            println!("Git版本: {}", git_hash);
        }
        return Ok(());
    }
    
    // 其余逻辑保持不变...
    Ok(())
}
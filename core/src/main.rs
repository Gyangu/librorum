use anyhow::{Context, Result};
use clap::Parser;
use librorum_core::logger;
use librorum_core::node_manager::NodeManager;
use librorum_shared::NodeConfig;
use std::path::PathBuf;
use tracing::{error, info};

/// librorum 核心守护进程
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// 配置文件路径
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// 日志级别 (trace, debug, info, warn, error)
    #[clap(short, long, default_value = "info")]
    log_level: String,
    
    /// 启用调试日志（相当于 --log-level=debug）
    #[clap(short, long)]
    verbose: bool,

    /// 作为守护进程运行
    #[clap(long)]
    daemon: bool,
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
    let mut cli = Cli::parse();
    
    // 如果指定了verbose参数，设置日志级别为debug
    if cli.verbose {
        cli.log_level = "debug".to_string();
    }

    // 配置日志
    if let Err(e) = logger::init_logger(&cli.log_level, cli.daemon) {
        eprintln!("无法初始化日志系统: {}", e);
        return Err(e);
    }

    // 输出调试信息
    info!("==== librorum daemon启动 ====");
    info!(
        "当前工作目录: {}",
        std::env::current_dir().unwrap_or_default().display()
    );
    info!(
        "可执行文件: {}",
        std::env::current_exe().unwrap_or_default().display()
    );
    info!("日志级别: {}", cli.log_level);
    info!("daemon模式: {}", cli.daemon);

    // 加载配置
    let node_config = match cli.config {
        Some(config_path) => {
            info!("使用指定的配置文件: {}", config_path.display());
            NodeConfig::from_file(&config_path)
                .with_context(|| format!("无法加载配置文件: {:?}", config_path))?
        }
        None => {
            info!("未指定配置文件，使用自动检测的配置");
            load_config()?
        }
    };

    // 创建数据目录
    node_config.create_data_dir()?;

    // 创建并启动节点管理器
    let config_str = toml::to_string(&node_config)
        .unwrap_or_else(|_| "无法序列化配置".to_string());
    info!("配置: {}", config_str);

    let node_manager = NodeManager::with_config(node_config);

    // 初始化gRPC服务
    let _node_id = node_manager.node_id().to_string();
    info!("节点ID: {}", node_manager.node_id());
    info!("绑定地址: {}", node_manager.bind_address());
    info!("系统: {}", node_manager.system_info());

    // 启动节点服务
    info!("启动节点服务...");
    match node_manager.start().await {
        Ok(_) => {
            info!("节点服务正常退出");
        }
        Err(e) => {
            error!("节点服务启动失败: {:?}", e);
            eprintln!("服务启动失败: {}", e);
            return Err(e);
        }
    }

    info!("节点服务已关闭");
    Ok(())
}

/// 加载配置
fn load_config() -> Result<NodeConfig> {
    if let Some(config_path) = NodeConfig::find_config_file() {
        // 使用自动找到的配置文件
        info!("使用自动检测的配置文件: {}", config_path.display());
        NodeConfig::from_file(config_path)
    } else {
        // 使用默认配置
        info!("未找到配置文件，使用默认配置");
        Ok(NodeConfig::default())
    }
}
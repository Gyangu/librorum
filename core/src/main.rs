use anyhow::Result;
use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use librorum_core::config::NodeConfig;
use librorum_core::node_manager::NodeManager;
use librorum_core::logger;
use librorum_core::daemon;
use tracing;

/// librorum 分布式文件系统命令行工具
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// 子命令
    #[clap(subcommand)]
    command: Command,
    
    /// 配置文件路径
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// 日志级别 (trace, debug, info, warn, error)
    #[clap(short, long, default_value = "info")]
    log_level: String,
}

/// 命令集
#[derive(Subcommand)]
enum Command {
    /// 启动服务（守护进程）
    Start {
        /// 配置文件路径
        #[clap(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
    
    /// 停止服务
    Stop,
    
    /// 重启服务
    Restart,
    
    /// 显示服务状态
    Status,
    
    /// 显示日志
    Logs {
        /// 显示最后几行
        #[clap(short, long, default_value = "20")]
        tail: usize,
    },
    
    /// 创建默认配置文件
    Init {
        /// 输出路径
        #[clap(default_value = "librorum.toml")]
        path: PathBuf,
    },
    
    /// 清理旧日志
    CleanLogs {
        /// 保留几天内的日志
        #[clap(default_value = "30")]
        days: u64,
    },
    
    
    /// 运行服务（内部命令，由守护进程调用）
    #[clap(hide = true)]
    Run {
        /// 作为守护进程运行
        #[clap(long)]
        daemon: bool,
        
        /// 配置文件路径
        #[clap(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },
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
    let cli = Cli::parse();
    
    // 根据命令执行不同操作
    match &cli.command {
        Command::Start { config: cmd_config } => {
            // 如果命令行参数中指定了配置文件，优先使用
            if let Some(config_path) = cmd_config {
                println!("使用指定的配置文件: {:?}", config_path);
                let config = NodeConfig::from_file(config_path)
                    .with_context(|| format!("无法加载配置文件: {:?}", config_path))?;
                daemon::start_daemon(&config)?;
            } else {
                // 否则使用自动检测的配置
                let config = load_config(&cli)?;
                daemon::start_daemon(&config)?;
            }
        }
        
        Command::Stop => {
            daemon::stop_daemon()?;
        }
        
        Command::Restart => {
            // 加载配置
            let config = load_config(&cli)?;
            daemon::restart_daemon(&config)?;
        }
        
        Command::Status => {
            let status = daemon::daemon_status();
            println!("{}", status);
        }
        
        Command::Logs { tail } => {
            let logs = daemon::view_logs(*tail)?;
            println!("{}", logs);
        }
        
        Command::Init { path } => {
            // 创建默认配置
            let config = NodeConfig::default();
            
            // 保存配置
            config.save_to_file(path)?;
            
            println!("已生成默认配置文件: {:?}", path);
        }
        
        Command::CleanLogs { days } => {
            let count = logger::clean_old_logs(*days)?;
            println!("已清理 {} 个旧日志文件", count);
        }
        
        Command::Run { daemon, config } => {
            // 配置日志
            if let Err(e) = logger::init_logger(&cli.log_level, *daemon) {
                eprintln!("无法初始化日志系统: {}", e);
                return Err(e);
            }
            
            // 输出调试信息
            tracing::info!("==== librorum daemon启动 ====");
            tracing::info!("当前工作目录: {:?}", std::env::current_dir().unwrap_or_default());
            tracing::info!("可执行文件: {:?}", std::env::current_exe().unwrap_or_default());
            tracing::info!("日志级别: {}", cli.log_level);
            tracing::info!("daemon模式: {}", daemon);
            
            // 加载配置
            let node_config = match config {
                Some(config_path) => {
                    tracing::info!("使用指定的配置文件: {:?}", config_path);
                    NodeConfig::from_file(config_path)
                        .with_context(|| format!("无法加载配置文件: {:?}", config_path))?
                },
                None => {
                    tracing::info!("未指定配置文件，使用自动检测的配置");
                    load_config(&cli)?
                }
            };
            
            // 确保数据目录存在
            if let Err(e) = node_config.create_data_dir() {
                tracing::error!("创建数据目录失败: {}", e);
                return Err(e);
            }
            
            // 输出启动信息
            tracing::info!("====== librorum 服务启动 ======");
            tracing::info!("配置: {:?}", node_config);
            
            // 创建节点管理器
            tracing::info!("创建节点管理器...");
            let node_manager = NodeManager::with_config(node_config);
            
            // 输出节点信息
            tracing::info!("节点ID: {}", node_manager.node_id());
            tracing::info!("绑定地址: {}", node_manager.bind_address());
            tracing::info!("系统: {}", node_manager.system_info());
            
            // 启动节点服务
            tracing::info!("启动节点服务...");
            match node_manager.start().await {
                Ok(_) => {
                    tracing::info!("节点服务正常退出");
                },
                Err(e) => {
                    tracing::error!("节点服务启动失败: {:?}", e);
                    eprintln!("服务启动失败: {}", e);
                    return Err(e);
                }
            }
            
            tracing::info!("节点服务已关闭");
        }
    }
    
    Ok(())
}

/// 加载配置
fn load_config(cli: &Cli) -> Result<NodeConfig> {
    if let Some(config_path) = &cli.config {
        // 使用指定的配置文件
        tracing::info!("使用配置文件: {:?}", config_path);
        NodeConfig::from_file(config_path)
    } else if let Some(config_path) = NodeConfig::find_config_file() {
        // 使用自动找到的配置文件
        tracing::info!("使用自动检测的配置文件: {:?}", config_path);
        NodeConfig::from_file(config_path)
    } else {
        // 使用默认配置
        tracing::info!("未找到配置文件，使用默认配置");
        Ok(NodeConfig::default())
    }
} 
use anyhow::Context;
use anyhow::Result;
use clap::{Parser, Subcommand};
use librorum_core::config::NodeConfig;
use librorum_core::daemon;
use librorum_core::logger;
use librorum_core::node_manager::NodeManager;
use std::path::PathBuf;
use tracing::{error, info};
use toml;

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
    
    /// 启用调试日志（相当于 --log-level=debug）
    #[clap(short, long)]
    verbose: bool,
}

/// 命令集
#[derive(Subcommand)]
enum Command {
    /// 启动服务（守护进程）
    Start {
        /// 配置文件路径
        #[clap(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// 启用调试日志
        #[clap(short, long)]
        verbose: bool,
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

    /// 清理全部日志
    CleanAllLogs,

    /// 显示节点健康状态
    NodesStatus,

    /// 运行服务（内部命令，由守护进程调用）
    #[clap(hide = true)]
    Run {
        /// 作为守护进程运行
        #[clap(long)]
        daemon: bool,

        /// 配置文件路径
        #[clap(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
        
        /// 启用调试日志
        #[clap(short, long)]
        verbose: bool,
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
    let mut cli = Cli::parse();
    
    // 如果指定了verbose参数，设置日志级别为debug
    if cli.verbose {
        cli.log_level = "debug".to_string();
    }

    // 根据命令执行不同操作
    match &cli.command {
        Command::Start { config: cmd_config, verbose } => {
            // 如果命令行开启了调试模式，设置环境变量
            if *verbose {
                println!("启用调试日志级别");
                // 使用unsafe块设置环境变量
                unsafe {
                    std::env::set_var("LIBRORUM_VERBOSE", "1");
                }
            }
            
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

        Command::CleanAllLogs => {
            let count = logger::clean_all_logs()?;
            println!("已清理 {} 个日志文件", count);
        }

        Command::NodesStatus => {
            // 获取服务状态，确保服务在运行
            let status = daemon::daemon_status();
            if !status.contains("正在运行") {
                println!("错误: 服务未运行，请先启动服务");
                return Ok(());
            }

            // 尝试获取节点健康状态
            match daemon::get_nodes_health_status() {
                Ok(status) => {
                    println!("节点健康状态:");
                    println!("{}", status);
                }
                Err(e) => {
                    println!("获取节点健康状态失败: {}", e);
                }
            }
        }

        Command::Run { daemon, config, verbose } => {
            // 配置日志
            // 如果指定了verbose参数，设置日志级别为debug
            let log_level = if *verbose {
                "debug".to_string()
            } else {
                cli.log_level.clone()
            };
            
            if let Err(e) = logger::init_logger(&log_level, *daemon) {
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
            info!("daemon模式: {}", daemon);

            // 加载配置
            let node_config = match config {
                Some(config_path) => {
                    info!("使用指定的配置文件: {}", config_path.display());
                    NodeConfig::from_file(config_path)
                        .with_context(|| format!("无法加载配置文件: {:?}", config_path))?
                }
                None => {
                    info!("未指定配置文件，使用自动检测的配置");
                    load_config(&cli)?
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
        }
    }

    Ok(())
}

/// 加载配置
fn load_config(cli: &Cli) -> Result<NodeConfig> {
    if let Some(config_path) = &cli.config {
        // 使用指定的配置文件
        info!("使用配置文件: {}", config_path.display());
        NodeConfig::from_file(config_path)
    } else if let Some(config_path) = NodeConfig::find_config_file() {
        // 使用自动找到的配置文件
        info!("使用自动检测的配置文件: {}", config_path.display());
        NodeConfig::from_file(config_path)
    } else {
        // 使用默认配置
        info!("未找到配置文件，使用默认配置");
        Ok(NodeConfig::default())
    }
}

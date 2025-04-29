use anyhow::Result;
use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use librorum_core::config::NodeConfig;
use librorum_core::daemon::{start_daemon, stop_daemon, restart_daemon, daemon_status, view_logs, is_running};
use librorum_core::node_manager::NodeManager;
use librorum_core::logger;
use tracing;
use chrono;
use hostname;

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
    
    /// 测试节点连接
    TestConnect {
        /// 远程节点地址，格式: IP:端口
        #[clap(required = true)]
        address: String,
    },
    
    /// 测试中文字符显示
    TestChinese,
    
    /// 运行服务（内部命令，由守护进程调用）
    #[clap(hide = true)]
    Run {
        /// 作为守护进程运行
        #[clap(long)]
        daemon: bool,
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
                start_daemon(&config)?;
            } else {
                // 否则使用自动检测的配置
                let config = load_config(&cli)?;
                start_daemon(&config)?;
            }
        }
        
        Command::Stop => {
            stop_daemon()?;
        }
        
        Command::Restart => {
            // 加载配置
            let config = load_config(&cli)?;
            restart_daemon(&config)?;
        }
        
        Command::Status => {
            let status = daemon_status();
            println!("{}", status);
        }
        
        Command::Logs { tail } => {
            let logs = view_logs(*tail)?;
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
        
        Command::TestConnect { address } => {
            // 配置日志
            logger::init_logger(&cli.log_level, false)?;
            tracing::info!("开始测试连接节点: {}", address);
            
            // 加载配置
            let config = load_config(&cli)?;
            
            // 检查服务是否已启动
            if is_running() {
                tracing::info!("检测到服务已运行，使用现有服务进行测试");
                
                // 创建节点管理器（使用配置）
                let node_manager = NodeManager::with_config(config);
                
                // 手动添加测试节点
                node_manager.add_node(address.clone()).await?;
                
                // 尝试连接节点
                match node_manager.connect_to_node(address.clone()).await {
                    Ok(info) => {
                        println!("成功连接到节点: {}", address);
                        println!("节点信息: ID={}, 系统={}, 最后活动={}", 
                            info.id, info.system, 
                            chrono::DateTime::<chrono::Utc>::from_timestamp(info.last_seen, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| "未知".to_string()));
                    },
                    Err(e) => {
                        eprintln!("连接失败: {}", e);
                        eprintln!("请检查节点 {} 是否在线，或检查网络连接", address);
                        return Err(e.into());
                    }
                }
            } else {
                // 服务未运行，使用临时模式测试连接
                tracing::info!("服务未运行，使用临时测试模式");
                
                // 直接创建临时节点客户端
                let device_name = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                    
                let node_id = format!("temp.{}.librorum.local", device_name);
                let bind_address = format!("0.0.0.0:{}", config.bind_port);
                let system_info = if cfg!(target_os = "windows") { "Windows" } 
                                  else if cfg!(target_os = "macos") { "macOS" } 
                                  else { "Unknown" };
                
                // 启动一个临时服务用于通信测试
                tracing::info!("正在启动临时通信服务...");
                
                // 创建临时服务并在后台运行
                let temp_port = config.bind_port; // 使用另一个端口避免冲突
                let temp_node_id = node_id.clone();
                let temp_system_info = system_info.to_string();
                
                // 创建临时节点服务
                use librorum_core::node_manager::node_service::NodeServiceImpl;
                use librorum_core::proto::node::node_service_server::NodeServiceServer;
                use tonic::transport::Server;
                use std::net::SocketAddr;
                
                // 后台启动微型服务
                let server_task = tokio::spawn(async move {
                    let service = NodeServiceImpl::new(
                        temp_node_id,
                        format!("0.0.0.0:{}", temp_port),
                        temp_system_info,
                    );
                    
                    let server = NodeServiceServer::new(service);
                    let addr = format!("0.0.0.0:{}", temp_port).parse::<SocketAddr>().unwrap();
                    
                    tracing::info!("临时测试服务启动于 {}", addr);
                    Server::builder()
                        .add_service(server)
                        .serve(addr)
                        .await
                        .unwrap();
                });
                
                // 等待服务启动
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                
                // 创建临时客户端
                use librorum_core::node_manager::node_client::NodeClient;
                let client = NodeClient::new(
                    node_id,
                    bind_address,
                    system_info.to_string()
                );
                
                // 测试连接
                println!("测试节点 {} 心跳连接", address);
                println!("正在发送心跳包...");
                let result = client.send_heartbeat(&address).await;
                
                // 终止临时服务
                server_task.abort();
                
                match result {
                    Ok(response) => {
                        println!("连接成功！");
                        println!("远程节点信息:");
                        println!("  节点ID: {}", response.node_id);
                        println!("  地址: {}", response.address);
                        println!("  系统: {}", response.system_info);
                        println!("  时间戳: {}", 
                            chrono::DateTime::<chrono::Utc>::from_timestamp(response.timestamp, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| "未知".to_string()));
                    },
                    Err(e) => {
                        eprintln!("连接失败: {}", e);
                        eprintln!("请检查节点 {} 是否在线，或检查网络连接", address);
                        return Err(e.into());
                    }
                }
            }
        }
        
        Command::TestChinese => {
            println!("这是一个测试中文字符显示的测试命令");
            println!("测试在Windows上是否能正常显示中文而不出现乱码");
            println!("如果显示正常，说明我们的修复方案生效了");
            println!("如果仍然显示乱码，我们需要尝试其他方法");
            
            // 如果在Windows上，应该已经在main函数开始执行了chcp 65001命令
            #[cfg(windows)]
            {
                println!("目前在Windows平台上运行");
            }
            
            #[cfg(not(windows))]
            {
                println!("目前在非Windows平台上运行");
            }
        },
        
        Command::Run { daemon } => {
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
            let config = match load_config(&cli) {
                Ok(cfg) => {
                    tracing::info!("成功加载配置");
                    cfg
                },
                Err(e) => {
                    tracing::error!("加载配置失败: {}", e);
                    return Err(e);
                }
            };
            
            // 确保数据目录存在
            if let Err(e) = config.create_data_dir() {
                tracing::error!("创建数据目录失败: {}", e);
                return Err(e);
            }
            
            // 输出启动信息
            tracing::info!("====== librorum 服务启动 ======");
            tracing::info!("配置: {:?}", config);
            
            // 创建节点管理器
            tracing::info!("创建节点管理器...");
            let node_manager = NodeManager::with_config(config);
            
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
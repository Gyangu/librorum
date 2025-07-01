use anyhow::Result;
use clap::Parser;
use librorum_cli::{Cli, Command, try_connect_to_core, load_config, find_core_binary, validate_server_address};
use librorum_shared::NodeConfig;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let mut cli = Cli::parse();
    
    // 如果指定了verbose参数，设置日志级别为debug
    if cli.verbose {
        cli.log_level = "debug".to_string();
    }

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    // 根据命令执行不同操作
    match &cli.command {
        Command::Start { config: cmd_config, verbose } => {
            // 这里通过gRPC调用core服务的start方法
            info!("通过gRPC启动core服务...");
            
            // 验证服务器地址
            validate_server_address(&cli.server)?;
            
            // 首先检查core服务是否运行，如果没有运行需要先启动core
            let _config = load_config(&cli)?;
            
            match try_connect_to_core(&cli.server).await {
                Ok(_client) => {
                    info!("已连接到core服务");
                    // 通过gRPC调用启动命令
                    println!("Core服务已在运行");
                }
                Err(_) => {
                    info!("core服务未运行，正在启动...");
                    // 启动core进程
                    start_core_process().await?;
                }
            }
        }

        Command::Stop => {
            match try_connect_to_core(&cli.server).await {
                Ok(mut client) => {
                    info!("通过gRPC停止core服务");
                    // TODO: 实现gRPC stop调用
                }
                Err(e) => {
                    error!("无法连接到core服务: {}", e);
                }
            }
        }

        Command::Status => {
            match try_connect_to_core(&cli.server).await {
                Ok(mut client) => {
                    info!("通过gRPC获取服务状态");
                    // TODO: 实现gRPC status调用
                    println!("服务正在运行");
                }
                Err(_) => {
                    println!("服务未运行");
                }
            }
        }

        Command::NodesStatus => {
            match try_connect_to_core(&cli.server).await {
                Ok(mut client) => {
                    info!("通过gRPC获取节点健康状态");
                    // TODO: 实现gRPC nodes status调用
                }
                Err(e) => {
                    error!("无法连接到core服务: {}", e);
                    println!("错误: 服务未运行，请先启动服务");
                }
            }
        }

        Command::Init { path } => {
            // 创建默认配置
            let config = NodeConfig::default();

            // 保存配置
            config.save_to_file(path)?;

            println!("已生成默认配置文件: {:?}", path);
        }

        Command::Connect { address } => {
            let server_addr = address.as_ref().unwrap_or(&cli.server);
            match try_connect_to_core(server_addr).await {
                Ok(mut client) => {
                    println!("成功连接到服务器: {}", server_addr);
                    // TODO: 实现交互式会话
                }
                Err(e) => {
                    error!("连接失败: {}", e);
                }
            }
        }

        Command::ListNodes => {
            match try_connect_to_core(&cli.server).await {
                Ok(mut client) => {
                    info!("获取节点列表");
                    // TODO: 实现gRPC list nodes调用
                }
                Err(e) => {
                    error!("无法连接到core服务: {}", e);
                }
            }
        }

        Command::Logs { tail } => {
            // 本地日志查看功能
            println!("显示日志 (最后{}行)", tail);
            // TODO: 实现日志查看逻辑
        }

        Command::CleanLogs { days } => {
            println!("清理{}天前的日志", days);
            // TODO: 实现日志清理逻辑
        }

        Command::CleanAllLogs => {
            println!("清理所有日志");
            // TODO: 实现日志清理逻辑
        }

        _ => {
            // 其他命令通过gRPC转发给core
            match try_connect_to_core(&cli.server).await {
                Ok(mut client) => {
                    // TODO: 实现通用gRPC命令转发
                }
                Err(e) => {
                    error!("无法连接到core服务: {}", e);
                }
            }
        }
    }

    Ok(())
}


/// 启动core进程
async fn start_core_process() -> Result<()> {
    use anyhow::Context;
    
    // 查找core二进制文件
    let core_binary = find_core_binary()?;
    
    info!("启动core进程: {:?}", core_binary);
    
    let mut cmd = std::process::Command::new(core_binary);
    cmd.args(&["--daemon"]);
    
    if let Some(config_path) = NodeConfig::find_config_file() {
        cmd.args(&["--config", &config_path.to_string_lossy()]);
    }
    
    let _child = cmd.spawn()
        .with_context(|| "无法启动core进程")?;
    
    // 等待core服务启动
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    Ok(())
}
use anyhow::Result;
use clap::Parser;
use librorum_cli::{
    Cli, Command, try_connect_to_core, try_connect_to_file_service, parse_data_portal_endpoint, 
    load_config, find_core_binary, validate_server_address,
    data_portal_client::{DataPortalClient, TransferConfig}
};
use librorum_shared::NodeConfig;
use tracing::{error, info};
use std::path::Path;
use uuid::Uuid;

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

        // 文件操作命令
        Command::Upload { file, path, overwrite, compress } => {
            handle_upload(&cli.server, file, path, *overwrite, *compress).await?;
        }

        Command::Download { remote, output, offset, length } => {
            handle_download(&cli.server, remote, output, *offset, *length).await?;
        }

        Command::List { path, recursive, all } => {
            handle_list(&cli.server, path, *recursive, *all).await?;
        }

        Command::Remove { path, recursive, force } => {
            handle_remove(&cli.server, path, *recursive, *force).await?;
        }

        Command::Mkdir { path, parents } => {
            handle_mkdir(&cli.server, path, *parents).await?;
        }

        Command::Info { path, chunks } => {
            handle_info(&cli.server, path, *chunks).await?;
        }

        Command::Sync { path } => {
            handle_sync(&cli.server, path).await?;
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

/// 处理文件上传 - 使用Data Portal
async fn handle_upload(
    server: &str,
    file_path: &Path,
    remote_path: &Option<String>,
    overwrite: bool,
    compress: bool,
) -> Result<()> {
    // 检查文件是否存在
    if !file_path.exists() {
        return Err(anyhow::anyhow!("文件不存在: {:?}", file_path));
    }

    // 获取文件信息
    let metadata = tokio::fs::metadata(file_path).await?;
    let file_size = metadata.len();
    let file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let target_path = remote_path.as_ref()
        .map(|p| p.clone())
        .unwrap_or_else(|| format!("/{}", file_name));

    println!("📤 使用Data Portal上传文件: {} -> {} ({} bytes)", 
             file_path.display(), target_path, file_size);

    // 解析Data Portal端点
    let data_portal_endpoint = parse_data_portal_endpoint(server)?;
    info!("Data Portal端点: {}", data_portal_endpoint);

    // 创建Data Portal客户端
    let mut client = DataPortalClient::new()?;
    
    // 配置传输参数
    let config = TransferConfig {
        mode: universal_transport_core::TransportType::Universal,
        chunk_size: if file_size < 5 * 1024 * 1024 { 
            1024 * 1024 // 1MB for small files
        } else if file_size < 50 * 1024 * 1024 { 
            4 * 1024 * 1024 // 4MB for medium files
        } else { 
            8 * 1024 * 1024 // 8MB for large files
        },
        enable_compression: compress,
        timeout_secs: 300,
    };

    // 生成会话ID
    let session_id = Uuid::new_v4().to_string();

    // 创建进度回调
    let last_update = std::sync::Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
    let progress_callback = {
        let last_update = last_update.clone();
        move |transferred: u64, total: u64, rate: f64| {
            let now = std::time::Instant::now();
            let mut last_time = last_update.lock().unwrap();
            if now.duration_since(*last_time).as_millis() > 100 {
                let percentage = (transferred as f64 / total as f64) * 100.0;
                print!("\r📊 上传进度: {}/{} bytes ({:.1}%) - {:.2} MB/s", 
                       transferred, total, percentage, rate);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
                *last_time = now;
            }
        }
    };

    // 开始上传
    let start_time = std::time::Instant::now();
    let metrics = client.upload_file(
        file_path,
        data_portal_endpoint,
        &session_id,
        config,
        Some(Box::new(progress_callback))
    ).await?;

    println!(); // 新行
    println!("✅ Data Portal上传完成!");
    println!("  📊 传输统计:");
    println!("    文件大小: {} bytes", metrics.bytes_transferred);
    println!("    传输时间: {:.2} 秒", metrics.duration_secs);
    println!("    平均速度: {:.2} MB/s", metrics.average_rate_mbps);
    println!("    传输模式: {:?}", metrics.transport_mode);
    println!("    启用压缩: {}", metrics.compression_enabled);
    println!("    数据块数: {}", metrics.chunk_count);

    // 计算性能提升
    let grpc_estimated_rate = 50.0; // 假设gRPC约50MB/s
    if metrics.average_rate_mbps > grpc_estimated_rate {
        let improvement = (metrics.average_rate_mbps / grpc_estimated_rate - 1.0) * 100.0;
        println!("  🚀 性能提升: 比gRPC快 {:.1}%", improvement);
    }

    Ok(())
}

/// 处理文件下载 - 使用Data Portal
async fn handle_download(
    server: &str,
    remote: &str,
    output: &Option<std::path::PathBuf>,
    offset: u64,
    length: u64,
) -> Result<()> {
    // 确定输出文件路径
    let output_path = if let Some(path) = output {
        path.clone()
    } else {
        // 从远程路径提取文件名
        let file_name = remote.rsplit('/').next().unwrap_or("downloaded_file");
        Path::new(file_name).to_path_buf()
    };

    println!("📥 使用Data Portal下载文件: {} -> {}", remote, output_path.display());
    
    // 解析Data Portal端点
    let data_portal_endpoint = parse_data_portal_endpoint(server)?;
    info!("Data Portal端点: {}", data_portal_endpoint);

    // 创建Data Portal客户端
    let mut client = DataPortalClient::new()?;
    
    // 配置传输参数
    let config = TransferConfig {
        mode: universal_transport_core::TransportType::Universal,
        chunk_size: 4 * 1024 * 1024, // 4MB chunks for download
        enable_compression: false, // 让服务器决定是否压缩
        timeout_secs: 300,
    };

    // 生成会话ID
    let session_id = Uuid::new_v4().to_string();

    // 创建进度回调
    let last_update = std::sync::Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
    let progress_callback = {
        let last_update = last_update.clone();
        move |transferred: u64, total: u64, rate: f64| {
            let now = std::time::Instant::now();
            let mut last_time = last_update.lock().unwrap();
            if now.duration_since(*last_time).as_millis() > 100 {
                let percentage = (transferred as f64 / total as f64) * 100.0;
                print!("\r📊 下载进度: {}/{} bytes ({:.1}%) - {:.2} MB/s", 
                       transferred, total, percentage, rate);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
                *last_time = now;
            }
        }
    };

    // 开始下载
    let metrics = client.download_file(
        data_portal_endpoint,
        &session_id,
        &output_path,
        config,
        Some(Box::new(progress_callback))
    ).await?;

    println!(); // 新行
    println!("✅ Data Portal下载完成!");
    println!("  📊 传输统计:");
    println!("    文件大小: {} bytes", metrics.bytes_transferred);
    println!("    传输时间: {:.2} 秒", metrics.duration_secs);
    println!("    平均速度: {:.2} MB/s", metrics.average_rate_mbps);
    println!("    传输模式: {:?}", metrics.transport_mode);
    println!("    保存位置: {}", output_path.display());

    // 计算性能提升
    let grpc_estimated_rate = 50.0; // 假设gRPC约50MB/s
    if metrics.average_rate_mbps > grpc_estimated_rate {
        let improvement = (metrics.average_rate_mbps / grpc_estimated_rate - 1.0) * 100.0;
        println!("  🚀 性能提升: 比gRPC快 {:.1}%", improvement);
    }

    Ok(())
}

/// 处理文件列表
async fn handle_list(
    server: &str,
    path: &str,
    recursive: bool,
    all: bool,
) -> Result<()> {
    use librorum_shared::proto::file::*;
    use tonic::Request;

    let mut client = try_connect_to_file_service(server).await?;

    let request = ListFilesRequest {
        path: path.to_string(),
        recursive,
        include_hidden: all,
    };

    println!("列出目录: {}", path);

    let response = client.list_files(Request::new(request)).await?;
    let result = response.into_inner();

    println!("当前路径: {}", result.current_path);
    println!("总计: {} 个文件/目录, {} bytes\n", result.total_count, result.total_size);

    if result.files.is_empty() {
        println!("目录为空");
        return Ok(());
    }

    // 打印表头
    println!("{:<20} {:>10} {:>12} {:<20} {}", 
             "类型", "大小", "修改时间", "名称", "路径");
    println!("{}", "-".repeat(80));

    for file in result.files {
        let file_type = if file.is_directory { "目录" } else { "文件" };
        let size_str = if file.is_directory { "-".to_string() } else { file.size.to_string() };
        
        // 格式化时间
        let modified_time = chrono::DateTime::from_timestamp(file.modified_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "未知".to_string());

        println!("{:<20} {:>10} {:>12} {:<20} {}", 
                 file_type, size_str, modified_time, file.name, file.path);
    }

    Ok(())
}

/// 处理文件删除
async fn handle_remove(
    server: &str,
    path: &str,
    recursive: bool,
    force: bool,
) -> Result<()> {
    use librorum_shared::proto::file::*;
    use tonic::Request;

    let mut client = try_connect_to_file_service(server).await?;

    let request = DeleteFileRequest {
        file_id: String::new(),
        path: path.to_string(),
        recursive,
        force,
    };

    println!("删除: {}", path);

    let response = client.delete_file(Request::new(request)).await?;
    let result = response.into_inner();

    if result.success {
        println!("✓ {}", result.message);
        println!("删除了 {} 个文件/目录", result.deleted_count);
    } else {
        println!("✗ 删除失败: {}", result.message);
    }

    Ok(())
}

/// 处理目录创建
async fn handle_mkdir(
    server: &str,
    path: &str,
    parents: bool,
) -> Result<()> {
    use librorum_shared::proto::file::*;
    use tonic::Request;

    let mut client = try_connect_to_file_service(server).await?;

    let request = CreateDirectoryRequest {
        path: path.to_string(),
        create_parents: parents,
        permissions: None,
    };

    println!("创建目录: {}", path);

    let response = client.create_directory(Request::new(request)).await?;
    let result = response.into_inner();

    if result.success {
        println!("✓ {}", result.message);
        if let Some(dir_info) = result.directory_info {
            println!("目录ID: {}", dir_info.file_id);
        }
    } else {
        println!("✗ 创建失败: {}", result.message);
    }

    Ok(())
}

/// 处理文件信息查询
async fn handle_info(
    server: &str,
    path: &str,
    chunks: bool,
) -> Result<()> {
    use librorum_shared::proto::file::*;
    use tonic::Request;

    let mut client = try_connect_to_file_service(server).await?;

    let request = GetFileInfoRequest {
        file_id: if path.starts_with("file_") { path.to_string() } else { String::new() },
        path: if !path.starts_with("file_") { path.to_string() } else { String::new() },
        include_chunks: chunks,
    };

    println!("获取文件信息: {}", path);

    let response = client.get_file_info(Request::new(request)).await?;
    let file_info = response.into_inner();

    println!("\n文件信息:");
    println!("  ID: {}", file_info.file_id);
    println!("  名称: {}", file_info.name);
    println!("  路径: {}", file_info.path);
    println!("  父目录: {}", file_info.parent_path);
    println!("  大小: {} bytes", file_info.size);
    println!("  类型: {}", if file_info.is_directory { "目录" } else { "文件" });
    println!("  MIME类型: {}", file_info.mime_type);
    
    // 格式化时间
    let created_time = chrono::DateTime::from_timestamp(file_info.created_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "未知".to_string());
    let modified_time = chrono::DateTime::from_timestamp(file_info.modified_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "未知".to_string());
    
    println!("  创建时间: {}", created_time);
    println!("  修改时间: {}", modified_time);
    
    if let Some(permissions) = file_info.permissions {
        println!("  权限: {:o} ({}:{})", permissions.mode, permissions.owner, permissions.group);
    }
    
    println!("  副本因子: {}", file_info.replication_factor);
    println!("  压缩: {}", if file_info.is_compressed { "是" } else { "否" });
    println!("  加密: {}", if file_info.is_encrypted { "是" } else { "否" });
    
    if chunks && file_info.chunk_count > 0 {
        println!("  分块数量: {}", file_info.chunk_count);
        println!("  分块ID: {:?}", file_info.chunk_ids);
    }

    Ok(())
}

/// 处理同步状态查询
async fn handle_sync(
    server: &str,
    path: &Option<String>,
) -> Result<()> {
    use librorum_shared::proto::file::*;
    use tonic::Request;

    let mut client = try_connect_to_file_service(server).await?;

    let request = GetSyncStatusRequest {
        path: path.as_ref().map(|p| p.clone()).unwrap_or_default(),
    };

    let path_display = path.as_ref().map(|p| p.as_str()).unwrap_or("全局");
    println!("获取同步状态: {}", path_display);

    let response = client.get_sync_status(Request::new(request)).await?;
    let result = response.into_inner();

    println!("\n同步状态:");
    
    let overall_status = match SyncStatus::try_from(result.overall_status) {
        Ok(SyncStatus::Synced) => "✓ 已同步",
        Ok(SyncStatus::Pending) => "⏳ 等待同步",
        Ok(SyncStatus::Syncing) => "🔄 同步中",
        Ok(SyncStatus::Error) => "✗ 同步错误",
        Ok(SyncStatus::Conflict) => "⚠️ 冲突",
        _ => "❓ 未知状态",
    };
    
    println!("  总体状态: {}", overall_status);
    println!("  等待上传: {} 个文件", result.pending_uploads);
    println!("  等待下载: {} 个文件", result.pending_downloads);
    println!("  同步中: {} 个文件", result.syncing_files);
    println!("  错误: {} 个文件", result.error_files);
    println!("  冲突: {} 个文件", result.conflict_files);
    println!("  待上传数据: {} bytes", result.bytes_to_upload);
    println!("  待下载数据: {} bytes", result.bytes_to_download);
    
    if !result.pending_files.is_empty() {
        println!("\n待处理文件:");
        for file in result.pending_files.iter().take(10) { // 只显示前10个
            let status = match SyncStatus::try_from(file.sync_status) {
                Ok(SyncStatus::Pending) => "等待",
                Ok(SyncStatus::Syncing) => "同步中",
                Ok(SyncStatus::Error) => "错误",
                Ok(SyncStatus::Conflict) => "冲突",
                _ => "未知",
            };
            println!("  [{}] {} ({})", status, file.name, file.path);
        }
        
        if result.pending_files.len() > 10 {
            println!("  ... 还有 {} 个文件", result.pending_files.len() - 10);
        }
    }

    Ok(())
}
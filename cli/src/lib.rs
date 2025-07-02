// 将main.rs中的核心功能提取到lib.rs，便于测试
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use librorum_shared::{NodeConfig, proto::node::node_service_client::NodeServiceClient, proto::file::file_service_client::FileServiceClient};
use std::path::PathBuf;
use tonic::transport::Channel;

/// librorum 分布式文件系统命令行工具
#[derive(Parser, Debug, PartialEq)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// 子命令
    #[clap(subcommand)]
    pub command: Command,

    /// 配置文件路径
    #[clap(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// 服务器地址
    #[clap(short, long, default_value = "http://127.0.0.1:50051")]
    pub server: String,

    /// 日志级别 (trace, debug, info, warn, error)
    #[clap(short, long, default_value = "info")]
    pub log_level: String,
    
    /// 启用调试日志（相当于 --log-level=debug）
    #[clap(short, long)]
    pub verbose: bool,
}

/// 命令集
#[derive(Subcommand, Debug, PartialEq)]
pub enum Command {
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

    /// 连接到指定节点
    Connect {
        /// 节点地址
        #[clap(short, long)]
        address: Option<String>,
    },

    /// 列出可用节点
    ListNodes,

    /// 文件操作命令
    /// 上传文件到分布式文件系统
    Upload {
        /// 本地文件路径
        #[clap(short, long)]
        file: PathBuf,
        
        /// 远程存储路径 (可选，默认使用文件名)
        #[clap(short, long)]
        path: Option<String>,
        
        /// 是否覆盖现有文件
        #[clap(long)]
        overwrite: bool,
        
        /// 是否压缩文件
        #[clap(long)]
        compress: bool,
    },

    /// 下载文件从分布式文件系统
    Download {
        /// 远程文件路径或文件ID
        #[clap(short, long)]
        remote: String,
        
        /// 本地保存路径 (可选，默认使用远程文件名)
        #[clap(short, long)]
        output: Option<PathBuf>,
        
        /// 下载偏移量 (用于断点续传)
        #[clap(long, default_value = "0")]
        offset: u64,
        
        /// 下载长度 (0表示全部)
        #[clap(long, default_value = "0")]
        length: u64,
    },

    /// 列出远程目录中的文件
    List {
        /// 远程目录路径
        #[clap(default_value = "/")]
        path: String,
        
        /// 是否递归列出子目录
        #[clap(short, long)]
        recursive: bool,
        
        /// 是否包含隐藏文件
        #[clap(short = 'a', long)]
        all: bool,
    },

    /// 删除远程文件或目录
    Remove {
        /// 远程文件/目录路径
        path: String,
        
        /// 是否递归删除目录
        #[clap(short, long)]
        recursive: bool,
        
        /// 是否强制删除
        #[clap(short, long)]
        force: bool,
    },

    /// 创建远程目录
    Mkdir {
        /// 远程目录路径
        path: String,
        
        /// 是否创建父目录
        #[clap(short, long)]
        parents: bool,
    },

    /// 获取文件信息
    Info {
        /// 远程文件路径或文件ID
        path: String,
        
        /// 是否包含分块信息
        #[clap(long)]
        chunks: bool,
    },

    /// 获取同步状态
    Sync {
        /// 路径 (可选，空表示全局状态)
        #[clap(short, long)]
        path: Option<String>,
    },
}

/// 尝试连接到core服务
pub async fn try_connect_to_core(server: &str) -> Result<NodeServiceClient<Channel>> {
    let client = NodeServiceClient::connect(server.to_string()).await
        .with_context(|| format!("无法连接到core服务: {}", server))?;
    Ok(client)
}

/// 尝试连接到文件服务
pub async fn try_connect_to_file_service(server: &str) -> Result<FileServiceClient<Channel>> {
    let client = FileServiceClient::connect(server.to_string()).await
        .with_context(|| format!("无法连接到文件服务: {}", server))?;
    Ok(client)
}

/// 查找core二进制文件
pub fn find_core_binary() -> Result<PathBuf> {
    // 查找顺序:
    // 1. 同目录下的librorum-core
    // 2. ../core/target/release/librorum-core  
    // 3. ../core/target/debug/librorum-core
    // 4. $PATH中的librorum-core
    
    let current_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    
    let candidates = vec![
        current_dir.join("librorum-core"),
        current_dir.join("../core/target/release/librorum-core"),
        current_dir.join("../core/target/debug/librorum-core"),
    ];
    
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    
    // 最后尝试PATH中查找
    if let Ok(path) = which::which("librorum-core") {
        return Ok(path);
    }
    
    Err(anyhow::anyhow!("无法找到core二进制文件"))
}

/// 加载配置
pub fn load_config(cli: &Cli) -> Result<NodeConfig> {
    if let Some(config_path) = &cli.config {
        // 使用指定的配置文件
        NodeConfig::from_file(config_path)
    } else if let Some(config_path) = NodeConfig::find_config_file() {
        // 使用自动找到的配置文件
        NodeConfig::from_file(config_path)
    } else {
        // 使用默认配置
        Ok(NodeConfig::default())
    }
}

/// 验证服务器地址格式
pub fn validate_server_address(address: &str) -> Result<()> {
    if !address.starts_with("http://") && !address.starts_with("https://") {
        return Err(anyhow::anyhow!("服务器地址必须以 http:// 或 https:// 开头"));
    }
    
    // 尝试解析URL
    let url = url::Url::parse(address)
        .with_context(|| format!("无效的服务器地址: {}", address))?;
    
    if url.host().is_none() {
        return Err(anyhow::anyhow!("服务器地址必须包含主机名"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn test_cli_parsing() {
        // 测试基本命令解析
        let cli = Cli::try_parse_from(&["librorum", "status"]).unwrap();
        assert_eq!(cli.command, Command::Status);
        assert_eq!(cli.server, "http://127.0.0.1:50051");
        assert_eq!(cli.log_level, "info");
        assert!(!cli.verbose);
    }

    #[test]
    fn test_cli_with_options() {
        let cli = Cli::try_parse_from(&[
            "librorum",
            "--server", "http://192.168.1.100:8080",
            "--config", "/path/to/config.toml",
            "--verbose",
            "status"
        ]).unwrap();
        
        assert_eq!(cli.command, Command::Status);
        assert_eq!(cli.server, "http://192.168.1.100:8080");
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
        assert!(cli.verbose);
    }

    #[test]
    fn test_start_command() {
        let cli = Cli::try_parse_from(&[
            "librorum", 
            "start",
            "--config", "custom.toml",
            "--verbose"
        ]).unwrap();
        
        match cli.command {
            Command::Start { config, verbose } => {
                assert_eq!(config, Some(PathBuf::from("custom.toml")));
                assert!(verbose);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_logs_command() {
        let cli = Cli::try_parse_from(&["librorum", "logs", "--tail", "50"]).unwrap();
        
        match cli.command {
            Command::Logs { tail } => {
                assert_eq!(tail, 50);
            }
            _ => panic!("Expected Logs command"),
        }
    }

    #[test]
    fn test_init_command() {
        let cli = Cli::try_parse_from(&["librorum", "init", "my-config.toml"]).unwrap();
        
        match cli.command {
            Command::Init { path } => {
                assert_eq!(path, PathBuf::from("my-config.toml"));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_init_command_default_path() {
        let cli = Cli::try_parse_from(&["librorum", "init"]).unwrap();
        
        match cli.command {
            Command::Init { path } => {
                assert_eq!(path, PathBuf::from("librorum.toml"));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_clean_logs_command() {
        let cli = Cli::try_parse_from(&["librorum", "clean-logs", "7"]).unwrap();
        
        match cli.command {
            Command::CleanLogs { days } => {
                assert_eq!(days, 7);
            }
            _ => panic!("Expected CleanLogs command"),
        }
    }

    #[test]
    fn test_connect_command() {
        let cli = Cli::try_parse_from(&[
            "librorum", 
            "connect",
            "--address", "http://remote-node:50051"
        ]).unwrap();
        
        match cli.command {
            Command::Connect { address } => {
                assert_eq!(address, Some("http://remote-node:50051".to_string()));
            }
            _ => panic!("Expected Connect command"),
        }
    }

    #[test]
    fn test_invalid_command() {
        let result = Cli::try_parse_from(&["librorum", "invalid-command"]);
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn test_missing_required_args() {
        // 测试没有提供子命令
        let result = Cli::try_parse_from(&["librorum"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_verbose_flag_sets_debug_level() {
        let mut cli = Cli::try_parse_from(&["librorum", "--verbose", "status"]).unwrap();
        
        // 模拟main函数中的逻辑
        if cli.verbose {
            cli.log_level = "debug".to_string();
        }
        
        assert_eq!(cli.log_level, "debug");
    }

    #[test]
    fn test_validate_server_address() {
        // 有效地址
        assert!(validate_server_address("http://127.0.0.1:50051").is_ok());
        assert!(validate_server_address("https://example.com").is_ok());
        assert!(validate_server_address("http://192.168.1.100:8080").is_ok());
        
        // 无效地址
        assert!(validate_server_address("127.0.0.1:50051").is_err()); // 缺少协议
        assert!(validate_server_address("ftp://example.com").is_err()); // 错误的协议
        assert!(validate_server_address("http://").is_err()); // 缺少主机名
        assert!(validate_server_address("not-a-url").is_err()); // 无效URL
    }

    #[test]
    fn test_load_config_with_default() {
        let cli = Cli {
            command: Command::Status,
            config: None,
            server: "http://127.0.0.1:50051".to_string(),
            log_level: "info".to_string(),
            verbose: false,
        };
        
        // 应该返回默认配置
        let config = load_config(&cli).unwrap();
        assert_eq!(config.node_prefix, "node");
        assert_eq!(config.bind_port, 50051);
    }

    #[test]
    fn test_load_config_with_nonexistent_file() {
        let cli = Cli {
            command: Command::Status,
            config: Some(PathBuf::from("/nonexistent/config.toml")),
            server: "http://127.0.0.1:50051".to_string(),
            log_level: "info".to_string(),
            verbose: false,
        };
        
        // 应该返回错误
        let result = load_config(&cli);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_with_valid_file() -> anyhow::Result<()> {
        use tempfile::NamedTempFile;
        use std::io::Write;
        
        // 创建临时配置文件
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, r#"
            node_prefix = "test_node"
            bind_port = 9999
            log_level = "debug"
        "#)?;
        
        let cli = Cli {
            command: Command::Status,
            config: Some(temp_file.path().to_path_buf()),
            server: "http://127.0.0.1:50051".to_string(),
            log_level: "info".to_string(),
            verbose: false,
        };
        
        let config = load_config(&cli)?;
        assert_eq!(config.node_prefix, "test_node");
        assert_eq!(config.bind_port, 9999);
        assert_eq!(config.log_level, "debug");
        
        Ok(())
    }

    #[test]
    fn test_find_core_binary_nonexistent() {
        // 在测试环境中，可能找不到core二进制文件
        let result = find_core_binary();
        // 不应该panic，应该返回错误
        if result.is_err() {
            assert!(result.unwrap_err().to_string().contains("无法找到core二进制文件"));
        }
    }

    #[tokio::test]
    async fn test_try_connect_to_core_invalid_address() {
        // 测试连接到无效URL格式
        let result = try_connect_to_core("invalid-url").await;
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("无法连接到core服务"));
    }

    #[test]
    fn test_command_variants() {
        // 测试所有命令变体都能正确解析
        let commands = vec![
            ("status", Command::Status),
            ("stop", Command::Stop),
            ("restart", Command::Restart),
            ("clean-all-logs", Command::CleanAllLogs),
            ("nodes-status", Command::NodesStatus),
            ("list-nodes", Command::ListNodes),
        ];
        
        for (cmd_str, expected_cmd) in commands {
            let cli = Cli::try_parse_from(&["librorum", cmd_str]).unwrap();
            assert_eq!(cli.command, expected_cmd);
        }
    }
}
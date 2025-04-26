use crate::cli::Cli;
use crate::config::NodeConfig;
use std::path::PathBuf;
use tokio::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tempfile::tempdir;
use clap::Parser;
use crate::cli::Commands;

#[tokio::test]
async fn test_cli_commands() {
    // Create a test configuration
    let config = NodeConfig {
        id: "test-node".to_string(),
        name: "Test Node".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        root_dir: PathBuf::from("test_data"),
        max_file_size: 1024 * 1024,
        chunk_size: 1024,
        workers: 1,
    };

    // Save the configuration
    let config_path = PathBuf::from("test_config.toml");
    std::fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();

    // Start the server
    let server_handle = tokio::spawn(async move {
        let addr = format!("{}:{}", config.host, config.port);
        crate::start_server(config, &addr).await.unwrap();
    });

    // Wait for the server to start
    sleep(Duration::from_secs(1)).await;

    // Test file operations
    let mut client = crate::client::VDFSClient::new(format!("http://localhost:50051")).await.unwrap();

    // Create a file
    client.create_file("test.txt".to_string(), "test-node".to_string()).await.unwrap();

    // Write to the file
    client.write_file(
        "test.txt".to_string(),
        "test-node".to_string(),
        "Hello, World!".as_bytes().to_vec()
    ).await.unwrap();

    // Read the file
    let content = client.read_file(
        "test.txt".to_string(),
        "test-node".to_string(),
        0,
        -1
    ).await.unwrap();
    assert_eq!(content, "Hello, World!".as_bytes());

    // List directory
    let entries = client.list_directory(".".to_string(), "test-node".to_string()).await.unwrap();
    assert!(entries.contains(&"test.txt".to_string()));

    // Delete the file
    client.delete_file("test.txt".to_string(), "test-node".to_string()).await.unwrap();

    // Clean up
    std::fs::remove_file(&config_path).unwrap();
    std::fs::remove_dir_all("test_data").unwrap();

    // Stop the server
    server_handle.abort();
}

#[tokio::test]
async fn test_cli_basic() {
    // 创建临时目录
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 测试启动命令
    let cli = Cli::parse_from(&["vdfs", "start", "-c", config_path.to_str().unwrap()]);
    match cli.command {
        Commands::Start { config } => {
            assert_eq!(config.unwrap(), config_path);
        }
        _ => panic!("Expected Start command"),
    }

    // 测试停止命令
    let cli = Cli::parse_from(&["vdfs", "stop", "-n", "test_node"]);
    match cli.command {
        Commands::Stop { node_id } => {
            assert_eq!(node_id, "test_node");
        }
        _ => panic!("Expected Stop command"),
    }

    // 测试状态命令
    let cli = Cli::parse_from(&["vdfs", "status", "-n", "test_node"]);
    match cli.command {
        Commands::Status { node_id } => {
            assert_eq!(node_id, "test_node");
        }
        _ => panic!("Expected Status command"),
    }
} 
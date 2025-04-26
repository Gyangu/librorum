use crate::cli::Cli;
use crate::config::NodeConfig;
use std::path::PathBuf;
use tokio::process::Command;
use std::time::Duration;
use tokio::time::sleep;

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
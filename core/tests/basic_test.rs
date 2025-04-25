use librorum_core::{config::{NodeConfig, ClusterConfig}, start_server};
use tokio;
use librorum_core::proto::vdfs::{
    vdfs_service_client::VdfsServiceClient,
    CreateFileRequest, ReadFileRequest, WriteFileRequest,
    ListDirectoryRequest, GetFileInfoRequest, FileType,
};
use std::path::PathBuf;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[tokio::test]
async fn test_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // 创建临时目录用于测试
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path();
    
    // 创建测试配置
    let node_config = NodeConfig {
        id: "test-node-1".to_string(),
        name: "Test Node".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        root_dir: test_dir.to_path_buf(),
        max_file_size: 1024 * 1024 * 1024, // 1GB
        chunk_size: 1024 * 1024, // 1MB
        workers: 2,
    };

    let cluster_config = ClusterConfig {
        sync_interval: 60,
        p2p_enabled: true,
        nodes: vec![node_config.clone()],
    };

    // 创建数据目录和数据库目录
    let root_dir = &node_config.root_dir;
    fs::create_dir_all(root_dir)?;
    
    // 确保数据库目录存在并设置正确的权限
    let db_dir = root_dir.join("db");
    fs::create_dir_all(&db_dir)?;
    let mut perms = fs::metadata(&db_dir)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&db_dir, perms)?;

    // 启动服务器
    let server_handle = tokio::spawn({
        let node_config = node_config.clone();
        let cluster_config = cluster_config.clone();
        async move {
            if let Err(e) = start_server(node_config, cluster_config).await {
                eprintln!("Server error: {}", e);
            }
        }
    });

    // 等待服务器启动
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // 创建客户端并重试连接
    let mut client = None;
    for _ in 0..5 {
        match VdfsServiceClient::connect("http://127.0.0.1:50051").await {
            Ok(c) => {
                client = Some(c);
                break;
            }
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    let mut client = client.ok_or("Failed to connect to server")?;

    // 1. 测试文件创建
    let create_response = client
        .create_file(CreateFileRequest {
            path: "test.txt".to_string(),
            node_id: "test-node-1".to_string(),
            r#type: FileType::File as i32,
        })
        .await?;
    assert!(create_response.get_ref().info.is_some());

    // 2. 测试文件写入
    let write_response = client
        .write_file(tonic::Request::new(tokio_stream::iter(vec![
            WriteFileRequest {
                path: "test.txt".to_string(),
                data: "Hello, World!".as_bytes().to_vec(),
                node_id: "test-node-1".to_string(),
                offset: 0,
            },
        ])))
        .await?;
    assert_eq!(write_response.get_ref().bytes_written, 13);

    // 3. 测试文件读取
    let read_response = client
        .read_file(ReadFileRequest {
            path: "test.txt".to_string(),
            node_id: "test-node-1".to_string(),
            offset: 0,
            length: 13,
        })
        .await?;
    let mut stream = read_response.into_inner();
    let response = stream.message().await?.unwrap();
    assert_eq!(response.data, "Hello, World!".as_bytes());

    // 4. 测试目录列表
    let list_response = client
        .list_directory(ListDirectoryRequest {
            path: ".".to_string(),
            node_id: "test-node-1".to_string(),
        })
        .await?;
    assert!(!list_response.get_ref().entries.is_empty());

    // 5. 测试文件信息获取
    let info_response = client
        .get_file_info(GetFileInfoRequest {
            path: "test.txt".to_string(),
            node_id: "test-node-1".to_string(),
        })
        .await?;
    assert!(info_response.get_ref().info.is_some());

    // 清理
    server_handle.abort();
    Ok(())
} 
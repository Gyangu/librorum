use librorum_core::proto::vdfs::{
    CreateFileRequest, CreateFileResponse,
    WriteFileRequest, WriteFileResponse,
    ReadFileRequest, ReadFileResponse,
    ListDirectoryRequest, ListDirectoryResponse,
    FileType, NodeStatus, NodeInfo, FileInfo,
    vdfs_service_server::VdfsService,
    RegisterNodeRequest, RegisterNodeResponse,
    GetNodeStatusRequest, GetNodeStatusResponse,
    HeartbeatRequest, HeartbeatResponse,
    JoinClusterRequest, JoinClusterResponse,
    LeaveClusterRequest, LeaveClusterResponse,
    GetFileInfoRequest, GetFileInfoResponse,
    DeleteFileRequest, DeleteFileResponse,
    MoveFileRequest, MoveFileResponse,
    CopyFileRequest, CopyFileResponse,
    SyncMetadataRequest, SyncMetadataResponse,
    DropFileRequest, DropFileResponse,
    ReceiveFileRequest, ReceiveFileResponse,
    GetClusterInfoRequest, GetClusterInfoResponse,
    UpdateNodeStatusRequest, UpdateNodeStatusResponse,
    DiscoverNodesRequest, DiscoverNodesResponse,
    ClusterInfo,
    CreateDirectoryRequest, CreateDirectoryResponse,
    GetFileRequest, GetFileResponse,
    UnregisterNodeRequest, UnregisterNodeResponse,
};
use tokio::sync::{mpsc, Mutex};
use std::{sync::Arc, collections::HashMap};
use tonic::{Request, Response, Status, Streaming};
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use chrono::Utc;
use librorum_core::service::VDFSServiceImpl;
use librorum_core::fs::LocalFileSystem;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;
use futures::Stream;
use std::pin::Pin;
use prost::Message;
use prost_codec::ProstCodec;

#[derive(Debug, Clone)]
struct TestNodeConfig {
    name: String,
    description: String,
    capacity: u64,
}

#[derive(Debug, Clone)]
struct TestNodeInfo {
    id: String,
    config: TestNodeConfig,
    online: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TestNodeStatus {
    NodeOnline,
    NodeOffline,
}

#[derive(Clone)]
struct SimpleTestNode {
    service: Arc<VDFSServiceImpl>,
    node_info: NodeInfo,
}

impl SimpleTestNode {
    async fn setup(node_id: &str, port: u16) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let node_info = NodeInfo {
            id: node_id.to_string(),
            name: format!("Test Node {}", node_id),
            host: "127.0.0.1".to_string(),
            port: port as i32,
            status: NodeStatus::NodeOnline as i32,
            last_seen: Utc::now().timestamp(),
        };

        let fs = LocalFileSystem::new(format!("test_data_{}", node_id)).await?;
        let service = VDFSServiceImpl::new(fs);
        Ok(Self {
            service: Arc::new(service),
            node_info,
        })
    }

    async fn write_file(&self, path: &str, data: Vec<u8>) -> Result<WriteFileResponse, Status> {
        let (tx, rx) = mpsc::channel(32);
        let stream = ReceiverStream::new(rx);
        
        tx.send(WriteFileRequest {
            node_id: self.node_info.id.clone(),
            path: path.to_string(),
            data,
            offset: 0,
        }).await.unwrap();

        let request = Request::new(Streaming::from_receiver(rx));
        self.service.write_file(request).await.map(|r| r.into_inner())
    }
}

#[derive(Debug)]
struct MockNodeService {
    node_info: Arc<Mutex<NodeInfo>>,
    config: TestNodeConfig,
}

impl MockNodeService {
    fn new(id: String, name: String, description: String, capacity: u64) -> Self {
        let config = TestNodeConfig {
            name: name.clone(),
            description: description.clone(),
            capacity,
        };

        let node_info = NodeInfo {
            id,
            name,
            host: "127.0.0.1".to_string(),
            port: 50051,
            status: NodeStatus::NodeOnline as i32,
            last_seen: 0,
        };

        Self {
            node_info: Arc::new(Mutex::new(node_info)),
            config,
        }
    }

    async fn get_node_info(&self) -> NodeInfo {
        self.node_info.lock().await.clone()
    }

    async fn set_status(&self, status: TestNodeStatus) {
        self.node_info.lock().await.status = match status {
            TestNodeStatus::NodeOnline => NodeStatus::NodeOnline as i32,
            TestNodeStatus::NodeOffline => NodeStatus::NodeOffline as i32,
        };
    }

    async fn create_file(
        &self,
        request: Request<CreateFileRequest>,
    ) -> Result<Response<CreateFileResponse>, Status> {
        let request = request.into_inner();
        
        // Validate path
        if request.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        // Create file info
        let file_info = FileInfo {
            id: format!("file_{}", Utc::now().timestamp()),
            name: request.path.split('/').last().unwrap_or("").to_string(),
            path: request.path.clone(),
            r#type: request.r#type,
            size: 0,
            created_at: Utc::now().timestamp(),
            modified_at: Utc::now().timestamp(),
            accessed_at: Utc::now().timestamp(),
            owner_node: request.node_id.clone(),
            available_nodes: vec![request.node_id],
            attributes: HashMap::new(),
        };

        Ok(Response::new(CreateFileResponse {
            info: Some(file_info),
        }))
    }

    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        let request = request.into_inner();
        
        // Validate path
        if request.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        // Create file info
        let file_info = FileInfo {
            id: format!("dir_{}", Utc::now().timestamp()),
            name: request.path.split('/').last().unwrap_or("").to_string(),
            path: request.path.clone(),
            r#type: FileType::Directory as i32,
            size: 0,
            created_at: Utc::now().timestamp(),
            modified_at: Utc::now().timestamp(),
            accessed_at: Utc::now().timestamp(),
            owner_node: request.node_id.clone(),
            available_nodes: vec![request.node_id],
            attributes: HashMap::new(),
        };

        Ok(Response::new(CreateDirectoryResponse {
            info: Some(file_info),
        }))
    }
}

#[tonic::async_trait]
impl VdfsService for MockNodeService {
    type ReadFileStream = ReceiverStream<Result<ReadFileResponse, Status>>;
    type SyncMetadataStream = ReceiverStream<Result<SyncMetadataResponse, Status>>;
    type DropFileStream = ReceiverStream<Result<DropFileResponse, Status>>;
    type WriteFileStream = ReceiverStream<Result<WriteFileResponse, Status>>;

    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let node_info = request.into_inner().node_info.unwrap();
        *self.node_info.lock().await = node_info.clone();
        Ok(Response::new(RegisterNodeResponse {
            success: true,
            node_info: Some(node_info),
            error: String::new(),
        }))
    }

    async fn get_node_status(
        &self,
        _request: Request<GetNodeStatusRequest>,
    ) -> Result<Response<GetNodeStatusResponse>, Status> {
        let node_info = self.node_info.lock().await.clone();
        Ok(Response::new(GetNodeStatusResponse {
            status: Some(node_info),
        }))
    }

    async fn heartbeat(
        &self,
        _request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let mut node_info = self.node_info.lock().await;
        node_info.last_seen = Utc::now().timestamp();
        Ok(Response::new(HeartbeatResponse {
            acknowledged: true,
            server_timestamp: Utc::now().timestamp(),
            cluster_id: "test_cluster".to_string(),
        }))
    }

    async fn join_cluster(
        &self,
        request: Request<JoinClusterRequest>,
    ) -> Result<Response<JoinClusterResponse>, Status> {
        let node_info = request.into_inner().node_info.unwrap();
        *self.node_info.lock().await = node_info.clone();
        Ok(Response::new(JoinClusterResponse {
            success: true,
            cluster_info: Some(ClusterInfo {
                nodes: vec![node_info],
                last_updated: Utc::now().timestamp(),
            }),
            error: String::new(),
        }))
    }

    async fn leave_cluster(
        &self,
        _request: Request<LeaveClusterRequest>,
    ) -> Result<Response<LeaveClusterResponse>, Status> {
        let mut node_info = self.node_info.lock().await;
        node_info.status = NodeStatus::NodeOffline as i32;
        Ok(Response::new(LeaveClusterResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn list_directory(
        &self,
        _request: Request<ListDirectoryRequest>,
    ) -> Result<Response<ListDirectoryResponse>, Status> {
        Ok(Response::new(ListDirectoryResponse {
            entries: vec![],
        }))
    }

    async fn get_file_info(
        &self,
        _request: Request<GetFileInfoRequest>,
    ) -> Result<Response<GetFileInfoResponse>, Status> {
        Ok(Response::new(GetFileInfoResponse {
            info: None,
        }))
    }

    async fn delete_file(
        &self,
        _request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        Ok(Response::new(DeleteFileResponse {
            success: false,
        }))
    }

    async fn move_file(
        &self,
        _request: Request<MoveFileRequest>,
    ) -> Result<Response<MoveFileResponse>, Status> {
        Ok(Response::new(MoveFileResponse {
            success: false,
        }))
    }

    async fn copy_file(
        &self,
        _request: Request<CopyFileRequest>,
    ) -> Result<Response<CopyFileResponse>, Status> {
        Ok(Response::new(CopyFileResponse {
            success: false,
        }))
    }

    async fn read_file(
        &self,
        _request: Request<ReadFileRequest>,
    ) -> Result<Response<Self::ReadFileStream>, Status> {
        let (_tx, rx) = mpsc::channel(32);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn write_file(
        &self,
        request: Request<Streaming<WriteFileRequest>>,
    ) -> Result<Response<Self::WriteFileStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);
        
        tokio::spawn(async move {
            while let Some(chunk) = stream.message().await.unwrap() {
                let response = WriteFileResponse {
                    bytes_written: chunk.data.len() as u64,
                    status: 0,
                };
                tx.send(Ok(response)).await.unwrap();
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn sync_metadata(
        &self,
        _request: Request<SyncMetadataRequest>,
    ) -> Result<Response<Self::SyncMetadataStream>, Status> {
        let (_tx, rx) = mpsc::channel(32);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn drop_file(
        &self,
        _request: Request<DropFileRequest>,
    ) -> Result<Response<Self::DropFileStream>, Status> {
        let (_tx, rx) = mpsc::channel(32);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn receive_file(
        &self,
        _request: Request<Streaming<ReceiveFileRequest>>,
    ) -> Result<Response<ReceiveFileResponse>, Status> {
        Ok(Response::new(ReceiveFileResponse {
            success: false,
            error: "Not implemented".to_string(),
        }))
    }

    async fn get_cluster_info(
        &self,
        _request: Request<GetClusterInfoRequest>,
    ) -> Result<Response<GetClusterInfoResponse>, Status> {
        let node_info = self.node_info.lock().await.clone();
        Ok(Response::new(GetClusterInfoResponse {
            cluster_info: Some(ClusterInfo {
                nodes: vec![node_info],
                last_updated: Utc::now().timestamp(),
            }),
        }))
    }

    async fn update_node_status(
        &self,
        request: Request<UpdateNodeStatusRequest>,
    ) -> Result<Response<UpdateNodeStatusResponse>, Status> {
        let request = request.into_inner();
        let mut node_info = self.node_info.lock().await;
        node_info.status = request.status;
        node_info.last_seen = request.last_seen;
        Ok(Response::new(UpdateNodeStatusResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn discover_nodes(
        &self,
        _request: Request<DiscoverNodesRequest>,
    ) -> Result<Response<DiscoverNodesResponse>, Status> {
        let node_info = self.node_info.lock().await.clone();
        Ok(Response::new(DiscoverNodesResponse {
            nodes: vec![node_info],
        }))
    }

    async fn create_file(
        &self,
        request: Request<CreateFileRequest>,
    ) -> Result<Response<CreateFileResponse>, Status> {
        self.create_file(request).await
    }

    async fn create_directory(
        &self,
        request: Request<CreateDirectoryRequest>,
    ) -> Result<Response<CreateDirectoryResponse>, Status> {
        let request = request.into_inner();
        
        // Validate path
        if request.path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        // Create file info
        let file_info = FileInfo {
            id: format!("dir_{}", Utc::now().timestamp()),
            name: request.path.split('/').last().unwrap_or("").to_string(),
            path: request.path.clone(),
            r#type: FileType::Directory as i32,
            size: 0,
            created_at: Utc::now().timestamp(),
            modified_at: Utc::now().timestamp(),
            accessed_at: Utc::now().timestamp(),
            owner_node: request.node_id.clone(),
            available_nodes: vec![request.node_id],
            attributes: HashMap::new(),
        };

        Ok(Response::new(CreateDirectoryResponse {
            info: Some(file_info),
        }))
    }
}

async fn setup_test_node() -> (Arc<MockNodeService>, NodeInfo) {
    let mock_service = Arc::new(MockNodeService::new(
        "test_node".to_string(),
        "Test Node".to_string(),
        "Test Node Description".to_string(),
        1024 * 1024 * 1024, // 1GB
    ));
    let node_info = NodeInfo {
        id: "test_node".to_string(),
        name: "Test Node".to_string(),
        host: "127.0.0.1".to_string(),
        port: 50051,
        status: NodeStatus::NodeOnline as i32,
        last_seen: Utc::now().timestamp(),
    };
    
    (mock_service, node_info)
}

#[tokio::test]
async fn test_basic_file_operations() -> Result<(), Status> {
    let (node, _) = setup_test_node().await;

    // Create a file
    let create_req = Request::new(CreateFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/file1.txt".to_string(),
        r#type: FileType::File as i32,
    });
    let create_resp = node.create_file(create_req).await?;
    assert!(create_resp.into_inner().info.is_some());

    // Write data to the file
    let (tx, rx) = mpsc::channel(32);
    let stream = ReceiverStream::new(rx);
    tx.send(WriteFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/file1.txt".to_string(),
        data: "Hello, World!".as_bytes().to_vec(),
        offset: 0,
    }).await.unwrap();
    
    let write_req = Request::new(stream);
    let write_resp = node.write_file(write_req).await?;
    assert!(write_resp.into_inner().bytes_written > 0);

    // Read the file
    let read_req = Request::new(ReadFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/file1.txt".to_string(),
        offset: 0,
        length: 12, // Length of "Hello, World!"
    });
    let mut read_stream = node.read_file(read_req).await?.into_inner();
    let mut read_data = Vec::new();
    while let Some(chunk) = read_stream.next().await {
        read_data.extend_from_slice(&chunk?.data);
    }
    assert_eq!(read_data, "Hello, World!".as_bytes());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_file_operations() -> Result<(), Status> {
    let (node, _) = setup_test_node().await;

    // Create a file
    let create_req = Request::new(CreateFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/concurrent.txt".to_string(),
        r#type: FileType::File as i32,
    });
    let create_resp = node.create_file(create_req).await?;
    assert!(create_resp.into_inner().info.is_some());

    // Spawn multiple write tasks
    let mut handles = Vec::new();
    for i in 0..5 {
        let node_clone = Arc::clone(&node);
        let node_id = node.node_info.lock().await.id.clone();
        let handle = tokio::spawn(async move {
            let write_req = Request::new(WriteFileRequest {
                node_id: node_id,
                path: "/test/concurrent.txt".to_string(),
                data: format!("Data from thread {}\n", i).as_bytes().to_vec(),
                offset: (i * 20) as i64,
            });
            let write_resp = node_clone.write_file(write_req).await?;
            assert!(write_resp.into_inner().bytes_written > 0);
            Ok::<_, Status>(())
        });
        handles.push(handle);
    }

    // Wait for all writes to complete
    for handle in handles {
        handle.await.unwrap()?;
    }

    // Read the file
    let read_req = Request::new(ReadFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/concurrent.txt".to_string(),
        offset: 0,
        length: 100, // Total length of all writes
    });
    let mut read_stream = node.read_file(read_req).await?.into_inner();
    let mut read_data = Vec::new();
    while let Some(chunk) = read_stream.message().await? {
        read_data.extend_from_slice(&chunk.data);
    }
    let expected_data = (0..5)
        .map(|i| format!("Data from thread {}\n", i))
        .collect::<Vec<_>>()
        .join("")
        .into_bytes();
    assert_eq!(read_data, expected_data);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_multi_node_operations() -> Result<(), Status> {
    let (node1, _) = setup_test_node().await;
    let (node2, _) = setup_test_node().await;

    // Create a file on node1
    let create_req = Request::new(CreateFileRequest {
        node_id: node1.node_info.lock().await.id.clone(),
        path: "/test/multi_node.txt".to_string(),
        r#type: FileType::File as i32,
    });
    let create_resp = node1.create_file(create_req).await?;
    assert!(create_resp.into_inner().info.is_some());

    // Write data to the file from node1
    let write_req = Request::new(WriteFileRequest {
        node_id: node1.node_info.lock().await.id.clone(),
        path: "/test/multi_node.txt".to_string(),
        data: "Data from node 1".as_bytes().to_vec(),
        offset: 0,
    });
    let write_resp = node1.write_file(write_req).await?;
    assert!(write_resp.into_inner().bytes_written > 0);

    // Read data from node2
    let read_req = Request::new(ReadFileRequest {
        node_id: node2.node_info.lock().await.id.clone(),
        path: "/test/multi_node.txt".to_string(),
        offset: 0,
        length: 13, // Length of "Data from node 1"
    });
    let mut read_stream = node2.read_file(read_req).await?.into_inner();
    let mut read_data = Vec::new();
    while let Some(chunk) = read_stream.message().await? {
        read_data.extend_from_slice(&chunk.data);
    }
    assert_eq!(read_data, "Data from node 1".as_bytes());

    Ok(())
}

#[tokio::test]
async fn test_file_metadata_operations() -> Result<(), Status> {
    let (node, _) = setup_test_node().await;

    // Create a file
    let create_req = Request::new(CreateFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/metadata.txt".to_string(),
        r#type: FileType::File as i32,
    });
    let create_resp = node.create_file(create_req).await?;
    assert!(create_resp.into_inner().info.is_some());

    // Write data to the file
    let write_req = Request::new(WriteFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/metadata.txt".to_string(),
        data: "Hello, World!".as_bytes().to_vec(),
        offset: 0,
    });
    let write_resp = node.write_file(write_req).await?;
    assert!(write_resp.into_inner().bytes_written > 0);

    // Get file metadata
    let metadata_req = Request::new(GetFileRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/metadata.txt".to_string(),
    });
    let metadata_resp = node.get_file(metadata_req).await?;
    let metadata = metadata_resp.into_inner().metadata.unwrap();
    assert_eq!(metadata.size, 12); // Length of "Hello, World!"
    assert_eq!(metadata.r#type, FileType::File as i32);

    // Update file metadata
    let update_req = Request::new(UpdateFileMetadataRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/metadata.txt".to_string(),
        metadata: Some(FileMetadata {
            size: 12,
            r#type: FileType::File as i32,
            created_at: metadata.created_at,
            modified_at: metadata.modified_at,
            accessed_at: metadata.accessed_at,
            permissions: metadata.permissions,
            owner: metadata.owner,
            group: metadata.group,
        }),
    });
    let update_resp = node.update_file_metadata(update_req).await?;
    assert!(update_resp.into_inner().info.is_some());

    Ok(())
}

#[tokio::test]
async fn test_directory_metadata_operations() -> Result<(), Status> {
    let (node, _) = setup_test_node().await;

    // Create a directory
    let create_dir_req = Request::new(CreateDirectoryRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/metadata_dir".to_string(),
    });
    let create_dir_resp = node.create_file(create_dir_req).await?;
    assert!(create_dir_resp.into_inner().info.is_some());

    // Create files in the directory
    for i in 0..3 {
        let create_req = Request::new(CreateFileRequest {
            node_id: node.node_info.lock().await.id.clone(),
            path: format!("/test/metadata_dir/file{}.txt", i),
            r#type: FileType::File as i32,
        });
        let create_resp = node.create_file(create_req).await?;
        assert!(create_resp.into_inner().info.is_some());

        let write_req = Request::new(WriteFileRequest {
            node_id: node.node_info.lock().await.id.clone(),
            path: format!("/test/metadata_dir/file{}.txt", i),
            data: format!("Data for file {}", i).as_bytes().to_vec(),
            offset: 0,
        });
        let write_resp = node.write_file(write_req).await?;
        assert!(write_resp.into_inner().bytes_written > 0);
    }

    // List directory contents
    let list_req = Request::new(ListDirectoryRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/metadata_dir".to_string(),
    });
    let list_resp = node.list_directory(list_req).await?;
    let entries = list_resp.into_inner().entries;
    assert_eq!(entries.len(), 3);

    // Get directory metadata
    let dir_metadata_req = Request::new(GetDirectoryMetadataRequest {
        node_id: node.node_info.lock().await.id.clone(),
        path: "/test/metadata_dir".to_string(),
    });
    let dir_metadata_resp = node.get_directory_metadata(dir_metadata_req).await?;
    let dir_metadata = dir_metadata_resp.into_inner().metadata.unwrap();
    assert_eq!(dir_metadata.r#type, FileType::Directory as i32);
    assert_eq!(dir_metadata.items.len(), 3); // Three files created in the directory

    Ok(())
}

#[tokio::test]
async fn test_create_file() -> Result<(), Status> {
    let (mock_service, node_info) = setup_test_node().await;
    
    // 创建父目录
    let create_dir_req = Request::new(CreateFileRequest {
        node_id: node_info.id.clone(),
        path: "/test".to_string(),
        r#type: FileType::Directory as i32,
    });
    mock_service.create_file(create_dir_req).await?;
    
    // 测试成功创建文件
    let request = Request::new(CreateFileRequest {
        path: "/test/file.txt".to_string(),
        r#type: FileType::File as i32,
        node_id: node_info.id.clone(),
    });
    
    let response = mock_service.create_file(request).await?;
    let file_info = response.into_inner().info.expect("文件信息应该存在");
    assert_eq!(file_info.path, "/test/file.txt");
    assert_eq!(file_info.r#type, FileType::File as i32);

    // 验证文件是否存在
    let get_info_request = Request::new(GetFileInfoRequest {
        path: "/test/file.txt".to_string(),
        node_id: node_info.id.clone(),
    });
    let get_info_response = mock_service.get_file_info(get_info_request).await?;
    assert!(get_info_response.into_inner().info.is_some());

    // 测试创建已存在的文件（应该失败）
    let duplicate_request = Request::new(CreateFileRequest {
        path: "/test/file.txt".to_string(),
        r#type: FileType::File as i32,
        node_id: node_info.id.clone(),
    });
    let duplicate_result = mock_service.create_file(duplicate_request).await;
    assert!(duplicate_result.is_err());

    // 测试创建无效路径的文件
    let invalid_request = Request::new(CreateFileRequest {
        path: "".to_string(),
        r#type: FileType::File as i32,
        node_id: node_info.id.clone(),
    });
    let invalid_result = mock_service.create_file(invalid_request).await;
    assert!(invalid_result.is_err());

    // 测试创建包含特殊字符的文件名
    let special_chars_request = Request::new(CreateFileRequest {
        path: "/test/special@#$%^&.txt".to_string(),
        r#type: FileType::File as i32,
        node_id: node_info.id.clone(),
    });
    let special_chars_result = mock_service.create_file(special_chars_request).await;
    assert!(special_chars_result.is_ok());

    // 测试创建嵌套目录中的文件
    let nested_dir_req = Request::new(CreateFileRequest {
        node_id: node_info.id.clone(),
        path: "/test/nested/dir".to_string(),
        r#type: FileType::Directory as i32,
    });
    mock_service.create_file(nested_dir_req).await?;

    let nested_file_req = Request::new(CreateFileRequest {
        path: "/test/nested/dir/nested_file.txt".to_string(),
        r#type: FileType::File as i32,
        node_id: node_info.id.clone(),
    });
    let nested_result = mock_service.create_file(nested_file_req).await;
    assert!(nested_result.is_ok());

    // 测试创建超长文件名
    let long_filename = format!("/test/{}.txt", "a".repeat(255));
    let long_name_request = Request::new(CreateFileRequest {
        path: long_filename,
        r#type: FileType::File as i32,
        node_id: node_info.id.clone(),
    });
    let long_name_result = mock_service.create_file(long_name_request).await;
    assert!(long_name_result.is_ok());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_node_service() {
        let mock_service = Arc::new(MockNodeService::new(
            "node1".to_string(),
            "测试节点".to_string(),
            "用于测试的节点".to_string(),
            1024 * 1024 * 1024, // 1GB
        ));

        // 测试初始状态
        let info = mock_service.get_node_info().await;
        assert_eq!(info.id, "node1");
        assert_eq!(info.name, "测试节点");
        assert_eq!(info.status, NodeStatus::NodeOnline as i32);

        // 测试状态变更
        mock_service.set_status(TestNodeStatus::NodeOffline).await;
        let info = mock_service.get_node_info().await;
        assert_eq!(info.status, NodeStatus::NodeOffline as i32);
    }
}

#[tokio::test]
async fn test_write_file() -> Result<(), Status> {
    let (mock_service, node_info) = setup_test_node().await;
    
    // Create parent directory first
    let create_dir_req = Request::new(CreateFileRequest {
        node_id: node_info.id.clone(),
        path: "/test".to_string(),
        r#type: FileType::Directory as i32,
    });
    mock_service.create_file(create_dir_req).await?;
    
    // Create the test file
    let create_file_req = Request::new(CreateFileRequest {
        node_id: node_info.id.clone(),
        path: "/test/test.txt".to_string(),
        r#type: FileType::File as i32,
    });
    mock_service.create_file(create_file_req).await?;
    
    // Test basic write
    let (tx, rx) = mpsc::channel(128);
    let data = b"Hello, World!".to_vec();
    tx.send(WriteFileRequest {
        node_id: node_info.id.clone(),
        path: "/test/test.txt".to_string(),
        data: data.clone(),
        offset: 0,
    }).await.unwrap();
    drop(tx);
    
    let request = Request::new(Streaming::new_request(
        ProstCodec::default(),
        ReceiverStream::new(rx),
        None,
    ));
    let response = mock_service.write_file(request).await?;
    assert_eq!(response.into_inner().bytes_written, data.len() as i64);
    
    // Test writing at offset
    let (tx, rx) = mpsc::channel(128);
    let offset_data = b" Again!".to_vec();
    tx.send(WriteFileRequest {
        node_id: node_info.id.clone(),
        path: "/test/test.txt".to_string(),
        data: offset_data.clone(),
        offset: data.len() as i64,
    }).await.unwrap();
    drop(tx);
    
    let request = Request::new(Streaming::new_request(
        ProstCodec::default(),
        ReceiverStream::new(rx),
        None,
    ));
    let response = mock_service.write_file(request).await?;
    assert_eq!(response.into_inner().bytes_written, offset_data.len() as i64);
    
    // Test writing to non-existent file (should fail)
    let (tx, rx) = mpsc::channel(128);
    tx.send(WriteFileRequest {
        node_id: node_info.id.clone(),
        path: "/test/nonexistent.txt".to_string(),
        data: b"Should fail".to_vec(),
        offset: 0,
    }).await.unwrap();
    drop(tx);
    
    let request = Request::new(Streaming::new_request(
        ProstCodec::default(),
        ReceiverStream::new(rx),
        None,
    ));
    let result = mock_service.write_file(request).await;
    assert!(result.is_err());
    
    // Test writing with invalid offset
    let (tx, rx) = mpsc::channel(128);
    tx.send(WriteFileRequest {
        node_id: node_info.id.clone(),
        path: "/test/test.txt".to_string(),
        data: b"Invalid offset".to_vec(),
        offset: -1,
    }).await.unwrap();
    drop(tx);
    
    let request = Request::new(Streaming::new_request(
        ProstCodec::default(),
        ReceiverStream::new(rx),
        None,
    ));
    let result = mock_service.write_file(request).await;
    assert!(result.is_err());
    
    Ok(())
} 
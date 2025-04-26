use crate::config::{ClusterConfig, NodeConfig};
use crate::metadata::{MetadataStore, NodeStatus, FileType};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use crate::error::{Result, Error};
use crate::proto::vdfs::FileInfo;
use chrono::Utc;
use tokio::sync::Mutex;

pub struct SyncManager {
    metadata_store: Arc<RwLock<MetadataStore>>,
    config: ClusterConfig,
    node_config: NodeConfig,
    sync_queue: Arc<Mutex<Vec<SyncTask>>>,
}

#[derive(Debug, Clone)]
pub struct SyncTask {
    pub file_path: String,
    pub source_node: String,
    pub target_node: String,
    pub priority: u32,
}

impl SyncManager {
    pub fn new(
        metadata_store: Arc<RwLock<MetadataStore>>,
        config: ClusterConfig,
        node_config: NodeConfig,
    ) -> Self {
        Self {
            metadata_store,
            config,
            node_config,
            sync_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn sync(&self) -> Result<()> {
        // TODO: 实现同步逻辑
        Ok(())
    }

    pub async fn update_node_status(&self, node_id: &str, status: NodeStatus) -> Result<()> {
        let mut metadata_store = self.metadata_store.write().await;
        metadata_store.update_node_status(node_id, status).await
    }

    pub async fn update_file_info(&self, file_info: FileInfo) -> Result<()> {
        let mut metadata_store = self.metadata_store.write().await;
        metadata_store.update_file_info(file_info).await
    }

    pub async fn list_files(&self) -> Result<Vec<FileInfo>> {
        let metadata_store = self.metadata_store.read().await;
        let files = metadata_store.list_files().await?;
        
        Ok(files.into_iter().map(|(path, file)| {
            FileInfo {
                id: uuid::Uuid::new_v4().to_string(),
                name: file.name,
                path,
                r#type: match file.file_type {
                    FileType::File => 1,
                    FileType::Directory => 2,
                    FileType::Symlink => 3,
                    FileType::Unknown => 0,
                },
                size: file.size as i64,
                created_at: file.created_at,
                modified_at: file.modified_at,
                accessed_at: file.accessed_at,
                owner_node: file.owner_node,
                available_nodes: file.available_nodes,
                attributes: file.attributes,
            }
        }).collect())
    }

    pub async fn list_nodes(&self) -> Result<Vec<NodeStatus>> {
        let metadata_store = self.metadata_store.read().await;
        metadata_store.list_nodes().await
    }

    pub async fn start_sync(&self) {
        let sync_interval = Duration::from_secs(self.config.sync_interval);
        let mut interval = time::interval(sync_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.sync_with_nodes().await {
                tracing::error!("同步失败: {}", e);
            }
        }
    }

    async fn sync_with_nodes(&self) -> Result<()> {
        let now = Utc::now();
        let store = self.metadata_store.read().await;
        
        // 获取需要同步的节点列表
        let nodes = store.list_nodes().await?;
        let stale_nodes = nodes.into_iter().filter(|node| {
            if let Some(status) = store.get_node(&node.id) {
                status.is_online && (now.timestamp() - status.last_seen) / 60 > 5
            } else {
                true
            }
        }).collect::<Vec<_>>();

        drop(store); // 释放读锁，准备写操作

        for node in stale_nodes {
            // TODO: 实现节点同步逻辑
            // 1. 连接到节点
            // 2. 获取文件列表和元数据
            // 3. 更新本地元数据存储
            // 4. 传输缺失的文件
            
            let mut store = self.metadata_store.write().await;
            let status = NodeStatus {
                id: node.id.clone(),
                name: node.name,
                host: node.host,
                port: node.port,
                last_seen: now.timestamp(),
                is_online: true,
            };
            store.update_node_status(&node.id, status).await?;
        }

        Ok(())
    }

    pub async fn drop_file(&self, file_id: &str, target_node: &str) -> Result<()> {
        let mut metadata = self.metadata_store.write().await;
        
        if let Ok(Some(file)) = metadata.get_file(file_id) {
            if file.owner_node != self.node_config.id {
                return Err(crate::error::VDFSError::NodeError(
                    "文件不属于当前节点".to_string()
                ));
            }

            // TODO: 实现文件传输
            // 1. 建立与目标节点的连接
            // 2. 传输文件数据
            // 3. 更新元数据

            // 更新文件元数据
            let mut new_metadata = file.clone();
            new_metadata.available_nodes.push(target_node.to_string());
            let file_info = FileInfo {
                id: new_metadata.id,
                name: new_metadata.name,
                path: new_metadata.path,
                r#type: match new_metadata.file_type {
                    FileType::File => 1,
                    FileType::Directory => 2,
                    FileType::Symlink => 3,
                    FileType::Unknown => 0,
                },
                size: new_metadata.size as i64,
                created_at: new_metadata.created_at,
                modified_at: new_metadata.modified_at,
                accessed_at: new_metadata.accessed_at,
                owner_node: new_metadata.owner_node,
                available_nodes: new_metadata.available_nodes,
                attributes: new_metadata.attributes,
            };
            metadata.update_file_info(file_info).await?;
        }

        Ok(())
    }

    pub async fn add_sync_task(&self, task: SyncTask) -> Result<()> {
        let mut queue = self.sync_queue.lock().await;
        queue.push(task);
        Ok(())
    }

    pub async fn get_next_task(&self) -> Option<SyncTask> {
        let mut queue = self.sync_queue.lock().await;
        queue.pop()
    }

    pub async fn sync_file(&self, file_path: &str, source_node: &str, target_node: &str) -> Result<()> {
        let metadata = self.metadata_store.read().await;
        let _file = metadata.get_file(file_path)?
            .ok_or_else(|| Error::FileSystem(format!("File not found: {}", file_path)))?;

        // 创建同步任务
        let task = SyncTask {
            file_path: file_path.to_string(),
            source_node: source_node.to_string(),
            target_node: target_node.to_string(),
            priority: 1,
        };

        // 添加到同步队列
        self.add_sync_task(task).await?;

        Ok(())
    }

    pub async fn get_sync_status(&self, file_path: &str) -> Result<Vec<String>> {
        let metadata = self.metadata_store.read().await;
        let file = metadata.get_file(file_path)?
            .ok_or_else(|| Error::FileSystem(format!("File not found: {}", file_path)))?;

        Ok(file.available_nodes.clone())
    }

    pub async fn list_files_to_sync(&self, node_id: &str) -> Result<Vec<FileInfo>> {
        let metadata = self.metadata_store.read().await;
        let files = metadata.list_files().await?;

        let mut sync_files = Vec::new();
        for (path, file) in files {
            if !file.available_nodes.contains(&node_id.to_string()) {
                sync_files.push(FileInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: file.name,
                    path,
                    r#type: match file.file_type {
                        FileType::File => 1,
                        FileType::Directory => 2,
                        FileType::Symlink => 3,
                        FileType::Unknown => 0,
                    },
                    size: file.size as i64,
                    created_at: file.created_at,
                    modified_at: file.modified_at,
                    accessed_at: file.accessed_at,
                    owner_node: file.owner_node,
                    available_nodes: file.available_nodes,
                    attributes: file.attributes,
                });
            }
        }

        Ok(sync_files)
    }

    pub async fn process_sync_queue(&self) -> Result<()> {
        while let Some(task) = self.get_next_task().await {
            // 处理同步任务
            self.sync_file(&task.file_path, &task.source_node, &task.target_node).await?;
        }
        Ok(())
    }
} 
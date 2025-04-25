use crate::config::{ClusterConfig, NodeConfig};
use crate::metadata::{MetadataStore, NodeStatus};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use crate::error::Result;
use crate::proto::vdfs::FileInfo;
use chrono::Utc;

pub struct SyncManager {
    metadata_store: Arc<RwLock<MetadataStore>>,
    config: ClusterConfig,
    node_config: NodeConfig,
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
        metadata_store.list_files().await
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
        
        if let Some(file) = metadata.get_file(file_id) {
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
            metadata.update_file(new_metadata);
        }

        Ok(())
    }
} 
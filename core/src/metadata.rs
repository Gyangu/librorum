use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sqlx::SqlitePool;
use crate::error::Result;
use crate::proto::vdfs::FileInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub file_type: FileType,
    pub created_at: i64,
    pub modified_at: i64,
    pub accessed_at: i64,
    pub owner_node: String,
    pub available_nodes: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub chunks: Vec<ChunkInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Unknown,
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub last_seen: i64,
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub id: String,
    pub size: u64,
    pub nodes: Vec<String>,
}

pub struct MetadataStore {
    pool: Arc<SqlitePool>,
    files: Arc<Mutex<HashMap<String, FileMetadata>>>,
    nodes: HashMap<String, NodeStatus>,
}

impl MetadataStore {
    pub async fn new() -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        Ok(Self {
            pool: Arc::new(pool),
            files: Arc::new(Mutex::new(HashMap::new())),
            nodes: HashMap::new(),
        })
    }

    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                size INTEGER NOT NULL,
                file_type INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                accessed_at INTEGER NOT NULL,
                owner_node TEXT NOT NULL,
                available_nodes TEXT NOT NULL,
                attributes TEXT NOT NULL
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| crate::error::VDFSError::Database(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                last_seen INTEGER NOT NULL,
                is_online BOOLEAN NOT NULL
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| crate::error::VDFSError::Database(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
            CREATE INDEX IF NOT EXISTS idx_files_owner ON files(owner_node);
            CREATE INDEX IF NOT EXISTS idx_nodes_host ON nodes(host, port);
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| crate::error::VDFSError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn sync(&mut self) -> Result<()> {
        // TODO: Implement sync logic
        Ok(())
    }

    pub async fn update_node_status(&mut self, node_id: &str, status: NodeStatus) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO nodes (id, name, host, port, last_seen, is_online)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&status.id)
        .bind(&status.name)
        .bind(&status.host)
        .bind(status.port)
        .bind(status.last_seen)
        .bind(status.is_online)
        .execute(&*self.pool)
        .await
        .map_err(|e| crate::error::VDFSError::Database(e.to_string()))?;

        self.nodes.insert(node_id.to_string(), status);
        Ok(())
    }

    pub async fn update_file_info(&mut self, file_info: FileInfo) -> Result<()> {
        let metadata = FileMetadata {
            id: file_info.id,
            name: file_info.name,
            path: file_info.path,
            size: file_info.size as u64,
            file_type: match file_info.r#type {
                1 => FileType::File,
                2 => FileType::Directory,
                3 => FileType::Symlink,
                _ => FileType::Unknown,
            },
            created_at: file_info.created_at,
            modified_at: file_info.modified_at,
            accessed_at: file_info.accessed_at,
            owner_node: file_info.owner_node,
            available_nodes: file_info.available_nodes,
            attributes: file_info.attributes,
            chunks: Vec::new(),
        };

        self.add_file(metadata.path.clone(), metadata).await?;
        Ok(())
    }

    pub async fn list_files(&self) -> Result<Vec<(String, FileMetadata)>> {
        let files = self.files.lock().unwrap();
        Ok(files.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    pub async fn list_nodes(&self) -> Result<Vec<NodeStatus>> {
        Ok(self.nodes.values().cloned().collect())
    }

    pub async fn add_file(&self, path: String, metadata: FileMetadata) -> Result<()> {
        let mut files = self.files.lock().unwrap();
        files.insert(path, metadata);
        Ok(())
    }

    pub fn get_file(&self, path: &str) -> Result<Option<FileMetadata>> {
        let files = self.files.lock().unwrap();
        Ok(files.get(path).cloned())
    }

    pub fn remove_file(&self, path: &str) -> Result<()> {
        let mut files = self.files.lock().unwrap();
        files.remove(path);
        Ok(())
    }

    pub fn add_node(&mut self, status: NodeStatus) {
        self.nodes.insert(status.id.clone(), status);
    }

    pub fn get_node(&self, id: &str) -> Option<&NodeStatus> {
        self.nodes.get(id)
    }

    pub fn update_node(&mut self, status: NodeStatus) {
        self.nodes.insert(status.id.clone(), status);
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.remove(id);
    }

    pub fn get_files_by_node(&self, node_id: &str) -> Vec<FileMetadata> {
        self.files.lock().unwrap()
            .values()
            .filter(|f| f.owner_node == node_id)
            .cloned()
            .collect()
    }

    pub fn get_available_files(&self) -> Vec<FileMetadata> {
        self.files.lock().unwrap()
            .values()
            .filter(|f| !f.available_nodes.is_empty())
            .cloned()
            .collect()
    }
} 
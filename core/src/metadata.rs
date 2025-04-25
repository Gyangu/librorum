use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
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

#[derive(Debug, Clone)]
pub struct MetadataStore {
    pool: Arc<SqlitePool>,
    files: HashMap<String, FileMetadata>,
    nodes: HashMap<String, NodeStatus>,
}

impl MetadataStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: Arc::new(pool),
            files: HashMap::new(),
            nodes: HashMap::new(),
        }
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
        .map_err(|e| crate::error::VDFSError::Database(e))?;

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
        .map_err(|e| crate::error::VDFSError::Database(e))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
            CREATE INDEX IF NOT EXISTS idx_files_owner ON files(owner_node);
            CREATE INDEX IF NOT EXISTS idx_nodes_host ON nodes(host, port);
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| crate::error::VDFSError::Database(e))?;

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
        .map_err(|e| crate::error::VDFSError::Database(e))?;

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
        };

        self.add_file(metadata);
        Ok(())
    }

    pub async fn list_files(&self) -> Result<Vec<FileInfo>> {
        let files = self.files.values().map(|f| FileInfo {
            id: f.id.clone(),
            name: f.name.clone(),
            path: f.path.clone(),
            r#type: match f.file_type {
                FileType::File => 1,
                FileType::Directory => 2,
                FileType::Symlink => 3,
                FileType::Unknown => 0,
            },
            size: f.size as i64,
            created_at: f.created_at,
            modified_at: f.modified_at,
            accessed_at: f.accessed_at,
            owner_node: f.owner_node.clone(),
            available_nodes: f.available_nodes.clone(),
            attributes: f.attributes.clone(),
        }).collect();

        Ok(files)
    }

    pub async fn list_nodes(&self) -> Result<Vec<NodeStatus>> {
        Ok(self.nodes.values().cloned().collect())
    }

    pub fn add_file(&mut self, metadata: FileMetadata) {
        self.files.insert(metadata.id.clone(), metadata);
    }

    pub fn get_file(&self, id: &str) -> Option<&FileMetadata> {
        self.files.get(id)
    }

    pub fn update_file(&mut self, metadata: FileMetadata) {
        self.files.insert(metadata.id.clone(), metadata);
    }

    pub fn remove_file(&mut self, id: &str) {
        self.files.remove(id);
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

    pub fn get_files_by_node(&self, node_id: &str) -> Vec<&FileMetadata> {
        self.files
            .values()
            .filter(|f| f.owner_node == node_id)
            .collect()
    }

    pub fn get_available_files(&self) -> Vec<&FileMetadata> {
        self.files
            .values()
            .filter(|f| !f.available_nodes.is_empty())
            .collect()
    }
} 
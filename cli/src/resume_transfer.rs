use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::{debug, info, warn};

/// 传输会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSession {
    /// 会话ID
    pub session_id: String,
    /// 本地文件路径
    pub local_path: PathBuf,
    /// 远程文件路径
    pub remote_path: String,
    /// 文件总大小
    pub total_size: u64,
    /// 文件哈希值
    pub file_hash: Option<String>,
    /// 已传输的字节数
    pub transferred_bytes: u64,
    /// 已完成的块列表
    pub completed_chunks: Vec<ChunkInfo>,
    /// 传输类型
    pub transfer_type: TransferType,
    /// 创建时间
    pub created_at: u64,
    /// 最后更新时间
    pub updated_at: u64,
    /// 传输配置
    pub config: TransferConfig,
}

/// 块信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// 块ID
    pub chunk_id: u32,
    /// 块偏移量
    pub offset: u64,
    /// 块大小
    pub size: usize,
    /// 块哈希值
    pub hash: Option<String>,
    /// 完成时间
    pub completed_at: u64,
}

/// 传输类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    Upload,
    Download,
}

/// 传输配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferConfig {
    /// 块大小
    pub chunk_size: usize,
    /// 最大并发数
    pub max_concurrent: usize,
    /// 是否使用大文件模式
    pub large_file_mode: bool,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024, // 64KB
            max_concurrent: 4,
            large_file_mode: false,
        }
    }
}

/// 断点续传管理器
pub struct ResumeManager {
    sessions_dir: PathBuf,
    sessions: HashMap<String, TransferSession>,
}

impl ResumeManager {
    /// 创建新的断点续传管理器
    pub fn new<P: AsRef<Path>>(sessions_dir: P) -> Self {
        Self {
            sessions_dir: sessions_dir.as_ref().to_path_buf(),
            sessions: HashMap::new(),
        }
    }

    /// 初始化管理器
    pub async fn init(&mut self) -> Result<()> {
        // 创建会话目录
        if !self.sessions_dir.exists() {
            fs::create_dir_all(&self.sessions_dir).await
                .with_context(|| format!("无法创建会话目录: {}", self.sessions_dir.display()))?;
        }

        // 加载现有会话
        self.load_sessions().await?;

        info!("断点续传管理器初始化完成，加载 {} 个会话", self.sessions.len());
        Ok(())
    }

    /// 加载所有会话
    async fn load_sessions(&mut self) -> Result<()> {
        let mut entries = fs::read_dir(&self.sessions_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "session") {
                match self.load_session_file(&path).await {
                    Ok(session) => {
                        self.sessions.insert(session.session_id.clone(), session);
                    }
                    Err(e) => {
                        warn!("加载会话文件失败: {} - {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 加载单个会话文件
    async fn load_session_file(&self, path: &Path) -> Result<TransferSession> {
        let content = fs::read_to_string(path).await?;
        let session: TransferSession = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// 保存会话到文件
    async fn save_session(&self, session: &TransferSession) -> Result<()> {
        let file_path = self.sessions_dir.join(format!("{}.session", session.session_id));
        let content = serde_json::to_string_pretty(session)?;
        fs::write(&file_path, content).await
            .with_context(|| format!("无法保存会话文件: {}", file_path.display()))?;
        Ok(())
    }

    /// 创建新的传输会话
    pub async fn create_session(
        &mut self,
        local_path: &Path,
        remote_path: &str,
        total_size: u64,
        transfer_type: TransferType,
        config: TransferConfig,
    ) -> Result<String> {
        let session_id = self.generate_session_id(local_path, remote_path);
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let session = TransferSession {
            session_id: session_id.clone(),
            local_path: local_path.to_path_buf(),
            remote_path: remote_path.to_string(),
            total_size,
            file_hash: None,
            transferred_bytes: 0,
            completed_chunks: Vec::new(),
            transfer_type,
            created_at: now,
            updated_at: now,
            config,
        };

        self.sessions.insert(session_id.clone(), session.clone());
        self.save_session(&session).await?;

        info!("创建新的传输会话: {}", session_id);
        Ok(session_id)
    }

    /// 生成会话ID
    fn generate_session_id(&self, local_path: &Path, remote_path: &str) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(local_path.to_string_lossy().as_bytes());
        hasher.update(remote_path.as_bytes());
        hasher.update(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().to_string().as_bytes());
        
        let hash = hasher.finalize();
        format!("{:x}", hash)[..16].to_string()
    }

    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<&TransferSession> {
        self.sessions.get(session_id)
    }

    /// 查找可恢复的会话
    pub fn find_resumable_session(&self, local_path: &Path, remote_path: &str) -> Option<&TransferSession> {
        self.sessions.values().find(|session| {
            session.local_path == local_path && 
            session.remote_path == remote_path &&
            session.transferred_bytes < session.total_size
        })
    }

    /// 更新会话进度
    pub async fn update_session_progress(
        &mut self,
        session_id: &str,
        transferred_bytes: u64,
        completed_chunks: Vec<ChunkInfo>,
    ) -> Result<()> {
        let session_copy = if let Some(session) = self.sessions.get_mut(session_id) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            session.transferred_bytes = transferred_bytes;
            session.completed_chunks = completed_chunks;
            session.updated_at = now;

            Some(session.clone())
        } else {
            None
        };

        if let Some(session) = session_copy {
            self.save_session(&session).await?;
            debug!("更新会话进度: {} - {}/{} 字节", session_id, transferred_bytes, session.total_size);
        }

        Ok(())
    }

    /// 标记块完成
    pub async fn mark_chunk_completed(
        &mut self,
        session_id: &str,
        chunk: ChunkInfo,
    ) -> Result<()> {
        let (session_copy, chunk_id) = if let Some(session) = self.sessions.get_mut(session_id) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            
            // 检查块是否已存在
            if !session.completed_chunks.iter().any(|c| c.chunk_id == chunk.chunk_id) {
                let chunk_id = chunk.chunk_id;
                let chunk_size = chunk.size;
                session.completed_chunks.push(chunk);
                session.transferred_bytes += chunk_size as u64;
                session.updated_at = now;

                (Some(session.clone()), chunk_id)
            } else {
                (None, 0)
            }
        } else {
            (None, 0)
        };

        if let Some(session) = session_copy {
            self.save_session(&session).await?;
            debug!("标记块完成: {} - 块 {}", session_id, chunk_id);
        }

        Ok(())
    }

    /// 完成传输会话
    pub async fn complete_session(&mut self, session_id: &str) -> Result<()> {
        if let Some(_session) = self.sessions.remove(session_id) {
            let file_path = self.sessions_dir.join(format!("{}.session", session_id));
            if file_path.exists() {
                fs::remove_file(&file_path).await?;
            }
            info!("传输会话完成并清理: {}", session_id);
        }

        Ok(())
    }

    /// 取消传输会话
    pub async fn cancel_session(&mut self, session_id: &str) -> Result<()> {
        self.complete_session(session_id).await
    }

    /// 清理过期会话
    pub async fn cleanup_expired_sessions(&mut self, max_age_days: u64) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let max_age_seconds = max_age_days * 24 * 60 * 60;

        let expired_sessions: Vec<String> = self.sessions
            .iter()
            .filter(|(_, session)| now - session.updated_at > max_age_seconds)
            .map(|(id, _)| id.clone())
            .collect();

        for session_id in expired_sessions {
            self.cancel_session(&session_id).await?;
            info!("清理过期会话: {}", session_id);
        }

        Ok(())
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<&TransferSession> {
        self.sessions.values().collect()
    }

    /// 计算未完成的块
    pub fn get_pending_chunks(&self, session_id: &str) -> Vec<ChunkInfo> {
        if let Some(session) = self.sessions.get(session_id) {
            let completed_chunk_ids: std::collections::HashSet<u32> = 
                session.completed_chunks.iter().map(|c| c.chunk_id).collect();

            let total_chunks = (session.total_size + session.config.chunk_size as u64 - 1) / session.config.chunk_size as u64;
            let mut pending_chunks = Vec::new();

            for chunk_id in 0..total_chunks as u32 {
                if !completed_chunk_ids.contains(&chunk_id) {
                    let offset = chunk_id as u64 * session.config.chunk_size as u64;
                    let remaining = session.total_size - offset;
                    let size = session.config.chunk_size.min(remaining as usize);

                    pending_chunks.push(ChunkInfo {
                        chunk_id,
                        offset,
                        size,
                        hash: None,
                        completed_at: 0,
                    });
                }
            }

            pending_chunks
        } else {
            Vec::new()
        }
    }

    /// 获取传输进度
    pub fn get_progress(&self, session_id: &str) -> Option<f64> {
        self.sessions.get(session_id).map(|session| {
            if session.total_size == 0 {
                0.0
            } else {
                session.transferred_bytes as f64 / session.total_size as f64
            }
        })
    }

    /// 估算剩余时间
    pub fn estimate_remaining_time(&self, session_id: &str, current_speed_mbps: f64) -> Option<Duration> {
        self.sessions.get(session_id).and_then(|session| {
            if current_speed_mbps <= 0.0 || session.transferred_bytes >= session.total_size {
                return None;
            }

            let remaining_bytes = session.total_size - session.transferred_bytes;
            let remaining_mb = remaining_bytes as f64 / (1024.0 * 1024.0);
            let remaining_seconds = remaining_mb / current_speed_mbps;

            Some(Duration::from_secs_f64(remaining_seconds))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_resume_manager_basic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut manager = ResumeManager::new(temp_dir.path());
        manager.init().await?;

        // 创建会话
        let session_id = manager.create_session(
            Path::new("/test/local.txt"),
            "/remote.txt",
            1024000,
            TransferType::Upload,
            TransferConfig::default(),
        ).await?;

        // 验证会话创建
        assert!(manager.get_session(&session_id).is_some());

        // 标记块完成
        let chunk = ChunkInfo {
            chunk_id: 0,
            offset: 0,
            size: 64 * 1024,
            hash: Some("test_hash".to_string()),
            completed_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };

        manager.mark_chunk_completed(&session_id, chunk).await?;

        // 验证进度更新
        let progress = manager.get_progress(&session_id).unwrap();
        assert!(progress > 0.0);

        // 完成会话
        manager.complete_session(&session_id).await?;
        assert!(manager.get_session(&session_id).is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_pending_chunks() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut manager = ResumeManager::new(temp_dir.path());
        manager.init().await?;

        let config = TransferConfig {
            chunk_size: 1024,
            max_concurrent: 4,
            large_file_mode: false,
        };

        let session_id = manager.create_session(
            Path::new("/test/local.txt"),
            "/remote.txt",
            3072, // 3 chunks
            TransferType::Upload,
            config,
        ).await?;

        // 标记第一个块完成
        let chunk = ChunkInfo {
            chunk_id: 0,
            offset: 0,
            size: 1024,
            hash: Some("test_hash".to_string()),
            completed_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };

        manager.mark_chunk_completed(&session_id, chunk).await?;

        // 获取未完成的块
        let pending = manager.get_pending_chunks(&session_id);
        assert_eq!(pending.len(), 2); // 剩余2个块
        assert_eq!(pending[0].chunk_id, 1);
        assert_eq!(pending[1].chunk_id, 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_find_resumable_session() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut manager = ResumeManager::new(temp_dir.path());
        manager.init().await?;

        let local_path = Path::new("/test/local.txt");
        let remote_path = "/remote.txt";

        let session_id = manager.create_session(
            local_path,
            remote_path,
            1024000,
            TransferType::Upload,
            TransferConfig::default(),
        ).await?;

        // 查找可恢复的会话
        let found = manager.find_resumable_session(local_path, remote_path);
        assert!(found.is_some());
        assert_eq!(found.unwrap().session_id, session_id);

        // 查找不存在的会话
        let not_found = manager.find_resumable_session(Path::new("/other.txt"), remote_path);
        assert!(not_found.is_none());

        Ok(())
    }
}
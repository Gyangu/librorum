use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};
use crate::error::{Result, VDFSError};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct LocalFileSystem {
    root_dir: PathBuf,
}

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<()>;
    async fn delete_file(&self, path: &Path) -> Result<()>;
    async fn list_dir(&self, path: &Path) -> Result<Vec<String>>;
    async fn create_dir(&self, path: &Path) -> Result<()>;
    async fn delete_dir(&self, path: &Path) -> Result<()>;
    async fn exists(&self, path: &Path) -> Result<bool>;
}

impl LocalFileSystem {
    pub async fn new(root_dir: impl AsRef<Path>) -> Result<Self> {
        let root_dir = root_dir.as_ref().to_path_buf();
        if !root_dir.exists() {
            fs::create_dir_all(&root_dir)
                .await
                .map_err(|e| VDFSError::Io(e))?;
        }
        Ok(Self { root_dir })
    }

    pub fn get_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.root_dir.join(path)
    }

    pub async fn create_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let full_path = self.get_path(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| VDFSError::Io(e))?;
        }
        fs::File::create(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    pub async fn create_dir(&self, path: impl AsRef<Path>) -> Result<()> {
        let full_path = self.get_path(path);
        fs::create_dir_all(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    pub async fn delete_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let full_path = self.get_path(path);
        if !full_path.exists() {
            return Err(VDFSError::FileNotFound(full_path.to_string_lossy().into_owned()));
        }
        fs::remove_file(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    pub async fn read_file(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let full_path = self.get_path(path);
        if !full_path.exists() {
            return Err(VDFSError::FileNotFound(full_path.to_string_lossy().into_owned()));
        }
        fs::read(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))
    }

    pub async fn write_file(&self, path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
        let full_path = self.get_path(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| VDFSError::Io(e))?;
        }
        fs::write(&full_path, content)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    pub async fn list_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let full_path = self.get_path(path);
        if !full_path.exists() {
            return Err(VDFSError::FileNotFound(full_path.to_string_lossy().into_owned()));
        }
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        while let Some(entry) = read_dir.next_entry()
            .await
            .map_err(|e| VDFSError::Io(e))? {
            entries.push(entry.path());
        }
        Ok(entries)
    }

    pub async fn read_file_chunk(&self, path: &str, offset: u64, size: u64) -> Result<Vec<u8>> {
        let file_path = self.root_dir.join(path);
        let mut file = tokio::fs::File::open(&file_path).await?;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        
        let mut buffer = vec![0; size as usize];
        let bytes_read = file.read(&mut buffer).await?;
        buffer.truncate(bytes_read);
        
        Ok(buffer)
    }

    pub async fn write_file_chunk(&self, path: &str, offset: u64, data: &[u8]) -> Result<()> {
        let file_path = self.root_dir.join(path);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path)
            .await?;
            
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        file.write_all(data).await?;
        
        Ok(())
    }
}

#[async_trait]
impl FileSystem for LocalFileSystem {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        let full_path = self.get_path(path);
        if !full_path.exists() {
            return Err(VDFSError::FileNotFound(full_path.to_string_lossy().into_owned()));
        }
        fs::read(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))
    }

    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<()> {
        let full_path = self.get_path(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| VDFSError::Io(e))?;
        }
        fs::write(&full_path, content)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<()> {
        let full_path = self.get_path(path);
        if !full_path.exists() {
            return Err(VDFSError::FileNotFound(full_path.to_string_lossy().into_owned()));
        }
        fs::remove_file(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<String>> {
        let full_path = self.get_path(path);
        if !full_path.exists() {
            return Err(VDFSError::FileNotFound(full_path.to_string_lossy().into_owned()));
        }
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        while let Some(entry) = read_dir.next_entry()
            .await
            .map_err(|e| VDFSError::Io(e))? {
            if let Ok(name) = entry.file_name().into_string() {
                entries.push(name);
            }
        }
        Ok(entries)
    }

    async fn create_dir(&self, path: &Path) -> Result<()> {
        let full_path = self.get_path(path);
        fs::create_dir_all(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    async fn delete_dir(&self, path: &Path) -> Result<()> {
        let full_path = self.get_path(path);
        if !full_path.exists() {
            return Err(VDFSError::FileNotFound(full_path.to_string_lossy().into_owned()));
        }
        fs::remove_dir_all(&full_path)
            .await
            .map_err(|e| VDFSError::Io(e))?;
        Ok(())
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        let full_path = self.get_path(path);
        Ok(full_path.exists())
    }
} 
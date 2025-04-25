use std::path::{Path, PathBuf};
use tokio::fs;
use crate::error::{Result, VDFSError};

#[derive(Debug, Clone)]
pub struct LocalFileSystem {
    root_dir: PathBuf,
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
} 
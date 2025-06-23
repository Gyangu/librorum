//! VDFS 使用示例

use librorum_core::vdfs::{
    VirtualPath, OpenMode,
    filesystem::{VirtualFileSystem, VirtualFileSystemImpl, FileOperations},
    storage::{StorageBackend, LocalStorage},
    metadata::{MetadataManager, DatabaseMetadataManager},
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建临时目录用于演示
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    println!("VDFS 演示开始...");
    println!("存储路径: {:?}", storage_path);
    
    // 1. 初始化存储后端
    let storage: Arc<dyn StorageBackend> = Arc::new(
        LocalStorage::new(storage_path.clone()).await?
    );
    
    // 2. 初始化元数据管理器
    let db_path = storage_path.join("metadata.db");
    let metadata: Arc<dyn MetadataManager> = Arc::new(
        DatabaseMetadataManager::new(&format!("sqlite://{}", db_path.display())).await?
    );
    
    // 3. 创建 VDFS 实例
    let vfs = VirtualFileSystemImpl::new(storage, metadata, 1024 * 1024); // 1MB chunks
    
    // 4. 创建目录
    println!("\n创建目录 /documents ...");
    vfs.create_dir(&VirtualPath::new("/documents")).await?;
    
    // 5. 创建并写入文件
    println!("\n创建文件 /documents/test.txt ...");
    let file_path = VirtualPath::new("/documents/test.txt");
    let file_handle = vfs.create_file(&file_path).await?;
    
    let content = "Hello, VDFS! 这是一个测试文件。\n这个文件存储在虚拟分布式文件系统中。";
    println!("写入内容: {} 字节", content.len());
    
    // 使用 FileOperations trait
    let file_ops: &dyn FileOperations = &vfs;
    file_ops.write(file_handle, 0, content.as_bytes()).await?;
    file_ops.close(file_handle).await?;
    
    // 6. 读取文件
    println!("\n读取文件 /documents/test.txt ...");
    let file_handle = vfs.open_file(&file_path, OpenMode::Read).await?;
    let mut buffer = vec![0u8; 1024];
    let bytes_read = file_ops.read(file_handle, 0, &mut buffer).await?;
    println!("读取内容: {}", String::from_utf8_lossy(&buffer[..bytes_read]));
    file_ops.close(file_handle).await?;
    
    // 7. 列出目录内容
    println!("\n列出 /documents 目录内容:");
    let entries = vfs.list_dir(&VirtualPath::new("/documents")).await?;
    for entry in entries {
        println!("  - {} ({}字节)", entry.name, entry.size);
    }
    
    // 8. 获取文件元数据
    println!("\n获取文件元数据:");
    let metadata = vfs.get_metadata(&file_path).await?;
    println!("  文件ID: {}", metadata.id);
    println!("  大小: {} 字节", metadata.size);
    println!("  创建时间: {:?}", metadata.created);
    
    println!("\nVDFS 演示完成！");
    println!("文件已成功存储在: {:?}", storage_path);
    
    Ok(())
}
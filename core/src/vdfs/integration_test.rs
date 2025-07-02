//! VDFS 集成测试 - 测试完整的 VDFS 实例创建和基本操作

use crate::vdfs::{VDFS, VDFSConfig};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_vdfs_creation() -> Result<(), Box<dyn std::error::Error>> {
    // 创建临时目录作为存储路径
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    // 创建 VDFS 配置
    let config = VDFSConfig {
        storage_path,
        chunk_size: 64 * 1024, // 64KB chunks for testing
        enable_compression: false,
        cache_memory_size: 16 * 1024 * 1024, // 16MB memory cache
        cache_disk_size: 64 * 1024 * 1024,   // 64MB disk cache
        replication_factor: 1, // Single replica for testing
        network_timeout: std::time::Duration::from_secs(5),
    };
    
    // 创建 VDFS 实例
    let vdfs = VDFS::new(config).await?;
    
    // 挂载文件系统
    vdfs.mount().await?;
    
    // 获取存储信息
    let stats = vdfs.stats().await?;
    println!("VDFS stats: {:?}", stats);
    
    // 卸载文件系统
    vdfs.unmount().await?;
    
    println!("✅ VDFS 实例创建和基本操作测试通过！");
    Ok(())
}

#[tokio::test]
async fn test_vdfs_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // 创建临时目录作为存储路径
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    let config = VDFSConfig {
        storage_path,
        ..Default::default()
    };
    
    let vdfs = VDFS::new(config).await?;
    vdfs.mount().await?;
    
    // 基础功能测试 - 只测试能否创建实例和挂载
    println!("✅ VDFS 实例创建和挂载成功");
    
    vdfs.unmount().await?;
    println!("✅ VDFS 卸载成功");
    
    println!("✅ VDFS 基础功能测试通过！");
    Ok(())
}

#[tokio::test]
async fn test_vdfs_with_compression() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    let config = VDFSConfig {
        storage_path,
        enable_compression: true,
        ..Default::default()
    };
    
    let vdfs = VDFS::new(config).await?;
    vdfs.mount().await?;
    
    // 测试带压缩的文件操作
    let test_data = b"This is a test file content that should be compressed when stored.";
    let file_handle = vdfs.create_file("/compressed_test.txt").await?;
    
    // 这里可以添加实际的写入测试，当 file_handle 实现了写入方法后
    println!("✅ 压缩模式下文件创建成功: {:?}", file_handle);
    
    vdfs.unmount().await?;
    
    println!("✅ 压缩模式测试通过！");
    Ok(())
}
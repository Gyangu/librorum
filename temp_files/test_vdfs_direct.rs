use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 直接测试VDFS功能
    let temp_dir = std::env::temp_dir().join("vdfs_test");
    std::fs::create_dir_all(&temp_dir)?;
    
    let vdfs_config = librorum_core::vdfs::VDFSConfig {
        storage_path: temp_dir,
        chunk_size: 4096,
        cache_memory_size: 64 * 1024 * 1024, // 64MB
        cache_disk_size: 512 * 1024 * 1024, // 512MB
        replication_factor: 3,
        network_timeout: std::time::Duration::from_secs(30),
    };
    
    println!("Creating VDFS instance...");
    let vdfs = librorum_core::vdfs::VDFS::new(vdfs_config).await?;
    
    println!("Testing write_file...");
    let test_data = b"Hello VDFS! This is a test file.";
    vdfs.write_file("/test.txt", test_data).await?;
    println!("✓ File written successfully");
    
    println!("Testing read_file...");
    let read_data = vdfs.read_file("/test.txt").await?;
    println!("✓ File read successfully: {} bytes", read_data.len());
    
    println!("Comparing data...");
    if read_data == test_data {
        println!("✓ Data matches perfectly!");
    } else {
        println!("✗ Data mismatch!");
        println!("Original: {:?}", std::str::from_utf8(test_data));
        println!("Read: {:?}", std::str::from_utf8(&read_data));
    }
    
    // 清理
    std::fs::remove_dir_all(&temp_dir).ok();
    
    Ok(())
}
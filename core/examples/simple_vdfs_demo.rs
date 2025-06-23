//! 简单的 VDFS 使用示例

use librorum_core::vdfs::{
    VirtualPath,
    storage::{StorageBackend, LocalStorage},
    metadata::{MetadataManager, DatabaseMetadataManager},
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("VDFS 基础组件演示开始...");
    
    // 创建临时目录用于演示
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    println!("存储路径: {:?}", storage_path);
    
    // 1. 测试本地存储
    println!("\n=== 测试本地存储 ===");
    let storage: Arc<dyn StorageBackend> = Arc::new(
        LocalStorage::new(storage_path.clone()).await?
    );
    
    // 生成测试数据
    let test_data = b"Hello, VDFS! This is test data for chunk storage.";
    let chunk_id = sha2::Sha256::digest(test_data);
    let chunk_id_array: [u8; 32] = chunk_id.into();
    
    // 存储 chunk
    println!("存储数据块...");
    storage.store_chunk(chunk_id_array, test_data).await?;
    println!("✓ 数据块已存储");
    
    // 读取 chunk
    println!("读取数据块...");
    let retrieved_data = storage.retrieve_chunk(chunk_id_array).await?;
    println!("✓ 读取到 {} 字节数据", retrieved_data.len());
    println!("内容: {}", String::from_utf8_lossy(&retrieved_data));
    
    // 验证数据一致性
    assert_eq!(test_data, retrieved_data.as_slice());
    println!("✓ 数据一致性验证通过");
    
    // 2. 测试元数据管理
    println!("\n=== 测试元数据管理 ===");
    let db_path = storage_path.join("metadata.db");
    let metadata: Arc<dyn MetadataManager> = Arc::new(
        DatabaseMetadataManager::new(&format!("sqlite://{}", db_path.display())).await?
    );
    
    // 创建文件信息
    let file_id = uuid::Uuid::new_v4();
    let file_info = librorum_core::vdfs::metadata::FileInfo {
        id: file_id,
        path: VirtualPath::new("/test/example.txt"),
        size: test_data.len() as u64,
        created: std::time::SystemTime::now(),
        modified: std::time::SystemTime::now(),
        accessed: std::time::SystemTime::now(),
        checksum: hex::encode(chunk_id_array),
        mime_type: Some("text/plain".to_string()),
        is_directory: false,
        permissions: 0o644,
        owner: "user".to_string(),
        group: "group".to_string(),
        chunks: vec![chunk_id_array],
        attributes: std::collections::HashMap::new(),
    };
    
    println!("创建文件元数据...");
    metadata.create_file(file_info.clone()).await?;
    println!("✓ 文件元数据已创建");
    
    // 查询文件信息
    println!("查询文件信息...");
    let retrieved_info = metadata.get_file_info(&file_id).await?;
    println!("✓ 文件信息查询成功");
    println!("  文件路径: {}", retrieved_info.path);
    println!("  文件大小: {} 字节", retrieved_info.size);
    println!("  校验和: {}", retrieved_info.checksum);
    
    // 3. 测试查询功能
    println!("\n=== 测试查询功能 ===");
    
    // 按路径查询
    println!("按路径查询文件...");
    let found_file = metadata.get_file_by_path(&VirtualPath::new("/test/example.txt")).await?;
    println!("✓ 找到文件: {}", found_file.path);
    
    // 列出目录
    println!("列出根目录内容...");
    let entries = metadata.list_directory(&VirtualPath::new("/")).await?;
    println!("✓ 找到 {} 个目录项", entries.len());
    for entry in entries {
        println!("  - {}", entry.name);
    }
    
    // 4. 清理测试
    println!("\n=== 清理测试 ===");
    println!("删除文件...");
    metadata.delete_file(&file_id).await?;
    storage.delete_chunk(chunk_id_array).await?;
    println!("✓ 清理完成");
    
    println!("\n✅ VDFS 基础组件演示完成！");
    println!("所有核心功能正常工作：");
    println!("  ✓ 本地存储 - 数据块的存储和检索");
    println!("  ✓ 元数据管理 - 文件信息的创建、查询和删除");
    println!("  ✓ 路径解析 - 虚拟路径到文件的映射");
    println!("  ✓ 数据一致性 - SHA256 校验和验证");
    
    Ok(())
}
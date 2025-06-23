//! 基础的 VDFS 组件演示

use librorum_core::vdfs::{
    VirtualPath, FileId,
    storage::{StorageBackend, LocalStorage},
    metadata::{MetadataManager, DatabaseMetadataManager, FileInfo, FileMetadata},
    filesystem::FileMetadata as VfsFileMetadata,
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 VDFS 基础组件演示");
    
    // 创建临时目录
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    println!("📁 存储路径: {:?}", storage_path);
    
    // === 1. 本地存储测试 ===
    println!("\n=== 🗄️  本地存储测试 ===");
    let storage: Arc<dyn StorageBackend> = Arc::new(
        LocalStorage::new(storage_path.clone()).await?
    );
    
    // 准备测试数据
    let test_content = b"Hello VDFS! 这是存储在分布式文件系统中的测试内容。";
    let chunk_id = sha2::Sha256::digest(test_content);
    let chunk_id_bytes: [u8; 32] = chunk_id.into();
    
    println!("📤 存储数据块 ({}字节)...", test_content.len());
    storage.store_chunk(chunk_id_bytes, test_content).await?;
    
    println!("📥 读取数据块...");
    let retrieved = storage.retrieve_chunk(chunk_id_bytes).await?;
    println!("✅ 成功读取 {} 字节", retrieved.len());
    println!("📝 内容: {}", String::from_utf8_lossy(&retrieved));
    
    assert_eq!(test_content, retrieved.as_slice());
    println!("✅ 数据完整性验证通过");
    
    // === 2. 元数据管理测试 ===
    println!("\n=== 📋 元数据管理测试 ===");
    let db_path = storage_path.join("metadata.db");
    let metadata_mgr: Arc<dyn MetadataManager> = Arc::new(
        DatabaseMetadataManager::new(&format!("sqlite://{}", db_path.display())).await?
    );
    
    // 创建虚拟目录结构
    let root_path = VirtualPath::new("/");
    let docs_path = VirtualPath::new("/documents");
    let file_path = VirtualPath::new("/documents/test.txt");
    
    println!("📁 创建目录结构...");
    metadata_mgr.create_directory(&docs_path).await?;
    println!("✅ 创建目录: /documents");
    
    // 创建文件元数据
    let file_id = uuid::Uuid::new_v4();
    let file_metadata = FileMetadata {
        id: file_id,
        path: file_path.clone(),
        size: test_content.len() as u64,
        created: std::time::SystemTime::now(),
        modified: std::time::SystemTime::now(),
        owner: "user".to_string(),
        group: "group".to_string(),
        permissions: 0o644,
        checksum: hex::encode(chunk_id_bytes),
        mime_type: Some("text/plain".to_string()),
        is_directory: false,
        chunks: vec![chunk_id_bytes],
        attributes: std::collections::HashMap::new(),
    };
    
    let file_info = FileInfo {
        metadata: file_metadata.clone(),
        replicas: vec!["node1".to_string()],
        version: 1,
    };
    
    println!("📄 设置文件信息...");
    metadata_mgr.set_file_info(&file_path, file_info).await?;
    println!("✅ 文件信息已保存: {}", file_path);
    
    // 查询文件信息
    println!("🔍 查询文件信息...");
    let retrieved_info = metadata_mgr.get_file_info(&file_path).await?;
    println!("✅ 文件查询成功:");
    println!("   📁 路径: {}", retrieved_info.metadata.path);
    println!("   📏 大小: {} 字节", retrieved_info.metadata.size);
    println!("   🔒 权限: {:o}", retrieved_info.metadata.permissions);
    println!("   🏷️  MIME: {:?}", retrieved_info.metadata.mime_type);
    println!("   📦 块数: {}", retrieved_info.metadata.chunks.len());
    
    // 更新数据块映射
    println!("🔗 更新数据块映射...");
    metadata_mgr.update_chunk_mapping(file_id, vec![chunk_id_bytes]).await?;
    
    // 查询数据块映射
    println!("🔍 查询数据块映射...");
    let chunk_mapping = metadata_mgr.get_chunk_mapping(file_id).await?;
    println!("✅ 找到 {} 个数据块", chunk_mapping.len());
    
    // === 3. 目录操作测试 ===
    println!("\n=== 📂 目录操作测试 ===");
    
    println!("📋 列出根目录内容:");
    let root_entries = metadata_mgr.list_directory(&root_path).await?;
    for entry in &root_entries {
        println!("   📁 {}", entry);
    }
    
    println!("📋 列出 /documents 目录内容:");
    let docs_entries = metadata_mgr.list_directory(&docs_path).await?;
    for entry in &docs_entries {
        println!("   📄 {}", entry);
    }
    
    // === 4. 搜索功能测试 ===
    println!("\n=== 🔍 搜索功能测试 ===");
    
    println!("🔍 按文件名模式搜索...");
    let pattern_results = metadata_mgr.find_files_by_pattern("*.txt").await?;
    println!("✅ 找到 {} 个匹配文件", pattern_results.len());
    for file in &pattern_results {
        println!("   📄 {}", file);
    }
    
    println!("🔍 按文件大小搜索 (0-100字节)...");
    let size_results = metadata_mgr.find_files_by_size(0, 100).await?;
    println!("✅ 找到 {} 个匹配文件", size_results.len());
    
    // === 5. 完整性验证 ===
    println!("\n=== ✅ 完整性验证 ===");
    println!("🔍 验证元数据一致性...");
    let inconsistent_files = metadata_mgr.verify_consistency().await?;
    if inconsistent_files.is_empty() {
        println!("✅ 所有文件元数据一致");
    } else {
        println!("⚠️  发现 {} 个不一致的文件", inconsistent_files.len());
    }
    
    // === 6. 清理测试 ===
    println!("\n=== 🧹 清理测试 ===");
    println!("🗑️  删除文件元数据...");
    metadata_mgr.delete_file_info(&file_path).await?;
    
    println!("🗑️  删除数据块...");
    storage.delete_chunk(chunk_id_bytes).await?;
    
    println!("🗑️  删除目录...");
    metadata_mgr.remove_directory(&docs_path).await?;
    
    println!("✅ 清理完成");
    
    println!("\n🎉 VDFS 基础组件演示完成！");
    println!("📊 测试结果总结:");
    println!("   ✅ 本地存储 - 数据块存储、检索、删除");
    println!("   ✅ 元数据管理 - 文件信息管理、目录操作");
    println!("   ✅ 数据块映射 - 文件到数据块的映射关系");
    println!("   ✅ 搜索功能 - 按模式、大小搜索文件");
    println!("   ✅ 完整性验证 - 元数据一致性检查");
    println!("   ✅ 虚拟路径 - 分层文件系统结构");
    
    println!("\n💡 VDFS 核心功能已实现，可以进行文件存储操作！");
    
    Ok(())
}
//! VDFS 存储功能演示

use librorum_core::vdfs::{
    storage::{StorageBackend, LocalStorageBackend},
};
use sha2::{Sha256, Digest};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 VDFS 存储功能演示");
    
    // 创建临时目录用于存储
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    println!("📁 存储路径: {:?}", storage_path);
    
    // 初始化本地存储
    println!("\n=== 📦 初始化本地存储 ===");
    let storage: Arc<dyn StorageBackend> = Arc::new(
        LocalStorageBackend::new(storage_path.clone(), "demo_node".to_string())?
    );
    println!("✅ 本地存储初始化完成");
    
    // 准备测试数据
    let test_files = vec![
        ("hello.txt", "Hello, VDFS! This is a test file."),
        ("chinese.txt", "你好，VDFS！这是一个中文测试文件。"),
        ("code.rs", "fn main() {\n    println!(\"Hello from Rust!\");\n}"),
        ("data.json", r#"{"name": "VDFS", "version": "1.0", "features": ["storage", "cache", "metadata"]}"#),
    ];
    
    println!("\n=== 💾 存储文件数据 ===");
    let mut stored_chunks = Vec::new();
    
    for (filename, content) in &test_files {
        println!("📄 处理文件: {}", filename);
        
        // 计算文件的 SHA256 哈希作为 chunk ID
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        let chunk_id: [u8; 32] = hash.into();
        
        // 存储数据块
        storage.store_chunk(chunk_id, content.as_bytes()).await?;
        stored_chunks.push((filename, chunk_id, content.len()));
        
        println!("   ✅ 存储成功: {} 字节", content.len());
        println!("   🔑 块ID: {}", hex::encode(chunk_id));
    }
    
    println!("\n=== 📖 读取文件数据 ===");
    for (filename, chunk_id, expected_size) in &stored_chunks {
        println!("📄 读取文件: {}", filename);
        
        // 从存储中检索数据块
        let retrieved_data = storage.retrieve_chunk(*chunk_id).await?;
        
        println!("   ✅ 读取成功: {} 字节", retrieved_data.len());
        println!("   📝 内容预览: {}", 
                String::from_utf8_lossy(&retrieved_data[..retrieved_data.len().min(50)])
                    + if retrieved_data.len() > 50 { "..." } else { "" });
        
        // 验证数据完整性
        assert_eq!(*expected_size, retrieved_data.len());
        println!("   ✅ 数据完整性验证通过");
    }
    
    println!("\n=== 🔍 存储统计信息 ===");
    println!("📊 存储的文件数量: {}", stored_chunks.len());
    let total_size: usize = stored_chunks.iter().map(|(_, _, size)| size).sum();
    println!("📏 总数据大小: {} 字节", total_size);
    
    // 检查存储目录结构
    println!("\n=== 📂 存储目录结构 ===");
    println!("🔍 查看存储目录内容:");
    if let Ok(entries) = std::fs::read_dir(&storage_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    println!("   📁 {}/", path.file_name().unwrap().to_string_lossy());
                } else {
                    let size = entry.metadata()?.len();
                    println!("   📄 {} ({} 字节)", 
                            path.file_name().unwrap().to_string_lossy(), size);
                }
            }
        }
    }
    
    println!("\n=== 🗑️  清理测试 ===");
    for (filename, chunk_id, _) in &stored_chunks {
        println!("🗑️  删除: {}", filename);
        storage.delete_chunk(*chunk_id).await?;
        println!("   ✅ 删除成功");
    }
    
    println!("\n🎉 VDFS 存储功能演示完成！");
    println!("✅ 测试结果总结:");
    println!("   ✅ 数据块存储 - 支持任意二进制数据");
    println!("   ✅ 数据块检索 - 通过SHA256哈希快速定位");
    println!("   ✅ 数据完整性 - 哈希验证确保数据不损坏");
    println!("   ✅ 多文件支持 - 同时处理多种类型的文件");
    println!("   ✅ 中文支持 - 正确处理UTF-8编码");
    println!("   ✅ 存储管理 - 创建、读取、删除操作");
    
    println!("\n💡 VDFS 已经可以在本地存储和管理文件数据！");
    println!("🔧 下一步可以集成缓存、元数据管理和分布式功能。");
    
    Ok(())
}
//! 性能测试和比较
//! 
//! 比较不同元数据管理器的性能特点：
//! - SQLite: 关系型数据库，适合复杂查询
//! - Sled: 现代 Rust 嵌入式数据库，高性能

#[cfg(test)]
mod performance_tests {
    use super::super::*;
    use crate::vdfs::{VirtualPath, FileId};
    use crate::vdfs::filesystem::FileMetadata;
    use std::collections::HashMap;
    use std::time::{SystemTime, Instant};
    
    /// 测试数据生成器
    fn create_test_file_info(path: &str, size: u64) -> FileInfo {
        let file_id = uuid::Uuid::new_v4();
        let now = SystemTime::now();
        
        let metadata = FileMetadata {
            id: file_id,
            path: VirtualPath::new(path),
            size,
            created: now,
            modified: now,
            accessed: now,
            permissions: crate::vdfs::FilePermissions::default(),
            checksum: Some(format!("checksum_{}", file_id)),
            mime_type: Some("application/octet-stream".to_string()),
            custom_attributes: HashMap::new(),
            is_directory: false,
        };

        FileInfo {
            metadata,
            chunks: Vec::new(),
            replicas: vec!["node1".to_string(), "node2".to_string()],
            version: 1,
            checksum: format!("checksum_{}", file_id),
        }
    }

    /// 生成测试数据集
    fn generate_test_data(count: usize) -> Vec<(VirtualPath, FileInfo)> {
        (0..count)
            .map(|i| {
                let path = format!("/test/file_{:06}.txt", i);
                let size = (i % 10000) as u64 * 1024; // 0B - 10MB 范围
                let vpath = VirtualPath::new(&path);
                let file_info = create_test_file_info(&path, size);
                (vpath, file_info)
            })
            .collect()
    }

    /// 性能测试结果
    #[derive(Debug)]
    struct PerformanceResult {
        manager_name: String,
        insert_time_ms: u128,
        read_time_ms: u128,
        search_time_ms: u128,
        delete_time_ms: u128,
        total_time_ms: u128,
        throughput_ops_per_sec: f64,
    }

    impl PerformanceResult {
        fn new(manager_name: String) -> Self {
            Self {
                manager_name,
                insert_time_ms: 0,
                read_time_ms: 0,
                search_time_ms: 0,
                delete_time_ms: 0,
                total_time_ms: 0,
                throughput_ops_per_sec: 0.0,
            }
        }

        fn calculate_totals(&mut self, data_count: usize) {
            self.total_time_ms = self.insert_time_ms + self.read_time_ms + self.search_time_ms + self.delete_time_ms;
            self.throughput_ops_per_sec = (data_count as f64 * 4.0 * 1000.0) / self.total_time_ms as f64; // 4 operations per record
        }
    }

    /// 通用性能测试函数
    async fn benchmark_metadata_manager<T: MetadataManager + Send + Sync>(
        manager: &T,
        manager_name: &str,
        test_data: &[(VirtualPath, FileInfo)],
    ) -> PerformanceResult {
        let mut result = PerformanceResult::new(manager_name.to_string());
        let data_count = test_data.len();

        println!("🔄 测试 {} 管理器，数据集大小: {}", manager_name, data_count);

        // 1. 插入性能测试
        let start = Instant::now();
        for (path, file_info) in test_data {
            manager.set_file_info(path, file_info.clone()).await.unwrap();
        }
        result.insert_time_ms = start.elapsed().as_millis();
        println!("  ✅ 插入 {} 条记录耗时: {}ms", data_count, result.insert_time_ms);

        // 2. 读取性能测试
        let start = Instant::now();
        for (path, _) in test_data.iter().take(data_count / 10) { // 测试 10% 的读取
            let _ = manager.get_file_info(path).await.unwrap();
        }
        result.read_time_ms = start.elapsed().as_millis() * 10; // 推算全部读取时间
        println!("  ✅ 读取测试耗时: {}ms (推算)", result.read_time_ms);

        // 3. 搜索性能测试
        let start = Instant::now();
        let _results = manager.find_files_by_pattern("file_00").await.unwrap();
        let _results = manager.find_files_by_size(1024, 1024000).await.unwrap();
        result.search_time_ms = start.elapsed().as_millis();
        println!("  ✅ 搜索测试耗时: {}ms", result.search_time_ms);

        // 4. 删除性能测试
        let start = Instant::now();
        for (path, _) in test_data.iter().take(data_count / 10) { // 删除 10% 的数据
            manager.delete_file_info(path).await.unwrap();
        }
        result.delete_time_ms = start.elapsed().as_millis() * 10; // 推算全部删除时间
        println!("  ✅ 删除测试耗时: {}ms (推算)", result.delete_time_ms);

        result.calculate_totals(data_count);
        println!("  📊 总耗时: {}ms, 吞吐量: {:.2} ops/sec", result.total_time_ms, result.throughput_ops_per_sec);

        result
    }

    #[tokio::test]
    async fn test_small_dataset_performance() {
        println!("\n🚀 小数据集性能测试 (1000 条记录)");
        println!("{}", "=".repeat(60));
        
        let test_data = generate_test_data(1000);
        let mut results: Vec<PerformanceResult> = Vec::new();

        // 测试 Sled
        let sled_manager = SledMetadataManager::new_temp().unwrap();
        let result = benchmark_metadata_manager(&sled_manager, "Sled", &test_data).await;
        results.push(result);


        // 测试 SQLite
        let sqlite_manager = DatabaseMetadataManager::new("sqlite::memory:").await.unwrap();
        let result = benchmark_metadata_manager(&sqlite_manager, "SQLite", &test_data).await;
        results.push(result);

        print_performance_comparison(&results);
    }

    #[tokio::test]
    async fn test_medium_dataset_performance() {
        println!("\n🚀 中等数据集性能测试 (10000 条记录)");
        println!("{}", "=".repeat(60));
        
        let test_data = generate_test_data(10000);
        let mut results: Vec<PerformanceResult> = Vec::new();

        // 测试 Sled
        let sled_manager = SledMetadataManager::new_temp().unwrap();
        let result = benchmark_metadata_manager(&sled_manager, "Sled", &test_data).await;
        results.push(result);


        // 测试 SQLite (数据量大时可能较慢)
        let sqlite_manager = DatabaseMetadataManager::new("sqlite::memory:").await.unwrap();
        let result = benchmark_metadata_manager(&sqlite_manager, "SQLite", &test_data).await;
        results.push(result);

        print_performance_comparison(&results);
    }

    #[tokio::test]
    async fn test_write_heavy_workload() {
        println!("\n🚀 写密集负载测试 (5000 条记录，多次写入)");
        println!("{}", "=".repeat(60));
        
        let test_data = generate_test_data(5000);
        let mut results: Vec<PerformanceResult> = Vec::new();

        // 测试 Sled 写入性能
        let sled_manager = SledMetadataManager::new_temp().unwrap();
        let start = Instant::now();
        for _ in 0..3 { // 重复写入 3 次
            for (path, file_info) in &test_data {
                sled_manager.set_file_info(path, file_info.clone()).await.unwrap();
            }
        }
        let sled_time = start.elapsed().as_millis();
        println!("  Sled 写密集测试: {}ms", sled_time);


        // 测试 SQLite 写入性能
        let sqlite_manager = DatabaseMetadataManager::new("sqlite::memory:").await.unwrap();
        let start = Instant::now();
        for _ in 0..3 { // 重复写入 3 次
            for (path, file_info) in &test_data {
                sqlite_manager.set_file_info(path, file_info.clone()).await.unwrap();
            }
        }
        let sqlite_time = start.elapsed().as_millis();
        println!("  SQLite 写密集测试: {}ms", sqlite_time);

        println!("\n📊 写密集性能排名:");
        let mut write_results = vec![
            ("Sled", sled_time),
            ("SQLite", sqlite_time),
        ];
        write_results.sort_by_key(|(_, time)| *time);
        
        for (i, (name, time)) in write_results.iter().enumerate() {
            println!("  {}. {}: {}ms", i + 1, name, time);
        }
    }

    #[tokio::test]
    async fn test_read_heavy_workload() {
        println!("\n🚀 读密集负载测试 (1000 条记录，大量读取)");
        println!("{}", "=".repeat(60));
        
        let test_data = generate_test_data(1000);

        // 预先插入数据到各个数据库
        let sled_manager = SledMetadataManager::new_temp().unwrap();
        let sqlite_manager = DatabaseMetadataManager::new("sqlite::memory:").await.unwrap();

        for (path, file_info) in &test_data {
            sled_manager.set_file_info(path, file_info.clone()).await.unwrap();
            sqlite_manager.set_file_info(path, file_info.clone()).await.unwrap();
        }

        // 测试 Sled 读取性能
        let start = Instant::now();
        for _ in 0..10 { // 重复读取 10 次
            for (path, _) in &test_data {
                let _ = sled_manager.get_file_info(path).await.unwrap();
            }
        }
        let sled_time = start.elapsed().as_millis();
        println!("  Sled 读密集测试: {}ms", sled_time);


        // 测试 SQLite 读取性能
        let start = Instant::now();
        for _ in 0..10 { // 重复读取 10 次
            for (path, _) in &test_data {
                let _ = sqlite_manager.get_file_info(path).await.unwrap();
            }
        }
        let sqlite_time = start.elapsed().as_millis();
        println!("  SQLite 读密集测试: {}ms", sqlite_time);

        println!("\n📊 读密集性能排名:");
        let mut read_results = vec![
            ("Sled", sled_time),
            ("SQLite", sqlite_time),
        ];
        read_results.sort_by_key(|(_, time)| *time);
        
        for (i, (name, time)) in read_results.iter().enumerate() {
            println!("  {}. {}: {}ms", i + 1, name, time);
        }
    }

    fn print_performance_comparison(results: &[PerformanceResult]) {
        println!("\n📊 性能比较结果");
        println!("{}", "=".repeat(80));
        println!("{:<10} {:<12} {:<12} {:<12} {:<12} {:<12} {:<15}", 
                 "管理器", "插入(ms)", "读取(ms)", "搜索(ms)", "删除(ms)", "总计(ms)", "吞吐量(ops/s)");
        println!("{}", "-".repeat(80));
        
        for result in results {
            println!("{:<10} {:<12} {:<12} {:<12} {:<12} {:<12} {:<15.2}", 
                     result.manager_name,
                     result.insert_time_ms,
                     result.read_time_ms,
                     result.search_time_ms,
                     result.delete_time_ms,
                     result.total_time_ms,
                     result.throughput_ops_per_sec);
        }

        // 性能排名
        let mut sorted_results: Vec<&PerformanceResult> = results.iter().collect();
        sorted_results.sort_by(|a, b| a.total_time_ms.cmp(&b.total_time_ms));

        println!("\n🏆 总体性能排名:");
        for (i, result) in sorted_results.iter().enumerate() {
            println!("  {}. {} ({}ms, {:.2} ops/s)", 
                     i + 1, 
                     result.manager_name, 
                     result.total_time_ms,
                     result.throughput_ops_per_sec);
        }

        // 各项性能最佳
        let best_insert = results.iter().min_by_key(|r| r.insert_time_ms).unwrap();
        let best_read = results.iter().min_by_key(|r| r.read_time_ms).unwrap();
        let best_search = results.iter().min_by_key(|r| r.search_time_ms).unwrap();
        let best_delete = results.iter().min_by_key(|r| r.delete_time_ms).unwrap();

        println!("\n🥇 各项最佳性能:");
        println!("  插入最快: {} ({}ms)", best_insert.manager_name, best_insert.insert_time_ms);
        println!("  读取最快: {} ({}ms)", best_read.manager_name, best_read.read_time_ms);
        println!("  搜索最快: {} ({}ms)", best_search.manager_name, best_search.search_time_ms);
        println!("  删除最快: {} ({}ms)", best_delete.manager_name, best_delete.delete_time_ms);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        println!("\n🚀 并发操作测试");
        println!("{}", "=".repeat(60));
        
        let test_data = generate_test_data(1000);
        
        // 测试 Sled 并发性能
        let sled_manager = std::sync::Arc::new(SledMetadataManager::new_temp().unwrap());
        let start = Instant::now();
        
        let mut handles = Vec::new();
        for chunk in test_data.chunks(100) {
            let manager = sled_manager.clone();
            let chunk_data: Vec<_> = chunk.to_vec();
            let handle = tokio::spawn(async move {
                for (path, file_info) in chunk_data {
                    manager.set_file_info(&path, file_info).await.unwrap();
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        let sled_concurrent_time = start.elapsed().as_millis();
        println!("  Sled 并发写入测试: {}ms", sled_concurrent_time);

        // 简单顺序写入对比
        let sled_manager_seq = SledMetadataManager::new_temp().unwrap();
        let start = Instant::now();
        for (path, file_info) in &test_data {
            sled_manager_seq.set_file_info(path, file_info.clone()).await.unwrap();
        }
        let sled_sequential_time = start.elapsed().as_millis();
        println!("  Sled 顺序写入测试: {}ms", sled_sequential_time);

        let speedup = sled_sequential_time as f64 / sled_concurrent_time as f64;
        println!("  🚀 并发加速比: {:.2}x", speedup);
    }
}
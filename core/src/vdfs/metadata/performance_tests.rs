//! Sled 元数据管理器性能测试
//! 
//! 测试 Sled 元数据管理器的性能特点：
//! - Sled: 现代 Rust 嵌入式数据库，高性能
//! - 内存管理器: 用于性能基准对比

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
        println!("  插入操作: {}ms", result.insert_time_ms);

        // 2. 读取性能测试
        let start = Instant::now();
        for (path, _) in test_data {
            let _ = manager.get_file_info(path).await.unwrap();
        }
        result.read_time_ms = start.elapsed().as_millis();
        println!("  读取操作: {}ms", result.read_time_ms);

        // 3. 搜索性能测试
        let start = Instant::now();
        for i in 0..10 {
            let pattern = format!("*file_{:02}*", i);
            let _ = manager.find_files_by_pattern(&pattern).await.unwrap();
        }
        result.search_time_ms = start.elapsed().as_millis();
        println!("  搜索操作: {}ms", result.search_time_ms);

        // 4. 删除性能测试
        let start = Instant::now();
        for (path, _) in test_data {
            manager.delete_file_info(path).await.unwrap();
        }
        result.delete_time_ms = start.elapsed().as_millis();
        println!("  删除操作: {}ms", result.delete_time_ms);

        result.calculate_totals(data_count);
        println!("  总时间: {}ms, 吞吐量: {:.2} ops/sec\n", 
                 result.total_time_ms, result.throughput_ops_per_sec);

        result
    }

    /// 性能对比报告
    fn print_performance_comparison(results: &[PerformanceResult]) {
        println!("📊 性能对比报告");
        println!("{}", "=".repeat(80));
        println!("{:<20} {:>12} {:>12} {:>12} {:>12} {:>15}", 
                 "管理器", "插入(ms)", "读取(ms)", "搜索(ms)", "删除(ms)", "吞吐量(ops/s)");
        println!("{}", "-".repeat(80));
        
        for result in results {
            println!("{:<20} {:>12} {:>12} {:>12} {:>12} {:>15.2}", 
                     result.manager_name,
                     result.insert_time_ms,
                     result.read_time_ms, 
                     result.search_time_ms,
                     result.delete_time_ms,
                     result.throughput_ops_per_sec);
        }
        
        // 找出最快的管理器
        if let Some(fastest) = results.iter().max_by(|a, b| 
            a.throughput_ops_per_sec.partial_cmp(&b.throughput_ops_per_sec).unwrap()) {
            println!("\n🏆 最佳性能: {} ({:.2} ops/sec)", 
                     fastest.manager_name, fastest.throughput_ops_per_sec);
        }
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

        // 测试内存管理器
        let memory_manager = SimpleMetadataManager::new();
        let result = benchmark_metadata_manager(&memory_manager, "Memory", &test_data).await;
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

        // 测试内存管理器 (大数据集可能较慢)
        let memory_manager = SimpleMetadataManager::new();
        let result = benchmark_metadata_manager(&memory_manager, "Memory", &test_data).await;
        results.push(result);

        print_performance_comparison(&results);
    }

    #[tokio::test]
    async fn test_write_heavy_workload() {
        println!("\n🚀 写密集负载测试 (5000 条记录，多次写入)");
        println!("{}", "=".repeat(60));
        
        let test_data = generate_test_data(5000);

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

        // 测试内存管理器写入性能
        let memory_manager = SimpleMetadataManager::new();
        let start = Instant::now();
        for _ in 0..3 { // 重复写入 3 次
            for (path, file_info) in &test_data {
                memory_manager.set_file_info(path, file_info.clone()).await.unwrap();
            }
        }
        let memory_time = start.elapsed().as_millis();
        println!("  Memory 写密集测试: {}ms", memory_time);

        println!("\n📊 写密集性能排名:");
        let mut write_results = vec![
            ("Sled", sled_time),
            ("Memory", memory_time),
        ];
        write_results.sort_by_key(|(_, time)| *time);
        
        for (i, (name, time)) in write_results.iter().enumerate() {
            let rank = match i {
                0 => "🥇",
                1 => "🥈", 
                _ => "🥉",
            };
            println!("  {} {}: {}ms", rank, name, time);
        }
    }

    #[tokio::test]
    async fn test_read_heavy_workload() {
        println!("\n🚀 读密集负载测试 (1000 条记录，多次读取)");
        println!("{}", "=".repeat(60));
        
        let test_data = generate_test_data(1000);

        // 预先插入数据到各个数据库
        let sled_manager = SledMetadataManager::new_temp().unwrap();
        let memory_manager = SimpleMetadataManager::new();

        for (path, file_info) in &test_data {
            sled_manager.set_file_info(path, file_info.clone()).await.unwrap();
            memory_manager.set_file_info(path, file_info.clone()).await.unwrap();
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

        // 测试内存管理器读取性能
        let start = Instant::now();
        for _ in 0..10 { // 重复读取 10 次
            for (path, _) in &test_data {
                let _ = memory_manager.get_file_info(path).await.unwrap();
            }
        }
        let memory_time = start.elapsed().as_millis();
        println!("  Memory 读密集测试: {}ms", memory_time);

        println!("\n📊 读密集性能排名:");
        let mut read_results = vec![
            ("Sled", sled_time),
            ("Memory", memory_time),
        ];
        read_results.sort_by_key(|(_, time)| *time);
        
        for (i, (name, time)) in read_results.iter().enumerate() {
            let rank = match i {
                0 => "🥇",
                1 => "🥈",
                _ => "🥉", 
            };
            println!("  {} {}: {}ms", rank, name, time);
        }
    }

    #[tokio::test]
    async fn test_sled_specific_features() {
        println!("\n🚀 Sled 特性测试");
        println!("{}", "=".repeat(40));
        
        let sled_manager = SledMetadataManager::new_temp().unwrap();
        let test_data = generate_test_data(1000);

        // 测试批量插入
        let start = Instant::now();
        for (path, file_info) in &test_data {
            sled_manager.set_file_info(path, file_info.clone()).await.unwrap();
        }
        let batch_time = start.elapsed().as_millis();
        println!("  批量插入 1000 条记录: {}ms", batch_time);

        // 测试一致性验证
        let start = Instant::now();
        let inconsistent_files = sled_manager.verify_consistency().await.unwrap();
        let verify_time = start.elapsed().as_millis();
        println!("  一致性验证: {}ms (发现 {} 个不一致文件)", verify_time, inconsistent_files.len());

        // 测试索引重建
        let start = Instant::now();
        sled_manager.rebuild_index().await.unwrap();
        let rebuild_time = start.elapsed().as_millis();
        println!("  索引重建: {}ms", rebuild_time);

        println!("\n✅ Sled 特性测试完成");
    }
}
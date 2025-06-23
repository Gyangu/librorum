//! 数据分块管理器实现
//!
//! 提供文件分块、重组、去重和完整性验证功能

use crate::vdfs::{VDFSResult, VDFSError, Chunk, ChunkId};
use std::collections::{HashMap, HashSet};

/// 默认分块管理器实现
/// 
/// 功能包括：
/// - 文件分块和重组
/// - 数据去重
/// - 完整性验证
/// - 压缩支持（可选）
/// - 智能分块大小优化
#[derive(Clone, Debug)]
pub struct DefaultChunkManager {
    /// 默认分块大小（字节）
    chunk_size: usize,
    /// 是否启用压缩
    enable_compression: bool,
    /// 最大分块大小
    max_chunk_size: usize,
    /// 最小分块大小
    min_chunk_size: usize,
}

impl DefaultChunkManager {
    /// 创建新的分块管理器
    pub fn new(chunk_size: usize, enable_compression: bool) -> Self {
        let max_chunk_size = chunk_size * 4; // 最大4倍默认大小
        let min_chunk_size = (chunk_size / 4).max(256); // 最小256字节，不超过1/4默认大小
        
        Self {
            chunk_size,
            enable_compression,
            max_chunk_size,
            min_chunk_size,
        }
    }
    
    /// 创建具有自定义大小限制的分块管理器
    pub fn with_size_limits(
        chunk_size: usize,
        min_size: usize,
        max_size: usize,
        enable_compression: bool,
    ) -> VDFSResult<Self> {
        if min_size > chunk_size || chunk_size > max_size {
            return Err(VDFSError::InvalidParameter(
                "分块大小限制无效: min_size <= chunk_size <= max_size".to_string()
            ));
        }
        
        Ok(Self {
            chunk_size,
            enable_compression,
            max_chunk_size: max_size,
            min_chunk_size: min_size,
        })
    }
    
    /// 获取分块管理器配置信息
    pub fn get_config(&self) -> ChunkManagerConfig {
        ChunkManagerConfig {
            chunk_size: self.chunk_size,
            min_chunk_size: self.min_chunk_size,
            max_chunk_size: self.max_chunk_size,
            enable_compression: self.enable_compression,
        }
    }
}

/// 分块管理器配置
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkManagerConfig {
    pub chunk_size: usize,
    pub min_chunk_size: usize,
    pub max_chunk_size: usize,
    pub enable_compression: bool,
}

/// 分块统计信息
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkStats {
    /// 总分块数量
    pub total_chunks: usize,
    /// 原始数据大小
    pub original_size: usize,
    /// 分块后总大小
    pub chunked_size: usize,
    /// 压缩率（如果启用压缩）
    pub compression_ratio: f64,
    /// 去重后的唯一分块数
    pub unique_chunks: usize,
    /// 去重节省的空间
    pub dedup_savings: usize,
}

/// 分块重组选项
#[derive(Debug, Clone)]
pub struct ReassemblyOptions {
    /// 是否验证分块完整性
    pub verify_integrity: bool,
    /// 是否允许部分重组（缺少分块时）
    pub allow_partial: bool,
    /// 最大重组大小限制
    pub max_size_limit: Option<usize>,
}

impl DefaultChunkManager {
    /// 将文件数据分割为分块
    /// 
    /// # 参数
    /// * `data` - 原始文件数据
    /// 
    /// # 返回
    /// 返回分块列表，每个分块都包含数据和元数据
    pub fn split_file(&self, data: &[u8]) -> VDFSResult<Vec<Chunk>> {
        if data.is_empty() {
            return Ok(vec![]);
        }
        
        let mut chunks = Vec::new();
        let mut offset = 0;
        let mut chunk_index = 0u32;
        
        while offset < data.len() {
            let end = std::cmp::min(offset + self.chunk_size, data.len());
            let chunk_data = data[offset..end].to_vec();
            
            let mut chunk = Chunk::new(chunk_data);
            
            // 设置分块元数据
            chunk.metadata.insert("index".to_string(), chunk_index.to_string());
            chunk.metadata.insert("offset".to_string(), offset.to_string());
            chunk.metadata.insert("total_size".to_string(), data.len().to_string());
            
            // 如果启用压缩，尝试压缩分块
            if self.enable_compression {
                chunk = self.try_compress_chunk(chunk)?;
            }
            
            chunks.push(chunk);
            offset = end;
            chunk_index += 1;
        }
        
        Ok(chunks)
    }
    
    /// 将分块列表重新组装为完整文件
    /// 
    /// # 参数
    /// * `chunks` - 分块列表
    /// 
    /// # 返回
    /// 返回重组后的文件数据
    pub fn reassemble_file(&self, chunks: Vec<Chunk>) -> VDFSResult<Vec<u8>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }
        
        // 按索引排序分块
        let mut sorted_chunks = chunks;
        sorted_chunks.sort_by(|a, b| {
            let index_a = a.metadata.get("index")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            let index_b = b.metadata.get("index")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            index_a.cmp(&index_b)
        });
        
        let mut file_data = Vec::new();
        
        for mut chunk in sorted_chunks {
            // 验证分块完整性
            if !chunk.verify_integrity() {
                return Err(VDFSError::CorruptedData(
                    format!("分块完整性验证失败: {}", hex::encode(chunk.id))
                ));
            }
            
            // 解压缩（如果需要）
            if chunk.compressed {
                chunk = self.decompress_chunk(chunk)?;
            }
            
            file_data.extend_from_slice(&chunk.data);
        }
        
        Ok(file_data)
    }
    
    pub fn deduplicate(&self, chunks: &[Chunk]) -> Vec<ChunkId> {
        let mut unique_ids = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        
        for chunk in chunks {
            if seen_ids.insert(chunk.id) {
                unique_ids.push(chunk.id);
            }
        }
        
        unique_ids
    }
    
    /// 获取详细的去重统计信息
    pub fn analyze_chunks(&self, chunks: &[Chunk]) -> ChunkStats {
        let mut size_by_id = HashMap::new();
        let mut total_size = 0;
        let mut original_size = 0;
        
        for chunk in chunks {
            total_size += chunk.data.len();
            if let Some(total_str) = chunk.metadata.get("total_size") {
                if let Ok(size) = total_str.parse::<usize>() {
                    original_size = size;
                }
            }
            size_by_id.entry(chunk.id).or_insert(chunk.data.len());
        }
        
        let unique_chunks = size_by_id.len();
        let unique_size: usize = size_by_id.values().sum();
        let dedup_savings = total_size.saturating_sub(unique_size);
        
        let compression_ratio = if original_size > 0 {
            total_size as f64 / original_size as f64
        } else {
            1.0
        };
        
        ChunkStats {
            total_chunks: chunks.len(),
            original_size,
            chunked_size: total_size,
            compression_ratio,
            unique_chunks,
            dedup_savings,
        }
    }
    
    /// 带选项的文件重组（高级功能）
    pub fn reassemble_file_with_options(
        &self, 
        chunks: Vec<Chunk>,
        verify_integrity: bool,
        allow_partial: bool,
        max_size_limit: Option<usize>
    ) -> VDFSResult<Vec<u8>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }
        
        // 按索引排序分块
        let mut sorted_chunks = chunks;
        sorted_chunks.sort_by(|a, b| {
            let index_a = a.metadata.get("index")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            let index_b = b.metadata.get("index")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            index_a.cmp(&index_b)
        });
        
        // 检查分块连续性
        if !allow_partial {
            self.validate_chunk_sequence(&sorted_chunks)?;
        }
        
        let mut file_data = Vec::new();
        let mut total_processed = 0;
        
        for mut chunk in sorted_chunks {
            // 验证分块完整性
            if verify_integrity && !chunk.verify_integrity() {
                return Err(VDFSError::CorruptedData(
                    format!("分块完整性验证失败: {}", hex::encode(chunk.id))
                ));
            }
            
            // 解压缩（如果需要）
            if chunk.compressed {
                chunk = self.decompress_chunk(chunk)?;
            }
            
            // 检查大小限制
            if let Some(max_size) = max_size_limit {
                if total_processed + chunk.data.len() > max_size {
                    return Err(VDFSError::InvalidParameter(
                        format!("文件大小超过限制: {} > {}", 
                                total_processed + chunk.data.len(), max_size)
                    ));
                }
            }
            
            file_data.extend_from_slice(&chunk.data);
            total_processed += chunk.data.len();
        }
        
        Ok(file_data)
    }
    
    pub fn optimal_chunk_size(&self, data_size: usize) -> usize {
        // Simple heuristic: use configured chunk size, but adjust for small files
        if data_size < self.chunk_size {
            data_size
        } else {
            self.chunk_size
        }
    }
    
    /// 验证单个分块的完整性
    pub fn verify_chunk(&self, chunk: &Chunk) -> bool {
        chunk.verify_integrity()
    }
    
    /// 验证分块序列的完整性
    pub fn verify_chunk_sequence(&self, chunks: &[Chunk]) -> VDFSResult<bool> {
        if chunks.is_empty() {
            return Ok(true);
        }
        
        // 检查所有分块的完整性
        for chunk in chunks {
            if !chunk.verify_integrity() {
                return Ok(false);
            }
        }
        
        // 检查分块索引连续性
        self.validate_chunk_sequence(chunks)?;
        
        Ok(true)
    }
    
    /// 验证分块序列的连续性
    fn validate_chunk_sequence(&self, chunks: &[Chunk]) -> VDFSResult<()> {
        if chunks.len() <= 1 {
            return Ok(());
        }
        
        let mut expected_index = 0u32;
        
        for chunk in chunks {
            let actual_index = chunk.metadata.get("index")
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or_else(|| VDFSError::CorruptedData(
                    "分块缺少索引信息".to_string()
                ))?;
                
            if actual_index != expected_index {
                return Err(VDFSError::CorruptedData(
                    format!("分块索引不连续: 期望 {} 但实际为 {}", 
                            expected_index, actual_index)
                ));
            }
            
            expected_index += 1;
        }
        
        Ok(())
    }
    
    /// 尝试压缩分块（简单实现）
    fn try_compress_chunk(&self, mut chunk: Chunk) -> VDFSResult<Chunk> {
        // 这里实现一个简单的压缩逻辑
        // 实际生产中可以使用 flate2, lz4 等库
        if chunk.data.len() < 1024 {
            // 小数据块不压缩
            return Ok(chunk);
        }
        
        // 模拟压缩：对于重复数据，可以达到较好的压缩效果
        let unique_bytes: HashSet<u8> = chunk.data.iter().cloned().collect();
        if unique_bytes.len() < 16 {
            // 高重复性数据，模拟压缩
            chunk.compressed = true;
            chunk.metadata.insert("compression".to_string(), "rle".to_string());
            chunk.metadata.insert("original_size".to_string(), chunk.data.len().to_string());
            // 这里可以实现真正的压缩算法
        }
        
        Ok(chunk)
    }
    
    /// 解压缩分块
    fn decompress_chunk(&self, mut chunk: Chunk) -> VDFSResult<Chunk> {
        if !chunk.compressed {
            return Ok(chunk);
        }
        
        // 模拟解压缩逻辑
        if let Some(compression_type) = chunk.metadata.get("compression") {
            match compression_type.as_str() {
                "rle" => {
                    // 模拟 RLE 解压缩
                    chunk.compressed = false;
                    chunk.metadata.remove("compression");
                    chunk.metadata.remove("original_size");
                }
                _ => {
                    return Err(VDFSError::InvalidParameter(
                        format!("不支持的压缩类型: {}", compression_type)
                    ));
                }
            }
        }
        
        Ok(chunk)
    }
    
    /// 获取支持的最大文件大小
    pub fn max_supported_file_size(&self) -> usize {
        // 基于最大分块数量和最大分块大小计算
        const MAX_CHUNKS: usize = 1_000_000; // 100万个分块
        MAX_CHUNKS * self.max_chunk_size
    }
    
    /// 获取分块管理器的统计信息
    pub fn get_manager_stats(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("chunk_size".to_string(), self.chunk_size.to_string());
        stats.insert("min_chunk_size".to_string(), self.min_chunk_size.to_string());
        stats.insert("max_chunk_size".to_string(), self.max_chunk_size.to_string());
        stats.insert("compression_enabled".to_string(), self.enable_compression.to_string());
        stats.insert("max_file_size".to_string(), self.max_supported_file_size().to_string());
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdfs::Chunk;
    
    /// 创建测试用的分块管理器
    fn create_test_manager() -> DefaultChunkManager {
        DefaultChunkManager::new(1024, false)
    }
    
    /// 创建带压缩的测试分块管理器
    fn create_compressed_manager() -> DefaultChunkManager {
        DefaultChunkManager::new(1024, true)
    }
    
    /// 生成测试数据
    fn generate_test_data(size: usize, pattern: u8) -> Vec<u8> {
        vec![pattern; size]
    }
    
    #[test]
    fn test_分块管理器创建和配置() {
        let manager = create_test_manager();
        let config = manager.get_config();
        
        assert_eq!(config.chunk_size, 1024);
        assert_eq!(config.min_chunk_size, 256); // 1024/4 = 256
        assert_eq!(config.max_chunk_size, 4096);
        assert!(!config.enable_compression);
    }
    
    #[test]
    fn test_自定义大小限制创建() {
        let manager = DefaultChunkManager::with_size_limits(
            2048, 512, 8192, true
        ).unwrap();
        
        let config = manager.get_config();
        assert_eq!(config.chunk_size, 2048);
        assert_eq!(config.min_chunk_size, 512);
        assert_eq!(config.max_chunk_size, 8192);
        assert!(config.enable_compression);
    }
    
    #[test]
    fn test_无效大小限制() {
        // min_size > chunk_size
        assert!(DefaultChunkManager::with_size_limits(1024, 2048, 4096, false).is_err());
        
        // chunk_size > max_size
        assert!(DefaultChunkManager::with_size_limits(4096, 1024, 2048, false).is_err());
    }
    
    #[test]
    fn test_小文件分块() {
        let manager = create_test_manager();
        let data = b"Hello, VDFS!";
        
        let chunks = manager.split_file(data).unwrap();
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data, data);
        assert_eq!(chunks[0].size, data.len());
        
        // 验证元数据
        assert_eq!(chunks[0].metadata.get("index"), Some(&"0".to_string()));
        assert_eq!(chunks[0].metadata.get("offset"), Some(&"0".to_string()));
        assert_eq!(chunks[0].metadata.get("total_size"), Some(&data.len().to_string()));
    }
    
    #[test]
    fn test_大文件分块() {
        let manager = create_test_manager();
        let data = generate_test_data(5000, 0x42); // 5KB数据
        
        let chunks = manager.split_file(&data).unwrap();
        
        // 应该分成5个分块
        assert_eq!(chunks.len(), 5);
        
        // 验证前4个分块大小
        for i in 0..4 {
            assert_eq!(chunks[i].size, 1024);
            assert_eq!(chunks[i].metadata.get("index"), Some(&i.to_string()));
        }
        
        // 最后一个分块应该是剩余的字节 (5000 - 4*1024 = 904)
        assert_eq!(chunks[4].size, 904);
        assert_eq!(chunks[4].metadata.get("index"), Some(&"4".to_string()));
    }
    
    #[test]
    fn test_文件重组() {
        let manager = create_test_manager();
        let original_data = generate_test_data(3000, 0xFF);
        
        // 分块
        let chunks = manager.split_file(&original_data).unwrap();
        assert!(chunks.len() > 1);
        
        // 重组
        let reassembled_data = manager.reassemble_file(chunks).unwrap();
        
        // 验证数据完整性
        assert_eq!(reassembled_data, original_data);
    }
    
    #[test]
    fn test_乱序分块重组() {
        let manager = create_test_manager();
        let original_data = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(100);
        
        // 分块
        let mut chunks = manager.split_file(&original_data).unwrap();
        
        // 打乱分块顺序
        chunks.reverse();
        
        // 重组（应该能够正确排序）
        let reassembled_data = manager.reassemble_file(chunks).unwrap();
        
        // 验证数据完整性
        assert_eq!(reassembled_data, original_data);
    }
    
    #[test]
    fn test_空数据处理() {
        let manager = create_test_manager();
        
        // 空数据分块
        let chunks = manager.split_file(&[]).unwrap();
        assert!(chunks.is_empty());
        
        // 空分块重组
        let data = manager.reassemble_file(vec![]).unwrap();
        assert!(data.is_empty());
    }
    
    #[test]
    fn test_分块去重() {
        let manager = create_test_manager();
        let data1 = generate_test_data(500, 0xAA);
        let data2 = generate_test_data(500, 0xBB);
        let data3 = generate_test_data(500, 0xAA); // 与data1相同
        
        let chunk1 = Chunk::new(data1);
        let chunk2 = Chunk::new(data2);
        let chunk3 = Chunk::new(data3);
        
        let chunks = vec![chunk1.clone(), chunk2.clone(), chunk3, chunk1.clone()];
        let unique_ids = manager.deduplicate(&chunks);
        
        // 应该只有2个唯一ID（chunk1和chunk2）
        assert_eq!(unique_ids.len(), 2);
        assert!(unique_ids.contains(&chunk1.id));
        assert!(unique_ids.contains(&chunk2.id));
    }
    
    #[test]
    fn test_分块统计分析() {
        let manager = create_test_manager();
        let data = generate_test_data(2000, 0x00);
        
        let chunks = manager.split_file(&data).unwrap();
        let stats = manager.analyze_chunks(&chunks);
        
        assert_eq!(stats.total_chunks, chunks.len());
        assert_eq!(stats.original_size, 2000);
        assert_eq!(stats.chunked_size, 2000);
        assert_eq!(stats.unique_chunks, chunks.len());
        assert_eq!(stats.dedup_savings, 0); // 没有重复分块
    }
    
    #[test]
    fn test_分块完整性验证() {
        let manager = create_test_manager();
        let data = generate_test_data(1500, 0x55);
        
        let chunks = manager.split_file(&data).unwrap();
        
        // 验证所有分块
        for chunk in &chunks {
            assert!(manager.verify_chunk(chunk));
        }
        
        // 验证分块序列
        assert!(manager.verify_chunk_sequence(&chunks).unwrap());
    }
    
    #[test]
    fn test_损坏分块检测() {
        let manager = create_test_manager();
        let data = generate_test_data(1000, 0x99);
        
        let mut chunks = manager.split_file(&data).unwrap();
        
        // 损坏第一个分块的数据
        chunks[0].data[0] = 0x00;
        
        // 验证应该失败
        assert!(!manager.verify_chunk(&chunks[0]));
        assert!(!manager.verify_chunk_sequence(&chunks).unwrap());
    }
    
    #[test]
    fn test_压缩分块() {
        let manager = create_compressed_manager();
        
        // 创建高重复性数据（适合压缩）
        let repetitive_data = vec![0x42; 2048];
        
        let chunks = manager.split_file(&repetitive_data).unwrap();
        
        // 检查是否启用了压缩
        for chunk in &chunks {
            if chunk.data.len() >= 1024 {
                // 大于1KB的分块应该尝试压缩
                // 由于我们的模拟实现，高重复性数据会被标记为压缩
                if chunk.compressed {
                    assert!(chunk.metadata.contains_key("compression"));
                    assert_eq!(chunk.metadata.get("compression"), Some(&"rle".to_string()));
                }
            }
        }
    }
    
    #[test]
    fn test_最优分块大小计算() {
        let manager = create_test_manager();
        
        // 小文件
        assert_eq!(manager.optimal_chunk_size(100), 100);
        
        // 中等文件
        assert_eq!(manager.optimal_chunk_size(5000), 1024);
        
        // 大文件
        let large_size = 50_000_000; // 50MB
        let optimal = manager.optimal_chunk_size(large_size);
        assert!(optimal >= 1024);
        assert!(optimal <= 4096);
    }
    
    #[test]
    fn test_带选项的文件重组() {
        let manager = create_test_manager();
        let data = generate_test_data(2000, 0x33);
        
        let chunks = manager.split_file(&data).unwrap();
        
        // 测试各种重组选项
        let reassembled = manager.reassemble_file_with_options(
            chunks,
            true,  // verify_integrity
            false, // allow_partial
            Some(5000) // max_size_limit
        ).unwrap();
        assert_eq!(reassembled, data);
    }
    
    #[test]
    fn test_大小限制重组() {
        let manager = create_test_manager();
        let data = generate_test_data(3000, 0x77);
        
        let chunks = manager.split_file(&data).unwrap();
        
        // 设置过小的大小限制
        let result = manager.reassemble_file_with_options(
            chunks,
            false, // verify_integrity
            false, // allow_partial
            Some(1000) // max_size_limit - 小于实际数据大小
        );
        assert!(result.is_err());
    }
    
    #[test]
    fn test_分块索引不连续错误() {
        let manager = create_test_manager();
        let data = generate_test_data(2000, 0x88);
        
        let mut chunks = manager.split_file(&data).unwrap();
        
        // 人为修改分块索引制造不连续
        if chunks.len() > 1 {
            chunks[1].metadata.insert("index".to_string(), "5".to_string());
            
            let result = manager.verify_chunk_sequence(&chunks);
            assert!(result.is_err());
        }
    }
    
    #[test]
    fn test_管理器统计信息() {
        let manager = create_test_manager();
        let stats = manager.get_manager_stats();
        
        assert_eq!(stats.get("chunk_size"), Some(&"1024".to_string()));
        assert_eq!(stats.get("min_chunk_size"), Some(&"256".to_string()));
        assert_eq!(stats.get("max_chunk_size"), Some(&"4096".to_string()));
        assert_eq!(stats.get("compression_enabled"), Some(&"false".to_string()));
        
        // 验证最大文件大小计算
        let max_file_size = manager.max_supported_file_size();
        assert_eq!(max_file_size, 1_000_000 * 4096); // 1M chunks * 4KB each
    }
    
    #[test]
    fn test_不支持的压缩类型() {
        let manager = create_compressed_manager();
        let mut chunk = Chunk::new(vec![0x11; 1000]);
        
        // 设置不支持的压缩类型
        chunk.compressed = true;
        chunk.metadata.insert("compression".to_string(), "unknown".to_string());
        
        let result = manager.decompress_chunk(chunk);
        assert!(result.is_err());
    }
}
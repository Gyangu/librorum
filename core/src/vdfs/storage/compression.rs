//! Compression Support

use crate::vdfs::{VDFSResult, VDFSError};
use std::io::{Read, Write};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use zstd::{stream::read::Decoder as ZstdDecoder, stream::write::Encoder as ZstdEncoder};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};

/// Compression algorithms
#[derive(Debug, Clone)]
pub enum CompressionAlgorithm {
    None,
    Zstd,
    Lz4,
    Gzip,
}

/// Compression manager
pub struct CompressionManager {
    algorithm: CompressionAlgorithm,
}

impl CompressionManager {
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self { algorithm }
    }
    
    pub fn compress(&self, data: &[u8]) -> VDFSResult<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)
                    .map_err(|e| VDFSError::InternalError(format!("Gzip compression failed: {}", e)))?;
                encoder.finish()
                    .map_err(|e| VDFSError::InternalError(format!("Gzip compression failed: {}", e)))
            },
            CompressionAlgorithm::Zstd => {
                let mut encoder = ZstdEncoder::new(Vec::new(), 3)
                    .map_err(|e| VDFSError::InternalError(format!("Zstd encoder creation failed: {}", e)))?;
                encoder.write_all(data)
                    .map_err(|e| VDFSError::InternalError(format!("Zstd compression failed: {}", e)))?;
                encoder.finish()
                    .map_err(|e| VDFSError::InternalError(format!("Zstd compression failed: {}", e)))
            },
            CompressionAlgorithm::Lz4 => {
                Ok(compress_prepend_size(data))
            }
        }
    }
    
    pub fn decompress(&self, data: &[u8]) -> VDFSResult<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Gzip => {
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| VDFSError::InternalError(format!("Gzip decompression failed: {}", e)))?;
                Ok(decompressed)
            },
            CompressionAlgorithm::Zstd => {
                let mut decoder = ZstdDecoder::new(data)
                    .map_err(|e| VDFSError::InternalError(format!("Zstd decoder creation failed: {}", e)))?;
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| VDFSError::InternalError(format!("Zstd decompression failed: {}", e)))?;
                Ok(decompressed)
            },
            CompressionAlgorithm::Lz4 => {
                decompress_size_prepended(data)
                    .map_err(|e| VDFSError::InternalError(format!("Lz4 decompression failed: {}", e)))
            }
        }
    }
    
    /// Get compression ratio for the given data
    pub fn compression_ratio(&self, original_data: &[u8]) -> VDFSResult<f64> {
        let compressed = self.compress(original_data)?;
        if original_data.is_empty() {
            return Ok(1.0);
        }
        Ok(compressed.len() as f64 / original_data.len() as f64)
    }
    
    /// Check if data should be compressed based on size and type
    pub fn should_compress(&self, data: &[u8], min_size: usize) -> bool {
        match self.algorithm {
            CompressionAlgorithm::None => false,
            _ => data.len() >= min_size
        }
    }
    
    /// Get algorithm name
    pub fn algorithm_name(&self) -> &'static str {
        match self.algorithm {
            CompressionAlgorithm::None => "none",
            CompressionAlgorithm::Gzip => "gzip",
            CompressionAlgorithm::Zstd => "zstd",
            CompressionAlgorithm::Lz4 => "lz4",
        }
    }
}

impl Default for CompressionManager {
    fn default() -> Self {
        Self::new(CompressionAlgorithm::Zstd) // Zstd as default for good performance/ratio balance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_compression() {
        let manager = CompressionManager::new(CompressionAlgorithm::None);
        let data = b"Hello, World!";
        
        let compressed = manager.compress(data).unwrap();
        let decompressed = manager.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed.as_slice());
        assert_eq!(compressed.len(), data.len());
    }

    #[test]
    fn test_gzip_compression() {
        let manager = CompressionManager::new(CompressionAlgorithm::Gzip);
        let data = b"Hello, World! This is a test string for compression.";
        
        let compressed = manager.compress(data).unwrap();
        let decompressed = manager.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed.as_slice());
        // For small data, compression might not reduce size much
        println!("Gzip: {} -> {} bytes", data.len(), compressed.len());
    }

    #[test]
    fn test_zstd_compression() {
        let manager = CompressionManager::new(CompressionAlgorithm::Zstd);
        let data = b"Hello, World! This is a test string for compression. ".repeat(10);
        
        let compressed = manager.compress(&data).unwrap();
        let decompressed = manager.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed);
        assert!(compressed.len() < data.len()); // Should compress repeated data
        println!("Zstd: {} -> {} bytes", data.len(), compressed.len());
    }

    #[test]
    fn test_lz4_compression() {
        let manager = CompressionManager::new(CompressionAlgorithm::Lz4);
        let data = b"Hello, World! This is a test string for compression. ".repeat(5);
        
        let compressed = manager.compress(&data).unwrap();
        let decompressed = manager.decompress(&compressed).unwrap();
        
        assert_eq!(data, decompressed);
        println!("Lz4: {} -> {} bytes", data.len(), compressed.len());
    }

    #[test]
    fn test_compression_ratio() {
        let manager = CompressionManager::new(CompressionAlgorithm::Zstd);
        let data = b"A".repeat(1000); // Highly compressible data
        
        let ratio = manager.compression_ratio(&data).unwrap();
        println!("Compression ratio: {:.3}", ratio);
        assert!(ratio < 1.0); // Should compress well
    }

    #[test]
    fn test_should_compress() {
        let manager = CompressionManager::new(CompressionAlgorithm::Zstd);
        
        assert!(!manager.should_compress(b"small", 100));
        assert!(manager.should_compress(&vec![0u8; 200], 100));
        
        let none_manager = CompressionManager::new(CompressionAlgorithm::None);
        assert!(!none_manager.should_compress(&vec![0u8; 200], 100));
    }
}
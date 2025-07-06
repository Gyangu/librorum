//! UTP传输工具函数

use std::path::Path;
use std::net::{IpAddr, Ipv4Addr};
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::Read;

use super::{UtpConfig, TransportMode, UtpResult, UtpError};

/// UTP工具函数
pub struct UtpUtils;

impl UtpUtils {
    /// 计算文件SHA-256哈希
    pub fn calculate_file_hash(file_path: &str) -> UtpResult<String> {
        let mut file = File::open(file_path)
            .map_err(|e| UtpError::IoError(format!("Failed to open file {}: {}", file_path, e)))?;
        
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .map_err(|e| UtpError::IoError(format!("Failed to read file: {}", e)))?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    /// 检测MIME类型
    pub fn detect_mime_type(file_path: &str) -> String {
        let path = Path::new(file_path);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        match extension.as_str() {
            // 图片
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "bmp" => "image/bmp",
            "ico" => "image/x-icon",
            
            // 视频
            "mp4" => "video/mp4",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",
            "wmv" => "video/x-ms-wmv",
            "flv" => "video/x-flv",
            "webm" => "video/webm",
            "mkv" => "video/x-matroska",
            
            // 音频
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "flac" => "audio/flac",
            "aac" => "audio/aac",
            "ogg" => "audio/ogg",
            "m4a" => "audio/mp4",
            
            // 文档
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            
            // 文本
            "txt" => "text/plain",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "csv" => "text/csv",
            
            // 压缩文件
            "zip" => "application/zip",
            "rar" => "application/vnd.rar",
            "7z" => "application/x-7z-compressed",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            
            // 代码文件
            "rs" => "text/x-rust",
            "py" => "text/x-python",
            "java" => "text/x-java-source",
            "cpp" | "cc" | "cxx" => "text/x-c++src",
            "c" => "text/x-csrc",
            "h" => "text/x-chdr",
            "swift" => "text/x-swift",
            "go" => "text/x-go",
            "php" => "text/x-php",
            "rb" => "text/x-ruby",
            
            // 默认
            _ => "application/octet-stream",
        }.to_string()
    }
    
    /// 判断是否为本地地址
    pub fn is_local_address(addr: &IpAddr) -> bool {
        match addr {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback() || 
                ipv4.is_private() ||
                *ipv4 == Ipv4Addr::new(0, 0, 0, 0)
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback()
            }
        }
    }
    
    /// 自动选择传输模式
    pub fn auto_select_transport_mode(config: &UtpConfig) -> TransportMode {
        if let Some(target_addr) = config.target_addr {
            if Self::is_local_address(&target_addr.ip()) {
                // 本地地址，优先使用共享内存
                TransportMode::SharedMemory
            } else {
                // 远程地址，使用网络传输
                TransportMode::Network
            }
        } else {
            // 没有目标地址，默认使用共享内存
            TransportMode::SharedMemory
        }
    }
    
    /// 格式化文件大小
    pub fn format_file_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
        let mut size = size as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
    
    /// 格式化传输速率
    pub fn format_transfer_rate(bytes_per_sec: f64) -> String {
        Self::format_file_size(bytes_per_sec as u64) + "/s"
    }
    
    /// 格式化时间
    pub fn format_duration(seconds: f64) -> String {
        if seconds < 60.0 {
            format!("{:.1}s", seconds)
        } else if seconds < 3600.0 {
            let minutes = (seconds / 60.0).floor() as u64;
            let secs = seconds % 60.0;
            format!("{}m{:.0}s", minutes, secs)
        } else {
            let hours = (seconds / 3600.0).floor() as u64;
            let minutes = ((seconds % 3600.0) / 60.0).floor() as u64;
            format!("{}h{}m", hours, minutes)
        }
    }
    
    /// 验证文件路径
    pub fn validate_file_path(file_path: &str) -> UtpResult<()> {
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Err(UtpError::IoError(format!("File does not exist: {}", file_path)));
        }
        
        if path.is_dir() {
            return Err(UtpError::IoError(format!("Path is a directory: {}", file_path)));
        }
        
        Ok(())
    }
    
    /// 创建会话ID
    pub fn generate_session_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        use std::process;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        
        let pid = process::id();
        
        format!("utp_{}_{}", timestamp, pid)
    }
    
    /// 计算最优块大小
    pub fn calculate_optimal_chunk_size(file_size: u64) -> usize {
        // 根据文件大小自动调整块大小
        if file_size < 1024 * 1024 {
            // 小文件 (<1MB): 64KB块
            64 * 1024
        } else if file_size < 100 * 1024 * 1024 {
            // 中等文件 (<100MB): 1MB块
            1024 * 1024
        } else if file_size < 1024 * 1024 * 1024 {
            // 大文件 (<1GB): 4MB块
            4 * 1024 * 1024
        } else {
            // 超大文件 (>=1GB): 8MB块
            8 * 1024 * 1024
        }
    }
    
    /// 压缩数据 (简单实现)
    pub fn compress_data(data: &[u8]) -> UtpResult<Vec<u8>> {
        // TODO: 实现实际的压缩算法 (如zstd, lz4等)
        // 现在只是返回原始数据
        Ok(data.to_vec())
    }
    
    /// 解压缩数据 (简单实现)
    pub fn decompress_data(data: &[u8]) -> UtpResult<Vec<u8>> {
        // TODO: 实现实际的解压缩算法
        // 现在只是返回原始数据
        Ok(data.to_vec())
    }
    
    /// 加密数据 (简单实现)
    pub fn encrypt_data(data: &[u8], _key: &[u8]) -> UtpResult<Vec<u8>> {
        // TODO: 实现实际的加密算法 (如AES-GCM)
        // 现在只是返回原始数据
        Ok(data.to_vec())
    }
    
    /// 解密数据 (简单实现)
    pub fn decrypt_data(data: &[u8], _key: &[u8]) -> UtpResult<Vec<u8>> {
        // TODO: 实现实际的解密算法
        // 现在只是返回原始数据
        Ok(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(UtpUtils::format_file_size(512), "512 B");
        assert_eq!(UtpUtils::format_file_size(1536), "1.5 KB");
        assert_eq!(UtpUtils::format_file_size(1048576), "1.0 MB");
        assert_eq!(UtpUtils::format_file_size(1073741824), "1.0 GB");
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(UtpUtils::format_duration(30.5), "30.5s");
        assert_eq!(UtpUtils::format_duration(90.0), "1m30s");
        assert_eq!(UtpUtils::format_duration(3660.0), "1h1m");
    }
    
    #[test]
    fn test_detect_mime_type() {
        assert_eq!(UtpUtils::detect_mime_type("test.jpg"), "image/jpeg");
        assert_eq!(UtpUtils::detect_mime_type("test.mp4"), "video/mp4");
        assert_eq!(UtpUtils::detect_mime_type("test.unknown"), "application/octet-stream");
    }
    
    #[test]
    fn test_calculate_optimal_chunk_size() {
        assert_eq!(UtpUtils::calculate_optimal_chunk_size(512 * 1024), 64 * 1024);
        assert_eq!(UtpUtils::calculate_optimal_chunk_size(50 * 1024 * 1024), 1024 * 1024);
        assert_eq!(UtpUtils::calculate_optimal_chunk_size(500 * 1024 * 1024), 4 * 1024 * 1024);
        assert_eq!(UtpUtils::calculate_optimal_chunk_size(2 * 1024 * 1024 * 1024), 8 * 1024 * 1024);
    }
}
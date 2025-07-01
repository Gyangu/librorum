use uuid::Uuid;
use std::net::{IpAddr, Ipv4Addr};

/// 通用工具函数
pub mod common {
    use super::*;

    /// 生成唯一ID
    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// 获取本地IP地址
    pub fn get_local_ip() -> IpAddr {
        // 默认返回localhost，实际实现可以更复杂
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    }

    /// 格式化字节数
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

/// 时间相关工具
pub mod time {
    use chrono::{DateTime, Utc};

    /// 获取当前时间戳
    pub fn now_timestamp() -> i64 {
        Utc::now().timestamp()
    }

    /// 时间戳转换为可读字符串
    pub fn timestamp_to_string(timestamp: i64) -> String {
        DateTime::from_timestamp(timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Invalid timestamp".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod common_tests {
        use super::*;

        #[test]
        fn test_generate_id() {
            let id1 = common::generate_id();
            let id2 = common::generate_id();
            
            // IDs should be different
            assert_ne!(id1, id2);
            
            // IDs should be valid UUIDs (36 characters with hyphens)
            assert_eq!(id1.len(), 36);
            assert_eq!(id2.len(), 36);
            
            // Should contain hyphens in correct positions
            assert_eq!(id1.chars().nth(8).unwrap(), '-');
            assert_eq!(id1.chars().nth(13).unwrap(), '-');
            assert_eq!(id1.chars().nth(18).unwrap(), '-');
            assert_eq!(id1.chars().nth(23).unwrap(), '-');
        }

        #[test]
        fn test_get_local_ip() {
            let ip = common::get_local_ip();
            
            // Should return a valid IP address
            match ip {
                IpAddr::V4(ipv4) => {
                    assert_eq!(ipv4, Ipv4Addr::new(127, 0, 0, 1)); // localhost
                }
                IpAddr::V6(_) => {
                    panic!("Expected IPv4 address");
                }
            }
        }

        #[test]
        fn test_format_bytes() {
            // Test bytes
            assert_eq!(common::format_bytes(0), "0 B");
            assert_eq!(common::format_bytes(512), "512 B");
            assert_eq!(common::format_bytes(1023), "1023 B");
            
            // Test KB
            assert_eq!(common::format_bytes(1024), "1.0 KB");
            assert_eq!(common::format_bytes(1536), "1.5 KB"); // 1.5 * 1024
            assert_eq!(common::format_bytes(2048), "2.0 KB");
            
            // Test MB
            assert_eq!(common::format_bytes(1024 * 1024), "1.0 MB");
            assert_eq!(common::format_bytes(1024 * 1024 + 1024 * 512), "1.5 MB");
            
            // Test GB
            assert_eq!(common::format_bytes(1024 * 1024 * 1024), "1.0 GB");
            assert_eq!(common::format_bytes(1024 * 1024 * 1024 * 2), "2.0 GB");
            
            // Test TB
            assert_eq!(common::format_bytes(1024_u64.pow(4)), "1.0 TB");
            assert_eq!(common::format_bytes(1024_u64.pow(4) * 5), "5.0 TB");
        }

        #[test]
        fn test_format_bytes_edge_cases() {
            // Very large number
            let large_number = u64::MAX;
            let result = common::format_bytes(large_number);
            assert!(result.contains("TB"));
            
            // Exact boundaries
            assert_eq!(common::format_bytes(1024 - 1), "1023 B");
            assert_eq!(common::format_bytes(1024), "1.0 KB");
            assert_eq!(common::format_bytes(1024 * 1024 - 1), "1024.0 KB");
            assert_eq!(common::format_bytes(1024 * 1024), "1.0 MB");
        }
    }

    mod time_tests {
        use super::*;
        use chrono::Utc;

        #[test]
        fn test_now_timestamp() {
            let timestamp1 = time::now_timestamp();
            std::thread::sleep(std::time::Duration::from_millis(1100)); // 确保超过1秒
            let timestamp2 = time::now_timestamp();
            
            // Timestamps should be different and increasing
            assert!(timestamp2 >= timestamp1); // 允许相等，以防时间精度问题
            
            // Should be reasonably close to current time
            let now = Utc::now().timestamp();
            assert!((now - timestamp1).abs() < 5); // Within 5 seconds to be safe
        }

        #[test]
        fn test_timestamp_to_string() {
            // Test with a known timestamp
            let timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
            let result = time::timestamp_to_string(timestamp);
            assert_eq!(result, "2022-01-01 00:00:00 UTC");
            
            // Test with current timestamp
            let now_timestamp = Utc::now().timestamp();
            let result = time::timestamp_to_string(now_timestamp);
            assert!(result.contains("UTC"));
            assert!(result.len() > 15); // Should be a reasonable length
        }

        #[test]
        fn test_timestamp_to_string_invalid() {
            // Test with invalid timestamp (way in the future)
            let invalid_timestamp = i64::MAX;
            let result = time::timestamp_to_string(invalid_timestamp);
            assert_eq!(result, "Invalid timestamp");
            
            // Test with negative timestamp (before Unix epoch)
            let negative_timestamp = -1;
            let result = time::timestamp_to_string(negative_timestamp);
            // This might be valid depending on chrono version, but should not panic
            assert!(!result.is_empty());
        }

        #[test]
        fn test_timestamp_roundtrip() {
            let original_timestamp = time::now_timestamp();
            let string_repr = time::timestamp_to_string(original_timestamp);
            
            // Should be able to parse back (not testing parsing here, just format)
            assert!(string_repr.contains("UTC"));
            assert!(string_repr.len() >= 19); // "YYYY-MM-DD HH:MM:SS UTC" minimum
        }

        #[test]
        fn test_timestamp_various_times() {
            // Test various known timestamps
            let test_cases = vec![
                (0, "1970-01-01 00:00:00 UTC"), // Unix epoch
                (946684800, "2000-01-01 00:00:00 UTC"), // Y2K
                (1234567890, "2009-02-13 23:31:30 UTC"), // Famous timestamp
            ];
            
            for (timestamp, expected) in test_cases {
                let result = time::timestamp_to_string(timestamp);
                assert_eq!(result, expected);
            }
        }
    }
}
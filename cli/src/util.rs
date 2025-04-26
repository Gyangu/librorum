use anyhow::Result;
use serde_json::Value;
use std::path::Path;
use std::fs;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Local};

/// 打印 JSON 格式的数据
pub fn print_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    println!("{}", json);
    Ok(())
}

/// 检查文件是否存在
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// 检查目录是否存在
pub fn dir_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

/// 创建目录（如果不存在）
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// 保存数据到文件
pub fn save_to_file(path: &Path, data: &[u8]) -> Result<()> {
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    Ok(())
}

/// 从文件加载数据
pub fn load_from_file(path: &Path) -> Result<Vec<u8>> {
    Ok(fs::read(path)?)
}

/// 格式化文件大小显示
pub fn format_size(size: usize) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// 格式化时间戳
pub fn format_timestamp(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
    datetime.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 解析配置值
pub fn parse_config_value(value: &str) -> Value {
    // 尝试解析数字
    if let Ok(int_val) = value.parse::<i64>() {
        return Value::Number(int_val.into());
    }
    
    // 尝试解析浮点数
    if let Ok(float_val) = value.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(float_val) {
            return Value::Number(num);
        }
    }
    
    // 尝试解析布尔值
    if value.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if value.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    
    // 默认为字符串
    Value::String(value.to_string())
}

pub fn get_current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

pub fn format_progress(current: u64, total: u64) -> String {
    let percentage = (current as f64 / total as f64 * 100.0) as u32;
    let width = 20;
    let filled = (width as f64 * current as f64 / total as f64) as usize;
    let empty = width - filled;
    
    format!(
        "[{}{}] {}%",
        "=".repeat(filled),
        " ".repeat(empty),
        percentage
    )
}

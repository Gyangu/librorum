use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use std::io::{self, BufReader, BufRead};
use std::fs::File;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;
use glob;
use tklog::{Format, LEVEL, LOG, MODE};

// 确保日志只初始化一次
static INIT: Once = Once::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// 日志目录路径
pub fn log_dir_path() -> PathBuf {
    #[cfg(not(windows))]
    {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("librorum").join("logs")
        } else {
            PathBuf::from("/tmp/librorum/logs")
        }
    }
    
    #[cfg(windows)]
    {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("librorum").join("logs")
        } else {
            let mut path = PathBuf::new();
            path.push("C:");
            path.push("ProgramData");
            path.push("librorum");
            path.push("logs");
            path
        }
    }
}

/// 日志文件路径
pub fn log_file_path() -> PathBuf {
    // 先检查是否有按日期命名的文件
    let log_dir = log_dir_path();
    if log_dir.exists() {
        // 寻找最新的日志文件
        if let Ok(entries) = fs::read_dir(&log_dir) {
            let latest = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    if let Ok(metadata) = e.metadata() {
                        metadata.is_file() && e.file_name().to_string_lossy().starts_with("librorum.")
                    } else {
                        false
                    }
                })
                .max_by_key(|e| {
                    e.metadata().ok()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or_else(|| std::time::SystemTime::UNIX_EPOCH)
                });
                
            if let Some(file) = latest {
                return file.path();
            }
        }
    }
    
    // 没有找到按日期命名的文件，返回默认的路径
    log_dir.join("librorum.log")
}

/// 初始化日志系统
pub fn init_logger(log_level: &str, to_file: bool) -> Result<()> {
    // 检查是否已初始化
    if INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    INIT.call_once(|| {
        // 创建日志目录
        if to_file {
            if let Err(e) = fs::create_dir_all(log_dir_path()) {
                eprintln!("无法创建日志目录: {:?}", e);
                return;
            }
        }
        
        // 将日志级别字符串转换为 tklog 的 LEVEL
        let level = match log_level.to_lowercase().as_str() {
            "trace" => LEVEL::Trace,
            "debug" => LEVEL::Debug,
            "info" => LEVEL::Info,
            "warn" => LEVEL::Warn,
            "error" => LEVEL::Error,
            _ => LEVEL::Info, // 默认使用 Info 级别
        };
        
        // 设置日志格式和输出
        LOG.set_level(level)
           .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName)
           .set_formatter("{level}{time} {file}:{message}\n") // 自定义格式，添加换行符
           .set_console(true); // 总是输出到控制台
           
        // 设置彩色输出（仅在控制台显示，文件中无颜色代码）
        LOG.set_attr_format(|fmt| {
            // 自定义日志级别格式
            fmt.set_level_fmt(|level| {
                match level {
                    LEVEL::Trace => "[TRACE]",
                    LEVEL::Debug => "[DEBUG]",
                    LEVEL::Info => "[INFO]",
                    LEVEL::Warn => "[WARN]",
                    LEVEL::Error => "[ERROR]",
                    LEVEL::Fatal => "[FATAL]",
                    LEVEL::Off => "",
                }.to_string()
            });

            // 设置控制台日志的正文格式（带颜色）
            fmt.set_console_body_fmt(|level, body| {
                // 如果body末尾有换行符，保留它并在内容后添加颜色重置
                let trimmed_body = if body.ends_with('\n') { 
                    format!("{}{}", &body[..body.len()-1], "\x1b[0m\n") 
                } else { 
                    format!("{}\x1b[0m", body) 
                };
                
                match level {
                    LEVEL::Trace => format!("\x1b[94m{}", trimmed_body), // 蓝色
                    LEVEL::Debug => format!("\x1b[36m{}", trimmed_body), // 青色
                    LEVEL::Info => format!("\x1b[32m{}", trimmed_body),  // 绿色
                    LEVEL::Warn => format!("\x1b[33m{}", trimmed_body),  // 黄色
                    LEVEL::Error => format!("\x1b[31m{}", trimmed_body), // 红色
                    LEVEL::Fatal => format!("\x1b[41m{}", trimmed_body), // 背景红色
                    LEVEL::Off => body.to_string(),
                }
            });
        });
        
        // 如果需要输出到文件
        if to_file {
            // 设置按日期和大小混合切割日志文件
            let log_file = log_dir_path().join("librorum.log").to_string_lossy().to_string();
            LOG.set_cutmode_by_mixed(
                &log_file,       // 日志文件名
                50 * 1024 * 1024, // 50MB 大小上限
                MODE::DAY,        // 按天切割
                30,               // 保留30天的日志
                true              // 压缩备份
            );
        }
        
        // 兼容标准 log API
        LOG.uselog();
        
        // 记录日志系统初始化完成
        if to_file {
            let log_path = log_file_path();
            tklog::info!("日志系统已初始化，输出到文件: {}", log_path.display());
        } else {
            tklog::info!("日志系统已初始化，输出到控制台");
        }
        
        INITIALIZED.store(true, Ordering::SeqCst);
    });
    
    Ok(())
}

/// 读取日志文件最后的行数
pub fn read_log_tail(lines: usize) -> Result<String> {
    let log_file = log_file_path();
    
    if !log_file.exists() {
        return Ok("日志文件不存在".to_string());
    }
    
    let file = File::open(log_file)
        .with_context(|| "无法打开日志文件")?;
        
    let reader = BufReader::new(file);
    let log_lines: Vec<String> = reader.lines()
        .collect::<io::Result<Vec<String>>>()
        .with_context(|| "读取日志文件失败")?;
        
    // 获取最后 N 行
    if log_lines.is_empty() {
        Ok("日志文件为空".to_string())
    } else if log_lines.len() <= lines {
        Ok(log_lines.join("\n"))
    } else {
        Ok(log_lines[log_lines.len() - lines..].join("\n"))
    }
}

/// 清除旧日志文件
pub fn clean_old_logs(days: u64) -> Result<usize> {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    
    let log_dir = log_dir_path();
    
    if !log_dir.exists() {
        return Ok(0);
    }
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .with_context(|| "无法获取当前时间")?;
    
    let cutoff = Duration::from_secs(days * 24 * 60 * 60);
    let mut removed = 0;
    
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        if !metadata.is_file() {
            continue;
        }
        
        if let Ok(modified) = metadata.modified() {
            if let Ok(age) = modified.duration_since(UNIX_EPOCH) {
                if now - age > cutoff {
                    fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }
    }
    
    Ok(removed)
}

/// 查看最近的日志
pub fn view_recent_logs(tail: usize) -> Result<String> {
    let log_dir = log_dir_path();
    let log_pattern = format!("{}/*.log", log_dir.display());
    
    // 查找所有日志文件
    let mut log_files = Vec::new();
    for entry in glob::glob(&log_pattern)? {
        if let Ok(path) = entry {
            log_files.push(path);
        }
    }
    
    // 按修改时间排序，最新的放前面
    log_files.sort_by(|a, b| {
        let a_time = fs::metadata(a).ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(|| SystemTime::UNIX_EPOCH);
        let b_time = fs::metadata(b).ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(|| SystemTime::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });
    
    // 如果没有日志文件，返回空结果
    if log_files.is_empty() {
        return Ok("没有找到日志文件".to_string());
    }
    
    // 读取最新的日志文件
    let latest_log = &log_files[0];
    let file = File::open(latest_log)
        .with_context(|| format!("无法打开日志文件: {:?}", latest_log))?;
        
    let reader = BufReader::new(file);
    let log_lines: Vec<String> = reader.lines()
        .collect::<io::Result<Vec<String>>>()
        .with_context(|| format!("读取日志文件失败: {:?}", latest_log))?;
        
    // 获取最后 N 行
    if log_lines.is_empty() {
        Ok(format!("日志文件为空: {:?}", latest_log))
    } else if log_lines.len() <= tail {
        Ok(format!("日志文件: {:?}\n{}", latest_log, log_lines.join("\n")))
    } else {
        Ok(format!("日志文件: {:?}\n{}", latest_log, log_lines[log_lines.len() - tail..].join("\n")))
    }
} 
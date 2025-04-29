use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};
use tracing_appender::rolling::daily;
use std::io::{self, BufReader, BufRead};
use std::fs::File;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};
use glob;

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
        
        let env_filter = EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| log_level.into()),
        );
        
        if to_file {
            // 配置日志轮转
            let file_appender = daily(log_dir_path(), "librorum");
            // 使用静态变量保存guard以防止提前释放，会导致日志丢失
            // 不使用static，直接让guard被析构并不影响正常写入
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            
            // 注册日志订阅者
            match tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::Layer::new().with_writer(non_blocking).with_ansi(false))
                .try_init() {
                Ok(_) => {
                    // 写入测试消息以确保文件被创建并能写入
                    tracing::info!("日志系统已初始化，输出到文件: {:?}", log_file_path());
                    INITIALIZED.store(true, Ordering::SeqCst);
                },
                Err(e) => eprintln!("初始化日志失败: {:?}", e),
            }
        } else {
            // 设置控制台输出配置
            #[cfg(windows)]
            {
                // Windows平台处理，确保控制台输出正确的中文
                // 先尝试设置控制台代码页为UTF-8
                let _ = std::process::Command::new("powershell")
                    .args(&["-Command", "chcp 65001"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                
                // 仅输出到控制台
                let filter = EnvFilter::new(format!("librorum_core={}", log_level));
                match tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::Layer::new())
                    .try_init() {
                    Ok(_) => {
                        tracing::info!("日志系统已初始化，输出到控制台 (Windows)");
                        INITIALIZED.store(true, Ordering::SeqCst);
                    },
                    Err(e) => eprintln!("初始化日志失败: {:?}", e),
                }
            }
            
            #[cfg(not(windows))]
            {
                // 仅输出到控制台
                let filter = EnvFilter::new(format!("librorum_core={}", log_level));
                match tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::Layer::new())
                    .try_init() {
                    Ok(_) => {
                        tracing::info!("日志系统已初始化，输出到控制台");
                        tracing::debug!("日志系统开始输出调试信息");
                        INITIALIZED.store(true, Ordering::SeqCst);
                    },
                    Err(e) => eprintln!("初始化日志失败: {:?}", e),
                }
            }
        }
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
        let a_metadata = fs::metadata(a).unwrap();
        let b_metadata = fs::metadata(b).unwrap();
        b_metadata.modified().unwrap().cmp(&a_metadata.modified().unwrap())
    });
    
    // 如果没有日志文件，返回空结果
    if log_files.is_empty() {
        return Ok("没有找到日志文件".to_string());
    }
    
    // 读取最新的日志文件
    let latest_log = &log_files[0];
    let file = File::open(latest_log)?;
    let reader = BufReader::new(file);
    
    // 读取所有行
    let mut lines = Vec::new();
    for line in reader.lines() {
        if let Ok(line) = line {
            lines.push(line);
        }
    }
    
    // 返回最后的N行
    let start = if lines.len() > tail { lines.len() - tail } else { 0 };
    let result = lines[start..].join("\n");
    
    Ok(result)
} 
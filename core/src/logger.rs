use anyhow::{Context, Result};
use glob;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::SystemTime;
use tracing::Level;
use tracing_appender::{
    non_blocking,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_log::LogTracer;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time},
    layer::SubscriberExt,
    EnvFilter,
};

// 确保日志只初始化一次
static INIT: Once = Once::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static mut GUARD: Option<Box<dyn std::any::Any + Send + Sync>> = None;

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
                        metadata.is_file()
                            && e.file_name().to_string_lossy().starts_with("librorum.")
                    } else {
                        false
                    }
                })
                .max_by_key(|e| {
                    e.metadata()
                        .ok()
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

/// Initialize the logging system
pub fn init_logger(log_level: &str, to_file: bool) -> Result<()> {
    // Check if already initialized
    if INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }

    INIT.call_once(|| {
        // Create log directory
        if to_file {
            if let Err(e) = fs::create_dir_all(log_dir_path()) {
                eprintln!("Failed to create log directory: {:?}", e);
                return;
            }
        }

        // Convert log level string to tracing Level
        let level = match log_level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO, // Default to Info level
        };

        // Setup filters
        let env_filter = EnvFilter::from_default_env().add_directive(level.into());

        // Terminal layer with colors enabled
        let terminal_layer = fmt::layer()
            .with_writer(io::stdout)
            .with_ansi(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_timer(time::SystemTime::default())
            .with_file(true)
            .with_line_number(true)
            .with_target(false); // 不显示目标模块路径

        if to_file {
            // Create log file appender with daily rotation
            let file_appender =
                RollingFileAppender::new(Rotation::DAILY, log_dir_path(), "librorum.log");

            // Use non-blocking writer for file output
            let (non_blocking_appender, guard) = non_blocking(file_appender);

            // File layer with colors disabled
            let file_layer = fmt::layer()
                .with_writer(non_blocking_appender)
                .with_ansi(false)
                .with_span_events(FmtSpan::CLOSE)
                .with_timer(time::SystemTime::default())
                .with_file(true)
                .with_line_number(true)
                .with_target(false); // 不显示目标模块路径

            // Set up subscriber with both layers
            let subscriber = tracing_subscriber::registry()
                .with(env_filter)
                .with(terminal_layer)
                .with(file_layer);

            // Initialize tracing
            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set global tracing subscriber");

            // Store guard to keep file appender alive
            unsafe {
                GUARD = Some(Box::new(guard));
            }

            // Forward events from log crate to tracing
            LogTracer::init().expect("Failed to initialize log tracer");

            let log_path = log_file_path();
            tracing::info!(
                "Logging system initialized, output to file: {}",
                log_path.display()
            );
        } else {
            // Set up subscriber with only terminal layer
            let subscriber = tracing_subscriber::registry()
                .with(env_filter)
                .with(terminal_layer);

            // Initialize tracing
            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set global tracing subscriber");

            // Forward events from log crate to tracing
            LogTracer::init().expect("Failed to initialize log tracer");

            tracing::info!("Logging system initialized, output to console");
        }

        INITIALIZED.store(true, Ordering::SeqCst);
    });

    Ok(())
}

/// Read the last lines of the log file
pub fn read_log_tail(lines: usize) -> Result<String> {
    let log_file = log_file_path();

    if !log_file.exists() {
        return Ok("Log file does not exist".to_string());
    }

    let file = File::open(log_file).with_context(|| "Cannot open log file")?;

    let reader = BufReader::new(file);
    let log_lines: Vec<String> = reader
        .lines()
        .collect::<io::Result<Vec<String>>>()
        .with_context(|| "Failed to read log file")?;

    // Get the last N lines
    if log_lines.is_empty() {
        Ok("Log file is empty".to_string())
    } else if log_lines.len() <= lines {
        Ok(log_lines.join("\n"))
    } else {
        Ok(log_lines[log_lines.len() - lines..].join("\n"))
    }
}

/// Clean old log files
pub fn clean_old_logs(days: u64) -> Result<usize> {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let log_dir = log_dir_path();

    if !log_dir.exists() {
        return Ok(0);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .with_context(|| "Cannot get current time")?;

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

/// View recent logs
pub fn view_recent_logs(tail: usize) -> Result<String> {
    let log_dir = log_dir_path();
    let log_pattern = format!("{}/*.log", log_dir.display());

    // Find all log files
    let mut log_files = Vec::new();
    for entry in glob::glob(&log_pattern)? {
        if let Ok(path) = entry {
            log_files.push(path);
        }
    }

    // Sort by modification time, newer first
    log_files.sort_by(|a, b| {
        let a_time = a
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let b_time = b
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });

    // Read content from the most recent file
    if let Some(latest_file) = log_files.first() {
        let file = File::open(latest_file)
            .with_context(|| format!("Cannot open log file: {}", latest_file.display()))?;

        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .collect::<io::Result<Vec<String>>>()
            .with_context(|| "Failed to read log file")?;

        if lines.is_empty() {
            Ok(format!("Log file is empty: {}", latest_file.display()))
        } else if lines.len() <= tail {
            Ok(lines.join("\n"))
        } else {
            Ok(lines[lines.len() - tail..].join("\n"))
        }
    } else {
        Ok("No log files found".to_string())
    }
}

/// Clean all log files
pub fn clean_all_logs() -> Result<usize> {
    let log_dir = log_dir_path();
    
    if !log_dir.exists() {
        return Ok(0);
    }
    
    let mut removed = 0;
    
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        if !metadata.is_file() {
            continue;
        }
        
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        if file_name_str.starts_with("librorum.") || file_name_str.contains("librorum-") {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    
    Ok(removed)
}

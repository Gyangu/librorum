#!/usr/bin/env rust-script
//! This is a demo of tracing with colored terminal output and file logging
//!
//! ```cargo
//! [dependencies]
//! tracing = "0.1"
//! tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "time"] }
//! tracing-appender = "0.2"
//! time = { version = "0.3", features = ["formatting"] }
//! chrono = "0.4"
//! anyhow = "1.0"
//! ```

use std::io::{self, Write};
use std::path::Path;
use std::fs;
use anyhow::Result;
use tracing::{info, error, warn, debug, trace, Level};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::UtcTime},
    EnvFilter,
    prelude::*,
};
use tracing_appender::{
    non_blocking,
    rolling::{RollingFileAppender, Rotation},
};

fn main() -> Result<()> {
    // 确保当前目录存在
    let log_dir = "./logs";
    fs::create_dir_all(log_dir)?;
    
    // 创建日志文件的appender，每天滚动一次
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY, 
        Path::new(log_dir),
        "app.log",
    );
    
    // 使用非阻塞writer
    let (non_blocking_appender, guard) = non_blocking(file_appender);

    // 终端层 - 启用颜色
    let terminal_layer = fmt::layer()
        .with_writer(io::stdout)
        .with_ansi(true)  // 启用终端颜色
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(UtcTime::rfc_3339());

    // 文件层 - 禁用颜色
    let file_layer = fmt::layer()
        .with_writer(non_blocking_appender)
        .with_ansi(false)  // 禁用文件中的颜色代码
        .with_span_events(FmtSpan::CLOSE)
        .with_timer(UtcTime::rfc_3339());
    
    // 设置订阅者，同时使用两个层
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(Level::TRACE.into()))
        .with(terminal_layer)
        .with(file_layer)
        .init();

    // 打印不同级别的日志
    info!("启动应用程序");
    debug!("这是一条调试信息");
    trace!("这是一条详细的跟踪信息");
    warn!("警告：操作可能会失败");
    error!("处理过程中发生错误");
    
    // 演示结构化日志
    let user_id = 42;
    let username = "test_user";
    info!(
        user_id,
        username,
        event = "user_login",
        "用户登录成功"
    );

    // 演示span跟踪操作
    let data = vec![1, 2, 3];
    for item in data {
        let span = tracing::span!(Level::INFO, "processing_item", item_id = item);
        let _enter = span.enter();
        
        info!("处理项目: {}", item);
        
        if item == 2 {
            warn!("发现需要特别注意的项目");
        }
        
        debug!("项目处理成功");
    }

    info!("应用程序关闭");
    
    // 确保所有日志都写入到文件中
    // 通过保持guard不被提前删除
    drop(guard);
    
    println!("\n检查生成的日志文件: {}/app.log", log_dir);
    
    Ok(())
} 
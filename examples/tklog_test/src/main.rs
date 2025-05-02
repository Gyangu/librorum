use anyhow::Result;
use chrono::{DateTime, Local};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use tklog::{Format, LEVEL, LOG, ASYNC_LOG, sync::Logger, MODE, 
    handle::{FileTimeMode, FileSizeMode}, LogOption};

#[tokio::main]
async fn main() -> Result<()> {
    println!("===== tklog 0.2.9 全功能测试 =====\n");
    
    // 测试1: 基本同步日志
    test_sync_log().await?;
    
    // 等待一秒钟确保日志写入
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // 测试2: 异步日志
    test_async_log().await?;
    
    // 测试3: 自定义格式化
    test_custom_format().await?;
    
    // 测试4: 按模块设置独立日志参数
    test_mod_logger().await?;
    
    // 测试5: 按日志级别设置独立参数
    test_level_option().await?;
    
    // 测试6: 混合模式文件切割
    test_mixed_mode().await?;
    
    // 测试7: 多实例日志
    test_multi_logger().await?;
    
    // 测试8: 自定义分隔符
    test_custom_separator().await?;
    
    // 测试9: 自定义日志处理函数
    test_custom_handler().await?;
    
    // 测试10: 自定义属性格式化（带颜色）
    test_attr_format().await?;
    
    // 测试11: 彩色日志特性
    test_colored_log().await?;
    
    // 等待确保所有日志处理完成
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    println!("\n===== tklog 0.2.9 测试完成 =====");
    println!("请查看生成的各种日志文件了解更多信息！");
    
    Ok(())
}

// 基本同步日志测试
async fn test_sync_log() -> Result<()> {
    println!("\n----- 测试基本同步日志功能 -----");
    
    // 初始化日志系统
    LOG.set_console(true) // 设置控制台日志
        .set_level(LEVEL::Debug) // 设置日志级别为Debug
        .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName) // 结构化日志
        .set_formatter("{level}{time} {file}:{message}\n") // 自定义格式，添加换行符
        .set_cutmode_by_size("sync_log_test.log", 1<<20, 5, true); // 每1MB切割一次，保留5个备份，压缩
    
    // 测试不同级别的日志
    tklog::trace!("【同步】这是一条[trace]级别的日志 (不会显示，因为低于设置的Debug级别)");
    tklog::debug!("【同步】这是一条[debug]级别的日志");
    tklog::info!("【同步】这是一条[info]级别的日志");
    tklog::warn!("【同步】这是一条[warn]级别的日志");
    tklog::error!("【同步】这是一条[error]级别的日志");
    tklog::fatal!("【同步】这是一条[fatal]级别的日志");
    
    // 测试带有附加字段的日志
    let user_id = 1001;
    let action = "login";
    tklog::info!("【同步】用户执行了操作，用户ID: ", user_id, "，操作类型: ", action);

    Ok(())
}

// 异步日志测试
async fn test_async_log() -> Result<()> {
    println!("\n----- 测试异步日志功能 -----");
    
    // 初始化异步日志系统
    ASYNC_LOG
        .set_console(true) // 设置控制台日志
        .set_level(LEVEL::Debug) // 设置日志级别为Debug
        .set_format(Format::LevelFlag | Format::Microseconds | Format::ShortFileName) // 结构化日志
        .set_formatter("{level}{time} {file}:{message}\n") // 添加换行符
        .set_cutmode_by_size("async_log_test.log", 1<<20, 5, true).await; // 按大小切割
    
    // 测试异步日志
    tklog::async_debug!("【异步】这是一条[debug]级别的日志");
    tklog::async_info!("【异步】这是一条[info]级别的日志");
    tklog::async_warn!("【异步】这是一条[warn]级别的日志");
    tklog::async_error!("【异步】这是一条[error]级别的日志");
    
    // 测试官方日志库API支持
    ASYNC_LOG.uselog();
    log::debug!("【异步】这是使用标准log库API的debug日志");
    log::info!("【异步】这是使用标准log库API的info日志");

    Ok(())
}

// 自定义格式化测试
async fn test_custom_format() -> Result<()> {
    println!("\n----- 测试自定义格式化功能 -----");
    
    // 创建自定义格式的日志实例
    let mut log = Logger::new();
    log.set_console(true)
        .set_level(LEVEL::Debug)
        .set_formatter("{level} | {time} | {file} | {message}\n"); // 自定义分隔格式
    
    // 使用普通日志宏打印消息
    tklog::info!("这是一条普通日志消息");
    
    // 使用带时间的日志消息
    let formatted_time = Local::now().format("%H:%M:%S").to_string();
    tklog::info!("这是一条带有时间的消息，当前时间: ", formatted_time);
    
    // 使用调试格式打印数据结构
    let vector = vec![1, 2, 3];
    let vector_str = format!("{:?}", vector);
    tklog::info!("这是一条带有Vec的消息，内容: ", vector_str);
    
    Ok(())
}

// 按模块设置独立日志参数测试
async fn test_mod_logger() -> Result<()> {
    println!("\n----- 测试按模块设置独立日志参数功能 -----");
    
    // 为当前模块设置特定的日志参数
    LOG.set_mod_option(
        module_path!(), // 获取当前模块路径
        LogOption { 
            level: Some(LEVEL::Info), 
            format: Some(Format::LevelFlag | Format::Time), 
            formatter: Some("{level}[模块专用] {time} - {message}\n".to_string()), 
            console: Some(true),
            fileoption: Some(Box::new(FileTimeMode::new("mod_test.log", MODE::DAY, 5, false))) 
        }
    );
    
    // 测试模块专用的日志
    tklog::debug!("这条DEBUG日志不会显示，因为模块日志级别设置为INFO");
    tklog::info!("这条INFO日志使用模块特定的格式");
    tklog::warn!("这条WARN日志同样使用模块特定的格式");
    
    Ok(())
}

// 按日志级别设置独立参数测试
async fn test_level_option() -> Result<()> {
    println!("\n----- 测试按日志级别设置独立参数功能 -----");
    
    // 重置日志设置
    LOG.set_console(true)
       .set_level(LEVEL::Debug)
       .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName);
    
    // 为ERROR级别设置特定格式
    LOG.set_level_option(
        LEVEL::Error, 
        LogOption { 
            level: None, 
            format: Some(Format::LevelFlag | Format::Time), 
            formatter: Some("{level}【错误】{time} - {message}\n".to_string()), 
            console: None,
            fileoption: Some(Box::new(FileSizeMode::new("error_logs.log", 1<<20, 5, false))) 
        }
    );
    
    // 为WARN级别设置特定格式
    LOG.set_level_option(
        LEVEL::Warn, 
        LogOption { 
            level: None, 
            format: Some(Format::LevelFlag | Format::Date), 
            formatter: Some("{level}【警告】{time} - {message}\n".to_string()), 
            console: None,
            fileoption: None
        }
    );
    
    // 测试不同级别的日志格式
    tklog::info!("这是普通INFO级别日志");
    tklog::warn!("这是WARN级别日志，使用特定格式");
    tklog::error!("这是ERROR级别日志，使用特定格式并写入专门的日志文件");
    
    Ok(())
}

// 混合模式文件切割测试
async fn test_mixed_mode() -> Result<()> {
    println!("\n----- 测试混合模式文件切割功能 -----");
    
    // 设置混合模式文件切割
    LOG.set_cutmode_by_mixed(
        "mixed_mode.log", // 日志文件名
        1<<20,           // 1MB大小上限
        MODE::DAY,       // 按天切割
        5,               // 保留5个备份
        true             // 压缩备份
    );
    
    tklog::info!("这条日志将写入混合模式文件，当文件达到1MB或每天都会切割");
    
    Ok(())
}

// 多实例日志测试
async fn test_multi_logger() -> Result<()> {
    println!("\n----- 测试多实例日志功能 -----");
    
    // 创建第一个日志实例并包装在Arc<Mutex<>>中
    let mut logger1 = Arc::new(Mutex::new(Logger::new()));
    {
        let mut log = logger1.lock().unwrap();
        log.set_console(true)
            .set_level(LEVEL::Info)
            .set_formatter("{level}[实例1] {message}\n");
    }
    
    // 创建第二个日志实例并包装在Arc<Mutex<>>中
    let mut logger2 = Arc::new(Mutex::new(Logger::new()));
    {
        let mut log = logger2.lock().unwrap();
        log.set_console(true)
            .set_level(LEVEL::Debug)
            .set_formatter("{level}[实例2] {message}\n");
    }
    
    // 多实例日志测试
    tklog::debugs!(&mut logger1, "这条DEBUG日志不会显示，因为实例1级别设置为INFO");
    tklog::debugs!(&mut logger2, "这条DEBUG日志会显示，因为实例2级别设置为DEBUG");
    
    tklog::infos!(&mut logger1, "这条INFO日志来自实例1");
    tklog::infos!(&mut logger2, "这条INFO日志来自实例2");
    
    Ok(())
}

// 自定义分隔符测试
async fn test_custom_separator() -> Result<()> {
    println!("\n----- 测试自定义分隔符功能 -----");
    
    // 重置日志设置并设置自定义分隔符
    LOG.set_console(true)
       .set_level(LEVEL::Debug)
       .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName)
       .set_separator(" | "); // 设置参数分隔符为 " | "
    
    // 测试带多个参数的日志
    tklog::info!("参数1", "参数2", "参数3");
    
    // 更改分隔符
    LOG.set_separator(" >>> ");
    tklog::info!("参数1", "参数2", "参数3");
    
    // 恢复默认分隔符
    LOG.set_separator("");
    
    Ok(())
}

// 自定义日志处理函数测试
async fn test_custom_handler() -> Result<()> {
    println!("\n----- 测试自定义日志处理函数功能 -----");
    
    // 自定义日志处理函数
    fn custom_handler(lc: &tklog::LogContext) -> bool {
        // 打印日志上下文信息
        println!("自定义处理: 级别={:?}, 模块={:?}", lc.level, lc.modname);
        
        // 修改低于INFO级别的日志处理
        if lc.level < LEVEL::Info {
            println!("自定义处理: 低级别日志被过滤, 内容: {}", lc.log_body);
            return false; // 返回false表示不继续处理
        }
        
        // 返回true表示继续常规处理
        true
    }
    
    // 设置自定义处理函数
    LOG.set_custom_handler(custom_handler);
    
    // 测试不同级别的日志
    tklog::debug!("这是DEBUG级别日志，将被自定义处理函数过滤");
    tklog::info!("这是INFO级别日志，将通过自定义处理函数并正常显示");
    
    // 移除自定义处理函数
    LOG.set_custom_handler(|_| true);
    
    Ok(())
}

// 自定义属性格式化测试
async fn test_attr_format() -> Result<()> {
    println!("\n----- 测试自定义属性格式化功能 -----");
    
    // 设置自定义属性格式化
    LOG.set_attr_format(|fmt| {
        // 自定义日志级别格式
        fmt.set_level_fmt(|level| {
            match level {
                LEVEL::Trace => "[T]",
                LEVEL::Debug => "[D]",
                LEVEL::Info => "[I]",
                LEVEL::Warn => "[W]",
                LEVEL::Error => "[E]",
                LEVEL::Fatal => "[F]",
                LEVEL::Off => "",
            }.to_string()
        });

        // 自定义时间格式
        fmt.set_time_fmt(|| {
            let now: DateTime<Local> = Local::now();
            (now.format("%Y/%m/%d").to_string(), 
             now.format("%H:%M:%S").to_string(), 
             "".to_string())
        });
    });
    
    // 测试自定义格式
    tklog::debug!("这是使用自定义属性格式的DEBUG日志");
    tklog::info!("这是使用自定义属性格式的INFO日志");
    tklog::warn!("这是使用自定义属性格式的WARN日志");
    
    Ok(())
}

// 彩色日志测试
async fn test_colored_log() -> Result<()> {
    println!("\n----- 测试彩色日志功能 -----");
    
    // 重置日志格式
    LOG.set_console(true)
       .set_level(LEVEL::Trace)
       .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName)
       .set_cutmode_by_size("colored_test.log", 1<<20, 5, true); // 添加日志文件输出用于测试
    
    // 设置自定义属性格式化（区分控制台和文件）
    LOG.set_attr_format(|fmt| {
        // 自定义日志级别格式
        fmt.set_level_fmt(|level| {
            // 这里只设置基础文本格式，无颜色
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
                LEVEL::Info => format!("\x1b[32m{}", trimmed_body),  // 纯绿色
                LEVEL::Warn => format!("\x1b[33m{}", trimmed_body),  // 黄色
                LEVEL::Error => format!("\x1b[31m{}", trimmed_body), // 红色
                LEVEL::Fatal => format!("\x1b[41m{}", trimmed_body), // 背景红色
                LEVEL::Off => body.to_string(),
            }
        });
        
        // 不需要为文件日志特别设置格式，它会使用默认的无颜色格式
    });
    
    // 测试彩色日志输出
    println!("下面的日志将在终端使用不同颜色显示，但文件中不会包含颜色代码：");
    tklog::trace!("这是蓝色的TRACE日志");
    tklog::debug!("这是青色的DEBUG日志");
    tklog::info!("这是绿色的INFO日志");
    tklog::warn!("这是黄色的WARN日志");
    tklog::error!("这是红色的ERROR日志");
    tklog::fatal!("这是带红色背景的FATAL日志");
    
    // 测试多参数日志
    tklog::info!("这是带有", "多个参数", "的彩色日志");
    
    // 恢复默认设置
    LOG.set_attr_format(|_| {});
    
    Ok(())
}

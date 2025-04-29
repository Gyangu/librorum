use crate::config::NodeConfig;
use crate::logger;
use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::{env, thread};

/// PID 文件路径
pub fn pid_file_path() -> PathBuf {
    let path = if let Some(data_dir) = dirs::data_dir() {
        let dir = data_dir.join("librorum");
        // 确保目录存在
        if !dir.exists() {
            println!("创建PID文件目录: {:?}", dir);
            if let Err(e) = std::fs::create_dir_all(&dir) {
                tracing::warn!("无法创建 PID 文件目录: {:?} 错误: {}", dir, e);
                println!("无法创建 PID 文件目录: {:?} 错误: {}", dir, e);
            } else {
                tracing::debug!("已创建 PID 文件目录: {:?}", dir);
                println!("已创建 PID 文件目录: {:?}", dir);
            }
        }
        dir.join("librorum.pid")
    } else {
        PathBuf::from("/tmp/librorum.pid")
    };
    
    tracing::debug!("PID 文件路径: {:?}", path);
    println!("PID 文件路径: {:?}", path);
    path
}

/// 检查服务是否已运行
pub fn is_running() -> bool {
    let pid_file = pid_file_path();
    
    tracing::debug!("检查服务状态，PID文件: {:?}", pid_file);
    
    // 首先检查进程是否在运行，即使PID文件不存在
    #[cfg(not(windows))]
    {
        // 使用ps命令查找运行中的librorum守护进程
        let ps_cmd = "ps -ef | grep '[l]ibrorum.*run.*--daemon' | grep -v grep | awk '{print $2}'";
        let output = Command::new("sh")
            .arg("-c")
            .arg(ps_cmd)
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let pids = stdout.trim();
                let exists = !pids.is_empty();
                tracing::debug!("ps查找结果: {}，找到的PID: '{}'", exists, pids);
                
                if exists {
                    // 显示进程详情
                    let detail_cmd = format!("ps -p {} -o pid,ppid,command", pids);
                    let detail_output = Command::new("sh")
                        .arg("-c")
                        .arg(&detail_cmd)
                        .output();
                        
                    if let Ok(detail) = detail_output {
                        let detail_stdout = String::from_utf8_lossy(&detail.stdout);
                        tracing::debug!("进程详情:\n{}", detail_stdout);
                    }
                    
                    // 更新PID文件
                    if !pid_file.exists() {
                        if let Err(e) = std::fs::write(&pid_file, pids) {
                            tracing::warn!("无法更新PID文件: {}", e);
                        } else {
                            tracing::info!("已创建PID文件: {:?}, 内容: {}", pid_file, pids);
                        }
                    }
                    
                    return true;
                }
            }
            Err(e) => {
                tracing::warn!("ps检查失败: {}", e);
            }
        }
    }
    
    #[cfg(windows)]
    {
        // 在Windows上查找librorum进程
        let output = Command::new("powershell")
            .arg("-Command")
            .arg("Get-Process -Name 'librorum' -ErrorAction SilentlyContinue | Measure-Object | Select-Object -ExpandProperty Count")
            .output();
        
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let count = stdout.trim();
                let exists = count != "0";
                tracing::debug!("Windows进程检查结果: {}，找到{}个进程", exists, count);
                
                if exists {
                    return true;
                }
            }
            Err(e) => {
                tracing::warn!("Windows进程检查失败: {}", e);
            }
        }
    }
    
    // 如果通过进程检查失败，再检查PID文件
    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            let clean_pid_str = pid_str.trim().replace("%", "");
            tracing::debug!("从PID文件读取内容: '{}' (清理后: '{}')", pid_str, clean_pid_str);
            
            // 检查是否是时间戳形式的临时PID (通常是大于10位的数字)
            let timestamp_check = match clean_pid_str.parse::<u64>() {
                Ok(n) if n > 1000000000 && clean_pid_str.len() >= 10 => true,
                _ => false
            };
                
            if timestamp_check {
                tracing::debug!("检测到时间戳形式的临时PID: {}", clean_pid_str);
                return is_process_running_by_pattern("librorum.*run.*--daemon");
            }
            
            // 常规PID检查
            if let Ok(pid) = clean_pid_str.parse::<u32>() {
                tracing::debug!("检查进程状态，PID: {} (原始文本: '{}')", pid, pid_str);
                return is_process_running(pid);
            } else {
                tracing::warn!("无效的PID格式: {} (清理后: {})", pid_str, clean_pid_str);
                // 删除无效的PID文件
                if let Err(e) = fs::remove_file(&pid_file) {
                    tracing::warn!("无法删除无效的PID文件: {}", e);
                } else {
                    tracing::info!("已删除无效的PID文件");
                }
            }
        } else {
            tracing::warn!("无法读取PID文件: {:?}", pid_file);
        }
    } else {
        tracing::debug!("PID文件不存在: {:?}", pid_file);
    }
    
    false
}

// 使用模式检查进程是否运行
fn is_process_running_by_pattern(pattern: &str) -> bool {
    #[cfg(not(windows))]
    {
        let ps_cmd = format!("ps -ef | grep '[{}]' | grep -v grep | wc -l", pattern);
        let output = Command::new("sh")
            .arg("-c")
            .arg(&ps_cmd)
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let count = stdout.trim().parse::<i32>().unwrap_or(0);
                let exists = count > 0;
                tracing::debug!("模式'{}'进程检查结果: {}, 找到{}个进程", pattern, exists, count);
                exists
            },
            Err(e) => {
                tracing::warn!("模式进程检查失败: {}", e);
                false
            }
        }
    }
    
    #[cfg(windows)]
    {
        // 在Windows上使用更通用的检查方式
        false
    }
}

/// 启动服务
pub fn start_daemon(config: &NodeConfig) -> Result<()> {
    // 检查服务是否已运行
    if is_running() {
        return Err(anyhow!("服务已经在运行中"));
    }
    
    // 创建数据目录
    config.create_data_dir()?;
    
    // 创建日志目录
    let log_dir = logger::log_dir_path();
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("无法创建日志目录: {:?}", log_dir))?;
    
    // 确保 PID 文件路径的父目录存在
    let pid_path = pid_file_path();
    if let Some(parent) = pid_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("无法创建PID文件目录: {:?}", parent))?;
            tracing::info!("已创建PID文件目录: {:?}", parent);
        }
    }
    
    // 获取当前可执行文件路径
    let exe_path = env::current_exe()
        .with_context(|| "无法获取当前可执行文件路径")?;
        
    tracing::info!("准备启动守护进程，可执行文件: {:?}", exe_path);
    
    #[cfg(target_os = "macos")]
    {
        // 在macOS上使用前台启动方式运行守护进程
        let exe_path_str = exe_path.to_string_lossy();
        let log_dir = logger::log_dir_path();
        
        // 创建日志目录
        if !log_dir.exists() {
            if let Err(e) = fs::create_dir_all(&log_dir) {
                println!("无法创建日志目录 {:?}: {}", log_dir, e);
            } else {
                println!("创建日志目录: {:?}", log_dir);
            }
        }
        
        let debug_log = log_dir.join("daemon_debug.log");
        println!("调试日志文件: {:?}", debug_log);

        // 先尝试清理可能已经存在但没有正确运行的进程
        let clean_cmd = "pkill -f 'librorum.*run.*--daemon' || true";
        println!("执行清理命令: {}", clean_cmd);
        let _ = Command::new("sh").arg("-c").arg(clean_cmd).status();
        
        // 构建完整的启动命令，注意子命令格式
        // 根据help输出，run --daemon不接受--config参数，需要在主命令中指定
        let config_path = if let Some(config_file) = NodeConfig::find_config_file() {
            format!("-c \"{}\"", config_file.to_string_lossy())
        } else {
            String::new()
        };
        
        // 以正确的参数格式启动：先是全局参数-c/-l，然后是子命令run，再是子命令参数--daemon
        let cmd = format!(
            "cd {} && {} -l debug {} run --daemon > /tmp/librorum_test.log 2>&1 &", 
            std::env::current_dir().unwrap_or_default().to_string_lossy(),
            exe_path_str,
            config_path
        );
        
        println!("执行命令: {}", cmd);
        
        // 执行启动命令
        let status = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .status()
            .with_context(|| "无法启动服务进程")?;
        
        println!("命令执行状态: {:?}", status);
        
        // 等待进程启动
        println!("等待进程启动...");
        thread::sleep(Duration::from_secs(2));
        
        // 查找进程
        let ps_cmd = "ps -ef | grep '[l]ibrorum.*run.*--daemon' | awk '{print $2}'";
        println!("执行ps查找: {}", ps_cmd);
        
        let ps_output = Command::new("sh")
            .arg("-c")
            .arg(ps_cmd)
            .output()
            .with_context(|| "无法执行ps命令")?;
            
        let pids = String::from_utf8_lossy(&ps_output.stdout).trim().to_string();
        println!("找到的PID: '{}'", pids);
        
        if !pids.is_empty() {
            // 显示进程详情
            let detail_cmd = format!("ps -p {} -o pid,ppid,command", pids);
            println!("查看进程详情: {}", detail_cmd);
            
            let detail_output = Command::new("sh")
                .arg("-c")
                .arg(&detail_cmd)
                .output();
                
            if let Ok(output) = detail_output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("进程详情:\n{}", stdout);
            }
            
            // 写入PID文件
            let mut pid_file = File::create(&pid_path)
                .with_context(|| format!("无法创建PID文件: {:?}", pid_path))?;
                
            println!("写入PID文件: {:?}, 内容: {}", pid_path, pids);
            pid_file
                .write_all(pids.as_bytes())
                .with_context(|| "无法写入PID文件")?;
                
            println!("服务已成功启动，PID: {}", pids);
            return Ok(());
        }
        
        // 检查临时日志
        println!("\n===== 检查临时日志文件 =====");
        let _ = Command::new("sh")
            .arg("-c")
            .arg("cat /tmp/librorum_test.log 2>/dev/null || echo '运行日志文件不存在'")
            .status();
        
        // 使用临时PID
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let temp_pid = format!("{}", timestamp);
        let mut pid_file = File::create(&pid_path)
            .with_context(|| format!("无法创建PID文件: {:?}", pid_path))?;
            
        println!("使用临时PID: {}, 文件: {:?}", temp_pid, pid_path);
        pid_file
            .write_all(temp_pid.as_bytes())
            .with_context(|| "无法写入PID文件")?;

        println!(
            "服务已启动，但无法确认是否正在运行，使用临时PID标记: {}",
            temp_pid
        );
        return Ok(());
    }
    
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        // Linux系统使用常规的nohup方式
        let exe_path_str = exe_path.to_string_lossy();

        // 构建shell命令
        let mut shell_cmd = format!("nohup {} run --daemon", exe_path_str);

        // 添加配置文件参数
        if let Some(config_file) = NodeConfig::find_config_file() {
            shell_cmd.push_str(&format!(" --config \"{}\"", config_file.to_string_lossy()));
        }

        // 添加环境变量
        shell_cmd.push_str(&format!(" --log-level {}", config.log_level));
        shell_cmd.push_str(" > /dev/null 2>&1 &");

        tracing::info!("启动守护进程命令: {}", shell_cmd);

        // 使用sh执行命令
        let status = Command::new("sh")
            .arg("-c")
            .arg(&shell_cmd)
            .status()
            .with_context(|| "无法启动服务进程")?;

        if !status.success() {
            return Err(anyhow!("启动服务进程失败，返回状态: {:?}", status.code()));
        }

        // 等待进程启动
        thread::sleep(Duration::from_secs(1));

        // 获取进程PID
        let ps_cmd = format!("pgrep -f '{} run --daemon'", exe_path_str);
        let ps_output = Command::new("sh")
            .arg("-c")
            .arg(&ps_cmd)
            .output()
            .with_context(|| "无法获取进程PID")?;

        let pid_str = String::from_utf8_lossy(&ps_output.stdout).trim().to_string();
        if !pid_str.is_empty() {
            let pid = pid_str.parse::<u32>()
                .with_context(|| format!("无效的PID格式: {}", pid_str))?;

            // 写入PID文件
            let mut pid_file = File::create(&pid_path)
                .with_context(|| format!("无法创建PID文件: {:?}", pid_path))?;

            pid_file.write_all(pid_str.as_bytes())
                .with_context(|| "无法写入PID文件")?;

            tracing::info!("已写入PID文件: {:?} 内容: {}", pid_path, pid_str);
            println!("服务已成功启动，PID: {}", pid);
            return Ok(());
        }

        // 使用临时PID标记
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let temp_pid = format!("{}", timestamp);
        let mut pid_file = File::create(&pid_path)
            .with_context(|| format!("无法创建PID文件: {:?}", pid_path))?;

        pid_file.write_all(temp_pid.as_bytes())
            .with_context(|| "无法写入PID文件")?;

        tracing::info!("已写入临时PID文件: {:?} 内容: {}", pid_path, temp_pid);
        println!("服务已成功启动（使用临时PID标记）");
        return Ok(());
    }
    
    #[cfg(windows)]
    {
        // 在 Windows 上使用 PowerShell 启动无窗口进程
        let mut windows_cmd = Command::new("powershell");
        
        // 构造PowerShell兼容的参数列表
        let mut ps_args = Vec::new();
        
        // 添加基本参数 (需要添加引号)
        ps_args.push("\"run\"".to_string());
        ps_args.push("\"--daemon\"".to_string());
        
        // 添加配置文件参数
        if let Some(config_path) = NodeConfig::find_config_file() {
            ps_args.push("\"--config\"".to_string());
            ps_args.push(format!("\"{}\"", config_path.to_string_lossy()));
        }
        
        // 将参数数组转换为PowerShell参数字符串
        let args_str = ps_args.join(",");
        
        // 执行启动命令 - 使用完全兼容的PowerShell语法
        let cmd_str = format!(
            "Start-Process -FilePath \"{}\" -ArgumentList {} -WindowStyle Hidden", 
            exe_path.to_string_lossy(),
            args_str
        );
        
        tracing::debug!("Windows启动命令: {}", cmd_str);
        
        windows_cmd
            .arg("-Command")
            .arg(cmd_str)
            .spawn()
            .with_context(|| "无法在 Windows 上启动服务进程")?;
            
        // 等待一会儿，确保进程启动
        thread::sleep(Duration::from_secs(1));
        
        // 获取 PID
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(format!("Get-Process -Name \"{}\" | Select-Object -ExpandProperty Id", 
                         exe_path.file_stem().unwrap().to_string_lossy()))
            .output()
            .with_context(|| "无法获取进程 PID")?;
            
        let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        if pid.is_empty() {
            return Err(anyhow!("服务启动失败，未能获取进程ID"));
        }
        
        // 写入 PID 文件 - 确保没有额外字符
        let mut pid_file = File::create(&pid_path)
            .with_context(|| format!("无法创建 PID 文件: {:?}", pid_path))?;
        
        // 确保PID写入格式正确 - 只写入纯数字    
        pid_file.write_all(pid.trim().as_bytes())
            .with_context(|| "无法写入 PID 文件")?;
            
        tracing::info!("Windows服务已启动，PID: {}", pid);
        
        // 等待服务启动
        thread::sleep(Duration::from_secs(2));
        
        // 最后验证Windows服务是否真的在运行
        if is_running() {
            println!("服务已成功启动");
            return Ok(());
        } else {
            tracing::error!("无法检测到Windows服务运行，尽管已启动进程，请检查日志文件");
            return Err(anyhow!("服务启动失败，请检查日志文件"));
        }
    }

    // 确保每个平台特定的代码路径都有返回，这里不再需要默认返回
    #[allow(unreachable_code)]
    Ok(())
}

/// 停止服务
pub fn stop_daemon() -> Result<()> {
    let pid_file = pid_file_path();
    
    // 检查是否有运行中的守护进程
    if !is_running() {
        return Err(anyhow!("服务未运行"));
    }
    
    // 读取PID文件内容（如果存在）
    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            // 分割并解析多个PID（用空格、换行或逗号分隔）
            let pids: Vec<u32> = pid_str
                .split(&[' ', '\n', ','][..])
                .filter_map(|s| s.trim().parse::<u32>().ok())
                .collect();
                
            if !pids.is_empty() {
                tracing::info!("从PID文件找到{}个进程: {:?}", pids.len(), pids);
                
                // 针对每个PID执行停止
                for pid in &pids {
                    if *pid > 0 {
                        tracing::info!("正在停止进程，PID: {}", pid);
                        stop_process(*pid)?;
                    }
                }
                
                // 删除PID文件
                if let Err(e) = fs::remove_file(&pid_file) {
                    tracing::warn!("无法删除PID文件: {:?}, 错误: {}", pid_file, e);
                } else {
                    tracing::info!("已删除PID文件: {:?}", pid_file);
                }
                
                println!("服务已成功停止");
                return Ok(());
            }
        }
    }
    
    // 如果没有有效的PID文件，使用进程模式匹配进行停止
    #[cfg(not(windows))]
    {
        tracing::info!("使用进程名称模式匹配停止服务");
        
        // 查找所有匹配的进程
        let ps_cmd = "ps -ef | grep '[l]ibrorum.*run.*--daemon' | grep -v grep | awk '{print $2}'";
        let output = Command::new("sh")
            .arg("-c")
            .arg(ps_cmd)
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let pids = stdout.trim();
                
                if !pids.is_empty() {
                    // 分割并解析多个PID
                    let pid_list: Vec<u32> = pids
                        .split_whitespace()
                        .filter_map(|s| s.parse::<u32>().ok())
                        .collect();
                        
                    if !pid_list.is_empty() {
                        tracing::info!("找到{}个进程: {:?}", pid_list.len(), pid_list);
                        
                        // 停止每个进程
                        for pid in &pid_list {
                            tracing::info!("正在停止进程，PID: {}", pid);
                            stop_process(*pid)?;
                        }
                        
                        // 删除PID文件（如果存在）
                        if pid_file.exists() {
                            if let Err(e) = fs::remove_file(&pid_file) {
                                tracing::warn!("无法删除PID文件: {:?}, 错误: {}", pid_file, e);
                            } else {
                                tracing::info!("已删除PID文件: {:?}", pid_file);
                            }
                        }
                        
                        println!("服务已成功停止");
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                tracing::warn!("获取进程列表失败: {}", e);
            }
        }
    }
    
    #[cfg(windows)]
    {
        tracing::info!("在Windows上查找librorum进程");
        
        // 在Windows上使用更通用的匹配方式
        let tasklist_cmd = "powershell -Command \"Get-Process | Where-Object {$_.Name -like '*librorum*'} | ForEach-Object {$_.Id}\"";
        let output = Command::new("cmd")
            .arg("/c")
            .arg(tasklist_cmd)
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let pid_list: Vec<u32> = stdout
                    .split_whitespace()
                    .filter_map(|s| s.parse::<u32>().ok())
                    .collect();
                    
                if !pid_list.is_empty() {
                    tracing::info!("找到{}个Windows进程: {:?}", pid_list.len(), pid_list);
                    
                    // 停止每个进程
                    for pid in &pid_list {
                        tracing::info!("正在停止Windows进程，PID: {}", pid);
                        
                        let status = Command::new("taskkill")
                            .args(&["/PID", &pid.to_string(), "/F"])
                            .status();
                            
                        if let Err(e) = status {
                            tracing::warn!("无法停止Windows进程: PID {}, 错误: {}", pid, e);
                        } else {
                            tracing::info!("已停止Windows进程: PID {}", pid);
                        }
                    }
                    
                    // 删除PID文件（如果存在）
                    if pid_file.exists() {
                        if let Err(e) = fs::remove_file(&pid_file) {
                            tracing::warn!("无法删除PID文件: {:?}, 错误: {}", pid_file, e);
                        } else {
                            tracing::info!("已删除PID文件: {:?}", pid_file);
                        }
                    }
                    
                    println!("服务已成功停止");
                    return Ok(());
                }
            }
            Err(e) => {
                tracing::warn!("获取Windows进程列表失败: {}", e);
            }
        }
    }
    
    // 最终检查服务是否已停止
    if is_running() {
        Err(anyhow!("服务未能成功停止"))
    } else {
        println!("服务已成功停止");
        Ok(())
    }
}

/// 停止单个进程
fn stop_process(pid: u32) -> Result<()> {
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "macos")]
        {
            // 检查是否有launchd服务在运行
            let launchctl_output = Command::new("launchctl")
                .arg("list")
                .arg("com.librorum.daemon")
                .output();
                
            match launchctl_output {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let error_str = String::from_utf8_lossy(&output.stderr);
                    
                    tracing::debug!("launchctl list输出: stdout='{}', stderr='{}'", output_str, error_str);
                    
                    if !output_str.contains("Could not find service") {
                        tracing::info!("通过launchctl停止服务");
                        
                        // 获取Launch Agent路径
                        let launch_agent_path = dirs::home_dir()
                            .unwrap_or_else(|| PathBuf::from("."))
                            .join("Library/LaunchAgents/com.librorum.daemon.plist");
                            
                        tracing::debug!("Launch Agent路径: {:?}", launch_agent_path);
                        
                        // 检查plist文件是否存在
                        if launch_agent_path.exists() {
                            tracing::info!("找到Launch Agent文件: {:?}", launch_agent_path);
                            
                            // 使用launchctl卸载服务
                            let status = Command::new("launchctl")
                                .arg("unload")
                                .arg("-w")
                                .arg(&launch_agent_path)
                                .output();
                                
                            match status {
                                Ok(result) => {
                                    if result.status.success() {
                                        tracing::info!("launchctl成功卸载服务");
                                    } else {
                                        let stderr = String::from_utf8_lossy(&result.stderr);
                                        tracing::warn!("launchctl卸载服务返回非零状态: {}, stderr: {}", 
                                            result.status, stderr);
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!("launchctl卸载服务失败: {}", e);
                                }
                            }
                        } else {
                            tracing::warn!("Launch Agent文件不存在: {:?}", launch_agent_path);
                        }
                    } else {
                        tracing::debug!("未找到launchctl服务");
                    }
                },
                Err(e) => {
                    tracing::warn!("无法检查launchctl服务状态: {}", e);
                }
            }
        }
        
        // 使用kill命令终止进程
        tracing::info!("使用kill命令终止进程 PID: {}", pid);
        let kill_status = Command::new("kill")
            .arg(pid.to_string())
            .status();
            
        if let Err(e) = kill_status {
            tracing::warn!("常规kill命令失败: {}", e);
            
            // 如果普通kill命令失败，尝试发送SIGKILL信号
            tracing::warn!("尝试发送SIGKILL信号");
            let force_kill_status = Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .status();
                
            if let Err(e) = force_kill_status {
                tracing::warn!("SIGKILL也未能终止进程: {}", e);
                return Err(anyhow!("无法终止进程 PID {}", pid));
            } else {
                tracing::info!("成功通过SIGKILL终止进程 PID: {}", pid);
            }
        } else {
            tracing::info!("成功通过常规kill终止进程 PID: {}", pid);
        }
        
        // 等待一小段时间后检查进程是否仍在运行
        thread::sleep(Duration::from_millis(500));
        if is_process_running(pid) {
            tracing::warn!("进程{}仍在运行，尝试发送SIGKILL信号", pid);
            let force_kill_status = Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .status();
                
            if let Err(e) = force_kill_status {
                tracing::warn!("SIGKILL也未能终止进程: {}", e);
                return Err(anyhow!("无法终止进程 PID {}", pid));
            }
            
            // 再次检查进程
            thread::sleep(Duration::from_millis(500));
            if is_process_running(pid) {
                tracing::error!("无法终止进程 PID: {}, 即使使用SIGKILL", pid);
                return Err(anyhow!("无法终止进程 PID {}", pid));
            }
        }
    }
    
    #[cfg(windows)]
    {
        // 在 Windows 上使用 taskkill 命令
        let status = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .status();
            
        if let Err(e) = status {
            tracing::warn!("无法停止Windows进程: PID {}, 错误: {}", pid, e);
            return Err(anyhow!("停止Windows进程失败: {}", e));
        } else {
            tracing::info!("已停止Windows进程: PID {}", pid);
        }
    }
    
    Ok(())
}

/// 重启服务
pub fn restart_daemon(config: &NodeConfig) -> Result<()> {
    // 如果服务正在运行，先停止
    if is_running() {
        stop_daemon()?;
    }
    
    // 等待服务完全停止
    thread::sleep(Duration::from_secs(2));
    
    // 启动服务
    start_daemon(config)
}

/// 获取服务状态
pub fn daemon_status() -> String {
    if is_running() {
        let pid_file = pid_file_path();
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            format!("服务正在运行，PID: {}", pid_str.trim())
        } else {
            "服务正在运行，但无法读取 PID".to_string()
        }
    } else {
        "服务未运行".to_string()
    }
}

/// 获取服务日志
pub fn view_logs(lines: usize) -> Result<String> {
    // 检查服务是否在运行
    let status_msg = if !is_running() {
        "警告: 服务当前未运行\n\n"
    } else {
        ""
    };
    
    // 读取日志文件
    let log_path = logger::log_file_path();
    
    // 检查日志文件是否存在
    if !log_path.exists() {
        return Ok(format!("{}服务状态: {}\n\n日志文件不存在: {:?}", 
            status_msg, daemon_status(), log_path));
    }
    
    // 如果日志文件存在，尝试读取
    match logger::read_log_tail(lines) {
        Ok(content) => {
            if content.is_empty() {
                Ok(format!("{}服务状态: {}\n\n日志文件为空", 
                    status_msg, daemon_status()))
            } else {
                // 在Windows上特殊处理，确保控制台代码页正确
                #[cfg(windows)]
                {
                    // 尝试设置控制台代码页为UTF-8以正确显示中文
                    let _ = std::process::Command::new("powershell")
                        .args(&["-Command", "chcp 65001"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                }
                
                Ok(format!("{}服务状态: {}\n\n日志内容 (最后 {} 行):\n{}", 
                    status_msg, daemon_status(), lines, content))
            }
        },
        Err(e) => {
            Ok(format!("{}服务状态: {}\n\n无法读取日志: {}", 
                status_msg, daemon_status(), e))
        }
    }
}

#[allow(dead_code)] // 由于条件编译，编译器可能认为此函数未使用
fn is_process_running(pid: u32) -> bool {
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "macos")]
        {
            // 先使用ps命令检查进程是否存在，获取命令
            let ps_check = Command::new("ps")
                .arg("-p")
                .arg(pid.to_string())
                .arg("-o")
                .arg("command=")
                .output();
                
            match ps_check {
                Ok(output) => {
                    let command = String::from_utf8_lossy(&output.stdout);
                    let command_str = command.trim();
                    let exists = !command_str.is_empty();
                    
                    if exists {
                        tracing::debug!("进程 {} 正在运行，命令: '{}'", pid, command_str);
                        true
                    } else {
                        tracing::debug!("进程 {} 不存在", pid);
                        false
                    }
                },
                Err(e) => {
                    tracing::warn!("检查进程 {} 状态时出错: {}", pid, e);
                    false
                }
            }
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // 在其他类 Unix 系统上检查进程是否存在
            let output = Command::new("ps")
                .arg("-p")
                .arg(pid.to_string())
                .arg("-o")
                .arg("comm=")
                .output();
                
            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let process_name = stdout.trim();
                    let exists = !process_name.is_empty();
                    if exists {
                        tracing::debug!("进程 {} 正在运行，名称: '{}'", pid, process_name);
                    } else {
                        tracing::debug!("进程 {} 不存在", pid);
                    }
                    exists
                },
                Err(e) => {
                    tracing::warn!("检查进程 {} 状态时出错: {}", pid, e);
                    false
                }
            }
        }
    }
    
    #[cfg(windows)]
    {
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(format!("Get-Process -Id {} -ErrorAction SilentlyContinue", pid))
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let exists = !stdout.trim().is_empty();
                if exists {
                    tracing::debug!("Windows进程 {} 正在运行", pid);
                } else {
                    tracing::debug!("Windows进程 {} 不存在", pid);
                }
                exists
            },
            Err(e) => {
                tracing::warn!("检查Windows进程 {} 状态时出错: {}", pid, e);
                false
            }
        }
    }
} 
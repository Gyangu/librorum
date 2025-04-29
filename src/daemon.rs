/// 启动服务
pub fn start_daemon(config: &NodeConfig) -> Result<()> {
    use std::fs;
    
    // 检查服务是否已运行
    if is_running() {
        return Err(anyhow!("服务已经在运行中"));
    }

    // 创建数据目录
    config.create_data_dir()?;

    // 创建日志目录
    let log_dir = logger::log_dir_path();
    fs::create_dir_all(&log_dir).with_context(|| format!("无法创建日志目录: {:?}", log_dir))?;

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
    let exe_path = env::current_exe().with_context(|| "无法获取当前可执行文件路径")?;

    // 构建命令
    let mut cmd = Command::new(&exe_path);
    cmd.arg("run")
        .arg("--daemon")
        .env("RUST_LOG", &config.log_level);

    // 如果有配置文件路径，添加为参数
    if let Some(config_file) = NodeConfig::find_config_file() {
        let config_path = config_file.to_string_lossy().to_string();
        cmd.arg("--config").arg(&config_path);
    }

    tracing::info!("准备启动守护进程，可执行文件: {:?}", exe_path);

    #[cfg(not(windows))]
    {
        // 在macOS上使用最简单的方法启动守护进程
        #[cfg(target_os = "macos")]
        {
            // 在macOS上使用最简单的方法启动守护进程
            let exe_path_str = exe_path.to_string_lossy();
            let log_dir = logger::log_dir_path();
            let debug_log = log_dir.join("daemon_debug.log");

            // 先尝试清理可能已经存在但没有正确运行的进程
            let clean_cmd = "pkill -f 'librorum.*run.*--daemon' || true";
            let _ = Command::new("sh").arg("-c").arg(clean_cmd).status();
            
            // 使用简单的nohup方式，并将日志输出到指定文件以便调试
            let cmd_str = format!(
                "nohup {} run --daemon --log-level debug > {} 2>&1 &",
                exe_path_str,
                debug_log.to_string_lossy()
            );

            if let Some(config_file) = NodeConfig::find_config_file() {
                let config_path = config_file.to_string_lossy().to_string();
                let cmd_str = format!(
                    "nohup {} run --daemon --config {} --log-level debug > {} 2>&1 &",
                    exe_path_str,
                    config_path,
                    debug_log.to_string_lossy()
                );

                tracing::info!("macOS启动命令(带配置): {}", cmd_str);

                // 使用sh执行命令
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(&cmd_str)
                    .status()
                    .with_context(|| "无法启动服务进程")?;

                if !status.success() {
                    return Err(anyhow!("启动服务进程失败，返回状态: {:?}", status.code()));
                }
            } else {
                tracing::info!("macOS启动命令(无配置): {}", cmd_str);

                // 使用sh执行命令
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(&cmd_str)
                    .status()
                    .with_context(|| "无法启动服务进程")?;

                if !status.success() {
                    return Err(anyhow!("启动服务进程失败，返回状态: {:?}", status.code()));
                }
            }

            // 等待进程启动
            thread::sleep(Duration::from_secs(2));

            // 使用pgrep查找进程PID
            let ps_cmd = format!("pgrep -f '{} run --daemon'", exe_path_str);
            let ps_output = Command::new("sh")
                .arg("-c")
                .arg(&ps_cmd)
                .output()
                .with_context(|| "无法获取进程PID")?;

            let pid_str = String::from_utf8_lossy(&ps_output.stdout)
                .trim()
                .to_string();
            tracing::info!("通过pgrep查找到的PID: '{}'", pid_str);

            // 如果找到了PID，则写入PID文件
            if !pid_str.is_empty() {
                let mut pid_file = File::create(&pid_path)
                    .with_context(|| format!("无法创建PID文件: {:?}", pid_path))?;

                pid_file
                    .write_all(pid_str.as_bytes())
                    .with_context(|| "无法写入PID文件")?;

                println!("服务已成功启动，PID: {}", pid_str);
                return Ok(());
            }

            // 显示启动日志以便调试
            if debug_log.exists() {
                let debug_content = fs::read_to_string(&debug_log)
                    .unwrap_or_else(|_| "无法读取调试日志".to_string());
                tracing::info!("守护进程调试日志:\n{}", debug_content);
                println!(
                    "请参阅日志文件获取更多信息: {}",
                    debug_log.to_string_lossy()
                );
            }
            
            // 最后一次尝试直接启动进程，不使用nohup
            let direct_cmd = format!(
                "{} run --daemon --log-level debug > {} 2>&1 &",
                exe_path_str,
                debug_log.to_string_lossy()
            );
            
            if let Some(config_file) = NodeConfig::find_config_file() {
                let config_path = config_file.to_string_lossy().to_string();
                let direct_cmd = format!(
                    "{} run --daemon --config {} --log-level debug > {} 2>&1 &",
                    exe_path_str,
                    config_path,
                    debug_log.to_string_lossy()
                );
                
                tracing::info!("macOS直接启动命令(带配置): {}", direct_cmd);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&direct_cmd)
                    .status();
            } else {
                tracing::info!("macOS直接启动命令(无配置): {}", direct_cmd);
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(&direct_cmd)
                    .status();
            }
            
            // 再次等待并检查进程
            thread::sleep(Duration::from_secs(1));
            
            let ps_cmd = format!("pgrep -f '{} run --daemon'", exe_path_str);
            let ps_output = Command::new("sh")
                .arg("-c")
                .arg(&ps_cmd)
                .output();
                
            if let Ok(output) = ps_output {
                let pid_str = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
                    
                if !pid_str.is_empty() {
                    if let Err(e) = fs::write(&pid_path, pid_str.as_bytes()) {
                        tracing::warn!("无法写入PID文件: {}", e);
                    } else {
                        println!("服务已成功启动，PID: {}", pid_str);
                        return Ok(());
                    }
                }
            }

            // 使用临时PID
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let temp_pid = format!("{}", timestamp);
            let mut pid_file = File::create(&pid_path)
                .with_context(|| format!("无法创建PID文件: {:?}", pid_path))?;

            pid_file
                .write_all(temp_pid.as_bytes())
                .with_context(|| "无法写入PID文件")?;

            println!(
                "服务已启动，但无法确认是否正在运行，使用临时PID标记: {}",
                temp_pid
            );
            return Ok(());
        }
    }

    Ok(())
}

/// 检查服务是否已运行
pub fn is_running() -> bool {
    use std::fs;
    
    let pid_file = pid_file_path();

    tracing::debug!("检查服务状态，PID文件: {:?}", pid_file);

    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            let clean_pid_str = pid_str.trim().replace("%", "");
            tracing::debug!(
                "从PID文件读取内容: '{}' (清理后: '{}')",
                pid_str,
                clean_pid_str
            );

            // 检查是否是时间戳形式的临时PID (通常是13位数字，大于1000000000000)
            let timestamp_check = match clean_pid_str.parse::<u64>() {
                Ok(n) if n > 1000000000 && clean_pid_str.len() >= 10 => true,
                _ => false,
            };

            if timestamp_check {
                tracing::debug!("检测到时间戳形式的临时PID: {}", clean_pid_str);

                // 对于临时PID，我们尝试多种方法检查进程
                #[cfg(not(windows))]
                {
                    // 方法1: 使用ps直接查找运行中的librorum守护进程
                    let ps_cmd = "ps -ef | grep 'librorum.*run.*--daemon' | grep -v grep | awk '{print $2}'";
                    let output = Command::new("sh").arg("-c").arg(ps_cmd).output();

                    match output {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let pids = stdout.trim();
                            let exists = !pids.is_empty();
                            tracing::debug!("ps查找结果: {}，找到的PID: '{}'", exists, pids);

                            if exists {
                                // 显示进程详情
                                let _ = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("ps -p {} -o pid,ppid,command", pids))
                                    .status();
                                    
                                // 更新PID文件为真实PID
                                if let Err(e) = fs::write(&pid_file, pids) {
                                    tracing::warn!("无法更新PID文件为真实PID: {}", e);
                                } else {
                                    tracing::info!("已更新PID文件为真实PID: {}", pids);
                                }
                                
                                return true;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("ps检查失败: {}", e);
                        }
                    }

                    // 方法2: 使用pgrep查找
                    let pgrep_cmd = "pgrep -f 'librorum.*run.*--daemon'";
                    let output = Command::new("sh").arg("-c").arg(pgrep_cmd).output();

                    match output {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let pids = stdout.trim();
                            let exists = !pids.is_empty();
                            tracing::debug!("pgrep检查结果: {}，找到的PID: '{}'", exists, pids);

                            if exists {
                                // 更新PID文件为真实PID
                                if let Err(e) = fs::write(&pid_file, pids) {
                                    tracing::warn!("无法更新PID文件为真实PID: {}", e);
                                } else {
                                    tracing::info!("已更新PID文件为真实PID: {}", pids);
                                }
                                
                                return true;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("pgrep检查失败: {}", e);
                        }
                    }

                    // 所有方法都失败了，返回false
                    return false;
                }
                
                // ... existing code for Windows ...
            }
            
            // ... existing code for regular PID check ...
        }
    }

    false
} 
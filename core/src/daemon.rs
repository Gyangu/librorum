use librorum_shared::NodeConfig;
use crate::logger;
use anyhow::{Context, Result, anyhow};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[cfg(all(unix, feature = "daemon-unix"))]
use libc;
#[cfg(all(unix, feature = "daemon-unix"))]
use daemonize::Daemonize;

#[cfg(all(windows, feature = "windows_service"))]
use std::ffi::OsString;
#[cfg(all(windows, feature = "windows_service"))]
use windows_service::{
    define_windows_service,
    service::{
        Service, ServiceAccess, ServiceAction, ServiceControl, ServiceControlAccept,
        ServiceErrorControl, ServiceInfo, ServiceManager, ServiceStartType, ServiceState,
        ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
    service_manager::{ServiceManagerAccess, ServiceManagerOpenOptions},
};

/// PID 文件目录
pub fn pid_dir_path() -> PathBuf {
    #[cfg(not(windows))]
    {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("librorum")
        } else {
            PathBuf::from("/tmp/librorum")
        }
    }

    #[cfg(windows)]
    {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("librorum")
        } else {
            let mut path = PathBuf::new();
            path.push("C:");
            path.push("ProgramData");
            path.push("librorum");
            path
        }
    }
}

/// PID 文件路径
/// 支持多实例的 PID 文件路径，根据 config 路径或端口号区分
pub fn pid_file_path(config: &NodeConfig, config_path: Option<&std::path::Path>) -> PathBuf {
    // 优先用端口号区分
    let port = config.bind_port;
    // 如果有 config_path，用文件名 hash
    let instance_tag = if let Some(path) = config_path {
        if let Some(fname) = path.file_stem() {
            format!("{}_{}", fname.to_string_lossy(), port)
        } else {
            format!("port{}", port)
        }
    } else {
        format!("port{}", port)
    };
    pid_dir_path().join(format!("librorum_{}.pid", instance_tag))
}

/// 获取可执行文件路径
fn get_executable_path() -> Result<PathBuf> {
    let exe = env::current_exe().with_context(|| "无法获取当前可执行文件路径")?;
    Ok(exe)
}

// Windows 服务相关常量
#[cfg(all(windows, feature = "windows_service"))]
const SERVICE_NAME: &str = "librorum";
#[cfg(all(windows, feature = "windows_service"))]
const SERVICE_DISPLAY_NAME: &str = "Librorum 分布式文件系统";
#[cfg(all(windows, feature = "windows_service"))]
const SERVICE_DESCRIPTION: &str = "Librorum 分布式文件系统服务";

// Windows 服务主入口点
#[cfg(all(windows, feature = "windows_service"))]
define_windows_service!(ffi_service_main, service_main);

#[cfg(all(windows, feature = "windows_service"))]
fn service_main(_arguments: Vec<OsString>) {
    // 创建服务控制处理器
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // 关闭服务
            ServiceControl::Stop => ServiceControlHandlerResult::NoError,
            // 其他控制命令，不处理
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(handle) => handle,
        Err(e) => {
            error!("无法注册服务控制处理器: {:?}", e);
            return;
        }
    };

    // 设置服务状态为运行中
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: 0,
        checkpoint: 0,
        wait_hint: 0,
        process_id: None,
    };

    if let Err(e) = status_handle.set_service_status(next_status) {
        error!("无法设置服务状态: {:?}", e);
        return;
    }

    // 在这里启动实际的服务代码
    // 实际上这会被 run_service 函数调用，这里不需要特别处理
}

/// 启动守护进程 (Unix)
#[cfg(all(unix, feature = "daemon-unix"))]
pub fn start_daemon(config: &NodeConfig, config_path: Option<&std::path::Path>) -> Result<()> {
    // 检查verbose环境变量
    let verbose = std::env::var("LIBRORUM_VERBOSE").is_ok();
    
    // 设置是否启用verbose模式
    if verbose {
        unsafe {
            std::env::set_var("LIBRORUM_VERBOSE", "1");
        }
    }
    
    // 确保 PID 目录存在
    fs::create_dir_all(pid_dir_path())?;

    // 检查是否已经运行
    if daemon_running(config, config_path) {
        println!("服务已经在运行中");
        return Ok(());
    }

    let executable = get_executable_path()?;

    // 序列化配置以传递给子进程
    let config_str = toml::to_string(config).with_context(|| "无法序列化配置")?;
    let config_file = pid_dir_path().join("tmp_config.toml");
    let mut file = fs::File::create(&config_file)?;
    file.write_all(config_str.as_bytes())?;

    // 启动守护进程
    let daemonize = Daemonize::new()
        .pid_file(pid_file_path(config, config_path))
        .chown_pid_file(false)
        .working_directory(".");

    match daemonize.start() {
        Ok(_) => {
            // 在守护进程内部
            let config_str = config_file.to_string_lossy();
            
            let mut args = vec!["run", "--daemon", "--config", &config_str];
            if verbose {
                args.push("--verbose");
            }
            
            let status = Command::new(&executable)
                .args(args)
                .status()
                .with_context(|| "无法启动服务进程")?;

            if !status.success() {
                let exit_code = status.code().unwrap_or(-1);
                return Err(anyhow!("服务进程异常退出，退出码: {}", exit_code));
            }

            // 删除临时配置文件
            let _ = fs::remove_file(config_file);

            Ok(())
        }
        Err(e) => Err(anyhow!("启动守护进程失败: {}", e)),
    }
}

/// 启动守护进程 (非Unix，非Windows Service)
#[cfg(not(any(
    all(unix, feature = "daemon-unix"),
    all(windows, feature = "windows_service")
)))]
pub fn start_daemon(config: &NodeConfig) -> Result<()> {
    println!("启动服务（标准模式，非守护进程）");

    // 确保 PID 目录存在
    fs::create_dir_all(pid_dir_path())?;

    // 检查是否已经运行
    if daemon_running() {
        println!("服务已经在运行中");
        return Ok(());
    }

    let executable = get_executable_path()?;

    // 序列化配置以传递给子进程
    let config_str = toml::to_string(config).with_context(|| "无法序列化配置")?;
    let config_file = pid_dir_path().join("tmp_config.toml");
    let mut file = fs::File::create(&config_file)?;
    file.write_all(config_str.as_bytes())?;

    // 在后台启动进程
    #[cfg(unix)]
    {
        let config_str = config_file.to_string_lossy();
        
        // 检查是否在环境变量中设置了verbose模式
        let mut args = vec!["run", "--daemon", "--config", &config_str];
        if std::env::var("LIBRORUM_VERBOSE").is_ok() {
            args.push("--verbose");
        }
        
        let child = Command::new(&executable)
            .args(args)
            .spawn()
            .with_context(|| "无法启动服务进程")?;

        // 将PID保存到文件
        let pid = child.id();
        let mut pid_file = fs::File::create(pid_file_path())?;
        pid_file.write_all(pid.to_string().as_bytes())?;

        println!("Librorum 服务已启动（后台进程，PID: {}）", pid);
        Ok(())
    }

    #[cfg(windows)]
    {
        let config_str = config_file.to_string_lossy();

        // 检查是否在环境变量中设置了verbose模式
        let mut args = vec!["run", "--daemon", "--config", &config_str];
        if std::env::var("LIBRORUM_VERBOSE").is_ok() {
            args.push("--verbose");
        }
        
        // 在Windows上使用CreateProcess API启动进程
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;

        let child = Command::new(&executable)
            .args(args)
            .creation_flags(DETACHED_PROCESS)
            .spawn()
            .with_context(|| "无法启动服务进程")?;

        // 将PID保存到文件
        let pid = child.id();
        let mut pid_file = fs::File::create(pid_file_path())?;
        pid_file.write_all(pid.to_string().as_bytes())?;

        println!("Librorum 服务已启动（后台进程，PID: {}）", pid);
        Ok(())
    }
}

/// 启动Windows服务
#[cfg(all(windows, feature = "windows_service"))]
pub fn start_daemon(config: &NodeConfig, config_path: Option<&std::path::Path>) -> Result<()> {
    // 确保 PID 目录存在
    fs::create_dir_all(pid_dir_path())?;

    // 检查服务是否已经注册和运行
    if let Ok(status) = get_service_status() {
        if status.current_state == ServiceState::Running {
            println!("服务已经在运行中");
            return Ok(());
        } else if status.current_state == ServiceState::Stopped {
            // 服务已注册但已停止，启动它
            let service_manager = open_service_manager()?;
            let service = service_manager.open_service(
                SERVICE_NAME,
                ServiceAccess::START | ServiceAccess::CHANGE_CONFIG,
            )?;
            service.start(&[])?;
            println!("Librorum 服务已启动");
            return Ok(());
        }
    }

    // 服务未注册，注册并启动
    let service_manager = open_service_manager()?;

    let executable = get_executable_path()?;

    // 序列化配置以传递给服务
    let config_str = toml::to_string(config).with_context(|| "无法序列化配置")?;
    let config_file = pid_dir_path().join("tmp_config.toml");
    let mut file = fs::File::create(&config_file)?;
    file.write_all(config_str.as_bytes())?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::Auto,
        error_control: ServiceErrorControl::Normal,
        executable: executable.into(),
        launch_arguments: vec![OsString::from("run"), OsString::from("--daemon")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = service_manager.create_service(
        &service_info,
        ServiceAccess::START | ServiceAccess::CHANGE_CONFIG,
    )?;

    // 设置服务描述
    service.set_description(SERVICE_DESCRIPTION)?;

    // 启动服务
    service.start(&[config_file.to_str().unwrap()])?;

    println!("Librorum 服务已注册并启动");
    Ok(())
}

/// 打开服务管理器
#[cfg(all(windows, feature = "windows_service"))]
fn open_service_manager() -> Result<ServiceManager> {
    ServiceManager::local_computer(
        None,
        ServiceManagerAccess::CONNECT
            | ServiceManagerAccess::CREATE_SERVICE
            | ServiceManagerAccess::ENUMERATE_SERVICE,
    )
    .with_context(|| "无法连接到服务管理器")
}

/// 获取服务状态
#[cfg(all(windows, feature = "windows_service"))]
fn get_service_status() -> Result<ServiceStatus> {
    let service_manager = open_service_manager()?;

    let service = match service_manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
        Ok(service) => service,
        Err(_) => return Err(anyhow!("服务未安装")),
    };

    service.query_status().with_context(|| "无法查询服务状态")
}

/// 停止守护进程 (Unix)
#[cfg(unix)]
pub fn stop_daemon(config: &NodeConfig, config_path: Option<&std::path::Path>) -> Result<()> {
    let pid_file = pid_file_path(config, config_path);

    if !pid_file.exists() {
        println!("服务未运行");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file).with_context(|| "无法读取PID文件")?;
    let pid = pid_str.trim().parse::<i32>().with_context(|| "无效的PID")?;

    // 发送终止信号
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }

    // 等待进程退出
    let mut attempts = 0;
    while daemon_running(config, config_path) && attempts < 10 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        attempts += 1;
    }

    if daemon_running(config, config_path) {
        // 如果进程仍在运行，尝试强制终止
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
        println!("已强制停止服务");
    } else {
        println!("服务已停止");
    }

    // 删除PID文件
    if pid_file.exists() {
        let _ = fs::remove_file(&pid_file);
    }

    Ok(())
}

/// 停止Windows进程 (非Service)
#[cfg(all(windows, not(feature = "windows_service")))]
pub fn stop_daemon() -> Result<()> {
    let pid_file = pid_file_path();

    if !pid_file.exists() {
        println!("服务未运行");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file).with_context(|| "无法读取PID文件")?;
    let pid = pid_str.trim().parse::<u32>().with_context(|| "无效的PID")?;

    // 在Windows上使用taskkill命令终止进程
    let status = Command::new("taskkill")
        .args(&["/F", "/PID", &pid.to_string()])
        .status()
        .with_context(|| "无法执行taskkill命令")?;

    if status.success() {
        println!("服务已停止");
    } else {
        println!("停止服务失败");
    }

    // 删除PID文件
    if pid_file.exists() {
        let _ = fs::remove_file(&pid_file);
    }

    Ok(())
}

/// 停止Windows服务
#[cfg(all(windows, feature = "windows_service"))]
pub fn stop_daemon() -> Result<()> {
    let service_manager = open_service_manager()?;

    let service = match service_manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(service) => service,
        Err(_) => {
            println!("服务未安装");
            return Ok(());
        }
    };

    let status = service.query_status()?;

    if status.current_state == ServiceState::Stopped {
        println!("服务已经停止");
        return Ok(());
    }

    service.stop()?;

    // 等待服务完全停止
    let mut attempts = 0;
    let max_attempts = 10;
    while attempts < max_attempts {
        let current_status = service.query_status()?;
        if current_status.current_state == ServiceState::Stopped {
            println!("服务已停止");
            return Ok(());
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
        attempts += 1;
    }

    println!("服务停止操作已发送，但服务可能仍在关闭中");
    Ok(())
}

/// 重启守护进程
pub fn restart_daemon(config: &NodeConfig, config_path: Option<&std::path::Path>) -> Result<()> {
    stop_daemon(config, config_path)?;

    // 等待一会儿确保服务完全停止
    std::thread::sleep(std::time::Duration::from_secs(2));

    start_daemon(config, config_path)?;
    Ok(())
}

/// 检查守护进程是否在运行 (Unix)
#[cfg(unix)]
pub fn daemon_running(config: &NodeConfig, config_path: Option<&std::path::Path>) -> bool {
    let pid_file = pid_file_path(config, config_path);

    if !pid_file.exists() {
        return false;
    }

    match fs::read_to_string(&pid_file) {
        Ok(pid_str) => {
            // 修复可能的额外字符，只保留数字
            let clean_pid_str: String = pid_str.chars().filter(|c| c.is_ascii_digit()).collect();

            if let Ok(pid) = clean_pid_str.parse::<i32>() {
                // 使用ps命令检查进程是否存在，更加可靠
                let output = Command::new("ps").args(&["-p", &pid.to_string()]).output();

                match output {
                    Ok(output) => {
                        let exit_status = output.status.code().unwrap_or(1);
                        // ps命令成功且输出中包含PID，说明进程存在
                        exit_status == 0
                            && String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
                    }
                    Err(_) => {
                        // 如果ps命令失败，使用老方法尝试
                        unsafe { libc::kill(pid, 0) == 0 }
                    }
                }
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// 检查守护进程是否在运行 (Windows非Service)
#[cfg(all(windows, not(feature = "windows_service")))]
pub fn daemon_running(config: &NodeConfig, config_path: Option<&std::path::Path>) -> bool {
    let pid_file = pid_file_path(config, config_path);

    if !pid_file.exists() {
        return false;
    }

    match fs::read_to_string(&pid_file) {
        Ok(pid_str) => {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // 在Windows上使用tasklist检查进程是否存在
                let output = Command::new("tasklist")
                    .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
                    .output();

                match output {
                    Ok(output) => {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        output_str.contains(&pid.to_string())
                    }
                    Err(_) => false,
                }
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// 检查Windows服务是否在运行
#[cfg(all(windows, feature = "windows_service"))]
pub fn daemon_running(_config: &NodeConfig, _config_path: Option<&std::path::Path>) -> bool {
    match get_service_status() {
        Ok(status) => status.current_state == ServiceState::Running,
        Err(_) => false,
    }
}

/// 获取守护进程状态信息
pub fn daemon_status(config: &NodeConfig, config_path: Option<&std::path::Path>) -> String {
    if daemon_running(config, config_path) {
        let pid_file = pid_file_path(config, config_path);

        #[cfg(not(all(windows, feature = "windows_service")))]
        let pid_info = match fs::read_to_string(&pid_file) {
            Ok(pid_str) => format!("PID: {}", pid_str.trim()),
            Err(_) => "无法读取PID".to_string(),
        };

        #[cfg(all(windows, feature = "windows_service"))]
        let pid_info = match get_service_status() {
            Ok(status) => {
                let state = match status.current_state {
                    ServiceState::Running => "运行中",
                    ServiceState::Stopped => "已停止",
                    ServiceState::StartPending => "正在启动",
                    ServiceState::StopPending => "正在停止",
                    ServiceState::PausePending => "正在暂停",
                    ServiceState::Paused => "已暂停",
                    ServiceState::ContinuePending => "正在继续",
                    _ => "未知状态",
                };
                format!("状态: {}", state)
            }
            Err(_) => "无法获取服务状态".to_string(),
        };

        format!("服务状态: 运行中\n{}", pid_info)
    } else {
        "服务状态: 未运行".to_string()
    }
}

/// 查看服务日志
pub fn view_logs(tail: usize) -> Result<String> {
    logger::view_recent_logs(tail)
}

/// 获取节点健康状态信息
pub fn get_nodes_health_status(config: &NodeConfig, config_path: Option<&std::path::Path>) -> Result<String> {
    // 确保PID文件存在，服务正在运行
    if !daemon_running(config, config_path) {
        return Err(anyhow::anyhow!("服务未运行"));
    }

    // 获取最近的日志
    let logs = logger::view_recent_logs(300)?;

    // 提取节点健康状态报告
    let mut health_status = String::new();
    let mut has_report = false;

    // 寻找最近的节点健康状态报告
    for line in logs.lines().rev() {
        if line.contains("节点健康状态报告:") {
            has_report = true;
            health_status.push_str(line);
            health_status.push('\n');

            // 继续读取报告的所有行
            for report_line in logs
                .lines()
                .rev()
                .skip(logs.lines().rev().position(|l| l == line).unwrap() + 1)
            {
                if report_line.contains("发现 ") && report_line.contains(" 个节点") {
                    health_status.push_str(report_line);
                    health_status.push('\n');
                } else if report_line.trim().starts_with("- ") || report_line.contains("节点详情:")
                {
                    health_status.push_str(report_line);
                    health_status.push('\n');
                } else if !report_line.trim().is_empty()
                    && !report_line.contains("===")
                    && !report_line.contains("INFO")
                {
                    // 如果遇到不相关的内容，停止读取
                    break;
                }
            }

            break;
        } else if line.contains("接收心跳请求统计:") {
            // 也收集心跳请求统计信息
            if !has_report {
                health_status.push_str("接收心跳请求统计:\n");

                // 查找后续几行相关信息
                for stat_line in logs
                    .lines()
                    .rev()
                    .skip(logs.lines().rev().position(|l| l == line).unwrap() + 1)
                {
                    if stat_line.contains("累计接收") || stat_line.contains("连接成功") {
                        health_status.push_str(stat_line);
                        health_status.push('\n');
                    } else if !stat_line.trim().is_empty()
                        && !stat_line.contains("===")
                        && !stat_line.contains("INFO")
                    {
                        break;
                    }
                }
            }
        }
    }

    // 如果没有找到健康状态报告，查找节点连接日志
    if !has_report && health_status.is_empty() {
        // 查找最近的连接日志
        let mut connections = Vec::new();
        for line in logs.lines() {
            if line.contains("到节点成功:") || line.contains("心跳发送失败:") {
                connections.push(line.to_string());
            }
        }

        // 只保留最近的20条连接记录
        if connections.len() > 20 {
            let start_idx = connections.len() - 20;
            connections = connections.split_off(start_idx);
        }

        if !connections.is_empty() {
            health_status.push_str("最近节点连接日志:\n");
            for conn in &connections {
                health_status.push_str(conn);
                health_status.push('\n');
            }
        } else {
            health_status.push_str("未找到节点健康状态报告或连接日志。\n");
            health_status.push_str("可能服务刚启动，还未生成任何节点状态报告。\n");
            health_status.push_str("请等待几分钟后重试，或查看完整日志。\n");
        }
    }

    Ok(health_status)
}

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 原来的proto编译逻辑
    println!("cargo:rerun-if-changed=src/proto");
    tonic_build::compile_protos("src/proto/node.proto")?;

    // 添加新的文件变更监听
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=build.rs");

    // 设置运行标志，确保复制操作在编译后执行
    println!("cargo:rerun-if-env-changed=LIBRORUM_SKIP_COPY");

    // 获取当前目录的绝对路径
    let current_dir = env::current_dir().expect("无法获取当前目录");
    println!("当前工作目录: {:?}", current_dir);

    // 获取Cargo.toml所在目录
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("无法获取CARGO_MANIFEST_DIR");
    println!("Cargo清单目录: {:?}", manifest_dir);

    // 获取OUT_DIR环境变量（编译输出目录）
    let out_dir = env::var("OUT_DIR").expect("无法获取OUT_DIR");
    println!("编译输出目录: {:?}", out_dir);

    // 获取项目根目录（假设是cargo manifest目录的父目录）
    let project_root = Path::new(&manifest_dir)
        .parent()
        .expect("无法获取项目根目录");
    println!("项目根目录: {:?}", project_root);

    // 获取target目录
    let target_dir = project_root.join("target");
    println!("目标目录: {:?}", target_dir);

    // 获取构建配置
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    println!("构建配置: {}", profile);

    // 构建二进制文件路径（多种可能位置）
    let possible_binary_paths = vec![
        // 常规位置
        target_dir.join(&profile).join("librorum"),
        // 直接在target下
        target_dir.join("librorum"),
        // 在manifest目录下
        Path::new(&manifest_dir)
            .join("target")
            .join(&profile)
            .join("librorum"),
        // 在当前目录下
        current_dir.join("target").join(&profile).join("librorum"),
        // 在当前目录的target下
        current_dir.join("target").join("librorum"),
    ];

    // 遍历可能的路径，找到二进制文件
    let mut binary_path = None;
    for path in &possible_binary_paths {
        println!("检查路径: {:?}", path);
        if path.exists() {
            binary_path = Some(path);
            println!("找到二进制文件: {:?}", path);
            break;
        }
    }

    // 如果找不到二进制文件，尝试执行cargo命令构建它
    if binary_path.is_none() {
        println!("没有找到编译好的二进制文件，尝试执行cargo build");
        let output = Command::new("cargo")
            .arg("build")
            .current_dir(&project_root)
            .output();

        match output {
            Ok(output) => {
                println!(
                    "cargo build输出: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                if !output.status.success() {
                    println!(
                        "cargo build错误: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(e) => println!("执行cargo build失败: {}", e),
        }

        // 重新检查二进制文件
        for path in &possible_binary_paths {
            if path.exists() {
                binary_path = Some(path);
                println!("现在找到二进制文件: {:?}", path);
                break;
            }
        }
    }

    // 如果仍然找不到二进制文件，返回错误
    let binary_path = match binary_path {
        Some(path) => path.to_path_buf(),
        None => {
            println!("错误: 无法找到编译后的二进制文件。");
            println!("已检查的路径:");
            for path in &possible_binary_paths {
                println!("  {:?}", path);
            }
            return Ok(());
        }
    };

    // 如果设置了LIBRORUM_SKIP_COPY环境变量，跳过复制
    if env::var("LIBRORUM_SKIP_COPY").is_ok() {
        println!("检测到LIBRORUM_SKIP_COPY环境变量，跳过复制操作");
        return Ok(());
    }

    // 构建目标目录路径（client/librorum/Resources）
    let client_dir = project_root.join("client");
    let swift_app_dir = client_dir.join("librorum");
    let resources_dir = swift_app_dir.join("Resources");

    println!("Swift客户端目录: {:?}", client_dir);
    println!("Swift应用目录: {:?}", swift_app_dir);
    println!("资源目录: {:?}", resources_dir);

    // 确保目标目录存在
    match fs::create_dir_all(&resources_dir) {
        Ok(_) => println!("资源目录已创建/存在: {:?}", resources_dir),
        Err(e) => println!("创建资源目录失败: {}", e),
    }

    // 执行复制操作
    let target_file = resources_dir.join("librorum");
    println!("目标文件: {:?}", target_file);

    match fs::copy(&binary_path, &target_file) {
        Ok(_) => println!("成功复制二进制文件: {:?} -> {:?}", binary_path, target_file),
        Err(e) => println!("复制二进制文件失败: {}。尝试使用命令行复制", e),
    }

    // 如果直接文件复制失败，尝试使用系统命令复制
    if !target_file.exists() {
        println!("使用系统命令复制文件");
        #[cfg(unix)]
        {
            let output = Command::new("cp")
                .arg(binary_path.to_str().unwrap())
                .arg(target_file.to_str().unwrap())
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        println!("使用cp命令成功复制文件");
                    } else {
                        println!("cp命令失败: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => println!("执行cp命令失败: {}", e),
            }
        }

        #[cfg(windows)]
        {
            let output = Command::new("cmd")
                .args(&[
                    "/C",
                    "copy",
                    binary_path.to_str().unwrap(),
                    target_file.to_str().unwrap(),
                ])
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        println!("使用copy命令成功复制文件");
                    } else {
                        println!("copy命令失败: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => println!("执行copy命令失败: {}", e),
            }
        }
    }

    // 设置可执行权限
    #[cfg(unix)]
    {
        if target_file.exists() {
            use std::os::unix::fs::PermissionsExt;
            match fs::metadata(&target_file) {
                Ok(metadata) => {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o755);
                    match fs::set_permissions(&target_file, perms) {
                        Ok(_) => println!("成功设置可执行权限"),
                        Err(e) => println!("设置可执行权限失败: {}", e),
                    }
                }
                Err(e) => println!("获取文件元数据失败: {}", e),
            }

            // 使用chmod命令设置权限（备用方法）
            let _ = Command::new("chmod")
                .arg("+x")
                .arg(target_file.to_str().unwrap())
                .output();
        } else {
            println!("警告：目标文件不存在，无法设置权限: {:?}", target_file);
        }
    }

    // 检查文件是否已成功复制
    if target_file.exists() {
        println!("确认文件已成功复制到: {:?}", target_file);
        println!(
            "文件大小: {:?} 字节",
            fs::metadata(&target_file).map(|m| m.len()).unwrap_or(0)
        );
    } else {
        println!("错误: 复制操作后文件仍不存在: {:?}", target_file);
    }

    Ok(())
}

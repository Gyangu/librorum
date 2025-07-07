use crate::simple_data_portal_client::ProgressInfo;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 终端进度条显示
pub struct ProgressBar {
    /// 上次显示的行数，用于清除之前的输出
    last_lines: Arc<Mutex<usize>>,
    /// 是否已完成
    finished: Arc<Mutex<bool>>,
}

impl ProgressBar {
    pub fn new() -> Self {
        Self {
            last_lines: Arc::new(Mutex::new(0)),
            finished: Arc::new(Mutex::new(false)),
        }
    }

    /// 创建进度回调函数
    pub fn create_callback(&self) -> Box<dyn Fn(ProgressInfo) + Send + Sync> {
        let last_lines = Arc::clone(&self.last_lines);
        let finished = Arc::clone(&self.finished);

        Box::new(move |progress: ProgressInfo| {
            // 检查是否已完成
            {
                let finished_guard = finished.lock().unwrap();
                if *finished_guard {
                    return;
                }
            }

            Self::display_progress(&progress, &last_lines);

            // 如果传输完成，标记为完成
            if progress.percentage >= 1.0 {
                let mut finished_guard = finished.lock().unwrap();
                *finished_guard = true;
            }
        })
    }

    /// 显示进度信息
    fn display_progress(progress: &ProgressInfo, last_lines: &Arc<Mutex<usize>>) {
        let mut stdout = io::stdout();
        
        // 清除之前的输出
        {
            let last_count = last_lines.lock().unwrap();
            for _ in 0..*last_count {
                print!("\x1B[1A\x1B[2K"); // 上移一行并清除
            }
        }

        // 创建进度条
        let bar_width = 40;
        let filled_width = (progress.percentage * bar_width as f64) as usize;
        let empty_width = bar_width - filled_width;
        
        let progress_bar = format!(
            "[{}{}]",
            "█".repeat(filled_width),
            "░".repeat(empty_width)
        );

        // 格式化显示信息
        let percentage = (progress.percentage * 100.0) as u8;
        let bytes_str = ProgressInfo::format_bytes(progress.bytes_transferred);
        let total_str = ProgressInfo::format_bytes(progress.total_bytes);
        let speed_str = format!("{:.1} MB/s", progress.average_speed_mbps);
        let elapsed_str = ProgressInfo::format_duration(progress.elapsed);
        
        let remaining_str = if let Some(remaining) = progress.estimated_remaining {
            format!(" | 剩余: {}", ProgressInfo::format_duration(remaining))
        } else {
            String::new()
        };

        // 根据传输状态选择图标
        let icon = if progress.percentage >= 1.0 {
            "✅"
        } else if progress.bytes_transferred > 0 {
            "🔄"
        } else {
            "⏳"
        };

        // 第一行：进度条和百分比
        println!(
            "{} {} {}%",
            icon,
            progress_bar,
            percentage
        );

        // 第二行：传输统计
        println!(
            "📊 {} / {} @ {} | 用时: {}{}",
            bytes_str,
            total_str,
            speed_str,
            elapsed_str,
            remaining_str
        );

        // 刷新输出
        let _ = stdout.flush();

        // 更新行数计数
        {
            let mut last_count = last_lines.lock().unwrap();
            *last_count = 2; // 我们显示了2行
        }
    }

    /// 完成进度显示
    pub fn finish(&self) {
        let mut finished_guard = self.finished.lock().unwrap();
        *finished_guard = true;
        
        // 移动到新行，避免覆盖最终的进度显示
        println!();
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        self.finish();
    }
}

/// 上传进度显示器
pub struct UploadProgressDisplay {
    progress_bar: ProgressBar,
}

impl UploadProgressDisplay {
    pub fn new() -> Self {
        println!("📤 开始上传...");
        Self {
            progress_bar: ProgressBar::new(),
        }
    }

    pub fn create_callback(&self) -> Box<dyn Fn(ProgressInfo) + Send + Sync> {
        self.progress_bar.create_callback()
    }

    pub fn finish(&self, result: &crate::simple_data_portal_client::TransferResult) {
        self.progress_bar.finish();
        println!("✅ 上传完成!");
        println!(
            "📊 传输统计: {} 字节, {:.2} 秒, {:.2} MB/s",
            result.bytes_transferred,
            result.duration.as_secs_f64(),
            result.throughput_mbps
        );
        
        // 显示完整性验证结果
        if result.integrity_verified {
            println!("🔒 完整性验证: ✅ 成功");
        } else {
            println!("🔒 完整性验证: ❌ 失败");
        }
        
        if let Some(ref message) = result.verification_message {
            println!("   详情: {}", message);
        }
        
        if let Some(ref hash) = result.file_hash {
            println!("   文件哈希: {}", hash);
        }
    }
}

/// 下载进度显示器
pub struct DownloadProgressDisplay {
    progress_bar: ProgressBar,
}

impl DownloadProgressDisplay {
    pub fn new() -> Self {
        println!("📥 开始下载...");
        Self {
            progress_bar: ProgressBar::new(),
        }
    }

    pub fn create_callback(&self) -> Box<dyn Fn(ProgressInfo) + Send + Sync> {
        self.progress_bar.create_callback()
    }

    pub fn finish(&self, result: &crate::simple_data_portal_client::TransferResult) {
        self.progress_bar.finish();
        println!("✅ 下载完成!");
        println!(
            "📊 传输统计: {} 字节, {:.2} 秒, {:.2} MB/s",
            result.bytes_transferred,
            result.duration.as_secs_f64(),
            result.throughput_mbps
        );
        
        // 显示完整性验证结果
        if result.integrity_verified {
            println!("🔒 完整性验证: ✅ 成功");
        } else {
            println!("🔒 完整性验证: ❌ 失败");
        }
        
        if let Some(ref message) = result.verification_message {
            println!("   详情: {}", message);
        }
        
        if let Some(ref hash) = result.file_hash {
            println!("   文件哈希: {}", hash);
        }
    }
}
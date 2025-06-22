use std::net::TcpListener;

/// 测试工具集合
pub struct TestUtils;

impl TestUtils {
    /// 找到可用的端口
    pub fn find_available_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }
}

/// 性能测试工具
pub struct PerformanceMonitor {
    start_time: std::time::Instant,
    measurements: Vec<std::time::Duration>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            measurements: Vec::new(),
        }
    }

    pub fn start_measurement(&mut self) {
        self.start_time = std::time::Instant::now();
    }

    pub fn end_measurement(&mut self) {
        let duration = self.start_time.elapsed();
        self.measurements.push(duration);
    }

    pub fn get_average_duration(&self) -> std::time::Duration {
        if self.measurements.is_empty() {
            return std::time::Duration::from_millis(0);
        }

        let total: std::time::Duration = self.measurements.iter().sum();
        total / self.measurements.len() as u32
    }

    pub fn print_stats(&self) {
        if self.measurements.is_empty() {
            println!("没有性能测量数据");
            return;
        }

        println!("性能统计:");
        println!("  测量次数: {}", self.measurements.len());
        println!("  平均时间: {:?}", self.get_average_duration());
        if let Some(min) = self.measurements.iter().min() {
            println!("  最小时间: {:?}", min);
        }
        if let Some(max) = self.measurements.iter().max() {
            println!("  最大时间: {:?}", max);
        }
    }
}
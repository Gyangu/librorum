use crate::proto::log::{
    log_service_server::LogService,
    ClearLogsRequest, ClearLogsResponse,
    ExportFormat, ExportLogsRequest, ExportLogsResponse,
    GetLogStatsRequest, GetLogStatsResponse,
    GetLogsRequest, GetLogsResponse,
    LogEntry, LogLevel, LogTrend,
    StreamLogsRequest,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, warn};

pub struct LogServiceImpl {
    // 内存中的日志存储
    logs: Arc<Mutex<Vec<LogEntry>>>,
    // 日志计数器
    log_counter: Arc<Mutex<u64>>,
}

impl LogServiceImpl {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            log_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// 添加日志条目（内部使用）
    pub async fn add_log_entry(&self, level: LogLevel, module: &str, message: &str) {
        let mut counter = self.log_counter.lock().await;
        *counter += 1;

        let entry = LogEntry {
            timestamp: chrono::Utc::now().timestamp(),
            level: level.into(),
            module: module.to_string(),
            message: message.to_string(),
            thread_id: format!("{:?}", std::thread::current().id()),
            file: String::new(),
            line: 0,
            fields: HashMap::new(),
        };

        let mut logs = self.logs.lock().await;
        logs.push(entry);

        // 保持最多10000条日志
        if logs.len() > 10000 {
            logs.remove(0);
        }
    }

    /// 初始化一些示例日志
    pub async fn init_sample_logs(&self) {
        let sample_logs = vec![
            (LogLevel::Info, "core", "Backend service started successfully"),
            (LogLevel::Info, "network", "Node discovery completed, found 3 nodes"),
            (LogLevel::Info, "storage", "File upload completed: example.txt"),
            (LogLevel::Debug, "grpc", "Heartbeat received from node: local.librorum.local"),
            (LogLevel::Info, "storage", "Storage usage: 25% (250MB/1GB)"),
            (LogLevel::Debug, "network", "Network latency check: 15ms"),
            (LogLevel::Info, "core", "Configuration reloaded"),
            (LogLevel::Info, "network", "Connection established with node: remote.librorum.local"),
            (LogLevel::Debug, "storage", "Chunk replication completed"),
            (LogLevel::Info, "core", "Health check passed"),
            (LogLevel::Warn, "network", "High latency detected: 150ms"),
            (LogLevel::Error, "storage", "Failed to replicate chunk: timeout"),
            (LogLevel::Debug, "grpc", "Processing file list request"),
            (LogLevel::Info, "storage", "Directory created: /user/documents"),
            (LogLevel::Trace, "core", "Processing internal event queue"),
        ];

        for (level, module, message) in sample_logs {
            self.add_log_entry(level, module, message).await;
            // 添加一些时间间隔
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

#[tonic::async_trait]
impl LogService for LogServiceImpl {
    async fn get_logs(
        &self,
        request: Request<GetLogsRequest>,
    ) -> Result<Response<GetLogsResponse>, Status> {
        let req = request.into_inner();
        debug!("Getting logs with filters: level={}, module={}, search={}", 
               req.level_filter, req.module_filter, req.search_text);

        let logs = self.logs.lock().await;
        
        // 应用过滤器
        let mut filtered_logs: Vec<LogEntry> = logs
            .iter()
            .filter(|log| {
                // 级别过滤
                if !req.level_filter.is_empty() && req.level_filter != "all" {
                    let filter_level = match req.level_filter.as_str() {
                        "trace" => LogLevel::Trace,
                        "debug" => LogLevel::Debug,
                        "info" => LogLevel::Info,
                        "warn" => LogLevel::Warn,
                        "error" => LogLevel::Error,
                        _ => return true,
                    };
                    if LogLevel::try_from(log.level).unwrap_or(LogLevel::Unknown) != filter_level {
                        return false;
                    }
                }

                // 模块过滤
                if !req.module_filter.is_empty() && !log.module.contains(&req.module_filter) {
                    return false;
                }

                // 搜索文本过滤
                if !req.search_text.is_empty() && 
                   !log.message.to_lowercase().contains(&req.search_text.to_lowercase()) {
                    return false;
                }

                // 时间范围过滤
                if req.start_time > 0 && log.timestamp < req.start_time {
                    return false;
                }
                if req.end_time > 0 && log.timestamp > req.end_time {
                    return false;
                }

                true
            })
            .cloned()
            .collect();

        // 排序
        if req.reverse {
            filtered_logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        } else {
            filtered_logs.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }

        // 限制数量
        let total_count = filtered_logs.len() as i32;
        let has_more = if req.limit > 0 && filtered_logs.len() > req.limit as usize {
            filtered_logs.truncate(req.limit as usize);
            true
        } else {
            false
        };

        debug!("Returning {} logs (total: {}, has_more: {})", 
               filtered_logs.len(), total_count, has_more);

        let response = GetLogsResponse {
            logs: filtered_logs,
            total_count,
            has_more,
        };

        Ok(Response::new(response))
    }

    type StreamLogsStream = Pin<Box<dyn Stream<Item = Result<LogEntry, Status>> + Send>>;

    async fn stream_logs(
        &self,
        request: Request<StreamLogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        let req = request.into_inner();
        info!("Starting log streaming with tail={}, follow={}", req.tail, req.follow);

        let logs = Arc::clone(&self.logs);
        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            // 首先发送尾部日志
            if req.tail > 0 {
                let logs_guard = logs.lock().await;
                let tail_logs: Vec<LogEntry> = logs_guard
                    .iter()
                    .rev()
                    .take(req.tail as usize)
                    .cloned()
                    .collect();
                drop(logs_guard);

                for log in tail_logs.into_iter().rev() {
                    if tx.send(Ok(log)).await.is_err() {
                        return;
                    }
                }
            }

            // 如果需要跟踪新日志，保持连接活跃
            if req.follow {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
                let mut last_count = {
                    let logs_guard = logs.lock().await;
                    logs_guard.len()
                };

                loop {
                    interval.tick().await;
                    
                    let logs_guard = logs.lock().await;
                    let current_count = logs_guard.len();
                    
                    if current_count > last_count {
                        // 发送新的日志条目
                        for log in logs_guard.iter().skip(last_count) {
                            if tx.send(Ok(log.clone())).await.is_err() {
                                return;
                            }
                        }
                        last_count = current_count;
                    }
                    drop(logs_guard);
                }
            }
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output_stream)))
    }

    async fn clear_logs(
        &self,
        request: Request<ClearLogsRequest>,
    ) -> Result<Response<ClearLogsResponse>, Status> {
        let req = request.into_inner();
        info!("Clearing logs: clear_all={}, before_timestamp={}", 
              req.clear_all, req.before_timestamp);

        let mut logs = self.logs.lock().await;
        let initial_count = logs.len();

        if req.clear_all {
            logs.clear();
        } else if req.before_timestamp > 0 {
            logs.retain(|log| log.timestamp >= req.before_timestamp);
        }

        let cleared_count = initial_count - logs.len();
        info!("Cleared {} log entries", cleared_count);

        let response = ClearLogsResponse {
            success: true,
            cleared_count: cleared_count as i32,
            message: format!("Successfully cleared {} log entries", cleared_count),
        };

        Ok(Response::new(response))
    }

    async fn export_logs(
        &self,
        request: Request<ExportLogsRequest>,
    ) -> Result<Response<ExportLogsResponse>, Status> {
        let req = request.into_inner();
        debug!("Exporting logs in format: {:?}", req.format);

        let logs = self.logs.lock().await;
        
        // 应用过滤器（简化版本）
        let filtered_logs: Vec<&LogEntry> = logs
            .iter()
            .filter(|log| {
                if !req.level_filter.is_empty() && req.level_filter != "all" {
                    // 简化的级别过滤
                    return log.module.contains(&req.level_filter);
                }
                if !req.module_filter.is_empty() && !log.module.contains(&req.module_filter) {
                    return false;
                }
                if req.start_time > 0 && log.timestamp < req.start_time {
                    return false;
                }
                if req.end_time > 0 && log.timestamp > req.end_time {
                    return false;
                }
                true
            })
            .collect();

        let (data, filename, mime_type) = match ExportFormat::try_from(req.format).unwrap_or(ExportFormat::Json) {
            ExportFormat::Json => {
                // 创建可序列化的结构体
                let serializable_logs: Vec<serde_json::Value> = filtered_logs
                    .iter()
                    .map(|log| {
                        serde_json::json!({
                            "timestamp": log.timestamp,
                            "level": match LogLevel::try_from(log.level).unwrap_or(LogLevel::Unknown) {
                                LogLevel::Trace => "TRACE",
                                LogLevel::Debug => "DEBUG",
                                LogLevel::Info => "INFO", 
                                LogLevel::Warn => "WARN",
                                LogLevel::Error => "ERROR",
                                _ => "UNKNOWN",
                            },
                            "module": log.module,
                            "message": log.message,
                            "thread_id": log.thread_id,
                            "file": log.file,
                            "line": log.line
                        })
                    })
                    .collect();
                
                let json_data = serde_json::to_string_pretty(&serializable_logs)
                    .map_err(|e| Status::internal(format!("JSON serialization failed: {}", e)))?;
                (json_data.into_bytes(), "logs.json".to_string(), "application/json".to_string())
            }
            ExportFormat::Csv => {
                let mut csv_data = "timestamp,level,module,message\n".to_string();
                for log in &filtered_logs {
                    let level_str = match LogLevel::try_from(log.level).unwrap_or(LogLevel::Unknown) {
                        LogLevel::Trace => "TRACE",
                        LogLevel::Debug => "DEBUG",
                        LogLevel::Info => "INFO",
                        LogLevel::Warn => "WARN",
                        LogLevel::Error => "ERROR",
                        _ => "UNKNOWN",
                    };
                    csv_data.push_str(&format!("{},{},{},\"{}\"\n", 
                                               log.timestamp, level_str, log.module, 
                                               log.message.replace("\"", "\"\"")));
                }
                (csv_data.into_bytes(), "logs.csv".to_string(), "text/csv".to_string())
            }
            ExportFormat::Plain => {
                let mut plain_data = String::new();
                for log in &filtered_logs {
                    let level_str = match LogLevel::try_from(log.level).unwrap_or(LogLevel::Unknown) {
                        LogLevel::Trace => "TRACE",
                        LogLevel::Debug => "DEBUG", 
                        LogLevel::Info => "INFO",
                        LogLevel::Warn => "WARN",
                        LogLevel::Error => "ERROR",
                        _ => "UNKNOWN",
                    };
                    let datetime = chrono::DateTime::from_timestamp(log.timestamp, 0)
                        .unwrap_or_else(|| chrono::Utc::now())
                        .format("%Y-%m-%d %H:%M:%S UTC");
                    plain_data.push_str(&format!("[{}] {} [{}] {}\n", 
                                                  datetime, level_str, log.module, log.message));
                }
                (plain_data.into_bytes(), "logs.txt".to_string(), "text/plain".to_string())
            }
            _ => {
                return Err(Status::invalid_argument("Unsupported export format"));
            }
        };

        let file_size = data.len() as i64;
        
        info!("Exported {} log entries in {} format", filtered_logs.len(), filename);

        let response = ExportLogsResponse {
            success: true,
            data,
            filename,
            mime_type,
            log_count: filtered_logs.len() as i32,
            file_size,
        };

        Ok(Response::new(response))
    }

    async fn get_log_stats(
        &self,
        request: Request<GetLogStatsRequest>,
    ) -> Result<Response<GetLogStatsResponse>, Status> {
        let req = request.into_inner();
        debug!("Getting log statistics for time range: {} - {}", req.start_time, req.end_time);

        let logs = self.logs.lock().await;
        
        // 过滤时间范围内的日志
        let filtered_logs: Vec<&LogEntry> = logs
            .iter()
            .filter(|log| {
                if req.start_time > 0 && log.timestamp < req.start_time {
                    return false;
                }
                if req.end_time > 0 && log.timestamp > req.end_time {
                    return false;
                }
                true
            })
            .collect();

        let total_logs = filtered_logs.len() as i64;

        // 统计各级别日志数量
        let mut level_counts = HashMap::new();
        let mut module_counts = HashMap::new();
        let mut error_count = 0i64;
        let mut warn_count = 0i64;

        for log in &filtered_logs {
            let level = LogLevel::try_from(log.level).unwrap_or(LogLevel::Unknown);
            let level_str = match level {
                LogLevel::Trace => "trace",
                LogLevel::Debug => "debug", 
                LogLevel::Info => "info",
                LogLevel::Warn => { warn_count += 1; "warn" },
                LogLevel::Error => { error_count += 1; "error" },
                _ => "unknown",
            };
            
            *level_counts.entry(level_str.to_string()).or_insert(0) += 1;
            *module_counts.entry(log.module.clone()).or_insert(0) += 1;
        }

        // 生成简单的趋势数据（每小时的日志数量）
        let mut trends = Vec::new();
        if !filtered_logs.is_empty() {
            let start_time = filtered_logs.iter().map(|l| l.timestamp).min().unwrap_or(0);
            let end_time = filtered_logs.iter().map(|l| l.timestamp).max().unwrap_or(0);
            let hour_interval = 3600; // 1小时

            let mut current_time = start_time;
            while current_time <= end_time {
                let hour_end = current_time + hour_interval;
                let hour_logs: Vec<&LogEntry> = filtered_logs
                    .iter()
                    .filter(|log| log.timestamp >= current_time && log.timestamp < hour_end)
                    .cloned()
                    .collect();

                let hour_error_count = hour_logs
                    .iter()
                    .filter(|log| matches!(LogLevel::try_from(log.level), Ok(LogLevel::Error)))
                    .count() as i64;

                let hour_warn_count = hour_logs
                    .iter()
                    .filter(|log| matches!(LogLevel::try_from(log.level), Ok(LogLevel::Warn)))
                    .count() as i64;

                trends.push(LogTrend {
                    timestamp: current_time,
                    log_count: hour_logs.len() as i64,
                    error_count: hour_error_count,
                    warn_count: hour_warn_count,
                });

                current_time = hour_end;
            }
        }

        debug!("Log statistics: total={}, errors={}, warnings={}", 
               total_logs, error_count, warn_count);

        let response = GetLogStatsResponse {
            total_logs,
            level_counts,
            module_counts,
            error_count,
            warn_count,
            trends,
        };

        Ok(Response::new(response))
    }
}
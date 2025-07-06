//! Rust service for Swift-Rust interoperability demonstration
//!
//! This example demonstrates high-performance communication between
//! Swift and Rust using the Universal Transport Protocol.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use universal_transport_core::{NodeInfo, Language, TransportManager, TransportStrategy};
use universal_transport_shared_memory::{SharedMemoryTransport, SharedMemoryConfiguration};

// MARK: - Data Structures (matching Swift)

/// Data processing request from Swift
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataProcessingRequest {
    operation: String,
    input_data: Vec<f64>,
    parameters: std::collections::HashMap<String, String>,
    timestamp: f64,
    request_id: String,
}

/// Data processing response to Swift
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataProcessingResponse {
    request_id: String,
    result: Vec<f64>,
    processing_time: f64,
    status: String,
    metadata: std::collections::HashMap<String, String>,
}

impl DataProcessingResponse {
    fn new(request_id: String, result: Vec<f64>, processing_time: f64) -> Self {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("rust_version".to_string(), "1.75".to_string());
        metadata.insert("processed_at".to_string(), chrono::Utc::now().to_rfc3339());
        
        Self {
            request_id,
            result,
            processing_time,
            status: "success".to_string(),
            metadata,
        }
    }
    
    fn error(request_id: String, error_msg: String, processing_time: f64) -> Self {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("error".to_string(), error_msg);
        metadata.insert("rust_version".to_string(), "1.75".to_string());
        
        Self {
            request_id,
            result: Vec::new(),
            processing_time,
            status: "error".to_string(),
            metadata,
        }
    }
}

// MARK: - Rust Data Processor

struct RustDataProcessor {
    transport: SharedMemoryTransport,
    rust_node: NodeInfo,
    swift_node: NodeInfo,
}

impl RustDataProcessor {
    async fn new() -> Result<Self> {
        let config = SharedMemoryConfiguration {
            default_region_size: 64 * 1024 * 1024, // 64MB
            max_regions: 32,
            enable_metrics: true,
            default_timeout: Duration::from_secs(30),
        };
        
        let transport = SharedMemoryTransport::new(config);
        
        let rust_node = NodeInfo::new("rust-service".to_string(), Language::Rust);
        let swift_node = NodeInfo::new("swift-service".to_string(), Language::Swift);
        
        info!("Rust data processor initialized");
        
        Ok(Self {
            transport,
            rust_node,
            swift_node,
        })
    }
    
    /// Process data request with Rust-optimized algorithms
    async fn process_request(&self, request: &DataProcessingRequest) -> DataProcessingResponse {
        let start_time = Instant::now();
        info!("Processing request: {} with {} data points", 
              request.operation, request.input_data.len());
        
        let result = match request.operation.as_str() {
            "rust_fft" => self.process_fft(&request.input_data, &request.parameters),
            "rust_filter" => self.process_filter(&request.input_data, &request.parameters),
            "rust_statistics" => self.process_statistics(&request.input_data),
            "rust_convolution" => self.process_convolution(&request.input_data, &request.parameters),
            "rust_matrix_multiply" => self.process_matrix_multiply(&request.input_data, &request.parameters),
            "rust_parallel_sum" => self.process_parallel_sum(&request.input_data),
            _ => {
                warn!("Unknown operation: {}, echoing back data", request.operation);
                request.input_data.clone()
            }
        };
        
        let processing_time = start_time.elapsed().as_secs_f64();
        info!("Processed {} in {:.3}s, result size: {}", 
              request.operation, processing_time, result.len());
        
        DataProcessingResponse::new(
            request.request_id.clone(),
            result,
            processing_time,
        )
    }
    
    /// Simulated FFT processing (in real implementation, use rustfft crate)
    fn process_fft(&self, data: &[f64], parameters: &std::collections::HashMap<String, String>) -> Vec<f64> {
        let window_type = parameters.get("window").unwrap_or(&"hamming".to_string());
        let size = parameters.get("size")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(data.len());
        
        debug!("FFT processing with window: {}, size: {}", window_type, size);
        
        // Simulate FFT with windowing
        let mut result = Vec::with_capacity(size);
        for i in 0..size.min(data.len()) {
            let window_factor = match window_type.as_str() {
                "hamming" => 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (size - 1) as f64).cos(),
                "hanning" => 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (size - 1) as f64).cos()),
                _ => 1.0, // rectangular
            };
            
            // Simulate complex FFT result (magnitude)
            let real = data[i] * window_factor * (2.0 * std::f64::consts::PI * i as f64 / size as f64).cos();
            let imag = data[i] * window_factor * (2.0 * std::f64::consts::PI * i as f64 / size as f64).sin();
            result.push((real * real + imag * imag).sqrt());
        }
        
        result
    }
    
    /// Advanced filtering
    fn process_filter(&self, data: &[f64], parameters: &std::collections::HashMap<String, String>) -> Vec<f64> {
        let filter_type = parameters.get("type").unwrap_or(&"lowpass".to_string());
        let cutoff = parameters.get("cutoff")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.1);
        
        debug!("Filtering with type: {}, cutoff: {}", filter_type, cutoff);
        
        match filter_type.as_str() {
            "lowpass" => {
                // Simple moving average lowpass filter
                let window_size = (1.0 / cutoff) as usize;
                data.windows(window_size)
                    .map(|window| window.iter().sum::<f64>() / window.len() as f64)
                    .collect()
            }
            "highpass" => {
                // Simple highpass filter (data - lowpass)
                let lowpass = self.process_filter(data, &{
                    let mut params = parameters.clone();
                    params.insert("type".to_string(), "lowpass".to_string());
                    params
                });
                data.iter().zip(lowpass.iter()).map(|(d, l)| d - l).collect()
            }
            _ => data.to_vec(),
        }
    }
    
    /// Statistical analysis
    fn process_statistics(&self, data: &[f64]) -> Vec<f64> {
        if data.is_empty() {
            return Vec::new();
        }
        
        let sum: f64 = data.iter().sum();
        let mean = sum / data.len() as f64;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;
        let std_dev = variance.sqrt();
        let min = data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        // Return [count, sum, mean, variance, std_dev, min, max]
        vec![data.len() as f64, sum, mean, variance, std_dev, min, max]
    }
    
    /// Convolution processing
    fn process_convolution(&self, data: &[f64], parameters: &std::collections::HashMap<String, String>) -> Vec<f64> {
        let kernel_type = parameters.get("kernel").unwrap_or(&"gaussian".to_string());
        let kernel_size = parameters.get("size")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(5);
        
        // Generate kernel
        let kernel = match kernel_type.as_str() {
            "gaussian" => {
                let sigma = 1.0;
                let center = kernel_size as f64 / 2.0;
                (0..kernel_size)
                    .map(|i| {
                        let x = i as f64 - center;
                        (-0.5 * (x / sigma).powi(2)).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
                    })
                    .collect::<Vec<_>>()
            }
            "edge" => vec![-1.0, 0.0, 1.0], // Simple edge detection
            _ => vec![1.0; kernel_size], // Box filter
        };
        
        // Apply convolution
        let mut result = Vec::new();
        for i in 0..data.len() {
            let mut sum = 0.0;
            for (j, &k) in kernel.iter().enumerate() {
                let idx = i as i32 + j as i32 - kernel_size as i32 / 2;
                if idx >= 0 && (idx as usize) < data.len() {
                    sum += data[idx as usize] * k;
                }
            }
            result.push(sum);
        }
        
        result
    }
    
    /// Matrix multiplication simulation
    fn process_matrix_multiply(&self, data: &[f64], parameters: &std::collections::HashMap<String, String>) -> Vec<f64> {
        let rows = parameters.get("rows")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or((data.len() as f64).sqrt() as usize);
        let cols = data.len() / rows;
        
        if rows * cols != data.len() {
            warn!("Data size {} doesn't match matrix dimensions {}x{}", data.len(), rows, cols);
            return data.to_vec();
        }
        
        // Multiply matrix by its transpose (A * A^T)
        let mut result = vec![0.0; rows * rows];
        for i in 0..rows {
            for j in 0..rows {
                let mut sum = 0.0;
                for k in 0..cols {
                    sum += data[i * cols + k] * data[j * cols + k];
                }
                result[i * rows + j] = sum;
            }
        }
        
        result
    }
    
    /// Parallel sum using rayon
    fn process_parallel_sum(&self, data: &[f64]) -> Vec<f64> {
        use rayon::prelude::*;
        
        // Parallel chunk sums
        let chunk_size = (data.len() / num_cpus::get()).max(1);
        let chunk_sums: Vec<f64> = data
            .par_chunks(chunk_size)
            .map(|chunk| chunk.iter().sum())
            .collect();
        
        vec![chunk_sums.iter().sum(), chunk_sums.len() as f64]
    }
    
    /// Start listening for requests from Swift
    async fn start_listening(&mut self, region_name: &str) -> Result<()> {
        info!("Starting Rust service listener on region: {}", region_name);
        
        // Create or get shared memory region
        self.transport.get_or_create_region(region_name.to_string(), 64 * 1024 * 1024).await
            .context("Failed to create shared memory region")?;
        
        loop {
            match timeout(
                Duration::from_secs(30),
                self.transport.receive::<DataProcessingRequest>(region_name.to_string())
            ).await {
                Ok(Ok(request)) => {
                    info!("Received request: {}", request.request_id);
                    
                    let response = self.process_request(&request).await;
                    
                    match self.transport.send(response, region_name.to_string()).await {
                        Ok(_) => {
                            info!("Sent response for request: {}", request.request_id);
                        }
                        Err(e) => {
                            error!("Failed to send response: {}", e);
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("Error receiving request: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(_) => {
                    debug!("Receive timeout, continuing to listen...");
                }
            }
        }
    }
    
    /// Send test request to Swift
    async fn send_test_request(&mut self, region_name: &str) -> Result<()> {
        info!("Sending test request to Swift");
        
        let test_data: Vec<f64> = (0..1000).map(|i| (i as f64) * 0.01).collect();
        let request = DataProcessingRequest {
            operation: "swift_fft".to_string(),
            input_data: test_data,
            parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert("window".to_string(), "hanning".to_string());
                params.insert("size".to_string(), "1024".to_string());
                params
            },
            timestamp: chrono::Utc::now().timestamp() as f64,
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        
        // Send request
        self.transport.send(request.clone(), region_name.to_string()).await
            .context("Failed to send request to Swift")?;
        
        // Wait for response
        let response = timeout(
            Duration::from_secs(60),
            self.transport.receive::<DataProcessingResponse>(region_name.to_string())
        ).await
        .context("Timeout waiting for Swift response")?
        .context("Failed to receive response from Swift")?;
        
        info!("Received response from Swift: {}, processing time: {:.3}s", 
              response.request_id, response.processing_time);
        
        println!("✅ Rust → Swift communication successful!");
        println!("   Request ID: {}", response.request_id);
        println!("   Result size: {}", response.result.len());
        println!("   Processing time: {:.3}s", response.processing_time);
        println!("   Status: {}", response.status);
        
        Ok(())
    }
    
    /// Get performance metrics
    async fn get_performance_metrics(&self) -> Result<()> {
        let metrics = self.transport.get_performance_metrics().await;
        
        println!("\n📊 Rust Performance Metrics:");
        println!("   Total regions: {}", metrics.total_regions);
        println!("   Active regions: {}", metrics.active_regions);
        println!("   Total messages sent: {}", metrics.total_messages_sent);
        println!("   Total messages received: {}", metrics.total_messages_received);
        println!("   Average latency: {:.3}s", metrics.average_latency);
        println!("   Total throughput: {:.1} MB/s", metrics.total_throughput / 1024.0 / 1024.0);
        
        Ok(())
    }
}

// MARK: - Example Runner

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    
    println!("🦀 Universal Transport Rust Service");
    println!("===================================");
    
    let mut processor = RustDataProcessor::new().await
        .context("Failed to initialize Rust data processor")?;
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("listen");
    
    match mode {
        "listen" => {
            println!("🎧 Starting Rust service in listening mode...");
            processor.start_listening("swift-rust-bridge").await?;
        }
        "send" => {
            println!("📤 Sending test request to Swift service...");
            processor.send_test_request("swift-rust-bridge").await?;
            processor.get_performance_metrics().await?;
        }
        "benchmark" => {
            println!("⚡ Running Rust performance benchmark...");
            run_rust_benchmark(&processor).await?;
        }
        _ => {
            println!("🔄 Running interactive demo...");
            run_interactive_demo(&mut processor).await?;
        }
    }
    
    Ok(())
}

/// Run Rust-specific benchmark
async fn run_rust_benchmark(processor: &RustDataProcessor) -> Result<()> {
    println!("⚡ Running Rust performance benchmark...");
    
    let test_sizes = vec![1000, 10000, 100000];
    let operations = vec!["rust_fft", "rust_filter", "rust_statistics", "rust_convolution"];
    
    for &size in &test_sizes {
        println!("\nTesting with {} data points:", size);
        
        let test_data: Vec<f64> = (0..size).map(|i| (i as f64).sin()).collect();
        
        for operation in &operations {
            let request = DataProcessingRequest {
                operation: operation.clone(),
                input_data: test_data.clone(),
                parameters: {
                    let mut params = std::collections::HashMap::new();
                    params.insert("size".to_string(), "1024".to_string());
                    params.insert("cutoff".to_string(), "0.1".to_string());
                    params
                },
                timestamp: chrono::Utc::now().timestamp() as f64,
                request_id: uuid::Uuid::new_v4().to_string(),
            };
            
            let response = processor.process_request(&request).await;
            let throughput = size as f64 / response.processing_time;
            
            println!("   📊 {}: {:.3}s, {:.0} items/s", 
                     operation, response.processing_time, throughput);
        }
    }
    
    Ok(())
}

/// Run interactive demo
async fn run_interactive_demo(processor: &mut RustDataProcessor) -> Result<()> {
    println!("\n📋 Rust Service Commands:");
    println!("   1. Test local Rust processing");
    println!("   2. Send request to Swift (if running)");
    println!("   3. Show performance metrics");
    println!("   4. Start listening for Swift requests");
    println!("   5. Exit");
    
    loop {
        print!("\nEnter command (1-5): ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            match input.trim().parse::<u32>() {
                Ok(1) => test_local_processing(processor).await,
                Ok(2) => {
                    if let Err(e) = processor.send_test_request("swift-rust-bridge").await {
                        println!("❌ Swift communication failed: {}", e);
                        println!("💡 Make sure the Swift service is running");
                    }
                }
                Ok(3) => {
                    if let Err(e) = processor.get_performance_metrics().await {
                        println!("❌ Failed to get metrics: {}", e);
                    }
                }
                Ok(4) => {
                    println!("🎧 Starting listener... (Ctrl+C to stop)");
                    if let Err(e) = processor.start_listening("swift-rust-bridge").await {
                        println!("❌ Listener failed: {}", e);
                    }
                }
                Ok(5) => {
                    println!("👋 Goodbye!");
                    break;
                }
                _ => println!("❌ Invalid choice. Please enter 1-5."),
            }
        }
    }
    
    Ok(())
}

/// Test local Rust processing
async fn test_local_processing(processor: &RustDataProcessor) {
    println!("\n🧮 Testing local Rust processing...");
    
    let test_data: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin()).collect();
    let operations = vec![
        ("rust_fft", "FFT analysis"),
        ("rust_filter", "Lowpass filtering"),
        ("rust_statistics", "Statistical analysis"),
        ("rust_convolution", "Gaussian convolution"),
        ("rust_parallel_sum", "Parallel summation"),
    ];
    
    for (operation, description) in operations {
        let request = DataProcessingRequest {
            operation: operation.to_string(),
            input_data: test_data.clone(),
            parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert("size".to_string(), "512".to_string());
                params.insert("cutoff".to_string(), "0.1".to_string());
                params.insert("kernel".to_string(), "gaussian".to_string());
                params
            },
            timestamp: chrono::Utc::now().timestamp() as f64,
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        
        let response = processor.process_request(&request).await;
        println!("   ✅ {} ({}): {} results in {:.3}s", 
                 description, operation, response.result.len(), response.processing_time);
    }
}
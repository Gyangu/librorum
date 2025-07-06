//! Universal Transport Protocol - Performance Analysis
//! 
//! This program provides realistic performance measurements for different
//! communication methods to establish accurate benchmarks.

mod simple_test;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_test::run_performance_comparison().await
}
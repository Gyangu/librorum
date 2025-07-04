use bytes::Bytes;
use clap::Parser;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(name = "grpc_baseline")]
#[command(about = "gRPC baseline performance test")]
struct Args {
    #[arg(long, default_value = "1048576")]
    message_size: usize,
    
    #[arg(long, default_value = "10000")]
    message_count: usize,
    
    #[arg(long, default_value = "server")]
    mode: String, // server or client
    
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    
    #[arg(long, default_value = "50051")]
    port: u16,
}

// Simple gRPC-like benchmark using tokio TCP
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("gRPC Baseline Performance Test");
    println!("Message Size: {} bytes", args.message_size);
    println!("Message Count: {}", args.message_count);
    println!("Mode: {}", args.mode);
    
    match args.mode.as_str() {
        "server" => run_server(&args).await,
        "client" => run_client(&args).await,
        _ => {
            eprintln!("Invalid mode. Use 'server' or 'client'");
            std::process::exit(1);
        }
    }
}

async fn run_server(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::TcpListener;
    use tokio::io::AsyncReadExt;
    
    println!("Starting gRPC baseline server...");
    
    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;
    println!("Server listening on {}:{}", args.host, args.port);
    
    let (mut socket, _) = listener.accept().await?;
    println!("Client connected");
    
    let mut received_count = 0;
    let mut total_bytes = 0;
    let start_time = Instant::now();
    let mut buffer = vec![0u8; args.message_size];
    
    while received_count < args.message_count {
        let bytes_read = socket.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        
        received_count += 1;
        total_bytes += bytes_read;
        
        if received_count % 1000 == 0 {
            println!("Received {} messages, {} bytes", received_count, total_bytes);
        }
    }
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== gRPC Baseline Server Results ===");
    println!("Total messages: {}", received_count);
    println!("Total bytes: {}", total_bytes);
    println!("Total time: {:?}", total_time);
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}

async fn run_client(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::TcpStream;
    use tokio::io::AsyncWriteExt;
    
    println!("Starting gRPC baseline client...");
    
    let mut stream = TcpStream::connect(format!("{}:{}", args.host, args.port)).await?;
    println!("Connected to server");
    
    // Create test data
    let test_data = vec![0u8; args.message_size];
    
    let start_time = Instant::now();
    let mut sent_count = 0;
    let mut total_bytes = 0;
    
    for i in 0..args.message_count {
        stream.write_all(&test_data).await?;
        sent_count += 1;
        total_bytes += test_data.len();
        
        if i % 1000 == 0 {
            println!("Sent {} messages", i);
        }
    }
    
    stream.flush().await?;
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== gRPC Baseline Client Results ===");
    println!("Total messages: {}", sent_count);
    println!("Total bytes: {}", total_bytes);
    println!("Total time: {:?}", total_time);
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}
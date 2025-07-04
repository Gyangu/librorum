use bytes::Bytes;
use clap::Parser;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(name = "simple_test")]
#[command(about = "Simple communication performance test")]
struct Args {
    #[arg(long, default_value = "1048576")]
    message_size: usize,
    
    #[arg(long, default_value = "10000")]
    message_count: usize,
    
    #[arg(long, default_value = "tcp")]
    protocol: String, // tcp, udp, ipc
    
    #[arg(long, default_value = "server")]
    mode: String, // server or client
    
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    
    #[arg(long, default_value = "50051")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("Simple Communication Performance Test");
    println!("Protocol: {}", args.protocol);
    println!("Message Size: {} bytes", args.message_size);
    println!("Message Count: {}", args.message_count);
    println!("Mode: {}", args.mode);
    
    match args.protocol.as_str() {
        "tcp" => match args.mode.as_str() {
            "server" => run_tcp_server(&args).await,
            "client" => run_tcp_client(&args).await,
            _ => Err("Invalid mode".into()),
        },
        "udp" => match args.mode.as_str() {
            "server" => run_udp_server(&args).await,
            "client" => run_udp_client(&args).await,
            _ => Err("Invalid mode".into()),
        },
        "ipc" => run_ipc_test(&args).await,
        _ => Err("Invalid protocol".into()),
    }
}

async fn run_tcp_server(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::TcpListener;
    use tokio::io::AsyncReadExt;
    
    println!("Starting TCP server on {}:{}...", args.host, args.port);
    
    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;
    let (mut socket, addr) = listener.accept().await?;
    println!("Client connected from: {}", addr);
    
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
    
    println!("\n=== TCP Server Results ===");
    println!("Total messages: {}", received_count);
    println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}

async fn run_tcp_client(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::TcpStream;
    use tokio::io::AsyncWriteExt;
    
    println!("Starting TCP client, connecting to {}:{}...", args.host, args.port);
    
    let mut stream = TcpStream::connect(format!("{}:{}", args.host, args.port)).await?;
    println!("Connected to server");
    
    // Create test data
    let test_data = vec![0u8; args.message_size];
    
    sleep(Duration::from_millis(500)).await; // Give server time to prepare
    
    let start_time = Instant::now();
    let mut sent_count = 0;
    let mut total_bytes = 0;
    
    for i in 0..args.message_count {
        stream.write_all(&test_data).await?;
        sent_count += 1;
        total_bytes += test_data.len();
        
        if i % 1000 == 0 {
            println!("Sent {} messages", i + 1);
        }
    }
    
    stream.flush().await?;
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== TCP Client Results ===");
    println!("Total messages: {}", sent_count);
    println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}

async fn run_udp_server(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::UdpSocket;
    
    println!("Starting UDP server on {}:{}...", args.host, args.port);
    
    let socket = UdpSocket::bind(format!("{}:{}", args.host, args.port)).await?;
    println!("UDP server listening...");
    
    let mut received_count = 0;
    let mut total_bytes = 0;
    let start_time = Instant::now();
    let mut buffer = vec![0u8; args.message_size];
    
    while received_count < args.message_count {
        let (bytes_read, addr) = socket.recv_from(&mut buffer).await?;
        
        if received_count == 0 {
            println!("First message from: {}", addr);
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
    
    println!("\n=== UDP Server Results ===");
    println!("Total messages: {}", received_count);
    println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}

async fn run_udp_client(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::UdpSocket;
    
    println!("Starting UDP client, connecting to {}:{}...", args.host, args.port);
    
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(format!("{}:{}", args.host, args.port)).await?;
    println!("Connected to server");
    
    // Create test data
    let test_data = vec![0u8; args.message_size];
    
    sleep(Duration::from_millis(500)).await; // Give server time to prepare
    
    let start_time = Instant::now();
    let mut sent_count = 0;
    let mut total_bytes = 0;
    
    for i in 0..args.message_count {
        socket.send(&test_data).await?;
        sent_count += 1;
        total_bytes += test_data.len();
        
        if i % 1000 == 0 {
            println!("Sent {} messages", i + 1);
        }
        
        // Small delay to prevent overwhelming the network
        if i % 100 == 0 {
            tokio::task::yield_now().await;
        }
    }
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== UDP Client Results ===");
    println!("Total messages: {}", sent_count);
    println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}

async fn run_ipc_test(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write, Seek, SeekFrom};
    use std::path::Path;
    
    println!("Starting IPC test (shared memory simulation)...");
    
    let shared_file = "/tmp/librorum_ipc_test.dat";
    
    match args.mode.as_str() {
        "server" => {
            // Server reads from shared memory
            println!("IPC Server: waiting for data...");
            
            let mut received_count = 0;
            let mut total_bytes = 0;
            let start_time = Instant::now();
            
            while received_count < args.message_count {
                if Path::new(shared_file).exists() {
                    let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(shared_file)?;
                    
                    let mut buffer = vec![0u8; args.message_size];
                    if let Ok(bytes_read) = file.read(&mut buffer) {
                        if bytes_read > 0 {
                            received_count += 1;
                            total_bytes += bytes_read;
                            
                            // Clear the file to signal consumption
                            file.seek(SeekFrom::Start(0))?;
                            file.set_len(0)?;
                            
                            if received_count % 1000 == 0 {
                                println!("Received {} messages, {} bytes", received_count, total_bytes);
                            }
                        }
                    }
                }
                
                tokio::task::yield_now().await;
            }
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== IPC Server Results ===");
            println!("Total messages: {}", received_count);
            println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
            println!("Total time: {:.2}s", total_time.as_secs_f64());
            println!("Throughput: {:.2} MB/s", throughput_mbps);
            println!("Messages/sec: {:.2}", messages_per_sec);
        },
        "client" => {
            // Client writes to shared memory
            println!("IPC Client: sending data...");
            
            let test_data = vec![0u8; args.message_size];
            let start_time = Instant::now();
            let mut sent_count = 0;
            let mut total_bytes = 0;
            
            for i in 0..args.message_count {
                // Wait for server to consume previous message
                while Path::new(shared_file).exists() {
                    tokio::task::yield_now().await;
                }
                
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(shared_file)?;
                
                file.write_all(&test_data)?;
                file.flush()?;
                
                sent_count += 1;
                total_bytes += test_data.len();
                
                if i % 1000 == 0 {
                    println!("Sent {} messages", i + 1);
                }
            }
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== IPC Client Results ===");
            println!("Total messages: {}", sent_count);
            println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
            println!("Total time: {:.2}s", total_time.as_secs_f64());
            println!("Throughput: {:.2} MB/s", throughput_mbps);
            println!("Messages/sec: {:.2}", messages_per_sec);
        },
        _ => return Err("Invalid mode".into()),
    }
    
    Ok(())
}
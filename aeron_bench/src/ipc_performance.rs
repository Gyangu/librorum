use clap::Parser;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(name = "ipc_performance")]
#[command(about = "IPC performance test using different methods")]
struct Args {
    #[arg(long, default_value = "1048576")]
    message_size: usize,
    
    #[arg(long, default_value = "1000")]
    message_count: usize,
    
    #[arg(long, default_value = "unix_socket")]
    method: String, // unix_socket, pipe, shared_memory
    
    #[arg(long, default_value = "server")]
    mode: String, // server or client
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("IPC Performance Test");
    println!("Method: {}", args.method);
    println!("Message Size: {} bytes", args.message_size);
    println!("Message Count: {}", args.message_count);
    println!("Mode: {}", args.mode);
    
    match args.method.as_str() {
        "unix_socket" => run_unix_socket_test(&args).await,
        "pipe" => run_pipe_test(&args).await,
        "shared_memory" => run_shared_memory_test(&args).await,
        _ => {
            eprintln!("Invalid method. Use: unix_socket, pipe, shared_memory");
            std::process::exit(1);
        }
    }
}

async fn run_unix_socket_test(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::{UnixListener, UnixStream};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let socket_path = "/tmp/librorum_ipc_test.sock";
    
    match args.mode.as_str() {
        "server" => {
            // Clean up any existing socket
            let _ = std::fs::remove_file(socket_path);
            
            println!("Starting Unix socket server at {}", socket_path);
            let listener = UnixListener::bind(socket_path)?;
            
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
                
                if received_count % 100 == 0 {
                    println!("Received {} messages, {} bytes", received_count, total_bytes);
                }
            }
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== Unix Socket Server Results ===");
            println!("Total messages: {}", received_count);
            println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
            println!("Total time: {:.2}s", total_time.as_secs_f64());
            println!("Throughput: {:.2} MB/s", throughput_mbps);
            println!("Messages/sec: {:.2}", messages_per_sec);
            
            // Clean up
            let _ = std::fs::remove_file(socket_path);
        },
        "client" => {
            println!("Connecting to Unix socket at {}", socket_path);
            
            // Wait for server to be ready
            sleep(Duration::from_millis(500)).await;
            
            let mut stream = UnixStream::connect(socket_path).await?;
            println!("Connected to server");
            
            let test_data = vec![0u8; args.message_size];
            let start_time = Instant::now();
            let mut sent_count = 0;
            let mut total_bytes = 0;
            
            for i in 0..args.message_count {
                stream.write_all(&test_data).await?;
                sent_count += 1;
                total_bytes += test_data.len();
                
                if i % 100 == 0 {
                    println!("Sent {} messages", i + 1);
                }
            }
            
            stream.flush().await?;
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== Unix Socket Client Results ===");
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

async fn run_pipe_test(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::process::Command;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let pipe_path = "/tmp/librorum_ipc_pipe";
    
    match args.mode.as_str() {
        "server" => {
            // Create named pipe
            let _ = std::fs::remove_file(pipe_path);
            Command::new("mkfifo")
                .arg(pipe_path)
                .output()
                .await?;
            
            println!("Starting pipe server at {}", pipe_path);
            
            use tokio::fs::OpenOptions;
            let mut file = OpenOptions::new()
                .read(true)
                .open(pipe_path)
                .await?;
            
            let mut received_count = 0;
            let mut total_bytes = 0;
            let start_time = Instant::now();
            let mut buffer = vec![0u8; args.message_size];
            
            while received_count < args.message_count {
                let bytes_read = file.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
                
                received_count += 1;
                total_bytes += bytes_read;
                
                if received_count % 100 == 0 {
                    println!("Received {} messages, {} bytes", received_count, total_bytes);
                }
            }
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== Named Pipe Server Results ===");
            println!("Total messages: {}", received_count);
            println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
            println!("Total time: {:.2}s", total_time.as_secs_f64());
            println!("Throughput: {:.2} MB/s", throughput_mbps);
            println!("Messages/sec: {:.2}", messages_per_sec);
            
            // Clean up
            let _ = std::fs::remove_file(pipe_path);
        },
        "client" => {
            println!("Connecting to pipe at {}", pipe_path);
            
            // Wait for server to create pipe
            sleep(Duration::from_millis(500)).await;
            
            use tokio::fs::OpenOptions;
            let mut file = OpenOptions::new()
                .write(true)
                .open(pipe_path)
                .await?;
            
            let test_data = vec![0u8; args.message_size];
            let start_time = Instant::now();
            let mut sent_count = 0;
            let mut total_bytes = 0;
            
            for i in 0..args.message_count {
                file.write_all(&test_data).await?;
                sent_count += 1;
                total_bytes += test_data.len();
                
                if i % 100 == 0 {
                    println!("Sent {} messages", i + 1);
                }
            }
            
            file.flush().await?;
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== Named Pipe Client Results ===");
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

async fn run_shared_memory_test(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write, Seek, SeekFrom};
    
    let shared_file = "/tmp/librorum_shared_memory.dat";
    let control_file = "/tmp/librorum_control.dat";
    
    match args.mode.as_str() {
        "server" => {
            println!("Starting shared memory server...");
            
            // Clean up existing files
            let _ = std::fs::remove_file(shared_file);
            let _ = std::fs::remove_file(control_file);
            
            let mut received_count = 0;
            let mut total_bytes = 0;
            let start_time = Instant::now();
            
            while received_count < args.message_count {
                // Check if data is available
                if std::path::Path::new(shared_file).exists() {
                    let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(shared_file)?;
                    
                    let mut buffer = vec![0u8; args.message_size];
                    if let Ok(bytes_read) = file.read(&mut buffer) {
                        if bytes_read > 0 {
                            received_count += 1;
                            total_bytes += bytes_read;
                            
                            // Signal consumption by removing file
                            drop(file);
                            let _ = std::fs::remove_file(shared_file);
                            
                            if received_count % 100 == 0 {
                                println!("Received {} messages, {} bytes", received_count, total_bytes);
                            }
                        }
                    }
                }
                
                // Small delay to prevent busy waiting
                sleep(Duration::from_micros(10)).await;
            }
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== Shared Memory Server Results ===");
            println!("Total messages: {}", received_count);
            println!("Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
            println!("Total time: {:.2}s", total_time.as_secs_f64());
            println!("Throughput: {:.2} MB/s", throughput_mbps);
            println!("Messages/sec: {:.2}", messages_per_sec);
            
            // Clean up
            let _ = std::fs::remove_file(shared_file);
            let _ = std::fs::remove_file(control_file);
        },
        "client" => {
            println!("Starting shared memory client...");
            
            let test_data = vec![0u8; args.message_size];
            let start_time = Instant::now();
            let mut sent_count = 0;
            let mut total_bytes = 0;
            
            for i in 0..args.message_count {
                // Wait for server to consume previous message
                while std::path::Path::new(shared_file).exists() {
                    sleep(Duration::from_micros(10)).await;
                }
                
                // Write new message
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(shared_file)?;
                
                file.write_all(&test_data)?;
                file.flush()?;
                drop(file);
                
                sent_count += 1;
                total_bytes += test_data.len();
                
                if i % 100 == 0 {
                    println!("Sent {} messages", i + 1);
                }
            }
            
            let total_time = start_time.elapsed();
            let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
            let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
            
            println!("\n=== Shared Memory Client Results ===");
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
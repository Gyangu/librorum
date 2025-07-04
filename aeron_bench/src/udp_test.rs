use aeron_rs::{Aeron, Context, Publication, Subscription};
use bytes::Bytes;
use clap::Parser;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(name = "aeron_udp_test")]
#[command(about = "Aeron UDP performance test")]
struct Args {
    #[arg(long, default_value = "1048576")]
    message_size: usize,
    
    #[arg(long, default_value = "10000")]
    message_count: usize,
    
    #[arg(long, default_value = "aeron:udp?endpoint=localhost:40001")]
    channel: String,
    
    #[arg(long, default_value = "server")]
    mode: String, // server or client
    
    #[arg(long, default_value = "localhost")]
    host: String,
    
    #[arg(long, default_value = "40001")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("Aeron UDP Performance Test");
    println!("Message Size: {} bytes", args.message_size);
    println!("Message Count: {}", args.message_count);
    println!("Channel: {}", args.channel);
    println!("Mode: {}", args.mode);
    
    let context = Context::new();
    let aeron = Aeron::new(context).await?;
    
    match args.mode.as_str() {
        "server" => run_server(aeron, &args).await,
        "client" => run_client(aeron, &args).await,
        _ => {
            eprintln!("Invalid mode. Use 'server' or 'client'");
            std::process::exit(1);
        }
    }
}

async fn run_server(aeron: Aeron, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting UDP server...");
    
    let subscription = aeron.add_subscription(&args.channel, 1).await?;
    println!("Subscription created, waiting for messages...");
    
    let mut received_count = 0;
    let mut total_bytes = 0;
    let start_time = Instant::now();
    let mut first_message_time = None;
    
    loop {
        let poll_result = subscription.poll(|buffer, offset, length, _header| {
            if first_message_time.is_none() {
                first_message_time = Some(Instant::now());
            }
            
            received_count += 1;
            total_bytes += length;
            
            if received_count % 1000 == 0 {
                println!("Received {} messages, {} bytes", received_count, total_bytes);
            }
            
            received_count >= args.message_count
        }).await?;
        
        if poll_result > 0 && received_count >= args.message_count {
            break;
        }
        
        sleep(Duration::from_millis(1)).await;
    }
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = received_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== Aeron UDP Server Results ===");
    println!("Total messages: {}", received_count);
    println!("Total bytes: {}", total_bytes);
    println!("Total time: {:?}", total_time);
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}

async fn run_client(aeron: Aeron, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting UDP client...");
    
    let publication = aeron.add_publication(&args.channel, 1).await?;
    
    // Wait for connection
    while !publication.is_connected() {
        sleep(Duration::from_millis(100)).await;
    }
    println!("Connected to server");
    
    // Create test data
    let test_data = vec![0u8; args.message_size];
    let test_buffer = Bytes::from(test_data);
    
    sleep(Duration::from_millis(1000)).await; // Give server time to start
    
    let start_time = Instant::now();
    let mut sent_count = 0;
    let mut total_bytes = 0;
    
    for i in 0..args.message_count {
        loop {
            let result = publication.offer(&test_buffer, 0, test_buffer.len()).await?;
            if result > 0 {
                sent_count += 1;
                total_bytes += test_buffer.len();
                break;
            }
            
            // Back pressure - wait a bit
            sleep(Duration::from_micros(1)).await;
        }
        
        if i % 1000 == 0 {
            println!("Sent {} messages", i);
        }
    }
    
    let total_time = start_time.elapsed();
    let throughput_mbps = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    let messages_per_sec = sent_count as f64 / total_time.as_secs_f64();
    
    println!("\n=== Aeron UDP Client Results ===");
    println!("Total messages: {}", sent_count);
    println!("Total bytes: {}", total_bytes);
    println!("Total time: {:?}", total_time);
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Messages/sec: {:.2}", messages_per_sec);
    
    Ok(())
}
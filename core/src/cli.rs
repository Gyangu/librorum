use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use tokio::process::Command;
use crate::config::NodeConfig;
use crate::error::Result;
use crate::start_server;
use crate::client::VDFSClient;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a VDFS node
    Start {
        /// Path to the configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Stop a running VDFS node
    Stop {
        /// Node ID to stop
        #[arg(short, long)]
        node_id: String,
    },
    /// Get the status of a VDFS node
    Status {
        /// Node ID to check
        #[arg(short, long)]
        node_id: String,
    },
    /// List all nodes in the cluster
    ListNodes,
    /// Create a new file
    CreateFile {
        /// Path to create the file
        #[arg(short, long)]
        path: String,
        /// Node ID to create the file on
        #[arg(short, long)]
        node_id: String,
    },
    /// Delete a file
    DeleteFile {
        /// Path of the file to delete
        #[arg(short, long)]
        path: String,
        /// Node ID to delete the file from
        #[arg(short, long)]
        node_id: String,
    },
    /// Read a file
    ReadFile {
        /// Path of the file to read
        #[arg(short, long)]
        path: String,
        /// Node ID to read the file from
        #[arg(short, long)]
        node_id: String,
        /// Offset to start reading from
        #[arg(short, long, default_value = "0")]
        offset: i64,
        /// Number of bytes to read
        #[arg(short, long, default_value = "-1")]
        length: i64,
    },
    /// Write to a file
    WriteFile {
        /// Path of the file to write to
        #[arg(short, long)]
        path: String,
        /// Node ID to write the file to
        #[arg(short, long)]
        node_id: String,
        /// Data to write
        #[arg(short, long)]
        data: String,
    },
    /// List directory contents
    ListDir {
        /// Path of the directory to list
        #[arg(short, long)]
        path: String,
        /// Node ID to list the directory from
        #[arg(short, long)]
        node_id: String,
    },
}

impl Cli {
    pub async fn run() -> Result<()> {
        let cli = Cli::parse();

        match cli.command {
            Commands::Start { config } => {
                let config_path = config.unwrap_or_else(|| PathBuf::from("config.toml"));
                let config = NodeConfig::load(&config_path)?;
                println!("Starting VDFS node with config: {:?}", config);
                
                // Start the server
                let addr = format!("{}:{}", config.host, config.port);
                if let Err(e) = start_server(config, &addr).await {
                    eprintln!("Failed to start server: {}", e);
                    process::exit(1);
                }
            }
            Commands::Stop { node_id } => {
                println!("Stopping node: {}", node_id);
                // TODO: Implement proper node stopping
                // For now, just kill the process
                let output = Command::new("pkill")
                    .arg("-f")
                    .arg(format!("vdfs.*{}", node_id))
                    .output()
                    .await?;
                
                if output.status.success() {
                    println!("Node {} stopped successfully", node_id);
                } else {
                    eprintln!("Failed to stop node {}: {}", node_id, String::from_utf8_lossy(&output.stderr));
                }
            }
            Commands::Status { node_id } => {
                println!("Getting status for node: {}", node_id);
                // TODO: Implement proper status check
                // For now, just check if the process is running
                let output = Command::new("pgrep")
                    .arg("-f")
                    .arg(format!("vdfs.*{}", node_id))
                    .output()
                    .await?;
                
                if output.status.success() {
                    println!("Node {} is running", node_id);
                } else {
                    println!("Node {} is not running", node_id);
                }
            }
            Commands::ListNodes => {
                println!("Listing all nodes");
                // TODO: Implement proper node listing
                // For now, just list running processes
                let output = Command::new("pgrep")
                    .arg("-f")
                    .arg("vdfs")
                    .output()
                    .await?;
                
                if output.status.success() {
                    let pids = String::from_utf8_lossy(&output.stdout);
                    println!("Running nodes:\n{}", pids);
                } else {
                    println!("No nodes are running");
                }
            }
            Commands::CreateFile { path, node_id } => {
                println!("Creating file: {}", path);
                let mut client = VDFSClient::new(format!("http://localhost:50051")).await?;
                client.create_file(path, node_id).await?;
                println!("File created successfully");
            }
            Commands::DeleteFile { path, node_id } => {
                println!("Deleting file: {}", path);
                let mut client = VDFSClient::new(format!("http://localhost:50051")).await?;
                client.delete_file(path, node_id).await?;
                println!("File deleted successfully");
            }
            Commands::ReadFile { path, node_id, offset, length } => {
                println!("Reading file: {}", path);
                let mut client = VDFSClient::new(format!("http://localhost:50051")).await?;
                let content = client.read_file(path, node_id, offset, length).await?;
                println!("File content: {}", String::from_utf8_lossy(&content));
            }
            Commands::WriteFile { path, node_id, data } => {
                println!("Writing to file: {}", path);
                let mut client = VDFSClient::new(format!("http://localhost:50051")).await?;
                client.write_file(path, node_id, data.into_bytes()).await?;
                println!("File written successfully");
            }
            Commands::ListDir { path, node_id } => {
                println!("Listing directory: {}", path);
                let mut client = VDFSClient::new(format!("http://localhost:50051")).await?;
                let entries = client.list_directory(path, node_id).await?;
                println!("Directory contents:");
                for entry in entries {
                    println!("  {}", entry);
                }
            }
        }

        Ok(())
    }
} 
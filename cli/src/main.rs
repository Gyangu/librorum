use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod client;
mod util;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 节点管理命令
    Node {
        #[command(subcommand)]
        command: commands::node::NodeCommands,
    },
    /// 文件系统命令
    File {
        #[command(subcommand)]
        command: commands::files::FileCommands,
    },
    /// 集群管理命令
    Cluster {
        #[command(subcommand)]
        command: commands::cluster::ClusterCommands,
    },
    /// 配置命令
    Config {
        #[command(subcommand)]
        command: commands::config::ConfigCommands,
    },
    /// 日志命令
    Logs {
        #[command(subcommand)]
        command: commands::logs::LogsCommands,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Node { command } => {
            commands::node::handle_command(command).await?;
        }
        Commands::File { command } => {
            commands::files::handle_command(command).await?;
        }
        Commands::Cluster { command } => {
            commands::cluster::handle_command(command).await?;
        }
        Commands::Config { command } => {
            commands::config::handle_command(command).await?;
        }
        Commands::Logs { command } => {
            commands::logs::handle_command(command).await?;
        }
    }

    Ok(())
} 
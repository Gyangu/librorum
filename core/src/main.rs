use librorum_core::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Cli::run().await?;
    Ok(())
} 
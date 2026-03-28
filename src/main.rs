mod config;
mod error;
mod mcp;
mod services;

use crate::config::Config;
use crate::mcp::server::McpServer;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenv::dotenv().ok();

    // Initialize tracing
    let filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into())
        .add_directive("omp_discord_bridge=debug".parse()?)
        .add_directive("serenity=warn".parse()?);

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    info!("Starting Oh My Pi Discord Bridge MCP Server");

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    info!("Configuration loaded successfully");

    // Run MCP server (this blocks until shutdown)
    McpServer::new(config).run().await?;

    Ok(())
}
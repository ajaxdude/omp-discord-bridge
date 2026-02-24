mod config;
mod discord;
mod error;
mod rpc;

use crate::config::Config;
use crate::discord::DiscordBot;
use crate::rpc::RpcClient;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into())
        .add_directive("omp_discord_bridge=debug".parse()?)
        .add_directive("serenity=warn".parse()?);
    
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
    
    info!("Starting Oh My Pi Discord Bridge");
    
    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;
    
    info!("Configuration loaded successfully");
    
    // Initialize RPC client
    info!("Connecting to Oh My Pi...");
    let rpc_client = match RpcClient::new().await {
        Ok(client) => {
            info!("Connected to Oh My Pi");
            client
        }
        Err(e) => {
            error!("Failed to connect to Oh My Pi: {}", e);
            return Err(anyhow::anyhow!("Failed to connect to Oh My Pi: {}", e));
        }
    };
    
    // Initialize Discord bot
    info!("Initializing Discord bot...");
    let mut discord_bot = match DiscordBot::new(config.clone(), rpc_client).await {
        Ok(bot) => {
            info!("Discord bot initialized");
            bot
        }
        Err(e) => {
            error!("Failed to initialize Discord bot: {}", e);
            return Err(anyhow::anyhow!("Failed to initialize Discord bot: {}", e));
        }
    };
    
    // Start the Discord bot
    info!("Starting Discord bot...");
    if let Err(e) = discord_bot.start_autosharded().await {
        error!("Discord bot error: {}", e);
        return Err(anyhow::anyhow!("Discord bot error: {}", e));
    }
    
    Ok(())
}

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
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(filter)
        .init();

    // Singleton guard: only one instance may run at a time.
    //
    // Multiple instances arise when OMP sessions load the .mcp.json and each
    // spawns the bridge as a subprocess. The first process to acquire the lock
    // is the real bot; all others exit cleanly so the MCP client sees a clean
    // exit (status 0) rather than an error.
    let lock_path = std::env::temp_dir().join("omp-discord-bridge.lock");
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lock_path)?;
    use std::os::unix::io::AsRawFd;
    let locked = unsafe {
        libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)
    };
    if locked != 0 {
        // Another instance is already running — exit quietly.
        eprintln!("omp-discord-bridge: another instance is running, exiting.");
        return Ok(());
    }
    // `lock_file` is held open for the lifetime of the process.
    // When the process exits, the OS releases the lock automatically.
    let _lock_guard = lock_file;

    info!("Starting Oh My Pi Discord Bridge MCP Server");

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    info!("Configuration loaded successfully");

    // Run MCP server (this blocks until shutdown)
    McpServer::new(config).run().await?;

    Ok(())
}
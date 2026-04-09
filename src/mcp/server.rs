//! MCP Server implementation with stdio transport
//!
//! This module sets up and runs the MCP server that exposes Discord tools.

use std::sync::Arc;
use tracing::info;

use rust_mcp_sdk::{
    mcp_server::{server_runtime, McpServerOptions},
    schema::InitializeResult,
    StdioTransport, TransportOptions,
    ToMcpServerHandler,
};
use rust_mcp_sdk::McpServer as RustMcpServer;

use crate::config::Config;
use crate::mcp::tools::DiscordToolHandler;
use crate::services::discord_service::DiscordService;

/// MCP Server wrapper that manages the Discord bot and MCP server lifecycle
pub struct McpServer {
    config: Config,
}

impl McpServer {
    /// Create a new MCP server instance
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Run the MCP server (blocks until shutdown)
    pub async fn run(self) -> anyhow::Result<()> {
        info!("Starting OMP Discord Bridge MCP Server");

		// Initialize Discord service - this also starts the bot in background
		info!("Creating Discord service and connecting to gateway...");
		let discord_service = Arc::new(DiscordService::new(self.config.clone()).await?);
		
		// Create tool handler with Discord service
		let tool_handler = DiscordToolHandler {
		    discord_service,
		};
        // Define server details and capabilities
        let server_info = InitializeResult {
            server_info: rust_mcp_sdk::schema::Implementation {
                name: "omp-discord-bridge".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: Some("OMP Discord Bridge".into()),
                description: Some("MCP server exposing Discord capabilities".into()),
                icons: Vec::new(),
                website_url: None,
            },
            capabilities: rust_mcp_sdk::schema::ServerCapabilities {
                tools: Some(rust_mcp_sdk::schema::ServerCapabilitiesTools { list_changed: None }),
                ..Default::default()
            },
            protocol_version: rust_mcp_sdk::schema::ProtocolVersion::V2025_11_25.into(),
            instructions: None,
            meta: None,
        };

        // Build and run MCP server with stdio transport
        info!("Starting MCP server on stdio...");

        let transport = StdioTransport::new(TransportOptions::default()).map_err(|e| anyhow::anyhow!("Failed to create stdio transport: {}", e))?;

        let handler = tool_handler.to_mcp_server_handler();

        let options = McpServerOptions {
            server_details: server_info,
            transport,
            handler,
            task_store: None,
            client_task_store: None,
            message_observer: None,
        };

        let server = server_runtime::create_server(options);
        // server.start() returns when the MCP client disconnects (e.g. stdin closes).
        // That is fine — the Discord gateway task keeps running. We wait here until
        // a shutdown signal (SIGINT / SIGTERM) arrives so the process doesn't exit.
        match server.start().await {
            Ok(_) => info!("MCP client disconnected; Discord gateway still active."),
            Err(e) => tracing::warn!("MCP server closed: {}", e),
        }

        // Block until Ctrl-C or SIGTERM so the Discord bot keeps answering messages
        // even when no MCP client is attached.
        info!("Waiting for shutdown signal (Ctrl-C / SIGTERM)...");
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received — exiting.");

        Ok(())
    }
}

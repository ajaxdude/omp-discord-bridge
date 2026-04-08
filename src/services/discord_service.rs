//! Discord service - core Discord operations
//!
//! This module provides a high-level interface for Discord REST API operations
//! that can be used by MCP tools.
//!
//! # Architecture
//!
//! The Discord gateway (WebSocket) is spawned as a background tokio task so
//! `new()` returns immediately. This is essential: the MCP stdio transport must
//! start responding to OMP's `initialize` request within a short timeout, so we
//! cannot block here waiting for a gateway handshake.
//!
//! All tool-facing operations (send_message, read_channel, …) use the HTTP client
//! directly and do not require the gateway to be established.

use std::sync::Arc;

use serenity::all::{Context, EventHandler, GatewayIntents, GetMessages, Ready};
use serenity::async_trait;
use serenity::client::ClientBuilder;
use tracing::info;

use crate::config::Config;

/// Represents a Discord server (guild).
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub member_count: u64,
}

/// Represents a single message in a channel.
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub author: String,
    pub content: String,
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Gateway event handler (no-op — we only care about HTTP API access for now)
// ---------------------------------------------------------------------------

struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Discord bot connected as: {}", ready.user.name);
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct DiscordService {
    /// HTTP client for REST API calls — valid without an active gateway.
    http: Arc<serenity::http::Http>,
    /// Keeps the background gateway task alive for the lifetime of the service.
    _gateway_task: tokio::task::JoinHandle<()>,
}

impl DiscordService {
    /// Create a new Discord service.
    ///
    /// Builds the serenity HTTP client, then **spawns** the Discord gateway as
    /// a background task and returns immediately. The MCP server can therefore
    /// start its stdio transport without waiting for the gateway handshake.
    pub async fn new(config: Config) -> Result<Self, serenity::Error> {
        let token = config.discord_token.clone();

        info!("Building Discord client...");
        let mut client = ClientBuilder::new(
            &token,
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT,
        )
        .event_handler(DiscordHandler)
        .await?;

        // Clone the HTTP handle before moving `client` into the spawn.
        // All REST calls use this; the gateway is not required.
        let http = client.http.clone();

        info!("Spawning Discord gateway in background...");
        let gateway_task = tokio::spawn(async move {
            if let Err(e) = client.start_autosharded().await {
                tracing::error!("Discord gateway exited with error: {}", e);
            }
        });

        info!("Discord service ready — gateway connecting in background");
        Ok(Self {
            http,
            _gateway_task: gateway_task,
        })
    }

    /// Send a text message to a Discord channel.
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let channel_id = channel_id
            .parse::<serenity::model::id::ChannelId>()
            .map_err(|e| format!("Invalid channel ID: {}", e))?;

        let message = channel_id.say(&self.http, content).await?;
        info!(
            "Sent message to channel {} (msg id: {})",
            channel_id, message.id
        );
        Ok(message.id.to_string())
    }

    /// Read recent messages from a Discord channel.
    pub async fn read_channel(
        &self,
        channel_id: &str,
        limit: u32,
    ) -> Result<Vec<ChannelMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let channel_id = channel_id
            .parse::<serenity::model::id::ChannelId>()
            .map_err(|e| format!("Invalid channel ID: {}", e))?;

        let limit = limit.min(100); // Discord enforces a max of 100
        let messages = channel_id
            .messages(&self.http, GetMessages::default().limit(limit as u8))
            .await?;

        let channel_messages = messages
            .into_iter()
            .map(|msg| ChannelMessage {
                id: msg.id.to_string(),
                author: msg.author.name.to_string(),
                content: msg.content,
                timestamp: msg.timestamp.to_rfc3339().unwrap_or_default(),
            })
            .collect();

        Ok(channel_messages)
    }

    /// List all Discord servers the bot has access to.
    pub async fn list_servers(
        &self,
    ) -> Result<Vec<ServerInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let guilds = self.http.get_guilds(None, None).await?;

        let servers = guilds
            .into_iter()
            .map(|g| ServerInfo {
                id: g.id.to_string(),
                name: g.name,
                // GuildInfo doesn't carry member_count; would need a full guild
                // fetch per entry which is expensive — leave as 0 for now.
                member_count: 0,
            })
            .collect();

        Ok(servers)
    }

    /// Send a message mentioning a specific user in a Discord channel.
    pub async fn mention_user(
        &self,
        channel_id: &str,
        user_id: &str,
        content: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let full_content = format!("<@{}> {}", user_id, content);
        self.send_message(channel_id, &full_content).await
    }

    /// Upload a local file to a Discord channel.
    pub async fn post_file(
        &self,
        channel_id: &str,
        file_path: &str,
        description: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use serenity::all::{CreateAttachment, CreateMessage};

        let channel_id = channel_id
            .parse::<serenity::model::id::ChannelId>()
            .map_err(|e| format!("Invalid channel ID: {}", e))?;

        let file_data = std::fs::read(file_path)?;
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("file"))
            .to_string_lossy()
            .to_string();

        let attachment = CreateAttachment::bytes(file_data, &file_name);
        let mut message = CreateMessage::default();
        if let Some(desc) = description {
            message = message.content(desc);
        }
        channel_id
            .send_files(&self.http, [attachment], message)
            .await?;

        Ok(format!("File {} uploaded successfully", file_name))
    }
}

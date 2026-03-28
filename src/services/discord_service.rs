//! Discord service - core Discord operations
//!
//! This module provides a high-level interface for Discord operations
//! that can be used by MCP tools.

use serenity::async_trait;
use serenity::all::{GatewayIntents, Ready};
use serenity::client::{Client, Context, EventHandler};
use tracing::info;

use crate::config::Config;

/// Represents a Discord server (guild)
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub member_count: u64,
}

/// Represents a message in a channel
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub author: String,
    pub content: String,
    pub timestamp: String,
}

/// Discord service that manages the Discord client and provides operations
pub struct DiscordService {
    client: Client,
}

impl DiscordService {
    /// Create a new Discord service and start the bot
    pub async fn new(config: Config) -> Result<Self, serenity::Error> {
        let token = config.discord_token.clone();
        
        // Create event handler
        let handler = DiscordHandler;
        
        // Build Discord client
        let client = Client::builder(
            &token, 
            GatewayIntents::GUILD_MESSAGES | 
            GatewayIntents::DIRECT_MESSAGES | 
            GatewayIntents::MESSAGE_CONTENT
        )
        .event_handler(handler)
        .await?;

        info!("Discord service initialized successfully");
        
        Ok(Self { client })
    }

    /// Start the Discord bot (blocks until shutdown)
    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        info!("Starting Discord bot...");
        self.client.start_autosharded().await
    }

    /// Send a message to a Discord channel
    pub async fn send_message(&self, channel_id: &str, content: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let channel_id = channel_id.parse::<serenity::model::id::ChannelId>()
            .map_err(|e| format!("Invalid channel ID: {}", e))?;

        let http = self.client.http.clone();
        let message = channel_id.say(&http, content).await?;
        
        info!("Sent message to channel {}: {} (msg id: {})", channel_id, content, message.id);
        Ok(message.id.to_string())
    }

    /// Read recent messages from a Discord channel
    pub async fn read_channel(
        &self, 
        channel_id: &str, 
        limit: u32
    ) -> Result<Vec<ChannelMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let channel_id = channel_id.parse::<serenity::model::id::ChannelId>()
            .map_err(|e| format!("Invalid channel ID: {}", e))?;

        let limit = limit.min(100); // Discord max is 100
        use serenity::all::GetMessages;
        
        let http = self.client.http.clone();
        
        let messages = channel_id.messages(&http, GetMessages::default().limit(limit as u8)).await?;
        let channel_messages: Vec<ChannelMessage> = messages
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

    /// List all Discord servers the bot has access to
    pub async fn list_servers(&self) -> Result<Vec<ServerInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let http = self.client.http.clone();
        let guilds = http.get_guilds(None, None).await?;
        
        let servers: Vec<ServerInfo> = guilds
            .into_iter()
            .map(|g| {
                ServerInfo {
                    id: g.id.to_string(),
                    name: g.name,
                    member_count: 0, // Will be updated if we fetch full guild
                }
            })
            .collect();

        Ok(servers)
    }

    /// Mention a user in a Discord channel
    pub async fn mention_user(
        &self, 
        channel_id: &str, 
        user_id: &str, 
        content: &str
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let user_mention = format!("<@{}>", user_id);
        let full_content = format!("{} {}", user_mention, content);
        
        self.send_message(channel_id, &full_content).await
    }

    /// Upload a file to a Discord channel
    pub async fn post_file(
        &self, 
        channel_id: &str, 
        file_path: &str, 
        description: Option<String>
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let channel_id = channel_id.parse::<serenity::model::id::ChannelId>()
            .map_err(|e| format!("Invalid channel ID: {}", e))?;

        let http = self.client.http.clone();
        
        // Read the file
        let file_data = std::fs::read(file_path)?;
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("file"))
            .to_string_lossy()
            .to_string();
        // Send the file with optional description
        use serenity::all::CreateAttachment;
        
        let attachment = CreateAttachment::bytes(file_data, &file_name);
        use serenity::all::CreateMessage;
        let mut message = CreateMessage::default();
        if let Some(desc) = description {
            message = message.content(desc);
        }
        channel_id.send_files(&http, [attachment], message).await?;

        Ok(format!("File {} uploaded successfully", file_name))
    }
}

/// Event handler for Discord events
struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Discord bot connected as {}", ready.user.name);
    }
}

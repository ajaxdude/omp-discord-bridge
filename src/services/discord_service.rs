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
//!
//! # Inbound message handling
//!
//! The gateway event handler listens for Discord messages and routes commands
//! to OMP:
//!
//! - `!ping`          → immediate "Pong!" health-check reply
//! - `!omp <query>`   → forwards <query> to the OMP CLI via stdin
//! - `@bot <query>`   → same as above, triggered by @mention

use std::sync::Arc;

use serenity::all::{ChannelId, Context, EventHandler, GatewayIntents, GetMessages, Message, Ready};
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
// Gateway event handler
// ---------------------------------------------------------------------------

struct DiscordHandler {
    /// Bot configuration — command prefix and OMP executable path.
    config: Config,
    /// The bot's own user ID, populated once the `ready` event fires.
    /// Used to detect @mention triggers.
    bot_id: Arc<std::sync::OnceLock<serenity::model::id::UserId>>,
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        // Capture the bot's own user ID so the message handler can strip @mentions.
        let _ = self.bot_id.set(ready.user.id);
        info!(
            "Discord bot connected as: {} (ID: {})",
            ready.user.name, ready.user.id
        );
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore all bot messages — includes our own — to prevent response loops.
        if msg.author.bot {
            return;
        }

        let prefix = &self.config.discord_prefix;

        // Classify the message and extract the OMP query, or handle inline.
        let mut text = msg.content.trim();

        if text == format!("{}ping", prefix) {
            let now = serenity::all::Timestamp::now();
            let now_f64 = now.unix_timestamp() as f64 + (now.nanosecond() as f64 / 1_000_000_000.0);
            let msg_f64 = msg.timestamp.unix_timestamp() as f64 + (msg.timestamp.nanosecond() as f64 / 1_000_000_000.0);
            let latency = (now_f64 - msg_f64).max(0.0);
            let _ = msg.channel_id.say(&ctx.http, format!("Pong! {:.3}s", latency)).await;
            return;
        }

        if let Some(rest) = text.strip_prefix(&format!("{}omp", prefix)) {
            text = rest.trim();
        } else if let Some(bot_id) = self.bot_id.get() {
            let long_mention = format!("<@{}>", bot_id);
            let nick_mention = format!("<@!{}>", bot_id);
            if let Some(rest) = text.strip_prefix(&long_mention) {
                text = rest.trim();
            } else if let Some(rest) = text.strip_prefix(&nick_mention) {
                text = rest.trim();
            }
        }

        if text.is_empty() {
            return;
        }

        let query = text.to_string();

        let (model, actual_query) = {
            let mut q = query.as_str();
            let mut m = None;
            if q.starts_with("--model ") {
                let parts: Vec<&str> = q.splitn(3, ' ').collect();
                if parts.len() >= 3 {
                    m = Some(parts[1]);
                    q = parts[2];
                }
            }
            (m, q)
        };

        // Broadcast a typing indicator while OMP processes — best-effort, ignore errors.
        let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

        match invoke_omp(&self.config.omp_path, model, actual_query).await {
            Ok(response) => {
                let text = if response.is_empty() {
                    "(OMP returned an empty response)".to_string()
                } else {
                    response
                };
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            Err(e) => {
                tracing::error!("OMP invocation failed: {}", e);
                let _ = msg
                    .channel_id
                    .say(&ctx.http, format!("OMP error: {}", e))
                    .await;
            }
        }
    }
}

/// Send a long string as successive Discord messages capped at 1 900 bytes each.
///
/// Discord enforces a 2 000-character hard limit per message.  We stay well
/// under it and always split on a valid UTF-8 char boundary so no codepoint
/// is ever mangled.
async fn send_chunked(ctx: &Context, channel_id: ChannelId, text: &str) {
    const MAX_BYTES: usize = 1_900;
    let mut rest = text;
    while !rest.is_empty() {
        let split = if rest.len() <= MAX_BYTES {
            rest.len()
        } else {
            // Walk back from MAX_BYTES until we land on a char boundary.
            let mut idx = MAX_BYTES;
            while !rest.is_char_boundary(idx) {
                idx -= 1;
            }
            idx
        };
        let (chunk, remainder) = rest.split_at(split);
        let _ = channel_id.say(&ctx.http, chunk).await;
        rest = remainder;
    }
}

/// Invoke the OMP CLI with `query` as an argument, and return stdout.
///
/// OMP is expected to process the query, write a single response to
/// stdout, and then exit.  A 1200-second timeout is enforced; the bot replies with
/// an error message if OMP does not respond in time.
async fn invoke_omp(omp_path: &str, model: Option<&str>, query: &str) -> Result<String, String> {
    use std::process::Stdio;
    use tokio::process::Command;

    let mut cmd = Command::new(omp_path);
    cmd.stdin(Stdio::null());
    cmd.arg("-p");
    if let Some(m) = model {
        cmd.arg("--model");
        cmd.arg(m);
    }
    cmd.arg(query);

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(1200),
        cmd.output(),
    )
    .await
    .map_err(|_| "OMP timed out after 20 minutes (1200 seconds)".to_string())?
    .map_err(|e| format!("OMP process I/O error: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("OMP exited with status {}", output.status)
        } else {
            stderr
        })
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

        // Shared cell written by `ready`, read by `message`.
        let bot_id = Arc::new(std::sync::OnceLock::new());

        let handler = DiscordHandler {
            config,
            bot_id: bot_id.clone(),
        };

        let mut client = ClientBuilder::new(
            &token,
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT,
        )
        .event_handler(handler)
        .await?;

        // Clone the HTTP handle before moving `client` into the spawn.
        // All REST tool calls use this; the gateway is not required.
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

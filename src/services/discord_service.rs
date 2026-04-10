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
//! - `!omp reset`     → clears the active OMP session for the current channel
//! - `!omp <query>`   → forwards <query> to the OMP CLI; reply sent back to Discord
//! - `@bot <query>`   → same as above, triggered by @mention
//!
//! # Session continuity
//!
//! Each Discord channel maps to a persistent OMP session.  When a query arrives,
//! the bridge resumes the channel's existing session via `omp --resume <id>`,
//! giving the agent full conversation history across messages.  Session IDs are
//! persisted to disk (~/.local/share/omp-discord-bridge/sessions.json) so they
//! survive service restarts.  Use `!omp reset` in a channel to start a new session.

use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{ChannelId, Context, EventHandler, GatewayIntents, GetMessages, Message, Ready};
use serenity::async_trait;
use serenity::client::ClientBuilder;
use tokio::sync::Mutex;
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
// Session persistence
// ---------------------------------------------------------------------------

/// Channel-ID (string snowflake) → OMP session ID.
type SessionMap = Arc<Mutex<HashMap<String, String>>>;

fn sessions_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home)
        .join(".local/share/omp-discord-bridge/sessions.json")
}

fn load_sessions() -> HashMap<String, String> {
    let path = sessions_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
            info!("Loaded {} session(s) from {}", map.len(), path.display());
            return map;
        }
    }
    HashMap::new()
}

/// Write the session map to disk.  Best-effort — failures are logged, not fatal.
fn save_sessions(sessions: &HashMap<String, String>) {
    let path = sessions_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!("Could not create session dir {}: {}", parent.display(), e);
            return;
        }
    }
    match serde_json::to_string(sessions) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("Could not write sessions to {}: {}", path.display(), e);
            }
        }
        Err(e) => tracing::warn!("Could not serialize sessions: {}", e),
    }
}

/// Load model aliases from the bridge's `config.yaml`.
///
/// The YAML file must contain a top-level `model_aliases` mapping:
/// ```yaml
/// model_aliases:
///   gemma: llama.cpp/gemma-4-31b-draft
///   qwen:  llama.cpp/qwen3-coder-next
/// ```
/// Keys are stored lowercased for case-insensitive matching at runtime.
/// A missing file or a file without a `model_aliases` key is silently
/// treated as an empty map so the bridge keeps running.
fn load_model_aliases(path: &str) -> HashMap<String, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Could not read aliases config {}: {}", path, e);
            return HashMap::new();
        }
    };

    // Parse as a generic YAML value so we can extract just the
    // `model_aliases` key without a rigid top-level struct.
    let doc: serde_yml::Value = match serde_yml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Could not parse aliases config {}: {}", path, e);
            return HashMap::new();
        }
    };

    let mapping = match doc.get("model_aliases").and_then(|v| v.as_mapping()) {
        Some(m) => m,
        None => {
            tracing::warn!(
                "No 'model_aliases' key found in {}; model aliases disabled",
                path
            );
            return HashMap::new();
        }
    };

    let mut aliases = HashMap::new();
    for (k, v) in mapping {
        if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
            // Store keys lowercased so the runtime lookup is always O(n) over
            // lowercase needles without per-lookup allocation.
            aliases.insert(key.to_lowercase(), val.to_string());
        }
    }
    info!("Loaded {} model alias(es) from {}", aliases.len(), path);
    aliases
}

// ---------------------------------------------------------------------------
// Gateway event handler
// ---------------------------------------------------------------------------

struct DiscordHandler {
    /// Bot configuration — command prefix and OMP executable path.
    config: Config,
    /// The bot's own user ID, populated once the `ready` event fires.
    bot_id: Arc<std::sync::OnceLock<serenity::model::id::UserId>>,
    /// Per-channel OMP session IDs, shared with DiscordService for visibility.
    sessions: SessionMap,
    /// Model alias map loaded from config.yaml at startup.
    /// Key: lowercase substring to match; value: canonical OMP model ID.
    model_aliases: HashMap<String, String>,
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
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
        // Discord (and some OS input methods) replace `--` with `—` (U+2014, em dash).
        // Normalize up-front so command parsing always sees ASCII hyphens.
        let content_normalized = msg.content.replace('\u{2014}', "--");
        let mut text = content_normalized.trim();

        // !ping health check
        if text == format!("{}ping", prefix) {
            let now = serenity::all::Timestamp::now();
            let now_f64 = now.unix_timestamp() as f64
                + (now.nanosecond() as f64 / 1_000_000_000.0);
            let msg_f64 = msg.timestamp.unix_timestamp() as f64
                + (msg.timestamp.nanosecond() as f64 / 1_000_000_000.0);
            let latency = (now_f64 - msg_f64).max(0.0);
            let _ = msg
                .channel_id
                .say(&ctx.http, format!("Pong! {:.3}s", latency))
                .await;
            return;
        }

        // !omp reset — clear the OMP session for this channel so the next
        // message starts a fresh conversation.
        if text == format!("{}omp reset", prefix) {
            let channel_key = msg.channel_id.to_string();
            let had_session = {
                let mut sessions = self.sessions.lock().await;
                let removed = sessions.remove(&channel_key).is_some();
                if removed {
                    save_sessions(&sessions);
                }
                removed
            };
            let reply = if had_session {
                "Session cleared. Starting fresh on your next message."
            } else {
                "No active session for this channel."
            };
            let _ = msg.channel_id.say(&ctx.http, reply).await;
            return;
        }

        // Classify the message: !omp <query> or @mention <query>.
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

        // Optional --model override: `!omp --model <alias> <query>`
        //
        // The raw alias (e.g. "gemma", "qwen", or a fully-qualified OMP model
        // ID) is resolved to a canonical OMP model string by `resolve_model`.
        let (model_owned, actual_query) = {
            let mut q = query.as_str();
            let mut m: Option<String> = None;
            if q.starts_with("--model ") {
                let parts: Vec<&str> = q.splitn(3, ' ').collect();
                if parts.len() >= 3 {
                    m = Some(resolve_model(parts[1], &self.model_aliases));
                    q = parts[2];
                }
            }
            (m, q)
        };
        let model = model_owned.as_deref();

        // Look up the existing session for this channel (if any).
        //
        // When the user supplies --model we deliberately ignore any stored session:
        // sessions are model-scoped in OMP and resuming a Claude session as Gemma
        // (or vice-versa) causes an immediate "session not found" rejection.
        // The stored session is preserved so model-less follow-up messages can
        // still resume the previous thread.
        let session_id: Option<String> = if model.is_some() {
            info!(
                "Model override {:?} — skipping session resume for channel {}",
                model, msg.channel_id
            );
            None
        } else {
            let sessions = self.sessions.lock().await;
            sessions.get(&msg.channel_id.to_string()).cloned()
        };

        if let Some(ref sid) = session_id {
            info!("Resuming session {} for channel {}", sid, msg.channel_id);
        } else if model.is_none() {
            info!("Starting new session for channel {}", msg.channel_id);
        }

        // Keep "F2 is typing..." alive for the full duration of the OMP call.
        // Discord's typing indicator expires after ~10 s, so we re-send every 8 s
        // on a background task and cancel it as soon as OMP returns.
        let (typing_cancel_tx, mut typing_cancel_rx) = tokio::sync::oneshot::channel::<()>();
        let typing_http = ctx.http.clone();
        let typing_channel = msg.channel_id;
        tokio::spawn(async move {
            loop {
                let _ = typing_channel.broadcast_typing(&typing_http).await;
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(8)) => {}
                    _ = &mut typing_cancel_rx => break,
                }
            }
        });

        // Helper: run OMP and, if it fails with a stale-session error, clear the
        // session and retry once without --resume before giving up.
        let result = {
            let first = invoke_omp(
                &self.config.omp_path,
                &self.config.omp_work_dir,
                model,
                actual_query,
                session_id.as_deref(),
            )
            .await;

            match first {
                // Success — use the result directly.
                Ok(v) => Ok(v),
                // Session-not-found: clear the stale entry and retry fresh.
                Err(ref e) if e.contains("not found") && session_id.is_some() => {
                    tracing::warn!(
                        "Stale session for channel {} cleared; retrying without --resume",
                        msg.channel_id
                    );
                    {
                        let mut sessions = self.sessions.lock().await;
                        sessions.remove(&msg.channel_id.to_string());
                        save_sessions(&sessions);
                    }
                    // One retry without --resume.
                    invoke_omp(
                        &self.config.omp_path,
                        &self.config.omp_work_dir,
                        model,
                        actual_query,
                        None,
                    )
                    .await
                }
                // Any other error — propagate as-is.
                Err(e) => Err(e),
            }
        };

        // Stop the typing indicator — best-effort, ignore if already dropped.
        let _ = typing_cancel_tx.send(());

        match result {
            Ok((response, new_session, model_info)) => {
                // Persist the new (or same) session ID for the next message.
                // When a model override was used we still save the returned
                // session so subsequent model-less messages can continue the
                // same thread.
                if let Some(sid) = new_session {
                    let mut sessions = self.sessions.lock().await;
                    sessions.insert(msg.channel_id.to_string(), sid.clone());
                    save_sessions(&sessions);
                    info!("Saved session {} for channel {}", sid, msg.channel_id);
                }

                // When a --model override was used, append a small footer so
                // the user can confirm which model actually answered.
                let text = if response.is_empty() {
                    "(OMP returned an empty response)".to_string()
                } else if let (Some(_), Some((provider, mdl))) = (model_owned.as_deref(), model_info) {
                    format!("{response}\n\n-# {provider}/{mdl}")
                } else {
                    response
                };
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            Err(e) => {
                // Clear any session that might have contributed to the error.
                {
                    let mut sessions = self.sessions.lock().await;
                    if sessions.remove(&msg.channel_id.to_string()).is_some() {
                        save_sessions(&sessions);
                        tracing::warn!(
                            "Cleared session for channel {} after unrecoverable error",
                            msg.channel_id
                        );
                    }
                }
                tracing::error!("OMP invocation failed: {}", e);
                let _ = msg
                    .channel_id
                    .say(&ctx.http, format!("OMP error: {}", e))
                    .await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OMP subprocess invocation
// ---------------------------------------------------------------------------

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

/// Resolve a user-supplied model alias to the canonical OMP model ID.
///
/// `aliases` is the map loaded from `config.yaml` at startup:
///   key   = lowercase substring to match (e.g. `"gemma"`)
///   value = canonical OMP model ID (e.g. `"llama.cpp/gemma-4-31b-draft"`)
///
/// Resolution rules (first match wins):
/// 1. If the raw alias already contains `/` or `.` it is fully-qualified — pass through.
/// 2. Case-insensitive substring search through the alias map.
/// 3. No match — return verbatim; OMP surfaces its own unknown-model error.
fn resolve_model(raw: &str, aliases: &HashMap<String, String>) -> String {
    let lower = raw.to_lowercase();

    // Already fully-qualified — pass through unchanged.
    if lower.contains('/') || lower.contains('.') {
        return raw.to_string();
    }

    for (needle, canonical) in aliases {
        if lower.contains(needle.as_str()) {
            tracing::debug!("resolved alias {:?} -> {:?}", raw, canonical);
            return canonical.clone();
        }
    }

    // Unknown alias — pass through; OMP will report the error.
    raw.to_string()
}

/// Invoke the OMP CLI and return `(assistant_text, session_id)`.
///
/// Uses `--mode json` so OMP writes NDJSON to stdout regardless of whether a
/// TTY is attached.  Only the assistant's human-readable text blocks are
/// extracted — tool calls, tool results, thinking blocks, and session metadata
/// are discarded by `parse_omp_json_output`.
///
/// If `session_id` is `Some`, the session is resumed via `--resume <id>` so
/// the agent retains full conversation history.  The session ID returned is
/// whatever OMP reports in the `{"type":"session"}` event; pass it back on the
/// next call to continue the same thread.
///
/// A 1 200-second (20 min) timeout is enforced.
async fn invoke_omp(
    omp_path: &str,
    work_dir: &str,
    model: Option<&str>,
    query: &str,
    session_id: Option<&str>,
) -> Result<(String, Option<String>, Option<(String, String)>), String> {
    use std::process::Stdio;
    use tokio::process::Command;

    let mut cmd = Command::new(omp_path);
    cmd.stdin(Stdio::null());
    cmd.current_dir(work_dir);
    cmd.arg("-p");
    cmd.arg("--mode");
    cmd.arg("json");

    if let Some(sid) = session_id {
        cmd.arg("--resume");
        cmd.arg(sid);
    }

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

    // A non-zero exit with no stdout is a real failure (e.g. bad session ID,
    // missing binary).  A non-zero exit WITH stdout means OMP ran but
    // encountered an application-level error after producing some output —
    // surface whatever text we extracted rather than swallowing it.
    if !output.status.success() && output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("OMP exited with status {}", output.status)
        } else {
            stderr
        });
    }

    Ok(parse_omp_json_output(&output.stdout))
}

/// Parse OMP's `--mode json` NDJSON output.
///
/// Returns `(assistant_text, session_id, Option<(provider, model)>)`.
///
/// Events we care about:
/// - `{"type":"session","id":"<id>"}` — the active session ID (first event)
/// - `{"type":"message_end","message":{"role":"assistant",...}}` —
///   a completed assistant turn; we collect `{"type":"text","text":"..."}` items
///   and ignore `toolCall` items.  We also capture the first `provider`/`model`
///   pair so the caller can display which model actually answered.
///
/// All other event types are skipped.
fn parse_omp_json_output(ndjson: &[u8]) -> (String, Option<String>, Option<(String, String)>) {
    let content = String::from_utf8_lossy(ndjson);
    let mut text_pieces: Vec<String> = Vec::new();
    let mut session_id: Option<String> = None;
    let mut model_info: Option<(String, String)> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(event_type) = val.get("type").and_then(|t| t.as_str()) else {
            continue;
        };

        match event_type {
            "session" => {
                // {"type":"session","id":"<id>",...} — first event in any run.
                if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                    session_id = Some(id.to_string());
                }
            }
            "message_end" => {
                // Extract text content from completed assistant messages only.
                let Some(msg) = val.get("message") else { continue };
                if msg.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                    continue;
                }
                // Capture provider+model from the first assistant message_end.
                if model_info.is_none() {
                    if let (Some(p), Some(m)) = (
                        msg.get("provider").and_then(|v| v.as_str()),
                        msg.get("model").and_then(|v| v.as_str()),
                    ) {
                        model_info = Some((p.to_string(), m.to_string()));
                    }
                }
                let Some(content) = msg.get("content").and_then(|c| c.as_array()) else {
                    continue;
                };
                for item in content {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                            let trimmed = text.trim().to_string();
                            if !trimmed.is_empty() {
                                text_pieces.push(trimmed);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    (text_pieces.join("\n\n"), session_id, model_info)
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

        // Load persisted sessions so conversation history survives restarts.
        let sessions: SessionMap = Arc::new(Mutex::new(load_sessions()));

        // Load model aliases from config.yaml.  Missing file is non-fatal:
        // warn and continue with an empty map (all aliases pass through to OMP).
        let model_aliases = load_model_aliases(&config.aliases_config_path);

        let handler = DiscordHandler {
            config,
            bot_id: bot_id.clone(),
            sessions,
            model_aliases,
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

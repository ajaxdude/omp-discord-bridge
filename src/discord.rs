use crate::config::Config;
use crate::rpc::RpcClient;
use crate::rpc::types::{RpcEvent, AssistantMessageEvent};
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::all::GatewayIntents;

use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Type map key for storing the RPC client in the Discord context
struct RpcClientKey;

impl TypeMapKey for RpcClientKey {
    type Value = Arc<Mutex<RpcClient>>;
}

/// Pending message information for tracking OMP responses
struct PendingMessage {
    channel_id: serenity::model::id::ChannelId,
    /// Message to edit with streaming updates (the "Processing..." message)
    processing_message_id: Option<serenity::model::id::MessageId>,
}

/// Type map key for storing the pending Discord messages
struct PendingMessagesKey;

impl TypeMapKey for PendingMessagesKey {
    type Value = Arc<Mutex<HashMap<String, PendingMessage>>>;
}

/// Discord bot event handler
struct DiscordHandler {
    config: Config,
}

impl DiscordHandler {
    fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore messages from bots
        if msg.author.bot {
            return;
        }

        // Check if the message is a command (starts with prefix)
        let prefix = &self.config.discord_prefix;
        
        if msg.content.starts_with(prefix) {
            let command = msg.content[prefix.len()..].trim();
            
            info!("Received command: {}", command);
            
            match command {
"ping" => {
                    info!("Ping command received from {}", msg.author.name);
                    if let Err(e) = msg.channel_id.say(&ctx, "Pong!").await {
                        error!("Failed to send ping response: {}", e);
                    } else {
                        info!("Pong! sent successfully");
                    }
                }
                "help" => {
                    let help_text = r#"
**Oh My Pi Discord Bot**

Commands:
- `!ping` - Test bot connectivity
- `!omp <message>` - Send a message to Oh My Pi
- `!status` - Check OMP connection status

Example:
`!omp List all files in the current directory`
"#;
                    if let Err(e) = msg.channel_id.say(&ctx, help_text).await {
                        error!("Failed to send help response: {}", e);
                    }
                }
                _ => {
                    // Try to parse as an OMP command
                    if let Some(omp_msg) = command.strip_prefix("omp ") {
                        // Send to OMP
                        let data = ctx.data.read().await;
                        if let Some(rpc_client) = data.get::<RpcClientKey>() {
                            let rpc = rpc_client.lock().await;
                            
                            // Send the message to OMP and get the correlation ID
                            match rpc.prompt(omp_msg.to_string()) {
                                Ok(correlation_id) => {
                                    // Send "Processing..." message and get its ID
                                    let processing_message = msg.channel_id.say(&ctx, "Processing...").await;
                                    let processing_msg_id = processing_message.ok().map(|m| m.id);
                                    
                                    // Register the pending message with correlation ID
                                    if let Some(pending) = data.get::<PendingMessagesKey>() {
                                        let mut pending_map = pending.lock().await;
                                        pending_map.insert(correlation_id.clone(), PendingMessage {
                                            channel_id: msg.channel_id,
                                            processing_message_id: processing_msg_id,
                                        });
                                        debug!("Registered correlation ID: {} for channel {}", correlation_id, msg.channel_id);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to send prompt to OMP: {}", e);
                                    let _ = msg.channel_id.say(&ctx, format!("Error: {}", e)).await;
                                }
                            }
                        } else {
                            error!("RPC client not found in context");
                            let _ = msg.channel_id.say(&ctx, "Error: RPC client not initialized").await;
                        }
                    } else {
                        error!("RPC client not found in context");
                        let _ = msg.channel_id.say(&ctx, "Error: RPC client not initialized").await;
                    }
                }
            }
        }
    }
}

/// Discord bot client
pub struct DiscordBot {
    client: Client,
    _event_streamer: Option<JoinHandle<()>>,
}

impl DiscordBot {
    /// Create a new Discord bot
    pub async fn new(config: Config, rpc_client: RpcClient) -> Result<Self, serenity::Error> {
        let token = config.discord_token.clone();
        let handler = DiscordHandler::new(config.clone());
        
        let client = Client::builder(&token, GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT)
            .event_handler(handler)
            .await?;
        
        // Store the RPC client and pending messages map in the context
        let rpc_client_arc = Arc::new(Mutex::new(rpc_client));
        let pending_map = Arc::new(Mutex::new(HashMap::new()));
        {
            let mut data = client.data.write().await;
            data.insert::<RpcClientKey>(rpc_client_arc.clone());
            data.insert::<PendingMessagesKey>(pending_map.clone());
        }
        
        // Clone the HTTP client for use in the event streamer
        let http = client.http.clone();
        
        // Spawn event streaming task
        let event_streamer = tokio::spawn(async move {
            Self::stream_rpc_events(rpc_client_arc, http, pending_map).await;
        });
        
        Ok(Self { 
            client,
            _event_streamer: Some(event_streamer),
        })
    }
    
    /// Stream RPC events to Discord
    async fn stream_rpc_events(
        rpc_client: Arc<Mutex<RpcClient>>,
        http: Arc<serenity::http::Http>,
        pending_map: Arc<Mutex<HashMap<String, PendingMessage>>>,
    ) {
        let mut rpc = rpc_client.lock().await;
        let mut buffer = String::new();
        
        info!("Starting RPC event streamer");
        
        loop {
            match rpc.recv_event().await {
                Some(event) => {
                    match event {
                        RpcEvent::MessageUpdate { id, assistant_message_event, .. } => {
                            match assistant_message_event {
                                AssistantMessageEvent::TextDelta { delta } => {
                                    buffer.push_str(&delta);
                                    
                                    // Send update if buffer is getting large
                                    if buffer.len() > 1500 {
                                        debug!("Streaming text update: {} chars", buffer.len());
                                        // Send to Discord channel using correlation ID
                                        if let Some(correlation_id) = id {
                                            let pending = pending_map.lock().await;
                                            if let Some(pending_msg) = pending.get(&correlation_id) {
                                                if let Err(e) = pending_msg.channel_id.say(&http, &buffer).await {
                                                    error!("Failed to send message to Discord: {}", e);
                                                }
                                            } else {
                                                warn!("Received event with correlation ID {} but no pending channel found", correlation_id);
                                            }
                                        }
                                        buffer.clear();
                                    }
                                }
                                AssistantMessageEvent::ThinkingDelta { delta } => {
                                    debug!("Thinking: {}", delta);
                                }
                                AssistantMessageEvent::ToolCallDelta { .. } => {
                                    debug!("Tool call in progress");
                                }
                            }
                        }
                        RpcEvent::AgentEnd { id, messages: _ } => {
                            info!("Agent finished");
                            if !buffer.is_empty() {
                                debug!("Final response: {} chars", buffer.len());
                                // Send final response to Discord using correlation ID
                                if let Some(correlation_id) = id {
                                    let mut pending = pending_map.lock().await;
                                    if let Some(pending_msg) = pending.get(&correlation_id) {
                                        if let Err(e) = pending_msg.channel_id.say(&http, &buffer).await {
                                            error!("Failed to send final message to Discord: {}", e);
                                        }
                                    } else {
                                        warn!("Received AgentEnd with correlation ID {} but no pending channel found", correlation_id);
                                    }
                                    // Clean up the pending map entry
                                    pending.remove(&correlation_id);
                                }
                                buffer.clear();
                            }
                        }
                        RpcEvent::ToolExecutionStart { tool_name, .. } => {
                            debug!("Tool started: {}", tool_name);
                        }
                        RpcEvent::ToolExecutionEnd { tool_name, .. } => {
                            debug!("Tool finished: {}", tool_name);
                        }
                        _ => {
                            debug!("Received RPC event: {:?}", event);
                        }
                    }
                }
                None => {
                    warn!("RPC event stream closed");
                    break;
                }
            }
        }
    }
    
    /// Start the Discord bot
    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        self.client.start().await
    }
    
    /// Start the Discord bot in the background
    pub async fn start_autosharded(&mut self) -> Result<(), serenity::Error> {
        self.client.start_autosharded().await
    }
}

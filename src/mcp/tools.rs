//! MCP Tool definitions and handlers
//!
//! This module defines all the Discord-related tools exposed via MCP.

use async_trait::async_trait;
use rust_mcp_sdk::macros;
use rust_mcp_sdk::schema::{CallToolResult, CallToolError, ListToolsResult, RpcError, PaginatedRequestParams};
use serde::{Deserialize, Serialize};
use rust_mcp_sdk::schema::CallToolRequestParams;

use crate::services::discord_service::DiscordService;

/// Ping tool - returns a simple pong response for testing connectivity
#[macros::mcp_tool(name = "ping", description = "Test the Discord bridge connection. Returns 'Pong!' to verify the server is responsive.")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct PingTool {}

/// Send message tool - sends a text message to a Discord channel
#[macros::mcp_tool(name = "send_message", description = "Send a text message to a Discord channel. Use this to communicate in Discord channels.")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct SendMessageTool {
    /// The Discord channel ID (snowflake format)
    channel_id: String,
    /// The message content to send
    content: String,
}

/// Read channel tool - retrieves recent messages from a Discord channel
#[macros::mcp_tool(name = "read_channel", description = "Read recent messages from a Discord channel. Useful for reviewing conversation history.")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ReadChannelTool {
    /// The Discord channel ID (snowflake format)
    channel_id: String,
    /// Number of messages to retrieve (default: 10, max: 100)
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    10
}

/// List servers tool - lists all Discord servers the bot has access to
#[macros::mcp_tool(name = "list_servers", description = "List all Discord servers that this bot has access to. Returns server names and IDs.")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ListServersTool {}

/// Mention user tool - sends a message mentioning a specific user
#[macros::mcp_tool(name = "mention_user", description = "Send a message that mentions a specific user in a Discord channel.")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct MentionUserTool {
    /// The Discord channel ID
    channel_id: String,
    /// The user ID to mention
    user_id: String,
    /// Message content after the mention
    content: String,
}

/// Post file tool - uploads a file to a Discord channel
#[macros::mcp_tool(name = "post_file", description = "Upload and send a file to a Discord channel.")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct PostFileTool {
    /// The Discord channel ID
    channel_id: String,
    /// Local file path to upload
    file_path: String,
    /// Optional description for the file
    #[serde(default)]
    description: Option<String>,
}

/// Tool handler for Discord operations
pub struct DiscordToolHandler {
    pub discord_service: std::sync::Arc<DiscordService>,
}

#[async_trait]
impl rust_mcp_sdk::mcp_server::ServerHandler for DiscordToolHandler {
    async fn handle_list_tools_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _runtime: std::sync::Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: vec![
                PingTool::tool(),
                SendMessageTool::tool(),
                ReadChannelTool::tool(),
                ListServersTool::tool(),
                MentionUserTool::tool(),
                PostFileTool::tool(),
            ],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: std::sync::Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        match params.name.as_str() {
            "ping" => self.handle_ping(params),
            "send_message" => self.handle_send_message(params).await,
            "read_channel" => self.handle_read_channel(params).await,
            "list_servers" => self.handle_list_servers(params).await,
            "mention_user" => self.handle_mention_user(params).await,
            "post_file" => self.handle_post_file(params).await,
            _ => Err(CallToolError::unknown_tool(params.name)),
        }
    }
}

impl DiscordToolHandler {
    fn handle_ping(
        &self,
        _params: CallToolRequestParams,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        Ok(CallToolResult::text_content(vec!["Pong!".into()]))
    }

    async fn handle_send_message(
        &self,
        params: CallToolRequestParams,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let args = params.arguments.unwrap_or_default();
        
        let channel_id = args.get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("channel_id", Some("Missing or invalid channel_id".into())))?;
        
        let content = args.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("content", Some("Missing or invalid content".into())))?;

        match self.discord_service.send_message(channel_id, content).await {
            Ok(msg_id) => Ok(CallToolResult::text_content(vec![format!(
                "Message sent successfully to channel {} (message ID: {})",
                channel_id, msg_id
            ).into()])),
            Err(e) => Err(CallToolError::from_message(format!("Failed to send message: {}", e))),
        }
    }

    async fn handle_read_channel(
        &self,
        params: CallToolRequestParams,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let args = params.arguments.unwrap_or_default();
        
        let channel_id = args.get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("channel_id", Some("Missing or invalid channel_id".into())))?;
        
        let limit = args.get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l as u32)
            .unwrap_or(10);

        match self.discord_service.read_channel(channel_id, limit).await {
            Ok(messages) => {
                let formatted = messages
                    .iter()
                    .map(|m| format!("[{}] {}: {}", m.timestamp, m.author, m.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                
                Ok(CallToolResult::text_content(vec![formatted.into()]))
            }
            Err(e) => Err(CallToolError::from_message(format!("Failed to read channel: {}", e))),
        }
    }

    async fn handle_list_servers(
        &self,
        _params: CallToolRequestParams,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        match self.discord_service.list_servers().await {
            Ok(servers) => {
                let formatted = servers
                    .iter()
                    .map(|s| format!("[{}] {} (ID: {})", s.member_count, s.name, s.id))
                    .collect::<Vec<_>>()
                    .join("\n");
                
                Ok(CallToolResult::text_content(vec![formatted.into()]))
            }
            Err(e) => Err(CallToolError::from_message(format!("Failed to list servers: {}", e))),
        }
    }

    async fn handle_mention_user(
        &self,
        params: CallToolRequestParams,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let args = params.arguments.unwrap_or_default();
        
        let channel_id = args.get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("channel_id", Some("Missing or invalid channel_id".into())))?;
        
        let user_id = args.get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("user_id", Some("Missing or invalid user_id".into())))?;
        
        let content = args.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("content", Some("Missing or invalid content".into())))?;

        match self.discord_service.mention_user(channel_id, user_id, content).await {
            Ok(msg_id) => Ok(CallToolResult::text_content(vec![format!(
                "Message with mention sent successfully (message ID: {})",
                msg_id
            ).into()])),
            Err(e) => Err(CallToolError::from_message(format!("Failed to mention user: {}", e))),
        }
    }

    async fn handle_post_file(
        &self,
        params: CallToolRequestParams,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let args = params.arguments.unwrap_or_default();
        
        let channel_id = args.get("channel_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("channel_id", Some("Missing or invalid channel_id".into())))?;
        
        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CallToolError::invalid_arguments("file_path", Some("Missing or invalid file_path".into())))?;
        
        let description = args.get("description").and_then(|v| v.as_str()).map(String::from);

        match self.discord_service.post_file(channel_id, file_path, description).await {
            Ok(msg_id) => Ok(CallToolResult::text_content(vec![format!(
                "File uploaded successfully (message ID: {})",
                msg_id
            ).into()])),
            Err(e) => Err(CallToolError::from_message(format!("Failed to post file: {}", e))),
        }
    }
}

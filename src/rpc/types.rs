use serde::{Deserialize, Serialize};

/// RPC command types that can be sent to Oh My Pi
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcCommand {
    #[serde(rename = "prompt")]
    Prompt {
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        streaming_behavior: Option<StreamingBehavior>,
    },
    #[serde(rename = "steer")]
    Steer {
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
    },
    #[serde(rename = "follow_up")]
    FollowUp {
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
    },
    #[serde(rename = "abort")]
    Abort { id: Option<String> },
    #[serde(rename = "get_state")]
    GetState { id: Option<String> },
    #[serde(rename = "get_session_stats")]
    GetSessionStats { id: Option<String> },
}

/// Image content for prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub source: ImageSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub data: String,
    pub media_type: String,
}

/// Streaming behavior options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamingBehavior {
    Steer,
    FollowUp,
}

/// RPC response from Oh My Pi
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcResponse {
    #[serde(rename = "response")]
    Response {
        id: Option<String>,
        command: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

/// Event types emitted by Oh My Pi
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcEvent {
    #[serde(rename = "agent_start")]
    AgentStart {
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "agent_end")]
    AgentEnd {
        #[serde(default)]
        messages: Vec<serde_json::Value>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "turn_start")]
    TurnStart {
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "turn_end")]
    TurnEnd {
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "message_start")]
    MessageStart,
    #[serde(rename = "message_update")]
    MessageUpdate {
        #[serde(default)]
        message: Message,
        assistant_message_event: AssistantMessageEvent,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "message_end")]
    MessageEnd {
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "tool_execution_start")]
    ToolExecutionStart {
        tool_name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    #[serde(rename = "tool_execution_update")]
    ToolExecutionUpdate {
        tool_name: String,
        #[serde(default)]
        update: serde_json::Value,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "tool_execution_end")]
    ToolExecutionEnd {
        tool_name: String,
        #[serde(default)]
        result: serde_json::Value,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "extension_ui_request")]
    ExtensionUiRequest {
        id: String,
        method: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
    },
    #[serde(rename = "extension_error")]
    ExtensionError {
        extension_path: String,
        event: String,
        error: String,
        #[serde(default)]
        id: Option<String>,
    },
}

/// Message structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    pub role: String,
    #[serde(default)]
    pub content: Vec<serde_json::Value>,
}

/// Assistant message event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantMessageEvent {
    #[serde(rename = "text_delta")]
    TextDelta { delta: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { delta: String },
    #[serde(rename = "tool_call_delta")]
    ToolCallDelta {
        #[serde(default)]
        delta: serde_json::Value,
    },
}

impl RpcCommand {
    pub fn prompt(message: String) -> Self {
        RpcCommand::Prompt {
            id: None,
            message,
            images: None,
            streaming_behavior: None,
        }
    }

    pub fn with_id(mut self, id: String) -> Self {
        match &mut self {
            RpcCommand::Prompt { id: id_field, .. } |
            RpcCommand::Steer { id: id_field, .. } |
            RpcCommand::FollowUp { id: id_field, .. } |
            RpcCommand::Abort { id: id_field, .. } |
            RpcCommand::GetState { id: id_field, .. } |
            RpcCommand::GetSessionStats { id: id_field, .. } => {
                *id_field = Some(id);
            }
        }
        self
    }
}

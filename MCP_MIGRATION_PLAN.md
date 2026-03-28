# MCP Migration Plan: Discord Bridge to MCP Server

## Overview

Transform the `omp_discord_bridge` from a direct OMP RPC client into an **MCP (Model Context Protocol) server** that exposes Discord capabilities as tools. OMP will connect to this server via stdio transport, enabling agentic work through Discord.

## Target Architecture

```
OMP (MCP Client) <--stdio--> omp_discord_bridge (MCP Server) <--Discord API--> Discord
```

The current Discord bot functionality remains the backend - it just changes how it receives commands (from Discord mentions to MCP tool calls).

## New Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies (keep all)
tokio = { version = "1.40", features = ["full", "process"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serenity = { version = "0.12", default-features = false, features = ["client", "gateway", "rustls_backend", "model"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
uuid = { version = "1.11", features = ["v4", "serde"] }
async-trait = "0.1"
thiserror = "2.0"
dotenv = "0.15"

# NEW: MCP SDK
mcp = { version = "0.1", features = ["stdio", "tools"] }  # or rmcp if that's the crate name
```

**Note:** Verify the exact crate name on crates.io - candidates are `mcp`, `rmcp`, or `model-context-protocol`. The search results indicate multiple implementations exist.

## File Structure Changes

### New Files

```
src/
├── main.rs              # Modified: Remove OMP RPC client, add MCP server startup
├── config.rs            # Keep: Discord config remains the same
├── error.rs             # Extend: Add MCP-specific errors
├── discord.rs           # Refactor: Extract Discord operations into service layer
├── mcp/                 # NEW: MCP server implementation
│   ├── mod.rs           # Module exports
│   ├── server.rs        # MCP server setup and lifecycle
│   ├── tools.rs         # Tool definitions and handlers
│   └── transport.rs     # Stdio transport configuration
└── services/            # NEW: Business logic layer
    ├── mod.rs
    └── discord_service.rs  # Discord operations (send_message, read_channel, etc.)
```

### Modified Files

**`main.rs`** - Replace OMP RPC initialization with MCP server:
```rust
mod config;
mod discord;
mod error;
mod mcp;
mod services;

use crate::config::Config;
use crate::mcp::server::McpServer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    dotenv::dotenv().ok();
    let filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into())
        .add_directive("omp_discord_bridge=debug".parse()?);
    
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
    
    // Load configuration
    let config = Config::from_env()?;
    
    // Start MCP server (blocks until shutdown)
    McpServer::run(config).await?;
    
    Ok(())
}
```

**`discord.rs`** - Extract Discord operations into a service:
- Remove: Direct OMP RPC client integration
- Keep: Serenity client and event handlers
- Extract: Core Discord operations into `DiscordService` struct

## Tool Definitions

### 1. `send_message`

```json
{
  "name": "send_message",
  "description": "Send a message to a Discord channel",
  "inputSchema": {
    "type": "object",
    "properties": {
      "channel_id": {
        "type": "string",
        "description": "The Discord channel ID (snowflake)"
      },
      "content": {
        "type": "string",
        "description": "Message content to send"
      }
    },
    "required": ["channel_id", "content"]
  }
}
```

### 2. `read_channel`

```json
{
  "name": "read_channel",
  "description": "Read recent messages from a Discord channel",
  "inputSchema": {
    "type": "object",
    "properties": {
      "channel_id": {
        "type": "string",
        "description": "The Discord channel ID (snowflake)"
      },
      "limit": {
        "type": "integer",
        "description": "Number of messages to retrieve (default: 10, max: 100)",
        "default": 10
      }
    },
    "required": ["channel_id"]
  }
}
```

### 3. `list_servers`

```json
{
  "name": "list_servers",
  "description": "List all Discord servers the bot has access to",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### 4. `mention_user`

```json
{
  "name": "mention_user",
  "description": "Send a message mentioning a specific user",
  "inputSchema": {
    "type": "object",
    "properties": {
      "channel_id": {
        "type": "string",
        "description": "The Discord channel ID"
      },
      "user_id": {
        "type": "string",
        "description": "The user ID to mention"
      },
      "content": {
        "type": "string",
        "description": "Message content after the mention"
      }
    },
    "required": ["channel_id", "user_id", "content"]
  }
}
```

### 5. `post_file`

```json
{
  "name": "post_file",
  "description": "Upload and send a file to a Discord channel",
  "inputSchema": {
    "type": "object",
    "properties": {
      "channel_id": {
        "type": "string",
        "description": "The Discord channel ID"
      },
      "file_path": {
        "type": "string",
        "description": "Local file path to upload"
      },
      "description": {
        "type": "string",
        "description": "Optional description for the file"
      }
    },
    "required": ["channel_id", "file_path"]
  }
}
```

## Implementation Phases

### Phase 1: MCP SDK Integration (Day 1)

**Goals:** Get a minimal MCP server running with stdio transport

1. Research and select the right MCP Rust crate (`mcp` vs `rmcp` vs `model-context-protocol`)
2. Add dependency to `Cargo.toml`
3. Create basic MCP server skeleton in `src/mcp/server.rs`
4. Implement stdio transport
5. Test with a simple "ping" tool

**Acceptance Criteria:**
- Server compiles and starts
- Can be invoked via stdin/stdout
- Responds to MCP initialize request
- Exposes at least one working tool

### Phase 2: Discord Service Layer (Day 2)

**Goals:** Extract Discord operations into a clean service API

1. Create `src/services/discord_service.rs`
2. Move existing Discord logic from `discord.rs` into service methods:
   - `send_message(channel_id, content)`
   - `get_channel_messages(channel_id, limit)`
   - `get_servers()`
   - `mention_user(channel_id, user_id, content)`
   - `upload_file(channel_id, file_path, description)`
3. Ensure service can be shared between MCP tools and any remaining Discord handlers
4. Add proper error handling and validation

**Acceptance Criteria:**
- All 5 Discord operations are implemented as async methods
- Service is testable in isolation
- Error types are well-defined

### Phase 3: Tool Implementation (Day 3)

**Goals:** Connect MCP tools to Discord service methods

1. Implement tool handlers in `src/mcp/tools.rs`
2. Register all 5 tools with the MCP server
3. Add input validation and sanitization
4. Format tool responses according to MCP spec
5. Handle errors gracefully (return proper MCP error format)

**Acceptance Criteria:**
- All 5 tools are registered and discoverable via `tools/list`
- Each tool correctly calls the Discord service
- Responses are properly formatted as MCP results
- Errors are returned in MCP error format

### Phase 4: Integration & Testing (Day 4)

**Goals:** End-to-end testing with OMP as MCP client

1. Update documentation with new usage instructions
2. Create test script to verify all tools work
3. Test with actual OMP instance in MCP client mode
4. Verify error cases (invalid channel, missing permissions, etc.)
5. Performance testing: ensure no memory leaks or resource exhaustion

**Acceptance Criteria:**
- All tools work end-to-end with OMP
- Error handling is robust
- Documentation is complete
- No resource leaks under load

### Phase 5: Production Hardening (Day 5)

**Goals:** Prepare for production deployment

1. Add rate limiting to prevent abuse
2. Implement request timeouts
3. Add comprehensive logging and monitoring
4. Security review: validate all inputs, check permission scopes
5. Create deployment documentation (systemd service, Docker)

**Acceptance Criteria:**
- Rate limiting prevents API abuse
- All inputs are validated
- Security review complete
- Deployment docs are production-ready

## Environment Variables

Keep existing variables, add MCP-specific ones:

```bash
# Discord Bot Configuration (existing)
DISCORD_TOKEN=your_discord_bot_token_here
DISCORD_PREFIX=!  # Keep for backward compatibility if needed

# MCP Server Configuration (new)
MCP_LOG_LEVEL=info  # debug, info, warn, error
```

## Migration Considerations

### What Changes

- **Entry Point**: `main.rs` no longer spawns OMP subprocess; instead runs MCP server
- **Command Flow**: Commands come from MCP tool calls instead of Discord mentions
- **Response Format**: Responses use MCP result format instead of Discord messages

### What Stays the Same

- **Discord Bot**: Still connects to Discord with same intents
- **Serenity Usage**: All Discord API interactions remain the same
- **Configuration**: Same `.env` file structure for Discord credentials
- **Bot Presence**: Bot still appears in Discord servers the same way

### Backward Compatibility

**Option A: Dual Mode (Recommended)**
- Add `--mode mcp` flag to run as MCP server
- Add `--mode discord` flag to run as traditional bot
- Default to MCP mode (the new primary use case)

**Option B: Complete Cutover**
- Remove Discord mention command handling entirely
- All interaction happens via MCP tools
- Simpler codebase, but breaks existing usage pattern

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_send_message() {
        // Test message sending logic
    }
    
    #[tokio::test]
    async fn test_read_channel() {
        // Test channel reading logic
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    // Mock Discord API responses
    // Test full MCP tool call flow
    // Verify error handling
}
```

### Manual Testing

1. Start server: `cargo run --release`
2. Connect OMP in MCP client mode
3. List available tools: `tools/list`
4. Call each tool with test parameters
5. Verify responses match expected format

## Deployment

### Systemd Service

```ini
[Unit]
Description=OMP Discord Bridge MCP Server
After=network.target

[Service]
Type=simple
User=your_user
WorkingDirectory=/path/to/omp-discord-bridge
Environment="DISCORD_TOKEN=your_token"
Environment="RUST_LOG=info"
ExecStart=/path/to/omp-discord-bridge/target/release/omp_discord_bridge
Restart=always

[Install]
WantedBy=multi-user.target
```

### Docker Deployment

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/omp_discord_bridge /usr/local/bin/
CMD ["omp_discord_bridge"]
```

## Rollback Plan

If issues arise:

1. **Immediate**: Switch back to previous version using deployment system
2. **Short-term**: Keep old binary available as `omp_discord_bridge_legacy`
3. **Communication**: Document breaking changes for users

## Success Metrics

- All 5 tools are functional and tested
- Response time < 1 second for simple operations
- No memory leaks after 24 hours of operation
- Successful end-to-end integration with OMP
- Clean security review with no critical issues

---

**Next Steps:**

1. Verify the correct MCP Rust crate to use
2. Set up Phase 1 skeleton project
3. Begin implementation with smallest viable tool

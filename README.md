# Oh My Pi Discord Bridge

An MCP (Model Context Protocol) server that exposes Discord capabilities as tools to Oh My Pi (OMP) via stdio transport, enabling agentic work through Discord.

## Features

- 🤖 **MCP Server**: Exposes Discord as tools to OMP via stdio transport
- 🔧 **Tool Support**: Full Discord toolset (send_message, read_channel, list_servers, mention_user, post_file)
- 📡 **Stdio Transport**: Simple subprocess spawning from OMP
- 🛠️ **Serenity Backend**: Built on serenity v0.12 for robust Discord integration
- 🎯 **Type-Safe**: Full Rust type safety with rust-mcp-sdk

## Prerequisites

- Rust 1.70+ and Cargo
- A Discord bot token ([create one here](https://discord.com/developers/applications))
- OMP configured to connect to MCP servers via stdio
## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/ajaxdude/omp-discord-bridge.git
cd omp-discord-bridge
```

### 2. Install Dependencies and Build

```bash
cargo build --release
```

### 3. Configure Environment

```bash
cp .env.example .env
# Edit .env and add your Discord bot token
nano .env
```

## Prerequisites
## Configuration

Create a `.env` file in the project directory with the following variables:

```bash
# Required: Your Discord bot token
DISCORD_TOKEN=your_discord_bot_token_here
```

### Getting a Discord Bot Token

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application
3. Go to the "Bot" section
4. Click "Add Bot"
5. Copy the bot token
6. Enable the following Privileged Gateway Intents:
   - Message Content Intent
   - Server Members Intent (if needed)
7. Invite the bot to your server with the following scopes:
   - `bot`
   - `applications.commands`

## Usage

### Starting the MCP Server

```bash
# Development mode
cargo run

# Production mode
cargo run --release

# Or directly
./target/release/omp_discord_bridge
```

The server will start and wait for MCP protocol messages on stdio. OMP should be configured to spawn this binary as an MCP server subprocess.

### Available Tools

Once connected, OMP can use these Discord tools:

- **ping**: Test the connection
- **send_message**: Send a text message to a Discord channel
- **read_channel**: Retrieve recent messages from a channel
- **list_servers**: List all Discord servers the bot has access to
- **mention_user**: Send a message mentioning a specific user
- **post_file**: Upload and send a file to a Discord channel

### Example OMP Configuration

In OMP's configuration, add this MCP server:

```json
{
  "mcp_servers": {
    "discord": {
      "command": "/path/to/omp_discord_bridge",
      "args": [],
      "transport": "stdio"
    }
  }
}
```

## Architecture

```
OMP (MCP Client) ──stdio──► Discord Bridge (MCP Server) ──Discord API──► Discord
                                  │
                                  ▼
                            Serenity Bot
```

### Components

- **MCP Server**: Handles MCP protocol over stdio using rust-mcp-sdk
- **Discord Service**: Core Discord operations wrapped in a service layer
- **Tool Handlers**: Map MCP tool calls to Discord service methods
- **Serenity Client**: Manages Discord connection and events

## Development

### Project Structure

```
src/
├── main.rs              # Entry point, MCP server startup
├── config.rs            # Configuration management
├── error.rs             # Error types
├── mcp/                 # MCP server implementation
│   ├── mod.rs
│   ├── server.rs        # MCP server setup and lifecycle
│   └── tools.rs         # Tool definitions and handlers
└── services/            # Business logic layer
    ├── mod.rs
    └── discord_service.rs  # Discord operations
```

### Running Tests

```bash
cargo test
```

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

For even more verbose logging:

```bash
RUST_LOG=trace cargo run
```

## Troubleshooting

### Bot doesn't start

1. Check that `DISCORD_TOKEN` is set correctly in `.env`
2. Verify the bot has Message Content Intent enabled
3. Check the logs for any errors

### MCP connection fails

1. Ensure OMP is configured to use stdio transport
2. Verify the binary path in OMP configuration is correct
3. Check that the Discord token is valid (bot connects successfully)

### Tools don't respond

1. Check that the bot has permission to read/write in the target channels
2. Verify channel IDs are correct (use Discord developer mode to copy IDs)
3. Check logs for error messages

## Deployment

### Using systemd

Create a systemd service file at `/etc/systemd/system/omp-discord-bridge.service`:

```ini
[Unit]
Description=Oh My Pi Discord Bridge MCP Server
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

Then:

```bash
sudo systemctl daemon-reload
sudo systemctl enable omp-discord-bridge
sudo systemctl start omp-discord-bridge
```

### Using Docker

Create a `Dockerfile`:

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

Build and run:

```bash
docker build -t omp-discord-bridge .
docker run -d --name omp-discord-bridge -e DISCORD_TOKEN=your_token omp-discord-bridge
```

## License

This project is provided as-is for educational and personal use.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Uses [Serenity](https://github.com/serenity-rs/serenity) for Discord integration
- Uses [rust-mcp-sdk](https://github.com/modelcontextprotocol/rust-sdk) for MCP protocol
- Integrates with [Oh My Pi](https://github.com/yourusername/omp) for AI-powered coding assistance

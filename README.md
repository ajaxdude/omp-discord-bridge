# Oh My Pi Discord Bridge

A Discord bot that bridges Discord messages to Oh My Pi (OMP) via RPC, allowing you to interact with OMP's coding agent directly from Discord.

## Features

- 🤖 **Discord Integration**: Interact with Oh My Pi directly from Discord
- 🔄 **RPC Protocol**: Uses OMP's native RPC mode for low-latency communication
- 📡 **Event Streaming**: Real-time streaming of OMP responses to Discord
- 🛠️ **Full Tool Support**: Access all of OMP's tools (read, grep, find, edit, write, etc.)
- 🎯 **Correlation Tracking**: Proper request-response correlation for reliable communication

## Prerequisites

- Rust 1.70+ and Cargo
- Oh My Pi (OMP) installed and accessible in your PATH
- A Discord bot token ([create one here](https://discord.com/developers/applications))

## Installation

1. **Clone the repository** (if applicable):
   ```bash
   cd ~/ai/projects/omp-discord-bridge
   ```

2. **Build the project**:
   ```bash
   cargo build --release
   ```

3. **Configure environment variables**:
   ```bash
   cp .env.example .env
   # Edit .env and add your Discord bot token
   ```

## Configuration

Create a `.env` file in the project directory with the following variables:

```bash
# Required: Your Discord bot token
DISCORD_TOKEN=your_discord_bot_token_here

# Optional: Command prefix (default: "!")
DISCORD_PREFIX=!

# Optional: Path to OMP executable (default: "omp")
OMP_PATH=omp
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

### Starting the Bot

```bash
# Development mode
cargo run

# Production mode
cargo run --release

# Or directly
./target/release/omp_discord_bridge
```

### Discord Commands

Once the bot is running and invited to your server, you can use these commands:

- `!ping` - Test bot connectivity
- `!help` - Show available commands
- `!omp <message>` - Send a message to Oh My Pi

### Example Interactions

```
User: !omp List all files in the current directory
Bot: Processing...
Bot: [OMP response with file listing]

User: !omp Find all TODO comments in this repo
Bot: Processing...
Bot: [OMP searches and lists TODOs]
```

## Architecture

```
Discord ──► Discord Bot ──► RPC Client ──► Oh My Pi (RPC Mode)
              ▲                  │
              │                  ▼
         Event Streamer ◄──── Events
```

### Components

- **RPC Client**: Manages the OMP subprocess and handles RPC communication
- **Discord Bot**: Handles Discord events and message routing
- **Event Streamer**: Listens to OMP events and streams responses to Discord
- **Configuration**: Manages environment-based configuration

### RPC Protocol

The bot uses OMP's RPC mode (`omp --mode rpc`), which communicates via newline-delimited JSON over stdin/stdout:

- **Commands**: JSON objects sent to stdin (prompt, steer, abort, etc.)
- **Responses**: JSON objects received on stdout with correlation IDs
- **Events**: Real-time events (agent_start, message_update, tool_execution, etc.)

## Development

### Project Structure

```
src/
├── main.rs         # Entry point
├── config.rs       # Configuration management
├── error.rs        # Error types
├── discord.rs      # Discord bot implementation
└── rpc/
    ├── mod.rs      # RPC module exports
    ├── types.rs    # RPC protocol types
    └── client.rs   # RPC client implementation
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

### Bot doesn't respond to commands

1. Check that the bot has Message Content Intent enabled
2. Verify the bot can read messages in the channel
3. Check the logs for any errors

### OMP subprocess fails to start

1. Ensure OMP is installed and accessible in your PATH
2. Check that `omp --mode rpc` works manually
3. Verify the OMP_PATH environment variable

### Long responses are cut off

Discord has a 2000 character limit per message. Long OMP responses may be truncated. This is a known limitation.

### Bot crashes or disconnects

1. Check the logs for error messages
2. Ensure the Discord bot token is valid
3. Verify network connectivity

## Deployment

### Using systemd

Create a systemd service file at `/etc/systemd/system/omp-discord-bridge.service`:

```ini
[Unit]
Description=Oh My Pi Discord Bridge
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
- Integrates with [Oh My Pi](https://github.com/yourusername/omp) for AI-powered coding assistance

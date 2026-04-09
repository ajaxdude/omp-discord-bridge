# Oh My Pi Discord Bridge

A robust, standalone Discord bot and MCP (Model Context Protocol) server that bridges [Oh My Pi (OMP)](https://github.com/yourusername/omp) with Discord. 

This bridge allows you to interact with OMP directly through Discord channels, whether you're using powerful cloud models or local inferences via `llama.cpp`. 

## Features

- 🤖 **Native Chat Experience**: Chat naturally in Discord without needing command prefixes or mentions (unless you want to!).
- 🧠 **Dynamic Model Switching**: Need more reasoning power? Switch models on the fly by adding `--model llama.cpp` or `--model gpt-4o` to your Discord messages.
- 🐳 **Docker & Docker Compose**: Fully containerized for easy deployment and background execution.
- 📡 **MCP Server Compatibility**: Acts as an MCP server over stdio for deep integration with OMP's agentic workflows.
- 🛠️ **Full Tool Support**: Send messages, read channels, upload files, mention users, and ping for latency.

## Prerequisites

- A Discord bot token ([create one here](https://discord.com/developers/applications))
- **Important**: Ensure you enable the **Message Content Intent** under the Privileged Gateway Intents in the Discord Developer Portal for your bot.
- Docker and Docker Compose (recommended for deployment) OR Rust 1.70+ and [Bun](https://bun.sh/) (for local execution).

## Deployment (Recommended)

The easiest way to run the bridge persistently in the background is via Docker Compose.

### 1. Clone the Repository

```bash
git clone https://github.com/ajaxdude/omp-discord-bridge.git
cd omp-discord-bridge
```

### 2. Configure Environment

Copy the example environment file and add your Discord token:

```bash
cp .env.example .env
nano .env
```

Ensure your `.env` file contains at minimum:
```env
DISCORD_TOKEN=your_discord_bot_token_here
```

### 3. Start with Docker Compose

Build and launch the container in the background:

```bash
docker compose up -d --build
```

The container automatically installs `oh-my-pi`, compiles the Rust bridge, and starts listening to Discord. It mounts your `~/.omp/agent` directory as a volume so that context and sessions are preserved across restarts.

To view logs:
```bash
docker compose logs -f
```

## Local Development & Usage

If you prefer to run the bridge directly on your host machine without Docker:

### 1. Install Dependencies

Ensure you have Rust, Cargo, Bun, and Oh My Pi installed globally:

```bash
curl -fsSL https://bun.sh/install | bash
bun install -g oh-my-pi
cargo build --release
```

### 2. Run the Bridge

Start the bridge in the background so it stays alive:

```bash
RUST_LOG=info nohup ./target/release/omp_discord_bridge </dev/null >/tmp/omp-bridge.log 2>&1 &
```

## Using the Bot in Discord

Once the bot is online in your server, simply type your question or command into the channel where the bot has access. 

**Example Queries:**
- `Write a Python script that calculates the Fibonacci sequence.`
- `!ping` (Returns the current one-way latency, e.g., `Pong! 0.123s`)
- `--model claude-sonnet summarize the main themes of cybernetics.`

## Architecture

```
User ──Discord Message──► Discord API ──WebSocket──► OMP Discord Bridge
                                                            │
                                                     Spawns `omp -p`
                                                            │
                                                            ▼
                                                        Oh My Pi 
                                                     (Local/Cloud LLM)
```

## Troubleshooting

- **Bot doesn't respond to messages**: Ensure the **Message Content Intent** is enabled in the Discord Developer portal.
- **OMP timeouts**: The bridge enforces a 20-minute timeout to allow local models (like `llama.cpp`) plenty of time to process complex queries. Ensure your local LLM server is reachable if using custom local models.

## License

This project is provided as-is for educational and personal use.

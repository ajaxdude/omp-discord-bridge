# Oh My Pi Discord Bridge

A robust, standalone Discord bot and MCP (Model Context Protocol) server that bridges [Oh My Pi (OMP)](https://github.com/yourusername/omp) with Discord. 

This bridge allows you to interact with OMP directly through Discord channels, whether you're using powerful cloud models or local inferences via `llama.cpp`. 

## Features

- 🤖 **Native Chat Experience**: Chat naturally in Discord without needing command prefixes or mentions (unless you want to!).
- 🧠 **Dynamic Model Switching**: Need more reasoning power? Switch models on the fly by adding `--model llama.cpp` or `--model gpt-4o` to your Discord messages.
- ⚙️ **Host-Native Background Execution**: Runs directly on your machine as a `systemd` service so it can perfectly reuse your existing `omp` installation and local `llama.cpp` models without any containerization headaches or duplication!
- 📡 **MCP Server Compatibility**: Acts as an MCP server over stdio for deep integration with OMP's agentic workflows.
- 🛠️ **Full Tool Support**: Send messages, read channels, upload files, mention users, and ping for latency.

## Prerequisites

- A Discord bot token ([create one here](https://discord.com/developers/applications))
- **Important**: Ensure you enable the **Message Content Intent** under the Privileged Gateway Intents in the Discord Developer Portal for your bot.
- Rust 1.70+ and [Bun](https://bun.sh/) installed locally on your host machine.
- Your existing installation of `oh-my-pi` (`omp`).

## Deployment (Recommended: systemd)

The absolute best way to run this bridge is natively on your machine using a user-level `systemd` service. This ensures the bot starts automatically on boot and has full access to your host's `omp` tools, plugins, and local `llama.cpp` processes.

### 1. Clone and Build

```bash
git clone https://github.com/ajaxdude/omp-discord-bridge.git
cd omp-discord-bridge
cargo build --release
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

### 3. Install and Start the Service

We've provided a simple script to register the bridge as a background service:

```bash
./install-service.sh
```

The script will configure `systemd` to keep the bot alive and launch it automatically. 

**Useful Commands:**
- **View Live Logs:** `journalctl --user -u omp-discord-bridge.service -f`
- **Stop Bot:** `systemctl --user stop omp-discord-bridge.service`
- **Restart Bot:** `systemctl --user restart omp-discord-bridge.service`

## Local Development / Manual Run

If you just want to run the bridge directly in your terminal for testing:

```bash
RUST_LOG=info ./target/release/omp_discord_bridge
```

## Using the Bot in Discord

Once the bot is online in your server, simply type your question or command into the channel where the bot has access. 

**Example Queries:**
- `Write a Python script that calculates the Fibonacci sequence.`
- `!ping` (Returns the current one-way latency, e.g., `Pong! 0.123s`)
- `--model llama.cpp summarize the main themes of cybernetics.`

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

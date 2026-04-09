# Oh My Pi Discord Bridge

A standalone Discord bot and MCP (Model Context Protocol) server that bridges [Oh My Pi (OMP)](https://github.com/ajaxdude/omp) with Discord. Send coding tasks from any Discord channel, get the agent's full response back — with persistent session continuity so the conversation carries across messages.

## Features

- **Persistent sessions per channel** — each Discord channel maintains its own OMP session, so the agent remembers what you were working on across messages. Sessions survive service restarts.
- **Dynamic model switching** — switch models on the fly with `--model` in any message.
- **Boot automation** — an `omp-update` systemd service runs `update-ai.sh` at login to keep OMP and your toolboxes current before the bridge starts.
- **MCP server** — also acts as an MCP server over stdio so OMP agents can read channels, send messages, and upload files as tools.
- **Host-native execution** — runs directly on your machine as a user-level `systemd` service, reusing your existing `omp` install and local `llama.cpp` models.

## Prerequisites

- A Discord bot token ([create one here](https://discord.com/developers/applications))
- Enable **Message Content Intent** under Privileged Gateway Intents in the Discord Developer Portal
- Rust 1.70+ and your existing `oh-my-pi` (`omp`) installation

## Setup

### 1. Clone and build

```bash
git clone https://github.com/ajaxdude/omp-discord-bridge.git
cd omp-discord-bridge
cargo build --release
```

### 2. Configure

```bash
cp .env.example .env
$EDITOR .env
```

Minimum required:

```env
DISCORD_TOKEN=your_discord_bot_token_here
```

See [Configuration](#configuration) for all options.

### 3. Install the service

```bash
./install-service.sh
```

This registers the bridge as a user-level `systemd` service that starts automatically on login.

### 4. Boot automation — keep OMP up to date

Create a one-shot service that runs your update script before the bridge starts:

```bash
# ~/.config/systemd/user/omp-update.service
[Unit]
Description=Oh My Pi and llama toolbox update
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/home/<user>/ai
ExecStart=/home/<user>/ai/update-ai.sh
Environment="PATH=/home/<user>/.bun/bin:/home/<user>/.cargo/bin:/home/<user>/.local/bin:/usr/local/bin:/usr/bin"
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

Enable it:

```bash
systemctl --user daemon-reload
systemctl --user enable omp-update.service
```

The bridge service already declares `After=omp-update.service` and `Wants=omp-update.service`, so OMP is always current before the bot comes online.

## Discord commands

All commands require the `!omp` prefix or an @mention of the bot. The prefix can be changed with the `DISCORD_PREFIX` env var.

| Command | Description |
|---|---|
| `!ping` | Health check — returns one-way latency, e.g. `Pong! 0.042s` |
| `!omp <query>` | Send a query to the OMP agent. The full response is sent back to the channel. |
| `@ompbot <query>` | Same as `!omp <query>`, triggered by mentioning the bot. |
| `!omp --model <name> <query>` | Use a specific model for this query, e.g. `--model opus` or `--model llama.cpp`. |
| `!omp reset` | Clear the active OMP session for this channel. The next message starts a fresh conversation. Use this when switching projects. |

### Session continuity

Each channel has its own OMP session. The bridge passes `--resume <session-id>` to `omp` on every message so the agent retains the full conversation history. Session IDs are persisted to:

```
~/.local/share/omp-discord-bridge/sessions.json
```

If a session becomes invalid (e.g. session files were cleaned up), the bridge automatically clears it and the next message starts fresh. You can also clear a session manually with `!omp reset`.

### Examples

```
!omp write a Python script that tails a log file and highlights errors
!omp --model opus refactor this function to use async/await
!omp what did we decide about the database schema earlier?
!omp reset
@ompbot explain the difference between Arc and Rc in Rust
```

## Service management

```bash
# View live logs
journalctl --user -u omp-discord-bridge.service -f

# View update logs
journalctl --user -u omp-update.service

# Stop / start / restart
systemctl --user stop omp-discord-bridge.service
systemctl --user start omp-discord-bridge.service
systemctl --user restart omp-discord-bridge.service
```

## Configuration

All options are set in `.env` in the project root.

| Variable | Default | Description |
|---|---|---|
| `DISCORD_TOKEN` | *(required)* | Your Discord bot token |
| `DISCORD_PREFIX` | `!` | Command prefix for bot commands |
| `OMP_PATH` | `omp` | Path to the `omp` binary (resolved via `PATH` if not absolute) |
| `OMP_WORK_DIR` | `$HOME` | Working directory for OMP subprocesses. Set to your project root so the agent's file tools resolve relative to the right place. |

## Architecture

```
Discord Message
      │
      ▼
Discord Gateway (serenity)
      │
      │  !omp <query>  or  @mention <query>
      ▼
invoke_omp()
  omp -p --mode json [--resume <session-id>] <query>
      │
      │  NDJSON stdout
      ▼
parse_omp_json_output()
  extracts assistant text blocks only
  (tool calls, tool results, thinking → discarded)
      │
      ▼
send_chunked() → Discord reply (≤1900 bytes per message)
      │
      ▼
session_id saved to ~/.local/share/omp-discord-bridge/sessions.json
```

The bridge also runs as an MCP server over stdio, exposing Discord tools (`send_message`, `read_channel`, `list_servers`, `mention_user`, `post_file`) to any connected OMP agent.

## Local development

```bash
RUST_LOG=debug ./target/release/omp_discord_bridge
```

## Troubleshooting

- **Bot doesn't respond** — check that Message Content Intent is enabled in the Discord Developer Portal.
- **Empty responses** — make sure `omp` is on the PATH the service uses. Check `OMP_PATH` in `.env` or set it to an absolute path.
- **Session errors** — run `!omp reset` in the affected channel to clear the stale session.
- **OMP timeouts** — the bridge enforces a 20-minute timeout. If your local model needs more time, the timeout is hardcoded in `discord_service.rs` (`1200` seconds).

## License

This project is provided as-is for personal and educational use.

# Oh My Pi Discord Bridge

A standalone Discord bot and MCP (Model Context Protocol) server that bridges [Oh My Pi (OMP)](https://github.com/ajaxdude/omp) with Discord. Send queries from any Discord channel, get the agent's full response back — with persistent session continuity so the conversation carries across messages.

## Features

- **Persistent sessions per channel** — each Discord channel maintains its own OMP session so the agent remembers context across messages. Sessions survive service restarts.
- **Sandboxed workspace navigation** — each channel tracks its own working directory. Use `!omp ls`, `!omp cd`, and `!omp ..` to navigate the project tree. Navigation is strictly limited to the configured `OMP_WORK_DIR` sandbox.
- **Local model routing** — `--model gemma` or `--model qwen` routes directly to your local llama-swap instance. Short aliases are resolved from a config file you edit without recompiling.
- **Dynamic model switching** — switch models on the fly with `--model` in any message. Fully-qualified OMP model IDs (e.g. `llama.cpp/gemma-4-31b-draft`) always pass through unchanged.
- **MCP server** — also acts as an MCP server over stdio so OMP agents can read channels, send messages, and upload files as tools.
- **Host-native execution** — runs directly on your machine as a user-level `systemd` service, reusing your existing `omp` install and local models.
- **Singleton enforcement** — a lockfile (`flock` on `/tmp/omp-discord-bridge.lock`) ensures only one live bot instance holds the Discord gateway, even if OMP spawns extra bridge processes.

## Prerequisites

- A Discord bot token ([create one here](https://discord.com/developers/applications))
- Enable **Message Content Intent** under Privileged Gateway Intents in the Discord Developer Portal
- Rust 1.70+ and your existing `oh-my-pi` (`omp`) installation
- [llama-swap](https://github.com/mostlygeek/llama-swap) running locally if you want local model support

## Architecture

Discord message
      │
      ▼
Discord Gateway (serenity)
      │
      │  parse --model <alias> | cd <dir> | ls | ..
      │  resolve_model() ──► ~/.config/omp-discord-bridge/config.yaml
      ▼
invoke_omp()
  omp -p --mode json [--resume <session-id>] [--model <id>] <query>
      │
      │  NDJSON stdout
      ▼
parse_omp_json_output()
  extracts assistant text blocks
  (tool calls, tool results, thinking → discarded)
      │
      ▼
send_chunked() → Discord reply (≤1900 bytes per message)
      │
      ▼
session_id & work_dir saved to ~/.local/share/omp-discord-bridge/

### Local model path (llama-swap)

OMP v14's `llama.cpp` provider calls `/responses` (no `/v1` prefix). llama-swap only
registers `/v1/responses`. A thin Python proxy bridges the gap:

```
OMP  →  :8080 (llama-swap-proxy)  →  :8081 (llama-swap)  →  llama-server
```

The proxy rewrites three paths and forwards everything else unchanged:

| Incoming | Forwarded to upstream |
|---|---|
| `GET /models` | `GET /v1/models` |
| `POST /responses` | `POST /v1/responses` |
| `POST /chat/completions` | `POST /v1/chat/completions` |
| anything else | unchanged |

The `/models` rewrite is critical: OMP's model discovery hits `GET /models` (no `/v1`
prefix) on startup. Without this rewrite OMP gets a 404, marks the llama.cpp provider
unavailable, and silently falls back to Claude for every `--model gemma/qwen` request.

Both `llama-swap` and `llama-swap-proxy` run as user-level systemd services and start on boot.

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

### 3. Install the bridge service

```bash
./install-service.sh
```

This registers and starts the bridge as a user-level `systemd` service that launches automatically on login.

### 4. Set up the llama-swap proxy

If you use local models via llama-swap, move llama-swap to port 8081 and run the proxy on 8080:

**`~/.config/systemd/user/llama-swap.service`** — add `--listen 0.0.0.0:8081` to the `ExecStart` line.

**`~/.config/systemd/user/llama-swap-proxy.service`**:

```ini
[Unit]
Description=llama-swap proxy — rewrites /responses → /v1/responses for OMP compatibility
After=llama-swap.service
Requires=llama-swap.service

[Service]
Type=simple
ExecStart=/usr/bin/python3 %h/.local/bin/llama-swap-proxy.py
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
```

The proxy script is at `~/.local/bin/llama-swap-proxy.py` (installed alongside this repo).

```bash
systemctl --user daemon-reload
systemctl --user enable --now llama-swap-proxy.service
```

### 5. Configure model aliases

Edit `~/.config/omp-discord-bridge/config.yaml` to map short names to canonical OMP model IDs:

```yaml
model_aliases:
  gemma: "llama.cpp/gemma-4-31b-draft"
  qwen:  "llama.cpp/qwen3-coder-next"
```

- Keys are matched **case-insensitively as substrings**: `gemma`, `Gemma`, `gemma4` all resolve to `llama.cpp/gemma-4-31b-draft`.
- Fully-qualified names (containing `/` or `.`) always pass through unchanged — no entry needed.
- **No recompile required** to add a new model. Edit the file and restart the service.

```bash
systemctl --user restart omp-discord-bridge.service
```

## Discord commands

All commands require the `!omp` prefix or an @mention of the bot. The prefix can be changed with `DISCORD_PREFIX` in `.env`.

| Command | Description |
|---|---|
| `!ping` | Health check — returns one-way latency, e.g. `Pong! 0.042s` |
| `!omp <query>` | Send a query to the OMP agent using the default model. |
| `@ompbot <query>` | Same as `!omp <query>`, triggered by mentioning the bot. |
| `!omp --model <alias> <query>` | Use a specific model for this query (see alias table below). |
| `!omp ls` | List files in the current channel's working directory. |
| `!omp cd <dir>` | Change the working directory (relative or `/` for root). Sandboxed to `OMP_WORK_DIR`. |
| `!omp ..` | Move up one directory level (stops at `OMP_WORK_DIR`). |
| `!omp reset` | Clear the active OMP session for this channel. The next message starts a fresh conversation. |

### Model alias examples

```
!omp --model gemma what is the capital of France?
!omp --model qwen  write a bubble sort in Rust
!omp --model llama.cpp/gemma-4-31b-draft explain speculative decoding
@ompbot --model gemma summarise this code
```

The alias `gemma` resolves to `llama.cpp/gemma-4-31b-draft` via the config file. The full OMP model ID is accepted directly and needs no alias entry.

### Session continuity

Each channel has its own OMP session. The bridge passes `--resume <session-id>` to `omp` on every message so the agent retains full conversation history. Session IDs are persisted to:

```
~/.local/share/omp-discord-bridge/sessions.json
```

If a session becomes invalid (e.g. session files were cleaned up), the bridge automatically clears it and the next message starts fresh. Use `!omp reset` to manually clear a session when switching projects.

## Service management

```bash
# View live bridge logs
journalctl --user -u omp-discord-bridge.service -f

# View proxy logs
journalctl --user -u llama-swap-proxy.service -f

# Stop / start / restart the bridge
systemctl --user stop    omp-discord-bridge.service
systemctl --user start   omp-discord-bridge.service
systemctl --user restart omp-discord-bridge.service

# Restart after editing config.yaml (no recompile needed)
systemctl --user restart omp-discord-bridge.service
```

## Configuration

All options are set in `.env` in the project root.

| Variable | Default | Description |
|---|---|---|
| `DISCORD_TOKEN` | *(required)* | Your Discord bot token |
| `DISCORD_PREFIX` | `!` | Command prefix for bot commands |
| `OMP_PATH` | `omp` | Path to the `omp` binary (resolved via `PATH` if not absolute) |
| `OMP_WORK_DIR` | `$HOME` | Sandbox root and default working directory for OMP subprocesses. The agent cannot navigate above this path. |
| `BRIDGE_CONFIG` | `~/.config/omp-discord-bridge/config.yaml` | Path to the bridge YAML config file (model aliases). |

## Adding a new local model

1. Add the model to `~/.config/llama-swap/config.yaml` under `models:`.
2. Add an alias entry to `~/.config/omp-discord-bridge/config.yaml`:
   ```yaml
   model_aliases:
     mymodel: "llama.cpp/<llama-swap-model-id>"
   ```
3. Restart both services:
   ```bash
   systemctl --user restart llama-swap.service llama-swap-proxy.service omp-discord-bridge.service
   ```

That's it — no recompile required.

## Local development

```bash
RUST_LOG=debug ./target/release/omp_discord_bridge
```

## Troubleshooting

- **Bot doesn't respond** — check that Message Content Intent is enabled in the Discord Developer Portal.
- **Claude answers instead of local model** — the alias wasn't resolved. Check `~/.config/omp-discord-bridge/config.yaml` exists and contains the right key. Restart the service after editing.
- **404 from llama-swap** — OMP is hitting `:8080` but `llama-swap-proxy` isn't running. Check `systemctl --user status llama-swap-proxy.service`.
- **Empty responses** — make sure `omp` is on the PATH the service uses. Check `OMP_PATH` in `.env` or set it to an absolute path.
- **Session errors** — run `!omp reset` in the affected channel to clear the stale session.
- **OMP timeouts** — the bridge enforces a 20-minute timeout per query. This is hardcoded as `1200` seconds in `discord_service.rs`.

## License

This project is provided as-is for personal and educational use.

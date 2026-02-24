# Quick Reference Card - Discord Bridge Testing

## Setup Checklist (Run this first!)
```bash
./test_checklist.sh
```

## Quick Setup Commands

### 1. Create .env file
```bash
cp .env.example .env
nano .env  # Add your DISCORD_TOKEN
```

### 2. Build the bot
```bash
cargo build --release
```

### 3. Run the bot (development)
```bash
cargo run
```

### 4. Run the bot (production)
```bash
./target/release/omp_discord_bridge
```

### 5. Run with logs
```bash
RUST_LOG=debug ./target/release/omp_discord_bridge
```

---

## Essential Discord Commands

### In Discord (to test bot):
```bash
!ping              # Test connectivity
!help              # Show help
!omp <message>     # Send query to OMP
```

---

## Common Issues & Quick Fixes

### Issue: Bot doesn't respond
```bash
# Check bot is running
ps aux | grep omp_discord_bridge

# Check permissions in Discord
# Server Settings → Roles → Bot Role
# Enable: Read Messages, Send Messages

# Check Message Content Intent
# Discord Developer Portal → Bot → Privileged Gateway Intents
# Enable: Message Content Intent
```

### Issue: "Failed to connect to Oh My Pi"
```bash
# Test OMP manually
omp --mode rpc

# Check OMP in PATH
which omp

# Test RPC mode with correlation ID
echo '{"type":"prompt","id":"test","message":"hello"}' | omp --mode rpc
```

### Issue: Wrong responses in channels
```bash
# This is the critical correlation ID bug we fixed
# Run with debug logs to verify
RUST_LOG=debug ./target/release/omp_discord_bridge

# Look for:
# Registered correlation ID: uuid-xxx for channel YYY
# Received event with correlation ID uuid-xxx
```

---

## Testing Commands (Terminal)

### Check OMP installation
```bash
omp --version
which omp
omp --mode rpc --help
```

### Test OMP RPC mode manually
```bash
# Simple test
echo '{"type":"prompt","message":"Hello"}' | omp --mode rpc

# Test with correlation ID
echo '{"type":"prompt","id":"test-123","message":"Hello"}' | omp --mode rpc
```

### Check bot binary
```bash
ls -lh target/release/omp_discord_bridge
file target/release/omp_discord_bridge
```

### Monitor logs
```bash
# If running in foreground
# Logs appear directly in terminal

# If running in background
tail -f bot.log

# Check recent errors
grep -i error bot.log | tail -20
```

---

## Discord Bot Setup URLs

- **Discord Developer Portal**: https://discord.com/developers/applications
- **Create Bot**: Applications → New Application → Bot → Add Bot
- **Get Token**: Bot → Reset Token → Copy
- **Enable Intents**: Bot → Privileged Gateway Intents → Message Content Intent ✅
- **Generate Invite URL**: OAuth2 → URL Generator → Select scopes → Copy URL
- **Invite Bot**: Paste URL in browser → Select server → Authorize

---

## Required Discord Permissions

For the bot role:
- ✅ Read Messages/View Channels
- ✅ Send Messages
- ✅ Read Message History
- ✅ Add Reactions (optional)

For the bot user:
- ✅ Message Content Intent (CRITICAL!)

---

## File Structure

```
omp-discord-bridge/
├── src/
│   ├── main.rs           # Entry point
│   ├── discord.rs        # Discord bot implementation
│   ├── rpc/
│   │   ├── client.rs     # RPC client
│   │   └── types.rs      # RPC types
│   ├── config.rs         # Configuration
│   └── error.rs          # Error types
├── target/release/
│   └── omp_discord_bridge  # Compiled binary
├── .env                  # Environment variables (create this!)
├── .env.example          # Example configuration
├── DETAILED_TESTING_GUIDE.md  # Comprehensive testing guide
├── SESSION_SUMMARY.md    # Session context
├── README.md             # Project documentation
└── test_checklist.sh     # Setup checker script
```

---

## Environment Variables

```bash
# Required
DISCORD_TOKEN=MTAwNT...your_token_here

# Optional
DISCORD_PREFIX=!
OMP_PATH=omp
```

---

## Log Levels

```bash
# Minimal (errors only)
RUST_LOG=error ./target/release/omp_discord_bridge

# Normal (info)
RUST_LOG=info ./target/release/omp_discord_bridge

# Verbose (debug)
RUST_LOG=debug ./target/release/omp_discord_bridge

# Very verbose (trace)
RUST_LOG=trace ./target/release/omp_discord_bridge
```

---

## Process Management

### Run in background
```bash
nohup ./target/release/omp_discord_bridge > bot.log 2>&1 &
```

### Check if running
```bash
ps aux | grep omp_discord_bridge
```

### Stop the bot
```bash
pkill omp_discord_bridge
# Or find PID and kill:
ps aux | grep omp_discord_bridge
kill <PID>
```

### Restart the bot
```bash
pkill omp_discord_bridge
./target/release/omp_discord_bridge
```

---

## Git Commands

```bash
# Check status
git status

# View commits
git log --oneline -5

# View changes
git diff

# Pull latest (if working with team)
git pull origin master

# Commit new changes
git add .
git commit -m "description"
```

---

## Testing Scenarios Quick Reference

### 1. Basic Test
```
Discord: !ping
Expected: Pong!
```

### 2. Simple OMP Query
```
Discord: !omp What is 2+2?
Expected: 2+2 equals 4.
```

### 3. Multi-User Test (CRITICAL!)
```
User A: !omp What is 2+2?
User B: !omp What is 3+3?
Expected: A gets "4", B gets "6" (in separate channels)
```

### 4. Tool Usage Test
```
Discord: !omp List all files in current directory
Expected: [OMP uses 'ls' or 'find' tool]
```

### 5. Error Handling Test
```
Discord: !xyz
Expected: Unknown command. Type `!help` for available commands.
```

---

## Expected Startup Logs

```
✅ Starting Oh My Pi Discord Bridge
✅ Configuration loaded successfully
✅ Connecting to Oh My Pi...
✅ Connected to Oh My Pi
✅ Initializing Discord bot...
✅ Discord bot initialized
✅ Starting Discord bot...
✅ Connected as YourBotName#1234
```

## Expected Runtime Logs (when user sends command)

```
DEBUG Received command: omp test
DEBUG Registered correlation ID: uuid-abc123 for channel 123456789
DEBUG Received RPC event: AgentStart
DEBUG Received RPC event: MessageUpdate
DEBUG Received RPC event: AgentEnd
DEBUG Final response: 45 chars
```

---

## Performance Benchmarks

- Startup: < 2 seconds
- !ping: < 500ms
- !omp simple: 2-5 seconds
- !omp complex: 5-30 seconds
- Memory: ~50-100 MB

---

## Documentation Files

| File | Purpose |
|------|---------|
| `SESSION_SUMMARY.md` | What was accomplished in this session |
| `DETAILED_TESTING_GUIDE.md` | Comprehensive 19,000-char testing guide |
| `QUICK_REFERENCE.md` | This file - quick lookup |
| `README.md` | Project overview and documentation |
| `TESTING_GUIDE.md` | Original testing guide |
| `test_checklist.sh` | Automated setup checker |

---

## When to Use Which Document

- **First time setup**: Read `DETAILED_TESTING_GUIDE.md`
- **Quick reference**: Keep `QUICK_REFERENCE.md` open
- **Session context**: Read `SESSION_SUMMARY.md`
- **Automated checks**: Run `./test_checklist.sh`
- **Project overview**: Read `README.md`

---

## Common Commands Summary

```bash
# Setup
./test_checklist.sh                    # Check setup
cp .env.example .env                   # Create config
nano .env                              # Edit config

# Build
cargo build --release                  # Build release binary

# Run
cargo run                              # Dev mode
./target/release/omp_discord_bridge    # Production mode
RUST_LOG=debug ./target/...            # With debug logs

# Monitor
ps aux | grep omp_discord_bridge       # Check if running
tail -f bot.log                        # View logs
pkill omp_discord_bridge              # Stop bot

# Test
!ping                                  # In Discord
!help                                  # In Discord
!omp <message>                         # In Discord
```

---

*Last Updated: Session 2025-02-23*
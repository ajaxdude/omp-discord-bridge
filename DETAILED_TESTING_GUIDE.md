# Comprehensive Testing Guide - Discord Bridge for OMP

## Table of Contents
1. [Prerequisites Check](#prerequisites-check)
2. [Discord Bot Setup](#discord-bot-setup)
3. [OMP Verification](#omp-verification)
4. [Environment Configuration](#environment-configuration)
5. [Building the Bot](#building-the-bot)
6. [Running the Bot](#running-the-bot)
7. [Testing Scenarios](#testing-scenarios)
8. [Troubleshooting](#troubleshooting)

---

## Prerequisites Check

### Check Rust Installation
```bash
rustc --version
# Should show: rustc 1.xx.x or higher

cargo --version
# Should show: cargo 1.xx.x or higher
```

**If not installed**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Check OMP Installation
```bash
omp --version
# Should show OMP version

which omp
# Should show path to OMP executable
```

**If not installed**: Follow OMP installation instructions from your repository

---

## Discord Bot Setup

### Step 1: Create Discord Application

1. **Go to Discord Developer Portal**
   - Visit: https://discord.com/developers/applications
   - Click "New Application"
   - Name it: "OMP Discord Bridge" (or your preferred name)
   - Click "Create"

2. **Create Bot User**
   - In the left sidebar, click "Bot"
   - Click "Add Bot"
   - Confirm by clicking "Yes, do it!"
   - **IMPORTANT**: Copy the bot token (click "Reset Token" if needed)
     ```
     Save this token somewhere secure - you'll need it later!
     Format: Long string like MTAwNT... (keep it secret!)
     ```

3. **Configure Privileged Gateway Intents**
   - Scroll down to "Privileged Gateway Intents"
   - **Enable these intents**:
     - ✅ **Message Content Intent** (CRITICAL - required to read user messages)
     - ✅ **Server Members Intent** (optional, for future features)
   - Click "Save Changes"

4. **Generate OAuth2 URL**
   - In the left sidebar, click "OAuth2" → "URL Generator"
   - Under "Scopes", check these boxes:
     - ✅ `bot`
     - ✅ `applications.commands` (for slash commands in future)
   - Under "Bot Permissions", check these:
     - ✅ `Send Messages`
     - ✅ `Read Messages/View Channels`
     - ✅ `Read Message History`
     - ✅ `Add Reactions`
     - ✅ `Use External Emojis` (optional)
   - **Copy the generated URL** at the bottom (it's long, starts with `https://discord.com/oauth2/...`)

5. **Invite Bot to Your Server**
   - Paste the OAuth2 URL into your browser
   - Select your server from the dropdown
   - Click "Authorize"
   - Complete the CAPTCHA if prompted
   - Bot should now appear in your server's member list

### Step 2: Verify Bot in Server

```bash
# In your Discord server, check:
# 1. Bot appears in member list (usually at bottom or in "Offline" section)
# 2. Bot name should be "OMP Discord Bridge" or whatever you named it
# 3. Bot should show a "BOT" badge next to its name
```

---

## OMP Verification

### Step 1: Test OMP Binary
```bash
# Check OMP is in PATH
which omp
# Expected: /usr/local/bin/omp or similar

# Test basic OMP command
omp --help
# Expected: OMP help output

# Test RPC mode (CRITICAL)
omp --mode rpc --help
# Expected: No error, help text or usage info
```

### Step 2: Test RPC Mode Manually
```bash
# Start OMP in RPC mode
echo '{"type":"prompt","message":"Hello"}' | omp --mode rpc

# Expected output:
# - JSON response(s) with correlation IDs
# - No error messages
# - Should not hang or timeout

# Test with correlation ID
echo '{"type":"prompt","id":"test-123","message":"Hello"}' | omp --mode rpc

# Expected: Events should include "id":"test-123"
```

**If RPC mode fails**:
```bash
# Check OMP version supports RPC mode
omp --version

# Check if patches were applied (see omp-patch/ directory)
# The correlation ID feature must be present in your OMP installation
```

---

## Environment Configuration

### Step 1: Create .env File
```bash
cd ~/ai/projects/omp-discord-bridge
cp .env.example .env
```

### Step 2: Edit .env File
```bash
nano .env
# OR use your preferred editor: vim .env, code .env, etc.
```

### Step 3: Add Your Configuration
```bash
# Required: Your Discord bot token
DISCORD_TOKEN=MTAwNT...your_long_token_here

# Optional: Command prefix (default: "!")
DISCORD_PREFIX=!

# Optional: Path to OMP executable (default: "omp")
# Only change if OMP is not in your PATH
# OMP_PATH=/custom/path/to/omp
```

**How to get your Discord token** (if you didn't save it earlier):
1. Go back to https://discord.com/developers/applications
2. Select your application
3. Click "Bot" in the left sidebar
4. Click "Reset Token" (if needed)
5. Click "Copy" button under the token field
6. **IMPORTANT**: Do not share this token publicly!

### Step 4: Verify .env File
```bash
# Check file exists
ls -la .env

# Verify contents (be careful not to expose token!)
cat .env | grep DISCORD_TOKEN
# Expected: DISCORD_TOKEN=MTAwNT... (your actual token)

# Check for syntax errors
cat .env
# No quotes around values, no trailing spaces, each on its own line
```

---

## Building the Bot

### Step 1: Clean Build (First Time)
```bash
cd ~/ai/projects/omp-discord-bridge

# Clean any previous builds
cargo clean

# Build in release mode (optimized)
cargo build --release

# Expected output:
# Compiling omp_discord_bridge v0.1.0
# Finished `release` profile [optimized] target(s) in X.XXs
```

### Step 2: Verify Binary
```bash
# Check if binary exists
ls -lh target/release/omp_discord_bridge

# Expected: File size around 2-4 MB (optimized Rust binary)
```

### Step 3: Quick Test Run (Dry Run)
```bash
# Test if binary runs (will fail without .env, but confirms it works)
./target/release/omp_discord_bridge

# Expected error:
# Error: Missing environment variable: DISCORD_TOKEN
# (This confirms the binary is working correctly)
```

---

## Running the Bot

### Step 1: Development Mode (First Run - Recommended)
```bash
cd ~/ai/projects/omp-discord-bridge

# Run with logging
RUST_LOG=debug cargo run

# Expected output:
# Starting Oh My Pi Discord Bridge
# Configuration loaded successfully
# Connecting to Oh My Pi...
# Connected to Oh My Pi
# Initializing Discord bot...
# Discord bot initialized
# Starting Discord bot...
# INFO discord_bridge: Connected as YourBotName#1234
```

**Watch for these indicators**:
- ✅ "Configuration loaded successfully"
- ✅ "Connected to Oh My Pi"
- ✅ "Connected as [bot name]"
- ✅ No error messages

**Common startup issues**:
- ❌ "Failed to connect to Oh My Pi" → Check OMP installation
- ❌ "Discord token is empty" → Check .env file
- ❌ "Failed to initialize Discord bot" → Check token and intents

### Step 2: Production Mode (After Testing)
```bash
# Use the optimized binary
./target/release/omp_discord_bridge

# Or with logging
RUST_LOG=info ./target/release/omp_discord_bridge

# Run in background with nohup
nohup ./target/release/omp_discord_bridge > bot.log 2>&1 &

# Or use screen/tmux for interactive background session
screen -S omp-bot
./target/release/omp_discord_bridge
# Press Ctrl+A, D to detach
```

### Step 3: Verify Bot is Running
```bash
# Check process
ps aux | grep omp_discord_bridge

# Check logs (if running in background)
tail -f bot.log

# In Discord, bot should show as "Online" in member list
```

---

## Testing Scenarios

### Scenario 1: Basic Connectivity Test

**Test**: Verify bot responds to commands

```bash
# In Discord (in a channel where bot has access):
User: !ping

Expected response:
Bot: Pong!
```

**What this tests**:
- ✅ Bot is running
- ✅ Bot can read messages
- ✅ Bot can send messages
- ✅ Message content intent is working

**If fails**:
- Check bot has "Read Messages" permission
- Check Message Content Intent is enabled
- Check logs for errors

---

### Scenario 2: Help Command Test

**Test**: Display help text

```bash
User: !help

Expected response:
Bot: **Oh My Pi Discord Bot**

Commands:
- `!ping` - Test bot connectivity
- `!omp <message>` - Send a message to Oh My Pi
- `!status` - Check OMP connection status

Example:
`!omp List all files in the current directory`
```

**What this tests**:
- ✅ Command parsing
- ✅ Message formatting
- ✅ All commands are registered

---

### Scenario 3: Simple OMP Query

**Test**: Send a simple query to OMP

```bash
User: !omp What files are in the current directory?

Expected response sequence:
Bot: Processing...
Bot: [OMP response with file listing]
```

**What this tests**:
- ✅ Discord → OMP communication
- ✅ RPC protocol working
- ✅ Correlation ID tracking
- ✅ Event streaming
- ✅ Response routing

**Expected in logs**:
```
DEBUG Received command: omp What files are in the current directory?
DEBUG Registered correlation ID: uuid-xxxx for channel 123456789
DEBUG Starting RPC event streamer
INFO Agent finished
DEBUG Final response: XXX chars
```

**If fails at "Processing..."**:
- Check OMP is accessible via `omp --mode rpc`
- Check correlation ID support in OMP
- Check logs for RPC errors

**If fails with no response**:
- Check bot has permission to send messages
- Check channel is not read-only
- Check logs for correlation ID errors

---

### Scenario 4: Streaming Response Test

**Test**: Query that generates a long response

```bash
User: !omp Explain how async/await works in Rust

Expected response:
Bot: Processing...
Bot: [First part of response...]
Bot: [Additional parts if response is long]
```

**What this tests**:
- ✅ Streaming events
- ✅ Buffer management (1500 char chunks)
- ✅ Event ordering
- ✅ Correlation ID consistency

**Expected in logs**:
```
DEBUG Streaming text update: 1500 chars
DEBUG Streaming text update: 1500 chars
DEBUG Final response: 450 chars
```

---

### Scenario 5: Multiple Tool Calls

**Test**: Query that uses OMP tools

```bash
User: !omp Find all TODO comments in this repository

Expected response:
Bot: Processing...
Bot: [OMP uses grep tool, shows results]
```

**What this tests**:
- ✅ Tool execution events
- ✅ Multiple event types
- ✅ Tool start/end tracking
- ✅ Complex workflows

**Expected in logs**:
```
DEBUG Tool started: grep
DEBUG Tool finished: grep
```

---

### Scenario 6: Error Handling Test

**Test**: Send invalid command

```bash
User: !xyz

Expected response:
Bot: Unknown command. Type `!help` for available commands.
```

**What this tests**:
- ✅ Error handling
- ✅ User feedback
- ✅ Graceful degradation

---

### Scenario 7: Concurrent Users Test (CRITICAL)

**Test**: Verify correlation ID isolation with multiple users

**Setup**:
1. Open Discord in two different channels or DMs
2. Or have two different users test simultaneously

**User A** (in Channel #general):
```bash
User A: !omp What is 2+2?
```

**User B** (in Channel #random):
```bash
User B: !omp What is the capital of France?
```

**Expected behavior**:
```
Channel #general:
Bot: Processing...
Bot: 2+2 equals 4.

Channel #random:
Bot: Processing...
Bot: The capital of France is Paris.
```

**CRITICAL**: Each user should receive their own response in their own channel!

**What this tests**:
- ✅ Correlation ID isolation
- ✅ Multi-user support
- ✅ No cross-contamination of responses
- ✅ Proper routing

**Expected in logs**:
```
DEBUG Registered correlation ID: uuid-aaa for channel #general
DEBUG Registered correlation ID: uuid-bbb for channel #random
DEBUG Received event with correlation ID uuid-aaa
DEBUG Received event with correlation ID uuid-bbb
```

**If fails** (responses go to wrong channel):
- ❌ Correlation ID tracking broken
- ❌ Pending message map not working
- ❌ This is the critical bug we fixed - recheck implementation!

---

### Scenario 8: Long Response Test

**Test**: Query that generates very long response

```bash
User: !omp Write a detailed explanation of Rust ownership

Expected response:
Bot: Processing...
Bot: [Long response, possibly split into multiple messages]
```

**What this tests**:
- ✅ Discord 2000 char limit handling
- ✅ Buffer overflow protection
- ✅ Message splitting
- ✅ Data integrity

**Note**: Responses >2000 chars will be split across multiple Discord messages

---

### Scenario 9: OMP Error Handling

**Test**: Query that causes OMP to error

```bash
User: !omp Read /nonexistent/file.txt

Expected response:
Bot: Processing...
Bot: [OMP error message about file not found]
```

**What this tests**:
- ✅ OMP error propagation
- ✅ Error event handling
- ✅ User-friendly error messages

---

### Scenario 10: Stress Test (Optional)

**Test**: Multiple rapid commands

```bash
User: !omp test
User: !omp hello
User: !omp help
User: !omp status
[Send multiple commands rapidly]

Expected behavior:
- All commands should be processed
- Responses should match correct commands
- No crashes or hangs
```

**What this tests**:
- ✅ Concurrency handling
- ✅ Queue management
- ✅ Resource cleanup
- ✅ Stability

---

## Troubleshooting

### Issue: Bot Doesn't Respond to Commands

**Symptoms**: `!ping` or `!help` commands get no response

**Diagnosis**:
```bash
# Check bot is running
ps aux | grep omp_discord_bridge

# Check logs
tail -f bot.log
# Or if running in foreground: Look at terminal output

# Check bot permissions in Discord:
# 1. Go to server settings → Roles
# 2. Find bot's role
# 3. Verify permissions:
#    - Read Messages/View Channels ✅
#    - Send Messages ✅
#    - Read Message History ✅
```

**Solutions**:
1. **Bot offline**: Start the bot
   ```bash
   ./target/release/omp_discord_bridge
   ```

2. **Missing permissions**: Grant permissions in Discord
   - Server Settings → Roles → Bot Role → Permissions
   - Enable: Read Messages, Send Messages, Read Message History

3. **Wrong channel**: Bot needs access to specific channel
   - Channel settings → Permissions → Bot Role
   - Grant access

4. **Message Content Intent not enabled**:
   - Discord Developer Portal → Bot → Privileged Gateway Intents
   - Enable "Message Content Intent"
   - Save changes
   - Restart bot

---

### Issue: "Failed to connect to Oh My Pi"

**Symptoms**: Bot starts but can't connect to OMP

**Diagnosis**:
```bash
# Test OMP manually
omp --mode rpc
echo '{"type":"prompt","message":"test"}' | omp --mode rpc

# Check OMP in PATH
which omp

# Check OMP version
omp --version
```

**Solutions**:
1. **OMP not installed**: Install OMP
   ```bash
   # Follow OMP installation instructions
   ```

2. **OMP not in PATH**: Add to PATH or set OMP_PATH in .env
   ```bash
   # In .env:
   OMP_PATH=/full/path/to/omp
   ```

3. **OMP doesn't support RPC mode**: Update OMP
   ```bash
   # Ensure OMP has correlation ID support
   # Check patches in omp-patch/ directory
   ```

4. **Wrong OMP executable**: Verify correct OMP
   ```bash
   which omp
   # Should point to correct OMP installation
   ```

---

### Issue: Responses Go to Wrong Channel

**Symptoms**: User A gets User B's responses

**Diagnosis**:
```bash
# Check logs for correlation ID tracking
RUST_LOG=debug ./target/release/omp_discord_bridge

# Look for:
DEBUG Registered correlation ID: uuid-xxx for channel YYY
DEBUG Received event with correlation ID uuid-xxx
```

**Solutions**:
1. **Correlation ID not being stored**: Check implementation
   - Verify `rpc.prompt()` returns correlation ID
   - Verify correlation ID is stored in pending_map

2. **Events don't have correlation IDs**: Check OMP
   ```bash
   # Test OMP correlation ID support
   echo '{"type":"prompt","id":"test-123","message":"hello"}' | omp --mode rpc
   # Verify events include "id":"test-123"
   ```

3. **Pending map not working**: Check event streamer
   - Verify `pending_map.get(&correlation_id)` works
   - Verify channel_id is extracted correctly

---

### Issue: Bot Crashes or Hangs

**Symptoms**: Bot stops responding or crashes

**Diagnosis**:
```bash
# Check for panic messages in logs
tail -100 bot.log

# Check for OMP subprocess issues
ps aux | grep omp

# Check for memory issues
free -h
```

**Solutions**:
1. **OOM (Out of Memory)**: Reduce concurrent users
   - Limit concurrent OMP sessions
   - Add rate limiting

2. **OMP subprocess died**: Restart bot
   ```bash
   # Bot will create new OMP subprocess on restart
   ```

3. **Panic in Rust code**: Check stack trace
   ```bash
   # Run with RUST_BACKTRACE=1
   RUST_BACKTRACE=1 ./target/release/omp_discord_bridge
   ```

---

### Issue: Long Responses Truncated

**Symptoms**: Long OMP responses are cut off at 2000 chars

**Diagnosis**:
```bash
# This is a Discord limitation, not a bug
# Discord has a 2000 character limit per message
```

**Solutions**:
1. **Accept limitation**: Responses >2000 chars are split
2. **Future enhancement**: Implement message splitting
3. **Workaround**: Ask for shorter responses
   ```bash
   User: !omp [query] - keep response under 2000 chars
   ```

---

### Issue: "Processing..." Message Never Updates

**Symptoms**: Bot says "Processing..." but never responds

**Diagnosis**:
```bash
# Check logs for OMP events
DEBUG Starting RPC event streamer
# Should see events flowing

# Check if OMP is responding
echo '{"type":"prompt","message":"test"}' | omp --mode rpc
```

**Solutions**:
1. **OMP not emitting events**: Check OMP RPC mode
2. **Event streamer died**: Check logs for errors
3. **Correlation ID mismatch**: Check correlation ID tracking

---

## Testing Checklist

Use this checklist to verify everything works:

- [ ] Discord bot created and invited to server
- [ ] Bot token configured in .env
- [ ] OMP installed and accessible via `omp --mode rpc`
- [ ] Bot starts successfully (`cargo run`)
- [ ] Bot shows as "Online" in Discord
- [ ] `!ping` command works
- [ ] `!help` command shows help text
- [ ] `!omp` queries get responses from OMP
- [ ] Correlation IDs are tracked (check logs)
- [ ] Multiple concurrent users get correct responses
- [ ] Tool execution works (grep, read, etc.)
- [ ] Long responses are handled (split or truncated)
- [ ] Errors are handled gracefully
- [ ] Bot doesn't crash under normal use
- [ ] Logs show expected debug/info messages

---

## Expected Log Output (Success Case)

When everything works correctly, logs should look like this:

```
INFO Starting Oh My Pi Discord Bridge
INFO Configuration loaded successfully
INFO Connecting to Oh My Pi...
INFO Connected to Oh My Pi
INFO Initializing Discord bot...
INFO Discord bot initialized
INFO Starting Discord bot...
INFO Connected as OMP Bot#1234
DEBUG Starting RPC event streamer
DEBUG Received command: omp test
DEBUG Registered correlation ID: uuid-abc123 for channel 123456789
DEBUG Received RPC event: AgentStart
DEBUG Received RPC event: MessageUpdate
DEBUG Received RPC event: AgentEnd
DEBUG Final response: 45 chars
INFO Agent finished
```

---

## Performance Benchmarks

Expected performance (your results may vary):

- Startup time: < 2 seconds
- `!ping` response: < 500ms
- `!omp` simple query: 2-5 seconds (depends on OMP)
- `!omp` complex query: 5-30 seconds (depends on OMP)
- Concurrent users: Tested with 5+ simultaneous users
- Memory usage: ~50-100 MB (Rust) + OMP subprocess

---

## Next Steps After Testing

Once testing is successful:

1. **Set up production deployment**
   - Create systemd service file
   - Configure auto-restart on crash
   - Set up log rotation

2. **Add monitoring**
   - Track uptime
   - Monitor error rates
   - Alert on failures

3. **Add features**
   - Edit "Processing..." message instead of sending new ones
   - Add rate limiting
   - Support for image content
   - Slash commands

4. **Document**
   - Create user guide for your team
   - Document common issues and solutions
   - Add examples of useful queries

---

*End of Detailed Testing Guide*
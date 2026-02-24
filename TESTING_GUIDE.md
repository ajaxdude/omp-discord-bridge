# Integration Testing Guide

## Prerequisites

Before running integration tests, ensure you have:

1. **Discord Bot Token**
   - Create a Discord application at https://discord.com/developers/applications
   - Enable bot functionality
   - Generate a bot token
   - Invite the bot to your test server with required permissions:
     - Send Messages
     - Read Messages/View Channels
     - Read Message History

2. **OMP Installation**
   - OMP should be installed and accessible via `omp` command
   - Or set `OMP_PATH` environment variable to your OMP executable

3. **Environment Setup**
   ```bash
   export DISCORD_TOKEN="your_discord_bot_token_here"
   export DISCORD_PREFIX="!"
   export OMP_PATH="omp"  # or path to your OMP executable
   ```

## Test Scenarios

### Test 1: Single User Basic Flow

**Objective**: Verify basic prompt-response flow works with correlation ID

**Steps**:
1. Start the Discord bridge:
   ```bash
   cargo run
   ```

2. In a Discord channel, send:
   ```
   !omp What is 2 + 2?
   ```

3. Expected behavior:
   - Bot replies with "Processing..."
   - Bot responds with the final answer (e.g., "2 + 2 = 4")
   - Response appears in the same channel
   - Check logs for correlation ID handling

**Verification**:
- ✅ Correlation ID generated and logged
- ✅ Pending message map contains correlation ID → channel mapping
- ✅ Events received with matching correlation ID
- ✅ Final response sent to correct channel
- ✅ Pending message map entry cleaned up after completion

---

### Test 2: Multi-User Concurrent Prompts

**Objective**: Verify two users can prompt simultaneously without interference

**Setup**:
- Have two different Discord users ready
- Both should be in different channels (or same channel)

**Steps**:
1. User A sends simultaneously:
   ```
   !omp Tell me a joke
   ```

2. User B sends simultaneously (within 1-2 seconds):
   ```
   !omp What is the capital of France?
   ```

3. Expected behavior:
   - Both users receive "Processing..."
   - Both users receive correct responses
   - User A gets a joke
   - User B gets "Paris"
   - No cross-contamination of responses

**Verification**:
- ✅ Each correlation ID is unique
- ✅ User A's events contain User A's correlation ID only
- ✅ User B's events contain User B's correlation ID only
- ✅ Responses route to correct channels
- ✅ No responses sent to wrong channels
- ✅ Both pending map entries cleaned up independently

---

### Test 3: Multiple Channels

**Objective**: Verify responses route correctly across different Discord channels

**Setup**:
- Same user in two different channels (#test1 and #test2)

**Steps**:
1. In #test1, send:
   ```
   !omp Channel 1 test
   ```

2. Immediately in #test2, send:
   ```
   !omp Channel 2 test
   ```

3. Expected behavior:
   - Both channels receive "Processing..."
   - Each channel receives its own response
   - No cross-channel responses

**Verification**:
- ✅ Each correlation ID maps to correct channel ID
- ✅ Events route to proper channel
- ✅ No channel confusion

---

### Test 4: Error Cases

**Objective**: Verify error handling and correlation cleanup

**Test 4a: Invalid Command**
1. Send an invalid command:
   ```
   !omp
   ```

2. Expected behavior:
   - Error message displayed
   - Correlation ID cleaned up
   - No orphaned entries in pending map

**Test 4b: OMP Process Crash**
1. While a prompt is processing, simulate OMP crash (if possible)
2. Expected behavior:
   - Error logged
   - Correlation ID cleaned up
   - Bot continues functioning for new prompts

**Test 4c: Network Interruption**
1. Disconnect internet while prompt is processing
2. Expected behavior:
   - Error logged appropriately
   - Correlation ID cleaned up
   - Bot recovers and processes new prompts

**Verification**:
- ✅ Errors logged with proper context
- ✅ Correlation IDs cleaned up on errors
- ✅ No memory leaks from orphaned map entries
- ✅ Bot remains functional after errors

---

### Test 5: Long-Running Prompts

**Objective**: Verify correlation tracking persists through long operations

**Steps**:
1. Send a complex prompt that takes time:
   ```
   !omp Explain quantum computing in detail
   ```

2. While waiting, send another simple prompt:
   ```
   !omp What is 1 + 1?
   ```

3. Expected behavior:
   - Both prompts process independently
   - Simple prompt may complete first
   - Complex prompt completes later
   - Both responses route correctly

**Verification**:
- ✅ Correlation IDs don't interfere with each other
- ✅ Long-running operations don't block short ones
- ✅ Both pending map entries persist correctly
- ✅ Both cleaned up independently

---

### Test 6: Edge Cases

**Test 6a: Rapid Consecutive Prompts**
1. Send 5 prompts rapidly:
   ```
   !omp test 1
   !omp test 2
   !omp test 3
   !omp test 4
   !omp test 5
   ```

2. Expected behavior:
   - All 5 correlation IDs are unique
   - All 5 responses route correctly
   - All 5 cleaned up properly

**Test 6b: Same User, Different Channels**
1. Send prompts in 3 different channels within 10 seconds
2. Verify no channel confusion

**Test 6c: Unicode and Special Characters**
1. Send prompts with emojis, unicode characters:
   ```
   !omp 解释这个: 🚀
   ```

2. Verify encoding doesn't break correlation

---

## Log Analysis

During testing, monitor logs for:

### Successful Flow
```
[INFO] Received prompt with correlation ID: uuid-1
[DEBUG] Registered pending message: uuid-1 -> ChannelId(123)
[INFO] Agent started with correlation ID: uuid-1
[DEBUG] Streaming text update: 150 chars
[INFO] Agent finished
[DEBUG] Sent final response: 150 chars to ChannelId(123)
[DEBUG] Cleaned up pending message: uuid-1
```

### Multi-User Flow
```
[INFO] Received prompt with correlation ID: uuid-A
[INFO] Received prompt with correlation ID: uuid-B
[DEBUG] Registered pending message: uuid-A -> ChannelId(100)
[DEBUG] Registered pending message: uuid-B -> ChannelId(200)
[INFO] Agent started with correlation ID: uuid-A
[INFO] Agent started with correlation ID: uuid-B
[DEBUG] Event for uuid-A: MessageUpdate
[DEBUG] Event for uuid-B: MessageUpdate
[INFO] Agent finished (uuid-A)
[INFO] Agent finished (uuid-B)
[DEBUG] Sent final response to ChannelId(100)
[DEBUG] Sent final response to ChannelId(200)
[DEBUG] Cleaned up pending message: uuid-A
[DEBUG] Cleaned up pending message: uuid-B
```

### Error Flow
```
[WARN] Received event with correlation ID uuid-123 but no pending channel found
[ERROR] Failed to send message to Discord: ...
```

---

## Performance Metrics

Monitor these metrics during testing:

1. **Correlation ID Lookup Time**
   - Should be < 1ms per lookup
   - HashMap should be O(1)

2. **Pending Map Size**
   - Should be small (0-10 entries typical)
   - Should grow linearly with concurrent users
   - Should shrink back to 0 after completion

3. **Memory Usage**
   - Monitor for memory leaks
   - Should remain stable over time
   - No unbounded growth of pending map

4. **Response Latency**
   - End-to-end latency should be acceptable
   - Correlation overhead should be minimal (< 1ms)

---

## Common Issues and Solutions

### Issue: "Failed to send message to Discord"

**Possible causes**:
- Bot lacks permissions
- Channel ID is invalid
- Network issue

**Solution**:
- Verify bot permissions
- Check Discord API status
- Review error logs

### Issue: "No pending channel found"

**Possible causes**:
- Correlation ID mismatch
- Pending map entry already cleaned up
- Event without correlation ID

**Solution**:
- Check correlation ID generation
- Verify timing of event emission
- Review pending map lifecycle

### Issue: Responses going to wrong channel

**Possible causes**:
- Correlation ID collision (UUIDs should prevent this)
- Pending map corruption
- Channel ID mismatch

**Solution**:
- Verify UUID generation
- Check pending map consistency
- Audit channel ID registration

---

## Test Results Template

Use this template to document results:

```
Test #1: Single User Basic Flow
Date: YYYY-MM-DD
Tester: [Your Name]
Result: PASS / FAIL
Notes:
- Correlation ID generated: YES / NO
- Response received: YES / NO
- Response correct: YES / NO
- Logs verified: YES / NO
- Issues: [List any issues]

Test #2: Multi-User Concurrent Prompts
Date: YYYY-MM-DD
Tester: [Your Name]
Result: PASS / FAIL
Notes:
- Both users received correct responses: YES / NO
- No cross-contamination: YES / NO
- Correlation IDs unique: YES / NO
- Issues: [List any issues]

[Continue for all tests...]
```

---

## Automated Testing (Future)

To automate these tests, consider:

1. **Discord Testing Framework**
   - Use Discord's API to simulate user messages
   - Automate test scenario execution
   - Verify responses programmatically

2. **Mock OMP Server**
   - Create mock OMP that emits test events
   - Test correlation ID handling without real OMP
   - Faster, more reliable testing

3. **Integration Test Suite**
   - Write Rust integration tests
   - Use test fixtures for Discord events
   - Automate regression testing

---

## Sign-off

After completing all tests:

- [ ] All test scenarios documented
- [ ] Test results recorded
- [ ] Issues identified and tracked
- [ ] Performance metrics within acceptable ranges
- [ ] Ready for production deployment

**Tester**: ______________________  
**Date**: ______________________  
**Signature**: ______________________

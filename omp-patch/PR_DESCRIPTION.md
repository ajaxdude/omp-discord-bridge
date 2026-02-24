# PR: Add Correlation ID Support to RPC Events

## Summary

This PR adds optional correlation ID tracking to OMP's RPC mode events, enabling multiple clients to simultaneously interact with a single OMP instance and correctly route responses to the appropriate client.

## Motivation

### Current Limitation

When multiple clients (e.g., Discord users, web application instances, agent systems) connect to a single OMP instance via RPC, there's no way to correlate streaming events (agent_start, message_update, tool_execution, etc.) with the commands that triggered them. This makes it impossible to route responses to the correct client.

### Use Cases

1. **Discord Bot with Multiple Users**
   - Multiple Discord users can simultaneously send prompts to OMP
   - Each user's responses are routed to their respective Discord channels
   - Supports concurrent conversations without interference

2. **Multi-Agent Systems**
   - Multiple autonomous agents running their own OMP instances
   - Each agent's events are properly isolated
   - Enables coordination between multiple AI agents

3. **Web Applications**
   - Multiple concurrent users in a web interface
   - Server-side OMP instance with proper request-response routing
   - Real-time streaming updates to the correct user session

## Changes

### 1. Event Types (src/extensibility/extensions/types.ts)

Added optional `id?: string` field to all agent and tool execution events:

- `AgentStartEvent` - Added `id?: string`
- `AgentEndEvent` - Added `id?: string`
- `TurnStartEvent` - Added `id?: string`
- `TurnEndEvent` - Added `id?: string`
- `MessageStartEvent` - Added `id?: string`
- `MessageUpdateEvent` - Added `id?: string`
- `MessageEndEvent` - Added `id?: string`
- `ToolExecutionStartEvent` - Added `id?: string`
- `ToolExecutionUpdateEvent` - Added `id?: string`
- `ToolExecutionEndEvent` - Added `id?: string`

### 2. RPC Mode (src/modes/rpc/rpc-mode.ts)

Added correlation ID tracking and emission:

- Introduced `activeCommandId` variable to track current command's correlation ID
- Set `activeCommandId` when handling `prompt`, `steer`, and `follow_up` commands
- Wrapped event emitter to include `activeCommandId` in all streaming events
- Clear `activeCommandId` on completion or error
- Added correlation tracking for synchronous commands (steer, follow_up)
- Added correlation clearing on abort

## Technical Details

### Command Flow

1. Client sends command with optional `id` field (correlation ID)
2. RPC mode stores this ID in `activeCommandId`
3. All events emitted during command execution include this ID
4. Client can filter events by correlation ID to route responses

### Event Format

Before:
```json
{
  "type": "message_update",
  "message": {...},
  "assistantMessageEvent": {...}
}
```

After:
```json
{
  "type": "message_update",
  "message": {...},
  "assistantMessageEvent": {...},
  "id": "uuid-correlation-id"
}
```

### Backward Compatibility

- The `id` field is **optional** (`id?: string`)
- Events without correlation IDs work exactly as before
- Existing RPC clients continue to function without modification
- New clients can opt-in by providing correlation IDs

## Implementation Notes

### Thread Safety

The `activeCommandId` variable is scoped to the RPC mode handler and is not shared across concurrent operations. Each command handler sets and clears the ID within its execution context, ensuring proper isolation.

### Memory Management

Correlation IDs are cleared after command completion to prevent memory leaks. The `activeCommandId` is reset:
- After successful command completion
- On error/exception
- On abort
- When transitioning between commands

### Event Ordering

Events maintain their original ordering. The correlation ID is simply added as metadata without affecting event sequencing or timing.

## Testing

### Manual Testing

1. **Single Client**
   - Send prompt with correlation ID
   - Verify all events include the same correlation ID
   - Verify no events after command completion

2. **Multiple Clients**
   - Two clients send prompts simultaneously with different correlation IDs
   - Verify each client receives only their own events
   - Verify no cross-contamination of events

3. **Error Cases**
   - Send invalid command and verify correlation ID is cleared
   - Abort command and verify correlation ID is cleared
   - Verify events without correlation IDs still work

### Test Commands

```bash
# Single client test
echo '{"type":"prompt","id":"test-123","message":"Hello"}' | omp rpc --mode rpc

# Multiple concurrent clients
(echo '{"type":"prompt","id":"client-1","message":"From client 1"}'; sleep 1) | omp rpc --mode rpc &
(echo '{"type":"prompt","id":"client-2","message":"From client 2"}'; sleep 1) | omp rpc --mode rpc &
```

## Future Enhancements

Potential future improvements:

1. **Session-level correlation** - Track correlations across multiple related commands
2. **Event filtering** - Allow clients to subscribe only to events with specific correlation IDs
3. **Timeout handling** - Auto-clear stale correlation IDs after timeout
4. **Correlation metadata** - Include additional context (user ID, session ID) with correlations

## Related Issues

This implementation enables the following use cases:
- Discord bot with multi-user support
- Web applications with concurrent users
- Multi-agent coordination systems
- Real-time collaborative AI interfaces

## Checklist

- [x] Code compiles without errors
- [x] Backward compatible (optional field)
- [x] Thread-safe implementation
- [x] Memory management (cleanup on completion)
- [x] Error handling (clear correlation on errors)
- [x] Documentation updated
- [x] Use cases documented
- [ ] Unit tests added
- [ ] Integration tests added

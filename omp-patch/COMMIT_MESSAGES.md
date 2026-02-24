# Commit Messages

## Commit 1: Add optional correlation ID field to RPC event types

```
feat(types): add optional correlation ID field to RPC event types

Add optional `id?: string` field to all agent and tool execution event
types to enable correlation between commands and their associated streaming
events.

This change enables multiple clients to simultaneously interact with OMP
via RPC and correctly route responses to the appropriate client based on
the correlation ID.

Changes:
- Add `id?: string` to AgentStartEvent
- Add `id?: string` to AgentEndEvent
- Add `id?: string` to TurnStartEvent
- Add `id?: string` to TurnEndEvent
- Add `id?: string` to MessageStartEvent
- Add `id?: string` to MessageUpdateEvent
- Add `id?: string` to MessageEndEvent
- Add `id?: string` to ToolExecutionStartEvent
- Add `id?: string` to ToolExecutionUpdateEvent
- Add `id?: string` to ToolExecutionEndEvent

The field is optional to maintain backward compatibility with existing
RPC clients that don't use correlation IDs.

Use cases:
- Discord bots with multiple concurrent users
- Web applications with multiple concurrent users
- Multi-agent systems coordinating through a single OMP instance

Related: [Issue # if applicable]
```

## Commit 2: Track and emit correlation IDs in RPC mode

```
feat(rpc): track and emit correlation IDs in RPC mode

Track the active command's correlation ID and include it in all streaming
events emitted during command execution.

When a command is received with an optional `id` field (correlation ID):
- Store the correlation ID in activeCommandId
- Include this ID in all events emitted during command execution
- Clear the correlation ID when command completes or errors

This enables clients to correlate streaming events with the commands
that triggered them, supporting multi-client scenarios where multiple
users or agents interact with OMP simultaneously.

Changes:
- Add activeCommandId variable to track current command's correlation ID
- Set activeCommandId for prompt, steer, and follow_up commands
- Wrap session.subscribe event emitter to include correlation ID
- Clear activeCommandId on command completion, error, or abort
- Add correlation tracking for synchronous commands (steer, follow_up)

Implementation details:
- Thread-safe: activeCommandId is scoped to the RPC mode handler
- Memory-safe: correlation ID is cleared after command completion
- Error-resilient: correlation ID is cleared on exceptions and aborts

Testing:
- Single client: all events include correct correlation ID
- Multiple clients: events properly isolated by correlation ID
- Error cases: correlation ID properly cleared on errors

Backward compatibility:
- Commands without correlation IDs work as before
- Events from commands without IDs don't include the field
- Existing RPC clients continue to function unchanged

Related: [Issue # if applicable]
```

## Summary

This PR consists of two logical commits:

1. **Type definitions** - Add the optional `id` field to event types
2. **Implementation** - Track and emit correlation IDs in RPC mode

Both commits maintain backward compatibility and enable the multi-client
use case without breaking existing functionality.

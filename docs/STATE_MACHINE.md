# State Machine Architecture

The coding agent uses an event-driven state machine that separates logic from I/O.

## Core Principle

The `StateMachine` is **pure and synchronous**. It receives `AgentEvent`s and returns `AgentAction`s. The caller (runner) executes actions and feeds results back as events.

```
User Input → Event → StateMachine → Action → Runner executes → Event → ...
```

## States

| State | Description |
|-------|-------------|
| `WaitingForUserInput` | Idle, ready for user message |
| `CallingLlm` | Making API request (tracks retry count) |
| `ProcessingLlmResponse` | Transient state parsing LLM response |
| `ExecutingTools` | Running tool calls from LLM |
| `Error` | Recoverable error, will retry |
| `ShuttingDown` | Terminal state |

## Events

- `UserInput(String)` - User submitted a message
- `LlmCompleted { content, stop_reason }` - API call succeeded
- `LlmError(String)` - API call failed
- `ToolCompleted { call_id, result }` - Tool finished executing
- `RetryTimeout` - Retry delay elapsed
- `ShutdownRequested` - User requested quit

## Actions

- `SendLlmRequest { messages }` - Call Claude API
- `ExecuteTools { calls }` - Run specified tools
- `DisplayText(String)` - Show text to user
- `DisplayError(String)` - Show error to user
- `PromptForInput` - Wait for user input
- `ScheduleRetry { delay_ms }` - Wait then send RetryTimeout
- `WaitForEvent` - No action needed
- `Shutdown` - Terminate

## State Transitions

```
WaitingForUserInput --UserInput--> CallingLlm --LlmCompleted--> ProcessingLlmResponse
                                       |                              |
                                       |                    +---------+---------+
                                       |                    |                   |
                                  LlmError              has tools           no tools
                                       |                    |                   |
                                       v                    v                   v
                                    Error          ExecutingTools     WaitingForUserInput
                                       |                    |
                                  RetryTimeout        ToolCompleted
                                       |               (all done)
                                       v                    |
                                  CallingLlm <--------------+
```

`ShutdownRequested` transitions to `ShuttingDown` from any state.

## Retry Logic

- Max retries: 3
- Exponential backoff: 1s, 2s, 3s
- After max retries, returns to `WaitingForUserInput` with error message

## Key Design Decisions

1. **State machine is pure** - No I/O, no side effects, fully testable
2. **Caller executes actions** - Runner handles API calls, tool execution, user I/O
3. **Conversation travels with state** - Full message history in each state variant
4. **Exhaustive matching** - Rust ensures all event/state combinations are handled

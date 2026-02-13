# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Context

This is a Rust workshop project building an AI coding assistant with an event-driven state machine architecture. The code prioritizes **testability**, **explicit state transitions**, and **educational clarity**.

## Working on the State Machine

When modifying `src/machine.rs` or `src/state.rs`:

1. **Every state transition must be tested** - Add a unit test for any new transition
2. **Keep the state machine pure** - No I/O operations; return `AgentAction` for the caller to execute
3. **Exhaustive matching** - All event/state combinations must be handled explicitly
4. **Update docs/STATE_MACHINE.md** - If adding states or events, update the design doc

## Adding New Tools

Tools follow a consistent pattern (see `examples/` for reference):

```rust
// 1. Define input schema struct with schemars
#[derive(Debug, Deserialize, JsonSchema)]
struct MyToolInput {
    required_field: String,
    #[serde(default)]
    optional_field: Option<String>,
}

// 2. Implement tool function
fn my_tool(input: Value) -> Result<String, String> {
    let params: MyToolInput = serde_json::from_value(input)
        .map_err(|e| format!("Invalid input: {}", e))?;
    // ... implementation
    Ok("result".to_string())
}

// 3. Register with agent
let tool = ToolDefinition {
    name: "my_tool".to_string(),
    description: "What the tool does".to_string(),
    function: my_tool,
    schema: generate_schema::<MyToolInput>(),
};
```

## Testing Strategy

- **Unit tests**: In `#[cfg(test)]` module within each source file
- **Integration tests**: In `tests/` directory, test full tool behavior
- **Fixtures**: Use `fixtures/` for test data; use `tempfile` crate for temp files

Run tests with `make test` or `make test-verbose`.

## Code Style

- Run `make fmt` before committing
- Run `make lint` to catch common issues
- Keep functions focused and small
- Prefer explicit error messages over `.unwrap()`

## Key Architectural Decisions

1. **State machine over async loops** - Easier to test and reason about
2. **Function pointers for tools** - Simple, no trait complexity
3. **String errors** - Keeps examples approachable; production code might use `thiserror`
4. **Conversation in state** - Full message history travels with state for multi-turn support

## Files to Understand First

1. `docs/STATE_MACHINE.md` - State machine architecture
2. `src/state.rs` - Core types (read this before machine.rs)
3. `examples/chat.rs` - Simplest working example
4. `src/machine.rs` - State transition logic with tests

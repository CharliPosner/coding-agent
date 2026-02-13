# CLAUDE.md

This file provides context to Claude Code about the coding-agent project.

## Project Overview

A Rust workshop demonstrating how to build an AI-powered coding assistant. The project uses an event-driven state machine architecture to handle interactions with the Claude API in a predictable, testable manner.

## Quick Reference

```bash
# Build
make build              # Build all examples
make build-release      # Release build

# Run examples
make run-chat           # Basic chatbot (no tools)
make run-read           # File reader
make run-list-files     # Directory listing
make run-bash           # Shell execution
make run-edit           # File editing
make run-code-search    # Pattern search (requires ripgrep)

# Test & Quality
make test               # Run all tests
make fmt                # Format code
make lint               # Run clippy
```

## Architecture

The codebase follows an **event-driven state machine** pattern:

- **State machine is pure logic** - `StateMachine` is synchronous and deterministic
- **Caller executes actions** - I/O operations are the caller's responsibility
- **Six states**: WaitingForUserInput → CallingLlm → ProcessingLlmResponse → ExecutingTools → Error → ShuttingDown
- **Bounded retries** - Exponential backoff (1s, 2s, 3s) with max 3 retries

### Key Files

| File | Purpose |
|------|---------|
| `src/machine.rs` | State machine implementation (~622 lines, 17 unit tests) |
| `src/state.rs` | State/Event/Action enums |
| `src/types.rs` | Anthropic API types and tool definitions |
| `src/lib.rs` | Public API and legacy Agent struct |

### Examples (Progressive Complexity)

1. `chat.rs` - Pure conversation, no tools
2. `read.rs` - File reading tool
3. `list_files.rs` - Directory traversal
4. `bash.rs` - Shell command execution
5. `edit.rs` - File editing with validation
6. `code_search.rs` - Pattern search via ripgrep

## Coding Conventions

- **Error handling**: `Result<T, String>` for simplicity
- **Naming**: snake_case functions, PascalCase types
- **Tool functions**: `fn(serde_json::Value) -> Result<String, String>`
- **Schemas**: Generated via `schemars` crate from Rust types
- **Tests**: Arrange → Act → Assert pattern; unit tests in `#[cfg(test)]` modules

## Dependencies

- `ureq` - HTTP client for API calls
- `serde`/`serde_json` - Serialization
- `schemars` - JSON Schema generation
- `walkdir` - Directory traversal
- `dotenvy` - Auto-loads `.env` file

## Environment

Requires `ANTHROPIC_API_KEY` in environment or `.env` file.

## Design Documents

See `docs/STATE_MACHINE.md` for the state machine architecture.

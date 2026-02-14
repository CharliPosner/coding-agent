# CLAUDE.md

This file provides context to Claude Code about the coding-agent project.

## Project Overview

A Rust workspace with two crates:

1. **`coding-agent-core`** - Event-driven state machine for AI agent interactions (workshop foundation)
2. **`coding-agent-cli`** - Terminal interface with REPL, session persistence, and token tracking

The project uses a pure state machine architecture to handle Claude API interactions in a predictable, testable manner.

## Quick Reference

```bash
# Build
make build              # Build all examples
make build-release      # Release build

# Run CLI
cargo run -p coding-agent-cli

# Run examples (core crate)
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

### Key Files (Core)

| File | Purpose |
|------|---------|
| `crates/coding-agent-core/src/machine.rs` | State machine implementation (~622 lines) |
| `crates/coding-agent-core/src/state.rs` | State/Event/Action enums |
| `crates/coding-agent-core/src/types.rs` | Anthropic API types and tool definitions |

### Key Files (CLI)

| File | Purpose |
|------|---------|
| `crates/coding-agent-cli/src/cli/repl.rs` | Main REPL loop |
| `crates/coding-agent-cli/src/cli/input.rs` | Multi-line input, double-enter |
| `crates/coding-agent-cli/src/cli/commands/` | Slash command registry |
| `crates/coding-agent-cli/src/ui/context_bar.rs` | Token usage visualization |
| `crates/coding-agent-cli/src/tokens/counter.rs` | Token counting (tiktoken-rs) |
| `crates/coding-agent-cli/src/integrations/specstory.rs` | Session persistence |

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

- `docs/STATE_MACHINE.md` - Core state machine architecture
- `docs/CLI_ARCHITECTURE.md` - CLI components, flow, and implementation status
- `specs/command-line-interface.md` - Full CLI specification with phases

## Session Discipline

- **One task per session** — don't let scope creep
- When you discover a bug while implementing a feature:
  1. Note it: "Discovered: [description]"
  2. Continue with current task
  3. File it at session end or suggest `/task add`
- When context reaches 60%+, suggest wrapping up via `/land`

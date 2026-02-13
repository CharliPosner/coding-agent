# CLI Architecture

The CLI (`coding-agent-cli`) provides a terminal interface for interacting with the coding agent.

## Crate Structure

```
crates/
├── coding-agent-core/     # State machine (shared library)
└── coding-agent-cli/      # Terminal application
    └── src/
        ├── main.rs        # Entry point
        ├── cli/           # REPL, input, commands
        ├── ui/            # Visual components
        ├── tokens/        # Token counting & cost
        ├── config/        # Settings management
        └── integrations/  # SpecStory, git, etc.
```

## Component Overview

| Module | Purpose | Key Files |
|--------|---------|-----------|
| `cli/` | Terminal interaction | `repl.rs`, `input.rs`, `startup.rs`, `terminal.rs` |
| `cli/commands/` | Slash command system | `mod.rs` (registry), `help.rs`, `clear.rs`, etc. |
| `ui/` | Visual output | `context_bar.rs`, `theme.rs`, `spinner.rs`, `syntax.rs` |
| `tokens/` | Token tracking | `counter.rs`, `pricing.rs` |
| `config/` | User settings | `settings.rs` |
| `integrations/` | External services | `specstory.rs` |

## Application Flow

```
main.rs
    │
    ▼
cli::run()
    │
    ├─► Terminal::enable_raw_mode()
    │
    ├─► StartupScreen::show()
    │       │
    │       ├─ [n] New session
    │       ├─ [r] Resume last ──► SessionManager::load_last()
    │       ├─ [h] Help
    │       └─ [c] Config
    │
    ├─► Repl::new(config)
    │       │
    │       ├─ SessionManager (persistence)
    │       ├─ TokenCounter (tracking)
    │       └─ ContextBar (visualization)
    │
    └─► Repl::run() ◄─────────────────────┐
            │                              │
            ▼                              │
        InputHandler::read_input()         │
            │                              │
            ├─ /command ──► CommandRegistry::execute()
            │                              │
            └─ message ──► Process ────────┘
                    │
                    ├─ Session::add_message()
                    ├─ TokenCounter::count()
                    ├─ ContextBar::update()
                    └─ Session::save()
```

## Input Handling

The `InputHandler` processes keyboard input in raw mode:

| Input | Behavior |
|-------|----------|
| Characters | Append to buffer |
| Enter | Add newline to buffer |
| Enter + Enter | Submit buffer |
| Ctrl+C | Clear buffer |
| Ctrl+D | Exit REPL |
| Backspace | Remove last character |

Multi-line input is supported - single Enter adds a newline, double-Enter submits.

## Command System

Commands follow a trait-based registry pattern:

```rust
trait Command {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &str, ctx: &mut Context) -> Result<()>;
}
```

| Command | Description |
|---------|-------------|
| `/help` | List all commands |
| `/clear` | Reset display and context |
| `/exit` | Quit the application |
| `/history` | Browse past sessions |
| `/config` | Open config in $EDITOR |
| `/cost` | Show token usage and cost |

Commands are registered in `CommandRegistry` and looked up by name.

## Context Bar

Visual indicator of token usage at the bottom of the screen:

```
Context: ████████████░░░░░░░░░░░░░░░░░░  38% │ 76k / 200k
```

Color thresholds:
- **Green**: 0-60% usage
- **Yellow**: 60-85% usage
- **Red**: 85%+ usage

Implementation in `ui/context_bar.rs` - updates after each message exchange.

## Token Counting

Uses `tiktoken-rs` with the `cl100k_base` tokenizer (Claude-compatible):

```rust
TokenCounter::count("Hello world")           // Simple text
TokenCounter::count_message(role, content)   // +4 tokens overhead
TokenCounter::count_conversation(messages)   // +3 tokens system overhead
```

The tokenizer is cached globally (`OnceLock`) for performance.

## Session Persistence (SpecStory)

Conversations are auto-saved as markdown files:

```
.specstory/history/
└── 2024-02-13_14-30-00_adding-auth-flow.md
```

File format:
```markdown
---
title: Adding auth flow
created: 2024-02-13T14:30:00Z
updated: 2024-02-13T15:45:00Z
model: claude-3-opus
version: 1
---

# Adding auth flow

## User
How do I add authentication?

## Agent
Here's how to implement auth...
```

The `SessionManager` handles load/save/list operations.

## Configuration

Settings stored in `~/.config/coding-agent/config.toml`:

```toml
[model]
default = "claude-3-opus"
context_window = 200000

[behavior]
streaming = true
show_context_bar = true

[persistence]
enabled = true
path = ".specstory/history/"
```

Default config is generated on first run if missing.

## Theme System

Colors are centralized in `ui/theme.rs`:

| Element | Color | Purpose |
|---------|-------|---------|
| User input | White | Neutral |
| Agent response | Cyan | Distinguish from user |
| Tool calls | Yellow | Action happening |
| Success | Green | Completion |
| Error | Red | Problems |
| Context bar | Green→Yellow→Red | Usage level |

Respects `NO_COLOR` environment variable for accessibility.

## Terminal Management

`Terminal` struct wraps `crossterm` for safe raw mode handling:

- Enables raw mode on construction
- Restores terminal on drop (even on panic)
- Provides screen clearing and cursor control

Panic hook is installed to ensure terminal cleanup.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `crossterm` | Cross-platform terminal control |
| `tokio` | Async runtime |
| `tiktoken-rs` | Token counting |
| `console` | Terminal styling |
| `syntect` | Syntax highlighting |
| `serde` + `toml` | Configuration |
| `clap` | Argument parsing |

## Implementation Status

| Phase | Status | Features |
|-------|--------|----------|
| 1. Foundation | Done | Raw mode, multi-line input, double-enter |
| 2. REPL & Commands | Done | Command registry, /help, /clear, /exit |
| 3. UI Components | Done | Theme, spinner, progress, syntax highlighting |
| 4. Session Persistence | Done | SpecStory format, startup screen, /history |
| 5. Context Tracking | Done | Token counting, context bar, color thresholds |
| 6. Permissions | Pending | Trusted paths, permission prompts |
| 7. Tool Execution | Pending | Self-healing, error recovery |
| 8. Git Integration | Pending | /commit, /diff, /undo |
| 9. Workflow Commands | Pending | /spec, /document, /model |
| 10. Fun Facts | Pending | API integration, thinking messages |
| 11. Multi-Agent | Pending | Status bar, parallel execution |

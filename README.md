# Coding Agent CLI

A modern, minimal CLI for an AI coding assistant built in Rust. Features intelligent tool execution, self-healing error recovery, multi-agent coordination, and a clean conversational interface.

## Features

- **Conversational REPL** - Natural back-and-forth with Claude AI
- **Multi-line Input** - Double-enter submission for complex prompts
- **Smart Tools** - Read, write, edit files, execute commands, search code
- **Self-Healing** - Automatically fixes errors and generates regression tests
- **Session Persistence** - Auto-save conversations in human-readable markdown
- **Context Tracking** - Real-time token usage visualization
- **Permission System** - Safe by default with trusted paths
- **Git Integration** - Smart commits with purpose-focused messages
- **Multi-Agent Status** - Visual progress for parallel operations
- **Syntax Highlighting** - Beautiful code display in responses
- **Obsidian Integration** - Create and update notes in your vault

## Quick Start

### Prerequisites

- **Rust** (1.70 or later) - Install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Anthropic API Key** - Get one from [Anthropic Console](https://console.anthropic.com)

- **ripgrep** (optional, for code search) - Install via package manager:
  ```bash
  # macOS
  brew install ripgrep

  # Ubuntu/Debian
  sudo apt install ripgrep

  # Windows (with Chocolatey)
  choco install ripgrep
  ```

### Installation

1. Clone and build:
   ```bash
   git clone <repository-url>
   cd coding-agent
   cargo build --release
   ```

2. Set your API key:
   ```bash
   export ANTHROPIC_API_KEY="your-api-key-here"
   ```

   Or create a `.env` file:
   ```bash
   echo "ANTHROPIC_API_KEY=your-api-key-here" > .env
   ```

3. Run the CLI:
   ```bash
   cargo run --release -p coding-agent-cli
   ```

## Usage

### Startup

On launch, you'll see:

```
┌─────────────────────────────────────────────────────┐
│                                                     │
│   ██████╗ ██████╗ ██████╗ ███████╗                 │
│  ██╔════╝██╔═══██╗██╔══██╗██╔════╝                 │
│  ██║     ██║   ██║██║  ██║█████╗                   │
│  ██║     ██║   ██║██║  ██║██╔══╝                   │
│  ╚██████╗╚██████╔╝██████╔╝███████╗                 │
│   ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝                 │
│                                                     │
│   coding-agent v0.1.0                               │
│                                                     │
│   [n] New session                                   │
│   [r] Resume last session                           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Basic Commands

| Command | Description |
|---------|-------------|
| `/help` | Show all available commands |
| `/clear` | Reset context and clear screen |
| `/exit` | Exit the CLI |
| `/status` | Show active agents and tasks |
| `/cost` | Show token usage and costs |
| `/context` | Show current context window usage |

### Workflow Commands

| Command | Description |
|---------|-------------|
| `/commit` | Analyze changes and create smart commit |
| `/commit --pick` | Interactive file picker for commits |
| `/diff` | Show pending changes |
| `/undo` | Revert last change |
| `/spec <name>` | Create specification file |
| `/document <topic>` | Create/update Obsidian note |
| `/model [name]` | Switch AI model |

### Example Session

```
> Help me refactor the authentication code

Agent: I'll analyze the authentication code. Let me read the relevant files.

● Reading src/auth/mod.rs
● Reading src/auth/jwt.rs
✓ Read 2 files (342 lines)

Agent: I found several opportunities for improvement...
[detailed analysis and suggestions]

> Implement those changes

● Writing src/auth/mod.rs
● Writing tests/auth_test.rs
✓ Changes applied successfully

> Run the tests

● Executing bash: cargo test auth
✓ All 12 tests passed

> Commit these changes

Agent: Analyzing changes for commit...

Proposed commit message:
┌────────────────────────────────────────────────┐
│ Refactor authentication to use trait-based     │
│ providers                                      │
│                                                │
│ Improves testability and extensibility by      │
│ abstracting auth provider implementations.     │
│ Enables easy addition of new auth methods.     │
└────────────────────────────────────────────────┘

[Enter] Commit  [e] Edit  [c] Cancel
```

## Project Structure

This is a Rust workspace with two crates:

```
coding-agent/
├── crates/
│   ├── coding-agent-core/     # State machine library
│   │   ├── src/
│   │   │   ├── machine.rs     # Event-driven state machine
│   │   │   ├── state.rs       # States, events, actions
│   │   │   └── types.rs       # API types
│   │   └── examples/          # Progressive examples
│   │       ├── chat.rs        # Basic chatbot
│   │       ├── read.rs        # File reader
│   │       ├── list_files.rs  # Directory listing
│   │       ├── bash.rs        # Shell execution
│   │       ├── edit.rs        # File editing
│   │       └── code_search.rs # Pattern search
│   │
│   └── coding-agent-cli/      # Terminal application
│       ├── src/
│       │   ├── cli/           # REPL and commands
│       │   ├── ui/            # Visual components
│       │   ├── tools/         # Tool execution
│       │   ├── agents/        # Multi-agent system
│       │   ├── permissions/   # Permission system
│       │   ├── integrations/  # Git, Obsidian, SpecStory
│       │   ├── tokens/        # Token counting
│       │   └── config/        # Configuration
│       └── tests/             # Integration tests
│
├── specs/                     # Specification documents
├── docs/                      # Architecture documentation
└── .specstory/                # Session history (auto-created)
```

## Architecture

The CLI is built on several key architectural patterns:

### Event-Driven State Machine

The core uses a pure state machine (`coding-agent-core`) for predictable AI interactions:
- **Six states**: WaitingForUserInput → CallingLlm → ProcessingLlmResponse → ExecutingTools → Error → ShuttingDown
- **Pure logic**: State machine is synchronous and deterministic
- **Caller executes**: I/O operations are the CLI's responsibility

### Self-Healing Pipeline

Tools automatically recover from errors:
- **Code errors**: Spawn FixAgent to diagnose and repair
- **Network errors**: Retry with exponential backoff
- **Permission errors**: Prompt user interactively
- **Test generation**: Create regression tests for fixes

### Multi-Agent Orchestration

Background agents handle complex tasks in parallel:
- Spawn, track, and cancel agents
- Real-time status bar with progress
- Coordinated execution of interdependent tasks

### Permission System

Three-tier security model:
- **Trusted paths**: Auto-approved (configured)
- **Interactive prompts**: Y/n/Always/Never
- **Session cache**: Remember decisions

See [docs/CLI_ARCHITECTURE.md](docs/CLI_ARCHITECTURE.md) for detailed architecture documentation.

## Configuration

Configuration is stored at `~/.config/coding-agent/config.toml`:

```toml
[permissions]
trusted_paths = [
    "/Users/username/coding-agent",
    "~/Documents/Personal/",
]

[model]
default = "claude-3-opus"
context_window = 200000

[behavior]
streaming = true
tool_verbosity = "standard"
show_context_bar = true
fun_facts = true
fun_fact_delay = 10

[integrations.obsidian]
vault_path = "~/Documents/Personal/"

[integrations.git]
commit_style = "purpose"

[error_recovery]
auto_fix = true
generate_tests = true
max_retry_attempts = 3
```

Edit with: `/config` command

## Running Examples

The `coding-agent-core` crate includes progressive examples showing how the state machine works:

```bash
# Basic chatbot (no tools)
make run-chat

# File reader
make run-read

# Directory listing
make run-list-files

# Shell execution
make run-bash

# File editing
make run-edit

# Code search (requires ripgrep)
make run-code-search
```

Or use cargo directly:
```bash
cargo run -p coding-agent-core --example chat
```

## Development

### Building

```bash
# Build all crates
make build

# Build release
make build-release

# Build CLI only
cargo build -p coding-agent-cli
```

### Testing

```bash
# Run all tests
make test

# Run CLI tests only
cargo test -p coding-agent-cli

# Run integration tests
cargo test --test '*'

# Run doc tests
cargo test --doc
```

### Code Quality

```bash
# Format code
make fmt

# Run linter
make lint

# Check types
cargo check --all-targets
```

## Troubleshooting

**API key not working?**
- Verify it's set: `echo $ANTHROPIC_API_KEY`
- Check `.env` file exists and is readable
- Verify quota on [Anthropic Console](https://console.anthropic.com)

**Terminal not restoring?**
- The CLI has panic handlers to restore terminal state
- If stuck, try: `reset` or restart your terminal

**Context bar not showing?**
- Check config: `show_context_bar = true`
- Verify terminal width is sufficient (min 60 chars)

**Tools not executing?**
- Check file permissions for writes
- Verify ripgrep installed for code search
- Check trusted_paths configuration

**Build errors?**
- Clean and rebuild: `cargo clean && cargo build`
- Update Rust: `rustup update`
- Check Rust version: `rustc --version` (need 1.70+)

## Documentation

- [CLI Architecture](docs/CLI_ARCHITECTURE.md) - Detailed architecture guide
- [State Machine](docs/STATE_MACHINE.md) - Core state machine docs
- [CLI Specification](specs/command-line-interface.md) - Full feature spec
- [Project Context](CLAUDE.md) - Overview for Claude Code

## Contributing

Contributions welcome! Please:

1. Follow existing code style (use `cargo fmt`)
2. Add tests for new features
3. Update documentation
4. Run `cargo clippy` before submitting

## License

See LICENSE file for details.

## Acknowledgments

Built with:
- [Anthropic Claude](https://www.anthropic.com/claude) - AI assistant
- [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs) - Token counting
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal control
- [indicatif](https://github.com/console-rs/indicatif) - Progress bars
- [syntect](https://github.com/trishume/syntect) - Syntax highlighting
- [git2](https://github.com/rust-lang/git2-rs) - Git integration

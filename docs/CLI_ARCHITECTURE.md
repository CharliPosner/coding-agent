# CLI Architecture

> Comprehensive architecture guide for the coding-agent CLI

The CLI (`coding-agent-cli`) is a conversational REPL interface that makes AI-assisted coding natural and transparent. Built on design principles of **transparency**, **minimal friction**, **clean aesthetics**, **progressive disclosure**, **workflow-native integration**, and **self-healing**.

## High-Level Architecture

The CLI follows a layered architecture where each layer has clear responsibilities:

```text
┌─────────────────────────────────────────────────────────────┐
│                    User Interface Layer                     │
│  (Terminal I/O, Spinners, Context Bar, Status Bar)         │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    REPL Orchestration                       │
│  (Input Handling, Command Routing, Conversation Flow)      │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    Agent Layer                              │
│  (AgentManager, FixAgent, Tool Execution, Self-Healing)    │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    Integration Layer                        │
│  (SpecStory, Git, Obsidian, Claude API)                    │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    Core State Machine                       │
│  (coding-agent-core: Pure state transitions)               │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```text
crates/
├── coding-agent-core/     # State machine (shared library)
└── coding-agent-cli/      # Terminal application
    └── src/
        ├── main.rs        # Entry point, panic handler
        ├── cli/           # REPL orchestration
        │   ├── repl.rs              # Main REPL loop
        │   ├── startup.rs           # Welcome screen
        │   ├── input.rs             # Multi-line input, double-enter
        │   ├── terminal.rs          # Raw mode management
        │   ├── output.rs            # Formatted output
        │   └── commands/            # Slash command registry
        │       ├── mod.rs           # Command trait & registry
        │       ├── help.rs          # /help
        │       ├── clear.rs         # /clear (reset context)
        │       ├── status.rs        # /status (active agents)
        │       ├── config.rs        # /config
        │       ├── cost.rs          # /cost (token breakdown)
        │       ├── context.rs       # /context
        │       ├── commit.rs        # /commit (smart git)
        │       ├── diff.rs          # /diff
        │       ├── undo.rs          # /undo
        │       ├── spec.rs          # /spec (planning mode)
        │       ├── document.rs      # /document (Obsidian)
        │       └── model.rs         # /model (switch AI model)
        ├── ui/              # Visual components
        │   ├── theme.rs             # Color definitions
        │   ├── spinner.rs           # Animated spinners
        │   ├── thinking.rs          # Thinking messages
        │   ├── fun_facts.rs         # Long-wait entertainment
        │   ├── context_bar.rs       # Token usage visualization
        │   ├── status_bar.rs        # Multi-agent progress
        │   ├── progress.rs          # Progress bars
        │   ├── syntax.rs            # Code syntax highlighting
        │   └── components.rs        # Reusable UI pieces
        ├── agents/          # Multi-agent orchestration
        │   ├── manager.rs           # Spawn, track, cancel agents
        │   ├── status.rs            # Agent state machine
        │   └── fix_agent.rs         # Self-healing agent
        ├── tools/           # Tool execution pipeline
        │   ├── executor.rs          # Run tools with retries
        │   ├── recovery.rs          # Error recovery strategies
        │   ├── diagnostics.rs       # Parse error messages
        │   └── definitions.rs       # Tool schemas for Claude
        ├── permissions/     # Permission system
        │   ├── trusted.rs           # Trusted path logic
        │   ├── checker.rs           # Permission checking
        │   └── prompt.rs            # Interactive prompts
        ├── integrations/    # External services
        │   ├── specstory.rs         # Session persistence
        │   ├── git.rs               # Git operations
        │   └── obsidian.rs          # Obsidian vault
        ├── tokens/          # Token tracking
        │   ├── counter.rs           # tiktoken-rs wrapper
        │   ├── pricing.rs           # Cost calculation
        │   └── context.rs           # Context window mgmt
        └── config/          # Configuration
            └── settings.rs          # ~/.config/coding-agent/
```

## Core Architectural Patterns

### 1. Event-Driven State Machine

The CLI orchestrates an event-driven state machine from `coding-agent-core`:

- **State machine is pure** - No I/O, synchronous, deterministic
- **CLI executes actions** - API calls, tool execution, file operations
- **Six states**: WaitingForUserInput → CallingLlm → ProcessingLlmResponse → ExecutingTools → Error → ShuttingDown
- **Bounded retries** - Exponential backoff (1s, 2s, 3s) with max 3 retries

### 2. Command Registry Pattern

Slash commands follow a trait-based extensible pattern:

```rust
pub trait Command {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<()>;
}

pub struct CommandContext<'a> {
    pub repl: &'a mut Repl,
    pub agent_manager: Arc<AgentManager>,
    pub session: &'a mut Session,
    pub config: &'a Config,
}
```

Commands register themselves in `CommandRegistry` on startup. Adding new commands requires implementing the trait and registering in `mod.rs`.

### 3. Multi-Agent Orchestration

The `AgentManager` coordinates parallel agent execution:

- **Spawn agents** - Background tasks for self-healing, search, refactoring
- **Track state** - queued → running → complete/failed
- **Display progress** - Status bar updates in real-time
- **Cancel agents** - User can abort long-running operations

### 4. Self-Healing Pipeline

Tool execution includes automatic error recovery:

```text
Tool Call → ToolExecutor → Error? → Categorize
                                        │
                                        ├─ Code Error → FixAgent
                                        ├─ Network Error → Retry with backoff
                                        ├─ Permission Error → Prompt user
                                        └─ Resource Error → Suggest alternatives
```

### 5. Permission System

Three-tier permission model:

- **Trusted paths** - Auto-approved (from config)
- **Interactive prompts** - Y/n/Always/Never for untrusted writes
- **Session cache** - Decisions remembered during session

### 6. Streaming UI Updates

All long-running operations provide real-time feedback:

- **Spinners** - Tool execution progress
- **Progress bars** - Multi-step operations
- **Thinking messages** - Rotated during API calls
- **Fun facts** - Displayed after 10+ second waits
- **Context bar** - Token usage updated after each exchange

## Execution Flow

### Startup Flow

```text
main.rs
    │
    ├─► Install panic handler (ensure terminal cleanup)
    ├─► Load config from ~/.config/coding-agent/config.toml
    ├─► Load .env for ANTHROPIC_API_KEY
    │
    ▼
cli::run()
    │
    ├─► Terminal::enable_raw_mode()
    │   └─► Drop guard ensures restoration on panic/exit
    │
    ├─► StartupScreen::show()
    │   ├─ Display ASCII logo
    │   ├─ [n] New session
    │   ├─ [r] Resume last ──► SessionManager::load_last()
    │   │   └─► Parse .specstory/history/<session>.md
    │   ├─ [h] Help
    │   └─ [c] Config
    │
    ├─► Repl::new(config)
    │   ├─ Initialize SessionManager
    │   ├─ Initialize TokenCounter (tiktoken-rs)
    │   ├─ Initialize AgentManager (Arc for sharing)
    │   ├─ Initialize ToolExecutor (with retry logic)
    │   ├─ Initialize PermissionChecker (trusted paths)
    │   ├─ Initialize ContextBar
    │   ├─ Initialize CommandRegistry
    │   └─ Load conversation history if resuming
    │
    └─► Repl::run() (main loop)
```

### Main REPL Loop

```text
loop {
    ┌─────────────────────────────────────────────┐
    │  1. Display Context Bar                     │
    │     - Show token usage at bottom of screen  │
    │     - Color-coded: green/yellow/red         │
    └─────────────────────────────────────────────┘
                       │
    ┌──────────────────▼──────────────────────────┐
    │  2. Display Status Bar (if agents active)   │
    │     - Show running agents with progress     │
    │     - Update in real-time                   │
    └─────────────────────────────────────────────┘
                       │
    ┌──────────────────▼──────────────────────────┐
    │  3. Read User Input                         │
    │     - InputHandler::read_input()            │
    │     - Multi-line with double-enter submit   │
    │     - Ctrl+C clears, Ctrl+D exits           │
    └─────────────────────────────────────────────┘
                       │
              ┌────────▼────────┐
              │  Slash command? │
              └────────┬────────┘
                Yes │       │ No
        ┌───────────▼       ▼───────────────┐
        │                                    │
    ┌───▼────────────────────┐  ┌──────────▼────────────┐
    │ CommandRegistry::      │  │ Process as message    │
    │  execute()             │  │  to Claude            │
    │                        │  └──────────┬────────────┘
    │ - Parse command & args │             │
    │ - Look up in registry  │  ┌──────────▼────────────┐
    │ - Execute with context │  │ Add to conversation   │
    └────────────────────────┘  │  history              │
                                │                       │
                                │ Call Claude API       │
                                │  (ureq HTTP client)   │
                                └──────────┬────────────┘
                                           │
                                ┌──────────▼────────────┐
                                │ Process response      │
                                │                       │
                                │ Text? → Display       │
                                │ Tool call? → Execute  │
                                └──────────┬────────────┘
                                           │
                                ┌──────────▼────────────┐
                                │ Tool Execution        │
                                │  (if tool calls)      │
                                │                       │
                                │ - ToolExecutor::      │
                                │    execute()          │
                                │ - Show spinner        │
                                │ - Check permissions   │
                                │ - Run tool            │
                                │ - Handle errors       │
                                │ - Return result       │
                                │                       │
                                │ Send results back     │
                                │  to Claude            │
                                └──────────┬────────────┘
                                           │
                                ┌──────────▼────────────┐
                                │ Update State          │
                                │                       │
                                │ - Count tokens        │
                                │ - Update context bar  │
                                │ - Save session        │
                                │   (.specstory/)       │
                                └───────────────────────┘
                                           │
                                           │
    } ←────────────────────────────────────┘
```

## Key Subsystems

### Input Handling

The `InputHandler` operates in terminal raw mode (via `crossterm`):

**Key Bindings:**

- **Enter** - Add newline to buffer
- **Enter + Enter** - Submit input
- **Ctrl+C** - Clear current input buffer
- **Ctrl+D** - Exit REPL
- **Backspace** - Remove last character

**Raw Mode Details:**

- All input processed character-by-character
- `\n` only moves cursor down, must use `\r\n` for proper newline
- Terminal restored on drop via RAII guard (handles panics)
- Panic handler installed to ensure cleanup

**Multi-line Input:**

- Buffer accumulates characters and newlines
- Double consecutive Enter triggers submission
- Cursor tracking for proper display in raw mode

### Command System

**Command Categories:**

| Category | Commands | Purpose |
|----------|----------|---------|
| Navigation | `/help`, `/clear`, `/exit`, `/status` | REPL control |
| Code & Files | `/undo`, `/diff`, `/context` | File operations |
| Git | `/commit`, `/commit --pick` | Smart git integration |
| Workflow | `/model`, `/cost`, `/spec`, `/document` | Productivity tools |
| Session | `/history`, `/config` | Session management |

**Key Commands:**

- **`/clear`** - Saves session first, then resets conversation context and clears display
- **`/commit`** - Agent analyzes changes, groups related files, generates purpose-focused message
- **`/status`** - Shows active agents, running tools, current operations
- **`/cost`** - Detailed token breakdown with pricing by model

### Claude API Integration

**Architecture:**

```rust
// Claude API call via ureq (synchronous HTTP)
let response = ureq::post("https://api.anthropic.com/v1/messages")
    .set("x-api-key", &api_key)
    .set("anthropic-version", "2023-06-01")
    .send_json(request)?;
```

**Request Flow:**

1. Build request with conversation history
2. Include available tools (read, write, edit, list, bash, search)
3. Send to Claude API via `ureq`
4. Parse response (text or tool calls)
5. Execute tools if requested
6. Send tool results back to Claude
7. Repeat until final text response

**Environment:**

- Requires `ANTHROPIC_API_KEY` in environment or `.env`
- Auto-loads via `dotenvy` crate on startup
- API key never logged or displayed

### Tool Execution Pipeline

**Available Tools:**

| Tool | Purpose | Permission |
|------|---------|------------|
| `read_file` | Read file contents | Always allowed |
| `write_file` | Create/overwrite files | Requires permission |
| `edit_file` | Targeted file edits | Requires permission |
| `list_files` | Directory listing | Always allowed |
| `bash` | Execute shell commands | Requires permission |
| `code_search` | Search via ripgrep | Always allowed |

**Execution Flow:**

```text
Tool Request from Claude
    │
    ▼
ToolExecutionSpinner::start()
    │
    ▼
PermissionChecker::check() ──► Untrusted? ──► Prompt user
    │                                              │
    │ Trusted/Approved                            │
    │ ◄───────────────────────────────────────────┘
    ▼
ToolExecutor::execute()
    │
    ├─► Success ──► finish_success()
    │
    └─► Error ──► Categorize ──► Auto-fixable? ──► FixAgent::spawn()
                                      │
                                      │ Fixed?
                                      │
                                      └─► Retry original tool
```

### Self-Healing Error Recovery

**Error Categories:**

| Category | Strategy |
|----------|----------|
| Code Error | Spawn FixAgent to diagnose and repair |
| Network Error | Retry with exponential backoff |
| Permission Error | Prompt user for permission |
| Resource Error | Suggest alternatives |

**FixAgent Flow:**

```text
1. Detect code error (e.g., missing dependency, syntax error)
2. Spawn FixAgent with error context
3. FixAgent analyzes error message
4. Generate fix (e.g., add to Cargo.toml, fix import)
5. Apply fix to codebase
6. Generate regression test
7. Verify fix compiles/passes tests
8. Retry original operation
9. Report success/failure to user
```

**Auto-Fixes:**

- Missing dependency → Add to `Cargo.toml` or `package.json`
- Missing import → Add import statement
- Type mismatch → Suggest type annotation
- Syntax error → Attempt correction

### Session Persistence (SpecStory)

**Format:**

Conversations saved as markdown in `.specstory/history/`:

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

**Operations:**

- **Auto-save** - After each message exchange
- **Load** - Parse markdown back to conversation
- **List** - Show all sessions with metadata
- **Resume** - Restore full conversation context

### Token Tracking

**Implementation:**

- Uses `tiktoken-rs` with `cl100k_base` tokenizer (Claude-compatible)
- Counts tokens for all messages in conversation
- Adds overhead: +4 tokens per message, +3 tokens system overhead
- Cached globally via `OnceLock` for performance

**Context Bar:**

```text
Context: ████████████░░░░░░░░░░░░░░░░░░  38% │ 76k / 200k
```

- Position: Bottom of screen, always visible
- Color-coded: Green (0-60%), Yellow (60-85%), Red (85%+)
- Updates after each message exchange

### Permission System

**Trusted Paths:**

Configured in `~/.config/coding-agent/config.toml`:

```toml
[permissions]
trusted_paths = [
    "/Users/username/coding-agent",
    "~/Documents/Personal/",
]
```

**Permission Prompt:**

```text
Write to untrusted path: /tmp/test.txt

Allow this operation?
  [y] Yes, this time
  [n] No
  [a] Always for this path
  [v] Never
```

**Behavior:**

- **"Always"** - Adds to `trusted_paths` in config
- **"Never"** - Caches denial for session
- **Read operations** - Always allowed everywhere

### Multi-Agent Status Bar

Displayed above input when agents are active:

```text
┌─────────────────────────────────────────────┐
│  AGENTS ─────────────────────────────────── │
│  ● search-agent    Searching...       ██░░  │
│  ● refactor-agent  Analyzing...       ███░  │
│  ○ test-agent      Queued                   │
└─────────────────────────────────────────────┘
```

**States:**

- **○ Queued** - Waiting to start
- **● Running** - Active with progress bar
- **✓ Complete** - Successfully finished
- **✗ Failed** - Error state

### Git Integration

**Smart Commits:**

```text
/commit flow:
1. Read git status (staged/unstaged changes)
2. Group related files (same dir, test+impl, etc.)
3. Analyze changes to understand purpose
4. Generate 3-sentence commit message focused on "why"
5. Show preview, allow edit
6. Commit with message
```

**File Grouping Logic:**

- Files in same directory → group together
- Test files + implementation → group together
- Unrelated changes → suggest separate commits

**Commit Message Style:**

```text
Add JWT token validation to login flow

Users can now stay logged in across sessions with
secure token refresh. This improves UX by eliminating
repeated login prompts.
```

### Obsidian Integration

**Operations:**

- **Search vault** - Find notes by content
- **Create note** - Suggest location, generate template
- **Update note** - Show diff, apply changes
- **Link notes** - Add backlinks and metadata

**Vault Path:**

Configured in `~/.config/coding-agent/config.toml`:

```toml
[integrations.obsidian]
vault_path = "~/Documents/Personal/"
```

## Configuration

Location: `~/.config/coding-agent/config.toml`

**Full Configuration:**

```toml
[permissions]
trusted_paths = [
    "/Users/username/coding-agent",
    "~/Documents/Personal/",
]
auto_read = true

[model]
default = "claude-3-opus"
available = ["claude-3-opus", "claude-3-sonnet"]
context_window = 200000

[theme]
style = "minimal"  # minimal | colorful | monochrome

[persistence]
enabled = true
format = "specstory"
path = ".specstory/history/"

[behavior]
streaming = true
tool_verbosity = "standard"  # minimal | standard | verbose
show_context_bar = true
fun_facts = true
fun_fact_delay = 10  # seconds

[fun_facts_api]
enabled = true
sources = ["uselessfacts", "jokes", "quotes"]
cache_size = 100
refresh_interval = 3600

[error_recovery]
auto_fix = true
generate_tests = true
max_retry_attempts = 3

[integrations.obsidian]
vault_path = "~/Documents/Personal/"

[integrations.git]
auto_stage = false
commit_style = "purpose"  # purpose | conventional | simple
```

## Theme System

Colors centralized in `ui/theme.rs`:

| Element | Color | Purpose |
|---------|-------|---------|
| User input | White | Neutral |
| Agent response | Cyan | Distinguish from user |
| Tool calls | Yellow | Action happening |
| Success | Green | Completion |
| Error | Red | Problems |
| Warning | Orange | Caution |
| Muted/secondary | Gray | Less important |
| Cost/tokens | Magenta | Resource usage |
| Context bar | Green→Yellow→Red | Usage level |

**Accessibility:**

- Respects `NO_COLOR` environment variable
- Falls back to plain text if colors unsupported

## Dependencies

**Core:**

| Crate | Purpose |
|-------|---------|
| `ureq` | HTTP client for Claude API |
| `dotenvy` | Load `.env` files |
| `serde` + `serde_json` | Serialization |
| `toml` | Config file parsing |

**Terminal & UI:**

| Crate | Purpose |
|-------|---------|
| `crossterm` | Cross-platform terminal control |
| `console` | Terminal styling |
| `indicatif` | Spinners, progress bars |
| `syntect` | Syntax highlighting |

**Tools & Integrations:**

| Crate | Purpose |
|-------|---------|
| `walkdir` | Directory traversal |
| `git2` | Git operations |
| `tiktoken-rs` | Token counting |

**Async:**

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime (for future streaming) |
| `reqwest` | Async HTTP (for fun facts API) |

## Design Decisions

### Why Synchronous API Calls?

- Simpler control flow in REPL
- Easier error handling
- `ureq` is lightweight and sufficient
- Can add async/streaming later without major refactor

### Why Raw Mode?

- Full control over input handling
- Enables double-enter submission
- Enables multi-line input buffer
- Better UX than line-buffered input

### Why SpecStory Format?

- Human-readable markdown
- Version controllable
- No database required
- Easy to backup/sync
- Searchable with standard tools

### Why Trait-Based Commands?

- Easy to extend
- Type-safe
- Compile-time guarantees
- No runtime reflection needed

### Why Separate Agent Manager?

- Enables parallel execution
- Clear cancellation model
- Progress tracking
- Future: distributed agents

## Testing Strategy

**Unit Tests:**

- Pure functions (token counting, parsing)
- Config loading/saving
- Command parsing
- Error categorization

**Integration Tests:**

- Git operations (commit, status, diff)
- SpecStory save/load
- Permission checking
- Tool execution

**End-to-End Tests:**

- Full REPL sessions (PTY-based with `expectrl`)
- Mock Claude API (with `wiremock`)
- Terminal output snapshots (with `insta`)

## Future Enhancements

**Phase 14+ Features:**

- Multi-agent coordination (status bar, parallel execution)
- Self-healing (FixAgent, auto-fix, test generation)
- Advanced git features (file grouping, smart commit messages)
- Obsidian full integration (search, create, update)
- Fun facts API (entertainment during long waits)
- Planning mode (`/spec` for collaborative planning)
- Voice input (Whisper integration)
- Plugin system (custom commands via Lua/Rust)

## Common Patterns for New Features

### Adding a New Command

1. Create new file in `src/cli/commands/`
2. Implement `Command` trait
3. Register in `src/cli/commands/mod.rs`
4. Add to `/help` output
5. Write unit tests
6. Update documentation

### Adding a New Tool

1. Define tool schema in `src/tools/definitions.rs`
2. Implement tool function (takes `serde_json::Value`)
3. Register in `ToolExecutor`
4. Add permission check if needed
5. Add to tool documentation
6. Write integration tests

### Adding a New UI Component

1. Create in `src/ui/`
2. Use `Theme` for colors
3. Handle terminal resize
4. Handle `NO_COLOR`
5. Write snapshot tests
6. Document in architecture

## Troubleshooting

**Terminal Not Restoring:**

- Check panic handler installed in `main.rs`
- Verify `Terminal` struct has proper Drop impl
- Test with intentional panics

**API Key Not Loading:**

- Check `.env` file exists in project root
- Verify `ANTHROPIC_API_KEY` set in environment
- Check `dotenvy` loaded before API call

**Context Bar Not Updating:**

- Verify `TokenCounter::count()` called after each message
- Check `ContextBar::update()` called
- Verify terminal width for proper rendering

**Permission Prompt Not Showing:**

- Check path not in `trusted_paths`
- Verify `PermissionChecker::check()` called
- Check session cache not caching decision

**Tools Not Executing:**

- Verify tool registered in `ToolExecutor`
- Check Claude API includes tool in request
- Verify tool schema matches Claude's format
- Check logs for parse errors

## References

- **Spec**: `specs/command-line-interface.md` - Full CLI specification
- **State Machine**: `docs/STATE_MACHINE.md` - Core state machine docs
- **CLAUDE.md**: `/Users/charliposner/coding-agent/CLAUDE.md` - Project overview

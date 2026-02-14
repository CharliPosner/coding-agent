# Coding Agent CLI Specification

> A modern, minimal CLI for an AI coding assistant

---

## Vision

A conversational REPL interface that makes AI-assisted coding feel natural and transparent. Clean aesthetics, clear tool visibility, and intelligent defaults that stay out of your way.

---

## Design Principles

1. **Transparency** - Always show what tools are being called and their results
2. **Minimal friction** - Smart defaults, auto-permissions for trusted repos
3. **Clean aesthetic** - Thoughtful color-coding, whitespace, no visual clutter
4. **Progressive disclosure** - Simple by default, power features when needed
5. **Workflow-native** - Built-in commands for common workflows (git, specs, docs)
6. **Self-healing** - When tools fail, fix them automatically

---

## Core Features

### Startup Behavior

On launch, show a welcome screen with session options:

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
│       └─ "Adding auth to dashboard" (2h ago)        │
│                                                     │
│   [h] Help  [c] Config                              │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Startup options:**
- **New session** - Fresh context, new conversation
- **Resume last** - Restore previous session from SpecStory history
- Shows preview of last session (title/summary + time ago)

### Input & Interaction

- **Conversational REPL** - Multi-line input support, natural back-and-forth
- **Input submission** - `Enter` adds newline, `Double-Enter` submits
- **Slash commands** - Extensible command system (see Commands section)
- **Command history** - Persistent across sessions, searchable
- **Auto-complete** - For slash commands and file paths

### Output & Display

- **Tool call visibility** - Standard verbosity: action + target (e.g., "Reading src/auth/mod.rs")
- **Streaming responses** - Real-time output as the agent thinks/writes
- **Result summaries** - Compact, scannable summaries of tool results
- **Syntax highlighting** - For code blocks in responses
- **Collapsible sections** - For verbose output (file contents, long diffs)
- **Multi-agent status bar** - Visual progress for parallel agent tasks
- **Context bar** - Sliding bar at bottom showing context used vs. remaining

### Context Bar

A persistent visual indicator at the **bottom** of the CLI showing context window usage:

```
┌─────────────────────────────────────────────────────┐
│  Context: ████████████░░░░░░░░░░░░░░░░░░  38% used  │
│           76k / 200k tokens                         │
└─────────────────────────────────────────────────────┘
```

- **Position:** Bottom of screen, always visible
- Shows cumulative token usage for the session
- Color-coded: green (0-60%), yellow (60-85%), red (85%+)
- Cost displayed separately via `/cost` command (not in bar)
- Updates after each message exchange

### Permission System

- **Trusted paths** (auto-yes for all operations):
  - `/Users/charliposner/coding-agent` (this repo)
  - `~/Documents/Personal/` (Obsidian vault)
- **Other locations** - Prompt for permission before writing/modifying
- **Read-only operations** - Always allowed everywhere

### Session Persistence (SpecStory Integration)

Conversations are automatically saved using [SpecStory](https://specstory.com) format:

- **Auto-save location:** `.specstory/history/` in project root
- **Format:** Markdown files, human-readable
- **Behavior:** Saves after each exchange, no manual intervention needed
- **Benefits:**
  - Searchable conversation history
  - Can be version-controlled
  - Readable by humans and machines
  - Resume sessions across restarts

---

## Commands & Shortcuts

### Navigation & UI

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/clear` | **Reset display AND context** - starts fresh conversation |
| `/status` | Show agent status, active tasks, running agents |
| `/config` | Open/edit configuration |
| `/history` | Browse conversation history (SpecStory) |
| `Ctrl+C` | Cancel current operation |
| `Ctrl+D` | Exit |

**`/clear` behavior:**
- Clears the terminal display
- Resets the conversation context (fresh start)
- Saves current conversation to SpecStory before clearing
- Resets context bar to 0%

### Code & Files

| Command | Description |
|---------|-------------|
| `/undo` | Undo last file change |
| `/diff` | Show pending/recent changes |
| `/context` | Show current context (loaded files, working directory, token usage) |

### Git Integration

| Command | Description |
|---------|-------------|
| `/commit` | Agent analyzes changes and commits with purpose-focused message |
| `/commit --pick` | Interactive file picker for manual selection |

**`/commit` behavior:**
- **Default (no flags):** Agent analyzes all changes, decides what's logically related, commits
- **With `--pick`:** Shows interactive file picker for user selection
- Generates 3-sentence commit message focused on *purpose* not code
- Preview message, allow edit, then commit

Example commit message style:
```
Add user authentication flow for the dashboard

This enables users to securely log in before accessing
sensitive data. Implements JWT-based session management
with automatic token refresh.
```

### Workflow Commands

| Command | Description |
|---------|-------------|
| `/model [name]` | Switch AI model (show current if no arg) |
| `/cost` | Show detailed token usage and cost breakdown |
| `/spec [name]` | Create new spec file and enter planning mode |
| `/document [topic]` | Add or update an Obsidian note |

**`/spec` behavior:**
1. Create `specs/<name>.md` if it doesn't exist
2. Enter planning/discussion mode
3. Collaborative back-and-forth to build out the spec
4. Agent suggests structure, user refines

**`/document` behavior:**
1. Search existing notes in `~/Documents/Personal/` for related content
2. If updating: show existing note, propose changes
3. If new: suggest file location and initial structure
4. Write/update the markdown file

---

## UI/UX Design

### Color Scheme & Theming

**Base palette (minimal, modern):**

| Element | Color | Purpose |
|---------|-------|---------|
| User input | White/default | Clean, neutral |
| Agent response | Cyan/light blue | Distinguish from user |
| Tool calls | Yellow/amber | Attention, action happening |
| Success | Green | Completion, confirmation |
| Error | Red | Problems, failures |
| Warning | Orange | Caution, needs attention |
| Muted/secondary | Gray | Less important info |
| Cost/tokens | Magenta | Resource usage |
| Context bar | Green → Yellow → Red | Usage level indicator |

### Layout

**Full screen layout:**
```
┌─────────────────────────────────────────────────────┐
│  coding-agent v0.1.0              claude-3-opus     │  <- Header (version + model)
├─────────────────────────────────────────────────────┤
│                                                     │
│  You: What files handle authentication?             │
│                                                     │
│  ● Reading src/auth/mod.rs                          │
│  ● Reading src/auth/jwt.rs                          │
│  ✓ Found 3 authentication files                     │
│                                                     │
│  Agent: Found 3 files related to authentication:    │
│                                                     │
│    src/auth/mod.rs         - Main auth module       │
│    src/auth/jwt.rs         - JWT token handling     │
│    src/middleware/auth.rs  - Auth middleware        │
│                                                     │
│                                                     │
│                                                     │  <- Conversation area (scrolls)
│                                                     │
├─────────────────────────────────────────────────────┤
│  > _                                                │  <- Input area
├─────────────────────────────────────────────────────┤
│  Context: ████████░░░░░░░░░░░░░░░░░░░░  25% │ 50k   │  <- Context bar (bottom)
└─────────────────────────────────────────────────────┘
```

**Multi-agent status bar (when active):**
```
┌─────────────────────────────────────────────────────┐
│  AGENTS ─────────────────────────────────────────── │
│  ● search-agent    Searching for auth files... ██░░ │
│  ● refactor-agent  Analyzing dependencies...   ███░ │
│  ○ test-agent      Queued                           │
└─────────────────────────────────────────────────────┘
```

### Thinking Messages & Long Wait Entertainment

When the agent is working, show contextual "thinking" messages:

**Standard thinking messages (rotate):**
- "Pondering..."
- "Percolating..."
- "Cogitating..."
- "Mulling it over..."
- "Connecting dots..."

**Long wait mode (>10 seconds):**
When operations take longer, fetch and display fun facts via API:

```
● Refactoring auth module...

  Did you know? The first computer bug was an actual bug—
  a moth found in Harvard's Mark II computer in 1947.

  ━━━━━━━━━━━━━━━━━━━━━━░░░░░░░░░░  65%
```

**Configurable:** Users can disable fun facts if they prefer minimal output.

### Fun Facts API Integration

Fun facts are fetched from external APIs for variety:

**Potential API sources:**
- [uselessfacts.jsph.pl](https://uselessfacts.jsph.pl/) - Random fun facts
- [official-joke-api](https://official-joke-api.appspot.com/) - Programming jokes
- [quotable.io](https://quotable.io/) - Motivational quotes
- Custom curated API (future) - Programming-specific facts

**Implementation:**
- Cache facts locally for offline use
- Fetch new batch periodically in background
- Fallback to curated list if API unavailable
- Filter for programming/tech relevance where possible

### Animations & Feedback

- **Spinners** - Contextual spinners showing current operation
- **Progress bars** - For long operations, multi-agent tasks
- **Subtle transitions** - Smooth appearance of new content
- **No excessive animation** - Keep it professional (except fun facts)

---

## Error Recovery & Self-Healing

When a tool fails, the system doesn't just report the error—it tries to fix it.

### Error Recovery Flow

```
1. Tool fails with error
2. Analyze error type:
   - Code error? → Spawn fix-agent
   - Permission error? → Request permission or suggest fix
   - Network error? → Retry with backoff
   - Resource error? → Suggest alternatives
3. If code error:
   a. fix-agent diagnoses the issue
   b. Proposes and applies fix
   c. Writes regression test to prevent recurrence
   d. Re-runs original tool
4. Report resolution to user
```

### Example: Self-Healing Tool

```
● Running build...
✗ Build failed: missing dependency 'serde_json'

  → Diagnosing issue...
  → Found: Cargo.toml missing serde_json dependency
  → Applying fix...

  + serde_json = "1.0"  (added to Cargo.toml)

  → Writing test to verify fix...
  → Re-running build...

✓ Build succeeded (auto-fixed missing dependency)
```

### Self-Healing Capabilities

| Error Type | Auto-Fix Action |
|------------|-----------------|
| Missing dependency | Add to Cargo.toml/package.json |
| Type mismatch | Suggest/apply type annotation |
| Import missing | Add import statement |
| Syntax error | Attempt correction |
| Test failure | Analyze and propose fix |
| Permission denied | Request elevation or suggest chmod |

### Test Generation

When auto-fixing code issues, the system generates a regression test:

```rust
// Auto-generated test for fix: missing serde_json dependency
#[test]
fn test_json_serialization_available() {
    // Ensures serde_json is properly configured
    let value = serde_json::json!({"test": true});
    assert!(value.is_object());
}
```

---

## Technical Architecture

### Platform Support

- **Primary:** macOS
- **Goal:** Cross-platform (macOS, Linux, Windows)
- Use cross-platform crates, test on multiple platforms

### Rust Crate Stack

| Layer | Crate | Purpose |
|-------|-------|---------|
| Prompts & UI | `cliclack` | Beautiful, minimal prompts and styled output |
| Spinners/Progress | `indicatif` | Spinners, progress bars, multi-progress |
| Terminal handling | `crossterm` | Cross-platform terminal control |
| Colors/Styling | `console` | Terminal styling (used by indicatif) |
| TUI (status bar) | `ratatui` | Multi-agent status bar, context bar |
| Async runtime | `tokio` | Async operations, streaming |
| HTTP client | `reqwest` | Fun facts API calls |
| Serialization | `serde` + `toml` | Config files, history persistence |
| CLI parsing | `clap` | Argument parsing, subcommands |
| Git operations | `git2` | Native git integration for /commit |
| File watching | `notify` | Watch for external file changes |
| Token counting | `tiktoken-rs` | Accurate token counting for context bar |

### Project Structure

```
src/
├── main.rs                 # Entry point
├── cli/
│   ├── mod.rs
│   ├── repl.rs             # Main REPL loop, input handling
│   ├── startup.rs          # Welcome screen, session selection
│   ├── input.rs            # Multi-line input, double-enter detection
│   ├── output.rs           # Rendering, streaming, formatting
│   ├── history.rs          # SpecStory integration
│   └── commands/
│       ├── mod.rs          # Command registry
│       ├── help.rs
│       ├── clear.rs        # Clear + reset context
│       ├── status.rs
│       ├── config.rs
│       ├── model.rs
│       ├── cost.rs
│       ├── context.rs
│       ├── commit.rs       # Git commit workflow
│       ├── spec.rs         # Spec creation workflow
│       └── document.rs     # Obsidian integration
├── ui/
│   ├── mod.rs
│   ├── theme.rs            # Colors, styling
│   ├── spinner.rs          # Tool call spinners
│   ├── thinking.rs         # Thinking messages
│   ├── fun_facts.rs        # API integration for fun facts
│   ├── context_bar.rs      # Context usage visualization (bottom)
│   ├── status_bar.rs       # Multi-agent status bar
│   └── components.rs       # Reusable UI pieces
├── agents/
│   ├── mod.rs
│   ├── manager.rs          # Multi-agent orchestration
│   ├── status.rs           # Agent state tracking
│   └── fix_agent.rs        # Self-healing error recovery
├── tools/
│   ├── mod.rs
│   ├── executor.rs         # Tool execution with error handling
│   └── recovery.rs         # Error recovery strategies
├── permissions/
│   ├── mod.rs
│   └── trusted.rs          # Trusted path logic
├── integrations/
│   ├── mod.rs
│   ├── git.rs              # Git operations
│   ├── obsidian.rs         # Obsidian vault operations
│   └── specstory.rs        # Conversation persistence
├── tokens/
│   ├── mod.rs
│   ├── counter.rs          # Token counting
│   └── context.rs          # Context window management
└── config/
    ├── mod.rs
    └── settings.rs         # User configuration
```

### Configuration File

Location: `~/.config/coding-agent/config.toml`

```toml
[permissions]
trusted_paths = [
    "/Users/charliposner/coding-agent",
    "~/Documents/Personal/",
]
auto_read = true

[model]
default = "claude-3-opus"
available = ["claude-3-opus", "claude-3-sonnet", "gpt-4", "gpt-4-turbo"]
context_window = 200000  # tokens

[theme]
style = "minimal"  # minimal | colorful | monochrome

[persistence]
enabled = true
format = "specstory"  # specstory | markdown | json
path = ".specstory/history/"

[behavior]
streaming = true
tool_verbosity = "standard"  # minimal | standard | verbose
show_context_bar = true
fun_facts = true  # Show fun facts during long waits
fun_fact_delay = 10  # Seconds before showing fun facts

[fun_facts_api]
enabled = true
sources = ["uselessfacts", "jokes", "quotes"]
cache_size = 100
refresh_interval = 3600  # seconds

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

---

## Detailed Command Specs

### `/commit` - Smart Git Commits

```
/commit [--pick] [--all] [--amend] [files...]

Options:
  --pick, -p  Interactive file picker (override agent decision)
  --all, -a   Stage all modified files
  --amend     Amend the previous commit
  files...    Specific files to stage

Default Flow (agent decides):
  1. Agent analyzes all staged/unstaged changes
  2. Groups logically related changes
  3. Commits with purpose-focused message
  4. May suggest splitting into multiple commits

Interactive Flow (--pick):
  1. Show git status
  2. Interactive file selection with checkboxes
  3. Generate commit message for selected files
  4. Preview, edit, commit

Example:
  > /commit

  Analyzing changes...

  Agent recommends committing:
    ✓ src/auth/login.rs      - Core login logic
    ✓ src/auth/jwt.rs        - JWT handling
    ○ tests/auth_test.rs     - (suggests separate commit)

  Proposed commit message:
  ┌────────────────────────────────────────────────┐
  │ Add JWT token validation to login flow         │
  │                                                │
  │ Users can now stay logged in across sessions   │
  │ with secure token refresh. This improves UX    │
  │ by eliminating repeated login prompts.         │
  └────────────────────────────────────────────────┘

  [Enter] Commit  [e] Edit  [a] Add more files  [c] Cancel
```

### `/spec` - Specification Planning

```
/spec <name>

Creates: specs/<name>.md

Flow:
  1. Create spec file with template
  2. Enter planning mode
  3. Back-and-forth discussion to refine
  4. Agent suggests structure, user guides direction
  5. Exit planning mode when satisfied

Example:
  > /spec authentication

  Created specs/authentication.md
  Entering planning mode...

  Let's design the authentication system.
  What's the primary auth method you want to support?

  1. Email/password
  2. OAuth (Google, GitHub, etc.)
  3. Magic links
  4. Multiple options
```

### `/document` - Obsidian Integration

```
/document <topic> [--new] [--search]

Options:
  --new       Force create new note (don't search existing)
  --search    Just search, don't create/edit

Flow:
  1. Search vault for related notes
  2. If matches found:
     - Show matches
     - User selects to update or create new
  3. If updating: show current content, propose changes
  4. If new: suggest location, create with structure
  5. Write to vault

Example:
  > /document rust error handling

  Found related notes:
    1. Programming/Rust/Basics.md (mentions error handling)
    2. Programming/Rust/Result-Type.md

  [1-2] Update existing  [n] New note  [c] Cancel
```

### `/cost` - Token Usage & Cost

```
/cost

Shows detailed breakdown of session costs:

  Session Cost Breakdown
  ──────────────────────────────────────────────

  Model: claude-3-opus

  Input tokens:   45,230   ($0.675)
  Output tokens:  12,450   ($0.935)
  ──────────────────────────────────────────────
  Total:          57,680   ($1.61)

  Context used:   57,680 / 200,000 (29%)

  This session: 45 messages over 2h 15m
```

---

## Implementation Phases

Each phase has clear deliverables, testing requirements, and stopping conditions. Phases build on each other—complete each fully before moving on.

### Testing Strategy Overview

```
tests/
├── unit/                    # Pure function tests (no I/O)
│   ├── config_test.rs
│   ├── token_counter_test.rs
│   ├── command_parser_test.rs
│   └── ...
├── integration/             # Tests with real I/O (files, git)
│   ├── git_integration_test.rs
│   ├── specstory_test.rs
│   └── ...
├── ui/                      # Snapshot tests for UI output
│   ├── snapshots/
│   ├── spinner_test.rs
│   ├── context_bar_test.rs
│   └── ...
├── e2e/                     # End-to-end workflow tests
│   ├── repl_session_test.rs
│   ├── commit_workflow_test.rs
│   └── ...
└── mocks/                   # Shared test doubles
    ├── mock_api.rs
    ├── mock_terminal.rs
    └── mock_git.rs
```

**Test separation from agent tools:** CLI tests live in `tests/` and test the CLI itself. Agent tool tests (if any) would be in a separate `agent/tests/` directory and test the AI agent's tool implementations. The CLI tests should never depend on actual AI responses—mock the agent interface.

---

### Phase 1: Foundation & Input System

**Goal:** Runnable binary that accepts multi-line input with double-enter submission.

**Deliverables:**
- [x] Cargo project with workspace structure
- [x] Dependencies: `crossterm`, `tokio`, `clap`
- [x] Terminal raw mode handling (enter/exit cleanly)
- [x] Multi-line input buffer
- [x] Double-enter detection for submission
- [x] `Ctrl+C` cancellation, `Ctrl+D` exit
- [x] Basic input echo (what you type appears on screen)

**Files to create:**
```
src/
├── main.rs
├── cli/
│   ├── mod.rs
│   ├── input.rs          # Input handling, key events
│   └── terminal.rs       # Raw mode, cleanup
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_double_enter_detection` | Unit | Two consecutive enters triggers submit |
| `test_single_enter_adds_newline` | Unit | Single enter adds `\n` to buffer |
| `test_ctrl_c_clears_input` | Unit | Ctrl+C empties current input buffer |
| `test_backspace_removes_char` | Unit | Backspace works across lines |
| `test_terminal_cleanup_on_panic` | Integration | Terminal restored after panic |
| `test_unicode_input` | Unit | Handles emoji, CJK characters |

**Edge cases to handle:**
- Terminal resize during input
- Very long lines (horizontal scroll or wrap?)
- Paste with embedded newlines
- Non-UTF8 input (reject gracefully)

**Stopping condition:**
```
✓ cargo run starts successfully
✓ Can type multi-line input
✓ Double-enter submits and prints "You entered: {input}"
✓ Ctrl+C clears current input
✓ Ctrl+D exits cleanly
✓ All unit tests pass
✓ Terminal is always restored (even on crash)
```

---

### Phase 2: REPL Loop & Command System

**Goal:** Working REPL with slash command infrastructure and basic commands.

**Deliverables:**
- [x] REPL loop (read → parse → execute → display → repeat)
- [x] Slash command parser (`/command arg1 arg2`)
- [x] Command registry pattern (easy to add new commands)
- [x] `/help` - lists all commands
- [x] `/clear` - clears screen (context reset comes later)
- [x] `/exit` - clean exit
- [x] Config system with `serde` + `toml`
- [x] Config file loading from `~/.config/coding-agent/config.toml`
- [x] Default config generation on first run

**Files to create:**
```
src/
├── cli/
│   ├── repl.rs           # Main loop
│   └── commands/
│       ├── mod.rs        # Command trait, registry
│       ├── help.rs
│       ├── clear.rs
│       └── exit.rs
├── config/
│   ├── mod.rs
│   └── settings.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_slash_command_parsing` | Unit | `/cmd arg1 arg2` parses correctly |
| `test_command_registry_lookup` | Unit | Commands found by name |
| `test_unknown_command_error` | Unit | Unknown `/foo` gives helpful error |
| `test_config_default_generation` | Unit | Missing config creates valid default |
| `test_config_load_valid` | Unit | Valid TOML loads correctly |
| `test_config_load_invalid` | Unit | Invalid TOML gives clear error |
| `test_config_merge_partial` | Unit | Partial config merges with defaults |
| `test_help_lists_all_commands` | Integration | /help output contains all registered commands |

**Edge cases to handle:**
- Config file has unknown keys (ignore with warning, don't fail)
- Config directory doesn't exist (create it)
- Config file has wrong permissions (warn, use defaults)
- Slash command with no arguments vs with arguments

**Stopping condition:**
```
✓ REPL loop runs continuously until /exit or Ctrl+D
✓ /help shows all available commands with descriptions
✓ /clear clears the terminal
✓ Unknown commands show "Unknown command. Try /help"
✓ Config file created on first run at ~/.config/coding-agent/config.toml
✓ Config changes take effect (e.g., change a value, restart, verify)
✓ All tests pass
```

---

### Phase 3: UI Components & Theming

**Goal:** Beautiful, styled output with spinners and colors.

**Deliverables:**
- [x] Theme system (colors defined in one place)
- [x] Colored output for different elements (user input, agent, tools, errors)
- [x] Spinner component with customizable messages
- [x] Progress bar component
- [x] Styled message boxes (for commit messages, etc.)
- [x] Syntax highlighting for code blocks (use `syntect`)
- [x] `/config` command to open config in `$EDITOR`

**Files to create:**
```
src/
├── ui/
│   ├── mod.rs
│   ├── theme.rs          # Color definitions
│   ├── spinner.rs        # Animated spinner
│   ├── progress.rs       # Progress bar
│   ├── output.rs         # Styled printing functions
│   └── components.rs     # Message boxes, etc.
├── cli/commands/
│   └── config.rs
```

**Dependencies to add:** `indicatif`, `console`, `syntect`

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_theme_all_colors_defined` | Unit | No missing color definitions |
| `test_spinner_messages_cycle` | Unit | Spinner cycles through messages |
| `test_spinner_stops_cleanly` | Unit | No artifacts left after spinner stops |
| `test_progress_bar_0_to_100` | Unit | Progress bar renders at boundaries |
| `test_syntax_highlight_rust` | Unit | Rust code gets highlighted |
| `test_syntax_highlight_unknown` | Unit | Unknown language falls back gracefully |
| `test_output_no_color_mode` | Unit | Respects `NO_COLOR` env var |
| Snapshot: `spinner_states.snap` | UI | Spinner looks correct at each frame |
| Snapshot: `progress_bar.snap` | UI | Progress bar renders correctly |
| Snapshot: `code_block.snap` | UI | Syntax highlighting looks right |

**Edge cases to handle:**
- Terminal doesn't support colors (detect and fallback)
- `NO_COLOR` environment variable (respect it)
- Very narrow terminal (truncate gracefully)
- Spinner running when program exits (cleanup)

**Stopping condition:**
```
✓ Output is visually styled with colors
✓ Spinner animates smoothly during "thinking"
✓ Progress bar shows 0% to 100% correctly
✓ Code blocks have syntax highlighting
✓ /config opens config file in $EDITOR
✓ Works in terminals without color support
✓ All tests pass, snapshots approved
```

---

### Phase 4: Session Persistence & Startup

**Goal:** Conversations saved automatically, can resume on restart.

**Deliverables:**
- [x] SpecStory format writer (markdown files)
- [x] Session save after each exchange
- [x] Session loader (parse markdown back to conversation)
- [x] Startup screen with ASCII logo
- [x] `[n]` New session option
- [x] `[r]` Resume last session option
- [x] Last session preview (title, time ago)
- [x] `/history` command to browse past sessions
- [x] Update `/clear` to save before clearing context

**Files to create:**
```
src/
├── cli/
│   ├── startup.rs        # Welcome screen
│   └── commands/
│       └── history.rs
├── integrations/
│   ├── mod.rs
│   └── specstory.rs      # Save/load sessions
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_session_save_format` | Unit | Output matches SpecStory markdown format |
| `test_session_load_roundtrip` | Unit | Save then load gives identical conversation |
| `test_session_load_corrupted` | Unit | Corrupted file gives clear error, doesn't crash |
| `test_session_filename_format` | Unit | Files named with timestamp correctly |
| `test_startup_no_previous_session` | Integration | Shows only "New session" if no history |
| `test_startup_with_previous` | Integration | Shows resume option with preview |
| `test_history_lists_sessions` | Integration | /history shows all saved sessions |
| `test_clear_saves_first` | Integration | /clear saves before clearing |

**Edge cases to handle:**
- No previous sessions (don't show resume option)
- Corrupted session file (skip it, don't crash)
- Very long conversation (pagination in /history)
- Session from different version (version marker in files)
- Disk full when saving (warn but don't crash)

**Stopping condition:**
```
✓ On startup, see ASCII logo and session options
✓ Selecting "New" starts fresh conversation
✓ Selecting "Resume" loads previous conversation with full context
✓ Every exchange auto-saves to .specstory/history/
✓ /history shows list of past sessions
✓ /clear saves current session before clearing
✓ Can recover from corrupted session files
✓ All tests pass
```

---

### Phase 5: Context Tracking & Cost Display

**Goal:** Know exactly how much context you've used and what it costs.

**Deliverables:**
- [x] Token counting with `tiktoken-rs`
- [x] Context bar component (bottom of screen)
- [x] Color-coded usage (green/yellow/red)
- [x] Real-time updates after each message
- [x] `/cost` command with detailed breakdown
- [x] `/context` command showing loaded files, working dir
- [x] Cost calculation based on model pricing

**Files to create:**
```
src/
├── tokens/
│   ├── mod.rs
│   ├── counter.rs        # Token counting
│   └── pricing.rs        # Cost calculation
├── ui/
│   └── context_bar.rs
├── cli/commands/
│   ├── cost.rs
│   └── context.rs
```

**Dependencies to add:** `tiktoken-rs`

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_token_count_empty` | Unit | Empty string = 0 tokens |
| `test_token_count_simple` | Unit | "Hello world" = expected tokens |
| `test_token_count_code` | Unit | Code snippet = expected tokens |
| `test_token_count_unicode` | Unit | Unicode/emoji counted correctly |
| `test_context_bar_color_green` | Unit | 0-60% = green |
| `test_context_bar_color_yellow` | Unit | 60-85% = yellow |
| `test_context_bar_color_red` | Unit | 85%+ = red |
| `test_cost_calculation_opus` | Unit | Claude opus pricing correct |
| `test_cost_calculation_sonnet` | Unit | Claude sonnet pricing correct |
| `test_context_bar_at_100_percent` | Unit | Full bar renders correctly |
| Snapshot: `context_bar_states.snap` | UI | Bar looks right at 25%, 50%, 75%, 100% |

**Edge cases to handle:**
- Unknown model (default to highest pricing, warn)
- Context overflow (what happens at 100%? warn user)
- Token count changes between models (show warning when switching)
- Very fast typing (debounce updates)

**Stopping condition:**
```
✓ Context bar visible at bottom of screen
✓ Bar updates after each message
✓ Colors change at 60% and 85% thresholds
✓ /cost shows accurate token counts and costs
✓ /context shows current working directory and loaded files
✓ Switching models updates context window size
✓ All tests pass, snapshots approved
```

---

### Phase 6: Permissions System

**Goal:** Safe by default—ask before writing to untrusted locations.

**Deliverables:**
- [x] Trusted paths configuration
- [x] Path matching logic (supports `~`, globs)
- [x] Permission check before file write/modify
- [x] Permission prompt UI (Y/n/always/never)
- [x] "Always" adds to trusted paths in config
- [x] Read operations always allowed
- [x] Permission decision caching (per-session)

**Files to create:**
```
src/
├── permissions/
│   ├── mod.rs
│   ├── trusted.rs        # Path matching
│   └── prompt.rs         # Permission UI
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_trusted_path_exact_match` | Unit | Exact path matches |
| `test_trusted_path_subdirectory` | Unit | Child of trusted path is trusted |
| `test_trusted_path_tilde_expansion` | Unit | `~/foo` expands correctly |
| `test_trusted_path_glob` | Unit | `~/projects/*` matches children |
| `test_untrusted_path_prompts` | Integration | Writing to untrusted triggers prompt |
| `test_trusted_path_no_prompt` | Integration | Writing to trusted doesn't prompt |
| `test_always_adds_to_config` | Integration | "Always" choice persists to config |
| `test_read_always_allowed` | Unit | Read operations never prompt |

**Edge cases to handle:**
- Symlinks (resolve to real path before checking)
- Relative paths (resolve to absolute)
- Path traversal attempts (`../../../etc/passwd`)
- Permission denied by OS (handle gracefully)
- Config file itself (always writable)

**Stopping condition:**
```
✓ Writing to trusted paths works without prompts
✓ Writing to untrusted paths shows permission prompt
✓ "Always" option adds path to trusted_paths in config
✓ "Never" option blocks and remembers for session
✓ Symlinks resolved correctly
✓ All tests pass
```

---

### Phase 7: Tool Execution & Self-Healing

**Goal:** Tools that fix their own failures and write tests to prevent recurrence.

**Deliverables:**
- [x] Tool executor framework
- [x] Error categorization (code, permission, network, resource)
- [x] Fix-agent spawning for code errors
- [x] Diagnostic analysis (parse compiler errors)
- [x] Auto-fix application
- [x] Regression test generation
- [x] Retry logic with backoff for network errors
- [x] Tool execution spinner with status

**Files to create:**
```
src/
├── tools/
│   ├── mod.rs
│   ├── executor.rs       # Run tools, handle errors
│   ├── recovery.rs       # Error recovery strategies
│   └── diagnostics.rs    # Parse error messages
├── agents/
│   ├── mod.rs
│   └── fix_agent.rs      # Self-healing agent
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_error_categorization_missing_dep` | Unit | "cannot find crate" = code error |
| `test_error_categorization_permission` | Unit | "permission denied" = permission error |
| `test_error_categorization_network` | Unit | "connection refused" = network error |
| `test_fix_missing_dependency` | Integration | Missing dep auto-added to Cargo.toml |
| `test_fix_missing_import` | Integration | Missing import auto-added |
| `test_generated_test_compiles` | Integration | Auto-generated test is valid Rust |
| `test_retry_backoff_timing` | Unit | Exponential backoff works correctly |
| `test_max_retries_exceeded` | Unit | Gives up after max attempts |
| `test_fix_agent_no_infinite_loop` | Integration | Doesn't loop forever on unfixable error |

**Edge cases to handle:**
- Unfixable error (give up after N attempts, report clearly)
- Fix introduces new error (detect loops, abort)
- Multiple errors at once (fix in order of dependency)
- Generated test fails (don't commit broken test)
- No write permission for fix (fall back to suggestion)

**Stopping condition:**
```
✓ Missing dependency auto-fixed with test generated
✓ Missing import auto-added
✓ Network errors retry with backoff
✓ Unfixable errors reported clearly after max attempts
✓ Fix loops detected and aborted
✓ All tests pass
```

---

### Phase 8: Tool Integration

**Goal:** Connect Claude to actual coding tools so it can read, write, and execute code.

**Deliverables:**
- [x] Wire tools from `coding-agent-core` into CLI's API calls
- [x] `read_file` tool - Read file contents
- [x] `write_file` tool - Create/overwrite files
- [x] `edit_file` tool - Make targeted edits to existing files
- [x] `list_files` tool - List directory contents
- [x] `bash` tool - Execute shell commands
- [x] `code_search` tool - Search codebase with ripgrep patterns
- [x] Tool result display in REPL (formatted, syntax highlighted)
- [x] Tool call visibility (show "Reading src/main.rs..." etc.)
- [x] Permission checks before write operations (use Phase 6 system)

**Files to modify:**
```
src/
├── cli/
│   └── repl.rs            # Add tools to API calls, display tool results
├── tools/
│   ├── mod.rs             # Re-export tool definitions
│   └── definitions.rs     # Tool schemas for Claude API
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_read_file_tool` | Integration | Claude can read file and discuss contents |
| `test_write_file_tool` | Integration | Claude can create new file |
| `test_edit_file_tool` | Integration | Claude can modify existing file |
| `test_bash_tool` | Integration | Claude can run shell commands |
| `test_list_files_tool` | Integration | Claude can explore directory structure |
| `test_code_search_tool` | Integration | Claude can search for patterns |
| `test_tool_permission_denied` | Integration | Write to untrusted path prompts user |
| `test_tool_result_display` | Unit | Tool results formatted correctly |
| `test_multi_tool_conversation` | Integration | Claude uses multiple tools in sequence |

**Edge cases to handle:**
- Large file (truncate with message, or refuse)
- Binary file (detect and refuse to read as text)
- File doesn't exist (clear error message)
- Permission denied (use permission system)
- Tool timeout (bash commands that hang)
- Dangerous commands (rm -rf, etc.) - warn or block

**Stopping condition:**
```
✓ "Read src/main.rs and summarize it" → Claude reads and discusses file
✓ "Create a hello.txt file with 'Hello World'" → File created
✓ "Run cargo test" → Tests execute, output shown
✓ "Find all TODO comments in the codebase" → Search results displayed
✓ Tool calls shown with status (● Reading... ✓ Read 150 lines)
✓ Write operations respect permission system
✓ All tests pass
```

---

### Phase 9: Git Integration

**Goal:** Smart commits with purpose-focused messages.

**Deliverables:**
- [x] Git status reading with `git2`
- [x] `/commit` command (agent decides what to commit)
- [x] `/commit --pick` (interactive file picker)
- [x] Smart file grouping (logically related changes)
- [x] 3-sentence purpose-focused commit message generation
- [x] Commit message preview and edit
- [x] `/diff` command
- [x] `/undo` command (revert last commit or file change)

**Files to create:**
```
src/
├── integrations/
│   └── git.rs
├── cli/commands/
│   ├── commit.rs
│   ├── diff.rs
│   └── undo.rs
```

**Dependencies to add:** `git2`

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_git_status_clean` | Integration | Clean repo returns empty changes |
| `test_git_status_modified` | Integration | Modified files detected |
| `test_git_status_untracked` | Integration | Untracked files detected |
| `test_file_grouping_same_dir` | Unit | Files in same dir grouped |
| `test_file_grouping_related` | Unit | test + impl grouped together |
| `test_commit_message_format` | Unit | Message has title + 2 body sentences |
| `test_commit_creates_commit` | Integration | Git log shows new commit |
| `test_undo_reverts_commit` | Integration | Undo removes last commit |
| `test_diff_shows_changes` | Integration | /diff shows staged + unstaged |
| `test_pick_mode_ui` | Integration | Checkboxes work correctly |

**Edge cases to handle:**
- Not in a git repo (helpful error message)
- Merge conflict in progress (detect and warn)
- Detached HEAD (warn before committing)
- No changes to commit (don't create empty commit)
- Commit hook fails (report error, don't retry)
- Binary files (warn, skip from message analysis)

**Stopping condition:**
```
✓ /commit analyzes changes and commits with good message
✓ /commit --pick shows interactive file picker
✓ Commit message follows 3-sentence purpose format
✓ /diff shows current changes
✓ /undo reverts last change
✓ Handles edge cases (no repo, no changes, conflicts)
✓ All tests pass
```

---

### Phase 10: Workflow Commands

**Goal:** Full workflow support for specs, docs, and model switching.

**Deliverables:**
- [x] `/spec <name>` - create spec file, enter planning mode
- [x] Planning mode (different prompt behavior)
- [x] Spec file template generation
- [x] `/document <topic>` - Obsidian integration
- [x] Vault search functionality
- [x] Note creation and update
- [x] `/model [name]` - switch AI model
- [x] Model availability validation

**Files to create:**
```
src/
├── integrations/
│   └── obsidian.rs
├── cli/commands/
│   ├── spec.rs
│   ├── document.rs
│   └── model.rs
├── cli/
│   └── modes.rs          # Normal vs planning mode
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_spec_creates_file` | Integration | /spec foo creates specs/foo.md |
| `test_spec_template_valid` | Unit | Generated template has required sections |
| `test_spec_existing_file` | Integration | Opening existing spec loads content |
| `test_document_search` | Integration | Finds notes by content |
| `test_document_create` | Integration | Creates note in correct location |
| `test_document_update` | Integration | Updates existing note |
| `test_model_switch_valid` | Unit | Switching to valid model works |
| `test_model_switch_invalid` | Unit | Invalid model shows error with suggestions |
| `test_planning_mode_indicator` | UI | Planning mode shows visual indicator |

**Edge cases to handle:**
- Spec file already exists (load it, don't overwrite)
- Obsidian vault not configured (helpful setup message)
- Vault doesn't exist (create? or error?)
- Multiple matching notes (show picker)
- Model not available (suggest alternatives)
- Planning mode with no spec file (error)

**Stopping condition:**
```
✓ /spec creates file and enters planning mode
✓ Planning mode shows visual indicator
✓ /document searches and finds related notes
✓ /document creates new notes with proper structure
✓ /model switches model and updates context window
✓ All tests pass
```

---

### Phase 11: Fun Facts & Polish

**Goal:** Delightful experience with entertainment during waits.

**Deliverables:**
- [x] Fun facts API integration (`reqwest`)
- [x] Fact caching (local storage)
- [x] Fallback to curated list
- [x] Thinking messages rotation
- [x] Long-wait detection (>10s)
- [x] Fun fact display during long waits
- [x] Configurable enable/disable
- [x] `/status` command (active tasks, running agents)

**Files to create:**
```
src/
├── ui/
│   ├── thinking.rs
│   └── fun_facts.rs
├── cli/commands/
│   └── status.rs
```

**Dependencies to add:** `reqwest`

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_api_fetch_success` | Integration | API returns valid fact (mocked) |
| `test_api_fetch_timeout` | Integration | Timeout falls back to cache |
| `test_api_fetch_error` | Integration | Error falls back to cache |
| `test_cache_save_load` | Unit | Cache persists and loads |
| `test_cache_expiry` | Unit | Old cache items refreshed |
| `test_thinking_messages_rotate` | Unit | Messages cycle, don't repeat consecutively |
| `test_long_wait_threshold` | Unit | Fun fact shown only after 10s |
| `test_fun_facts_disabled` | Unit | No facts when config disabled |
| `test_status_shows_tasks` | Integration | /status lists active operations |

**Edge cases to handle:**
- No internet (use cached facts)
- Cache empty and no internet (use hardcoded fallback)
- API rate limited (respect limits, use cache)
- Very fast operation (don't flash fact then hide it)
- User disabled fun facts (respect config)

**Stopping condition:**
```
✓ Thinking messages rotate during waits
✓ Fun facts appear after 10+ seconds
✓ Works offline with cached facts
✓ Facts disabled via config works
✓ /status shows current operations
✓ All tests pass
```

---

### Phase 12: Multi-Agent Status Bar

**Goal:** Visual orchestration of parallel agent tasks.

**Deliverables:**
- [x] Agent manager (spawn, track, cancel agents)
- [x] Agent state machine (queued → running → complete/failed)
- [x] Multi-agent status bar UI
- [x] Progress tracking per agent
- [x] Agent result aggregation
- [x] Cancel individual agents
- [x] Parallel execution with `tokio`

**Files to create:**
```
src/
├── agents/
│   ├── manager.rs
│   └── status.rs
├── ui/
│   └── status_bar.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_agent_lifecycle` | Unit | queued → running → complete |
| `test_agent_cancellation` | Unit | Cancel stops agent, updates status |
| `test_parallel_execution` | Integration | Multiple agents run concurrently |
| `test_status_bar_single_agent` | UI | One agent shows correctly |
| `test_status_bar_multiple_agents` | UI | Multiple agents stacked correctly |
| `test_status_bar_progress` | UI | Progress bar updates |
| `test_agent_failure_handling` | Unit | Failed agent shows error state |
| `test_result_aggregation` | Unit | Results from multiple agents combined |
| Snapshot: `status_bar_states.snap` | UI | Various states render correctly |

**Edge cases to handle:**
- Agent crashes (mark failed, don't crash CLI)
- All agents cancelled (clean state)
- Too many agents to display (pagination/scrolling)
- Agent stuck (timeout detection)
- Memory pressure (limit concurrent agents)

**Stopping condition:**
```
✓ Status bar shows all running agents
✓ Progress updates in real-time
✓ Agents run in parallel
✓ Can cancel individual agents
✓ Failed agents show error state
✓ Status bar handles many agents gracefully
✓ All tests pass, snapshots approved
```

---

### Phase 13: Comprehensive End-to-End Testing

**Goal:** Full integration testing that validates the entire CLI works correctly—graphics, LLM interactions, tool calls, commands, and user workflows.

**Testing Strategy Overview:**

This phase establishes a comprehensive test suite using multiple testing approaches:
1. **Interactive PTY Testing** - Simulate real terminal sessions
2. **Snapshot Testing** - Capture and compare terminal output
3. **Mock LLM Server** - Test API interactions without costs
4. **Workflow Integration** - End-to-end user journey tests

**Deliverables:**
- [x] PTY test harness using `expectrl` or `rexpect`
- [x] Mock Claude API server for deterministic testing
- [x] Terminal output snapshot tests with `insta` + `term-transcript`
- [x] Full workflow integration tests
- [x] Visual regression tests for UI components
- [x] Performance benchmarks for startup and response times
- [x] CI/CD integration with test matrix

**Dependencies to add:**
```toml
[dev-dependencies]
expectrl = "0.7"           # PTY-based interactive testing
insta = "1.34"             # Snapshot testing
term-transcript = "0.3"    # Terminal output capture with ANSI
assert_cmd = "2.0"         # CLI subprocess testing
predicates = "3.0"         # Output assertions
wiremock = "0.6"           # Mock HTTP server for API
tokio-test = "0.4"         # Async test utilities
```

**Files to create:**
```
tests/
├── e2e/
│   ├── mod.rs
│   ├── harness.rs           # Test harness setup (mock server, PTY)
│   ├── mock_claude.rs       # Mock Claude API responses
│   ├── pty_helpers.rs       # PTY interaction utilities
│   │
│   ├── startup_test.rs      # Startup screen tests
│   ├── input_test.rs        # Multi-line input, double-enter
│   ├── commands_test.rs     # All slash commands
│   ├── conversation_test.rs # Multi-turn LLM interactions
│   ├── tools_test.rs        # Tool execution flows
│   ├── session_test.rs      # Save/resume workflows
│   ├── context_bar_test.rs  # Token tracking display
│   ├── error_recovery_test.rs # Self-healing flows
│   └── full_workflow_test.rs  # Complete user journeys
│
├── snapshots/               # insta snapshot files
│   ├── startup_screen.snap
│   ├── help_output.snap
│   ├── context_bar_states.snap
│   ├── tool_execution.snap
│   └── error_messages.snap
│
└── fixtures/
    ├── mock_responses/      # Canned Claude API responses
    │   ├── simple_response.json
    │   ├── tool_call_response.json
    │   ├── multi_turn_conversation.json
    │   └── streaming_chunks.json
    └── test_projects/       # Minimal test codebases
        └── rust_project/
            ├── Cargo.toml
            └── src/main.rs
```

---

#### 13.1: PTY Test Harness

**Purpose:** Test interactive terminal behavior in a real pseudo-terminal.

**Implementation:**
```rust
// tests/e2e/harness.rs
use expectrl::{Session, Eof};
use std::time::Duration;

pub struct CliTestSession {
    session: Session,
}

impl CliTestSession {
    pub fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let session = expectrl::spawn("cargo run -p coding-agent-cli")?;
        Ok(Self { session })
    }

    pub fn expect_startup_screen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Wait for ASCII logo
        self.session.expect("CODE")?;
        self.session.expect("[n] New session")?;
        self.session.expect("[r] Resume last session")?;
        Ok(())
    }

    pub fn select_new_session(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.session.send_line("n")?;
        self.session.expect(">")?; // Wait for prompt
        Ok(())
    }

    pub fn send_message(&mut self, msg: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.session.send_line(msg)?;
        self.session.send_line("")?; // Double-enter to submit
        Ok(())
    }

    pub fn expect_response(&mut self, timeout: Duration) -> Result<String, Box<dyn std::error::Error>> {
        self.session.set_expect_timeout(Some(timeout));
        let output = self.session.expect(">")?; // Wait for next prompt
        Ok(String::from_utf8_lossy(output.as_bytes()).to_string())
    }

    pub fn run_command(&mut self, cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.session.send_line(cmd)?;
        self.session.send_line("")?;
        self.expect_response(Duration::from_secs(5))
    }
}
```

**Tests:**

| Test | What it verifies |
|------|------------------|
| `test_pty_startup_displays_logo` | ASCII art renders correctly in PTY |
| `test_pty_double_enter_submits` | Input only submits on double-enter |
| `test_pty_ctrl_c_cancels` | Ctrl+C clears current input |
| `test_pty_ctrl_d_exits` | Ctrl+D exits cleanly |
| `test_pty_terminal_resize` | UI reflows on resize |
| `test_pty_raw_mode_cleanup` | Terminal restored after exit/crash |
| `test_pty_unicode_rendering` | Emoji and CJK display correctly |
| `test_pty_long_output_scrolls` | Long responses scroll properly |

---

#### 13.2: Mock Claude API Server

**Purpose:** Test LLM interactions without hitting real API (cost-free, deterministic).

**Implementation:**
```rust
// tests/e2e/mock_claude.rs
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

pub struct MockClaudeServer {
    server: MockServer,
}

impl MockClaudeServer {
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }

    pub fn url(&self) -> String {
        self.server.uri()
    }

    pub async fn mock_simple_response(&self, content: &str) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_test123",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": content}],
                "model": "claude-3-opus-20240229",
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 10, "output_tokens": 20}
            })))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_tool_call(&self, tool_name: &str, tool_input: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_test456",
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": "tool_call_123",
                    "name": tool_name,
                    "input": tool_input
                }],
                "model": "claude-3-opus-20240229",
                "stop_reason": "tool_use",
                "usage": {"input_tokens": 15, "output_tokens": 30}
            })))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_streaming_response(&self, chunks: Vec<&str>) {
        // SSE streaming format
        let body = chunks.iter()
            .map(|chunk| format!("data: {{\"type\":\"content_block_delta\",\"delta\":{{\"text\":\"{}\"}}}}\n\n", chunk))
            .collect::<String>();

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("accept", "text/event-stream"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_rate_limit(&self) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429)
                .set_body_json(serde_json::json!({
                    "type": "error",
                    "error": {"type": "rate_limit_error", "message": "Rate limited"}
                })))
            .mount(&self.server)
            .await;
    }

    pub async fn mock_network_error(&self) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&self.server)
            .await;
    }
}
```

**Tests:**

| Test | What it verifies |
|------|------------------|
| `test_api_simple_conversation` | Basic request/response flow |
| `test_api_multi_turn_context` | Conversation history sent correctly |
| `test_api_tool_call_execution` | Tool calls parsed and executed |
| `test_api_tool_result_sent` | Tool results returned to Claude |
| `test_api_streaming_display` | Streaming responses render in real-time |
| `test_api_rate_limit_retry` | 429 triggers exponential backoff |
| `test_api_network_error_recovery` | Network errors handled gracefully |
| `test_api_token_counting_accurate` | Usage tracked matches API response |

---

#### 13.3: Terminal Output Snapshots

**Purpose:** Capture terminal output including ANSI colors for visual regression testing.

**Implementation:**
```rust
// tests/e2e/snapshots.rs
use term_transcript::{Transcript, UserInput, ShellOptions};
use insta::assert_snapshot;

#[test]
fn test_startup_screen_snapshot() {
    let transcript = Transcript::from_inputs(
        ShellOptions::default().with_cargo_path(),
        vec![UserInput::command("cargo run -p coding-agent-cli")],
    ).unwrap();

    // Capture terminal output as text
    let output = transcript.to_string();
    insta::assert_snapshot!("startup_screen", output);
}

#[test]
fn test_help_command_snapshot() {
    let transcript = Transcript::from_inputs(
        ShellOptions::default(),
        vec![
            UserInput::command("cargo run -p coding-agent-cli"),
            UserInput::command("n"),      // New session
            UserInput::command("/help"),
            UserInput::command(""),       // Double-enter
        ],
    ).unwrap();

    insta::assert_snapshot!("help_output", transcript.to_string());
}

#[test]
fn test_context_bar_colors() {
    // Test at 25%, 60%, 85% thresholds
    // Verify green → yellow → red transitions
}

#[test]
fn test_error_message_formatting() {
    // Verify error messages are red, properly formatted
}
```

**Snapshot files generated:**
- `startup_screen.snap` - ASCII logo and session options
- `help_output.snap` - All available commands
- `context_bar_25.snap` - Green bar at low usage
- `context_bar_70.snap` - Yellow bar at medium usage
- `context_bar_90.snap` - Red bar at high usage
- `tool_execution.snap` - "● Reading..." / "✓ Read 150 lines"
- `error_permission.snap` - Permission denied error
- `error_unknown_command.snap` - Unknown command suggestion

---

#### 13.4: Command Tests

**Purpose:** Verify all slash commands work correctly.

**Tests:**

| Test | Command | What it verifies |
|------|---------|------------------|
| `test_cmd_help` | `/help` | Lists all commands with descriptions |
| `test_cmd_clear` | `/clear` | Clears screen, resets context, saves session |
| `test_cmd_exit` | `/exit` | Exits cleanly |
| `test_cmd_quit_alias` | `/quit`, `/q` | Aliases work |
| `test_cmd_config` | `/config` | Opens editor with config file |
| `test_cmd_history` | `/history` | Lists past sessions |
| `test_cmd_cost` | `/cost` | Shows token breakdown and costs |
| `test_cmd_context` | `/context` | Shows loaded files and token usage |
| `test_cmd_commit` | `/commit` | Analyzes changes, generates message |
| `test_cmd_commit_pick` | `/commit --pick` | Shows file picker |
| `test_cmd_diff` | `/diff` | Shows staged/unstaged changes |
| `test_cmd_undo` | `/undo` | Reverts last change |
| `test_cmd_spec` | `/spec auth` | Creates spec file, enters planning mode |
| `test_cmd_document` | `/document topic` | Searches/creates Obsidian notes |
| `test_cmd_model` | `/model sonnet` | Switches model |
| `test_cmd_status` | `/status` | Shows active tasks/agents |
| `test_cmd_unknown` | `/foobar` | Shows helpful error message |
| `test_cmd_bare_slash` | `/` | Shows error, not sent as message |

---

#### 13.5: Full Workflow Integration Tests

**Purpose:** Test complete user journeys from start to finish.

**Test scenarios:**

```rust
#[tokio::test]
async fn test_workflow_new_session_conversation() {
    // 1. Start CLI
    // 2. Select new session
    // 3. Send message, get response
    // 4. Send follow-up, verify context retained
    // 5. Check /cost shows token usage
    // 6. Exit with /exit
    // 7. Verify session saved to .specstory/
}

#[tokio::test]
async fn test_workflow_resume_session() {
    // 1. Create a session with conversation
    // 2. Exit
    // 3. Restart CLI
    // 4. Select resume
    // 5. Verify previous context loaded
    // 6. Continue conversation
}

#[tokio::test]
async fn test_workflow_tool_execution() {
    // 1. Ask Claude to read a file
    // 2. Verify tool call displayed ("● Reading...")
    // 3. Verify tool result displayed ("✓ Read 150 lines")
    // 4. Verify Claude summarizes file contents
}

#[tokio::test]
async fn test_workflow_self_healing() {
    // 1. Run a command that fails (missing dependency)
    // 2. Verify error categorized correctly
    // 3. Verify fix-agent spawned
    // 4. Verify fix applied
    // 5. Verify regression test generated
    // 6. Verify retry succeeds
}

#[tokio::test]
async fn test_workflow_git_commit() {
    // Setup: Create test repo with changes
    // 1. Run /commit
    // 2. Verify agent analyzes changes
    // 3. Verify commit message generated
    // 4. Verify preview shown
    // 5. Confirm commit
    // 6. Verify git log shows commit
}

#[tokio::test]
async fn test_workflow_permission_prompt() {
    // 1. Ask Claude to write to untrusted path
    // 2. Verify permission prompt appears
    // 3. Select "Always"
    // 4. Verify added to config
    // 5. Verify subsequent writes don't prompt
}

#[tokio::test]
async fn test_workflow_long_wait_fun_facts() {
    // 1. Mock slow API response (>10s)
    // 2. Verify thinking messages rotate
    // 3. Verify fun fact appears after threshold
    // 4. Verify response eventually displays
}
```

---

#### 13.6: Performance Tests

**Purpose:** Ensure CLI is responsive and doesn't regress.

**Benchmarks:**

| Metric | Target | Test |
|--------|--------|------|
| Startup time | <500ms | `bench_startup_to_prompt` |
| Input latency | <16ms | `bench_keypress_to_display` |
| Token counting | <10ms | `bench_token_count_10k` |
| Context bar update | <5ms | `bench_context_bar_render` |
| Session save | <100ms | `bench_session_save_large` |
| Session load | <200ms | `bench_session_load_large` |

**Implementation:**
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_startup(c: &mut Criterion) {
    c.bench_function("startup_to_prompt", |b| {
        b.iter(|| {
            let mut session = CliTestSession::spawn().unwrap();
            session.expect_startup_screen().unwrap();
        });
    });
}

fn bench_token_counting(c: &mut Criterion) {
    let large_text = "word ".repeat(2000); // ~10k tokens
    c.bench_function("token_count_10k", |b| {
        b.iter(|| {
            count_tokens(&large_text)
        });
    });
}

criterion_group!(benches, bench_startup, bench_token_counting);
criterion_main!(benches);
```

---

#### 13.7: CI/CD Integration

**GitHub Actions workflow:**

```yaml
# .github/workflows/e2e.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run E2E tests
        run: cargo test --test e2e -- --test-threads=1
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY_TEST }}
          # Or use mock server:
          ANTHROPIC_BASE_URL: http://localhost:8080

      - name: Run snapshot tests
        run: cargo insta test --review

      - name: Upload snapshots on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: failed-snapshots-${{ matrix.os }}
          path: tests/snapshots/*.snap.new

  performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable

      - name: Run benchmarks
        run: cargo bench -- --noplot

      - name: Check for regressions
        run: |
          # Compare against baseline
          cargo bench -- --baseline main --noplot
```

---

**Tests Summary:**

| Category | Test Count | Tools Used |
|----------|------------|------------|
| PTY Interactive | 8 | `expectrl` |
| Mock API | 8 | `wiremock` |
| Terminal Snapshots | 10+ | `insta`, `term-transcript` |
| Command Tests | 18 | `assert_cmd` |
| Workflow Integration | 7 | Combined |
| Performance | 6 | `criterion` |
| **Total** | **~57** | |

**Edge cases to handle:**
- CI environment has no TTY (use PTY emulation)
- Mock server port conflicts (use dynamic ports)
- Snapshot tests on different terminal widths (normalize output)
- Flaky tests from timing issues (add retries, increase timeouts)
- Platform-specific behavior (test matrix for macOS/Linux/Windows)

**Stopping condition:**
```
✓ All PTY tests pass on all platforms
✓ Mock server tests cover happy path and error cases
✓ Snapshots reviewed and approved
✓ All commands tested
✓ Full workflows pass end-to-end
✓ Performance within targets
✓ CI pipeline green on all platforms
✓ No flaky tests
```

---

### Phase 14: Integration of Advanced Features

**Goal:** Wire up all implemented-but-unused modules to eliminate compiler warnings and activate advanced features.

**Context:** Phases 1-13 implemented all the code for the CLI specification. However, many advanced features were built as complete modules but not yet integrated into the main execution flow. This phase systematically wires everything together.

---

#### 14.1: Core Infrastructure Integration

**Deliverables:**

1. **AgentManager Integration**
   - [x] Create `AgentManager` instance in `Repl::new()`
   - [x] Store as `Arc<AgentManager>` field in `Repl` struct
   - [x] Pass to `CommandContext` so all commands can access it
   - [x] Update `/status` command to query and display agent status

2. **Permission System Activation**
   - [x] Wire `PermissionPrompt` into `PermissionChecker`
   - [x] When write operation hits untrusted path, show interactive prompt
   - [x] Handle "Always" responses by updating config file
   - [x] Handle "Never" responses by caching in session permissions

**Files to modify:**
```
src/cli/repl.rs              # Add agent_manager field
src/tools/definitions.rs     # Pass permission checker through
src/permissions/checker.rs   # Call PermissionPrompt::prompt()
```

**Tests:**
| Test | What it verifies |
|------|------------------|
| `test_agent_manager_initialized` | AgentManager exists in REPL |
| `test_status_shows_active_agents` | /status displays running agents |
| `test_permission_prompt_untrusted` | Prompt shown for untrusted writes |
| `test_permission_always_saves` | "Always" updates config |

**Stopping condition:**
```
✓ AgentManager successfully initialized
✓ /status shows "No active agents" on fresh start
✓ Permission prompts appear for untrusted file writes
✓ Config updated when "Always" selected
```

---

#### 14.2: Tool Execution Enhancement

**Deliverables:**

1. **ToolExecutor Integration**
   - [x] Create `ToolExecutor` instance in `Repl::new()`
   - [x] Register all tools (read, write, edit, list, bash, search)
   - [x] Replace `execute_tool_with_permissions()` calls with `executor.execute()`
   - [x] Handle `ToolExecutionResult` with retry logic
   - [x] Display retry attempts to user

2. **ToolExecutionSpinner Integration**
   - [x] Replace simple "● Running..." with `ToolExecutionSpinner::with_target()`
   - [x] Show spinner during tool execution
   - [x] Call `finish_success()` or `finish_failed()` on completion
   - [x] Display elapsed time for long operations

**Implementation approach:**

```rust
// In process_conversation(), replace:
self.print_line(&format!("● {}", self.format_tool_call(&name, &input)));
let result = execute_tool_with_permissions(...);

// With:
let spinner = ToolExecutionSpinner::with_target(
    &name,
    extract_target(&name, &input),
    self.theme.clone()
);
let result = self.tool_executor.execute(id.clone(), &name, input);
if result.is_success() {
    spinner.finish_success();
} else {
    spinner.finish_failed(result.error().unwrap());
}
```

**Tests:**
| Test | What it verifies |
|------|------------------|
| `test_tool_executor_retry_transient` | Network errors retried 3 times |
| `test_tool_spinner_shows_target` | Spinner displays file path |
| `test_tool_spinner_timing` | Elapsed time shown on completion |

**Stopping condition:**
```
✓ ToolExecutor handles all tool execution
✓ Spinners animate during tool calls
✓ Retry attempts displayed to user
✓ Success/failure with timing shown
```

---

#### 14.3: Self-Healing Error Recovery

**Deliverables:**

1. **FixAgent Integration**
   - [x] Check `ToolExecutionResult::is_auto_fixable()` after errors
   - [x] Spawn `FixAgent::spawn()` for fixable errors
   - [x] Display fix agent progress via status bar
   - [x] Apply fix if successful, re-run original tool
   - [x] Generate regression test via `generate_regression_test()`

2. **Error Recovery Flow**
   - [x] Categorize errors via `ToolError::category`
   - [x] For `ErrorCategory::Code`, attempt auto-fix
   - [x] For `ErrorCategory::Network`, retry with backoff (already in ToolExecutor)
   - [x] For `ErrorCategory::Permission`, show permission prompt
   - [x] For `ErrorCategory::Resource`, suggest alternatives

**Implementation approach:**

```rust
// In process_conversation(), after tool execution:
match result.result {
    Ok(output) => { /* success path */ },
    Err(error) if error.is_auto_fixable() => {
        // Spawn fix agent
        if let Some(fix_agent) = FixAgent::spawn(result, FixAgentConfig::default()) {
            let fix_result = fix_agent.attempt_fix(
                |fix_info| apply_fix(fix_info, &config),
                |_| verify_fix()
            );

            if fix_result.success {
                // Re-run the original tool
                let retry_result = self.tool_executor.execute(...);
                // ... handle retry
            }
        }
    },
    Err(error) => { /* show error */ }
}
```

**Tests:**
| Test | What it verifies |
|------|------------------|
| `test_auto_fix_missing_dependency` | Adds missing crate to Cargo.toml |
| `test_auto_fix_missing_import` | Adds missing import statement |
| `test_regression_test_generated` | Test written after successful fix |
| `test_no_fix_infinite_loop` | Gives up after max attempts |

**Stopping condition:**
```
✓ Missing dependency errors trigger auto-fix
✓ Fix agent progress shown in /status
✓ Regression tests generated in tests/auto_fix/
✓ Original operation retried after fix
✓ Max retry limit prevents infinite loops
```

---

#### 14.4: UI Enhancements

**Deliverables:**

1. **StatusBar for Multi-Agent Display**
   - [x] Create `StatusBar` instance
   - [x] Query `agent_manager.get_all_statuses()` periodically
   - [x] Render status bar above input prompt when agents active
   - [x] Clear status bar when all agents complete

2. **LongWaitDetector Integration**
   - [x] Replace manual `elapsed.as_secs()` check with `LongWaitDetector`
   - [x] Call `detector.start()` before API call
   - [x] Call `detector.check()` periodically
   - [x] Trigger fun fact on first threshold crossing

3. **Enhanced Tool Result Formatting**
   - [x] Use `ToolResultFormatter::format_result()` consistently
   - [x] Add syntax highlighting for code in results
   - [x] Implement collapsible sections for long outputs
   - [x] Add line numbers to file reads

**Implementation approach:**

```rust
// Status bar rendering
if !agent_statuses.is_empty() {
    self.status_bar.render(&agent_statuses)?;
    // Move cursor below status bar
    execute!(stdout(), cursor::MoveDown(agent_statuses.len() as u16 + 2))?;
}

// Long wait detection
let mut detector = LongWaitDetector::new(Duration::from_secs(self.fun_fact_delay as u64));
detector.start();

// ... start API call ...

// Periodically check during wait
if detector.check() && !triggered {
    self.display_fun_fact();
    triggered = true;
}
```

**Tests:**
| Test | What it verifies |
|------|------------------|
| `test_status_bar_multiple_agents` | Displays all active agents |
| `test_status_bar_progress_updates` | Progress bars update correctly |
| `test_long_wait_detector_threshold` | Fun fact triggers after 10s |
| `test_tool_result_syntax_highlight` | Code highlighted in results |

**Stopping condition:**
```
✓ Status bar appears when agents spawn
✓ Progress bars update in real-time
✓ Fun facts trigger at exact threshold
✓ Tool results nicely formatted with syntax highlighting
```

---

#### 14.5: Advanced Git Features

**Deliverables:**

1. **File Grouping in /commit**
   - [x] Use `FileGrouper::group_files()` in commit command
   - [x] Suggest logical splits for unrelated changes
   - [x] Show grouping rationale to user
   - [x] Ask which group to commit

2. **Smart Commit Message Generation**
   - [x] Analyze all files in group via `suggest_commit_splits()`
   - [x] Generate purpose-focused message from changes
   - [x] Show commit message preview with edit option
   - [x] Support multi-commit workflow for logical separation

**Implementation approach:**

```rust
// In commit.rs
let repo = GitRepo::open(".")?;
let status = repo.status()?;
let files = status.files;

// Group related files
let groups = FileGrouper::group_files(&files);

if groups.len() > 1 {
    // Suggest splits
    println!("Found {} logical groups:", groups.len());
    for (i, group) in groups.iter().enumerate() {
        println!("  {}. {} ({} files)", i+1, group.reason.description(), group.files.len());
    }
    // Let user pick which to commit
}
```

**Tests:**
| Test | What it verifies |
|------|------------------|
| `test_file_grouping_same_dir` | Files grouped by directory |
| `test_file_grouping_test_impl` | Tests paired with implementations |
| `test_commit_split_suggestion` | Suggests separating unrelated changes |
| `test_commit_message_purpose` | Message focuses on "why" not "what" |

**Stopping condition:**
```
✓ /commit suggests logical file groups
✓ User can commit subsets of changes
✓ Commit messages explain purpose clearly
✓ Test/impl files grouped intelligently
```

---

#### 14.6: Obsidian Integration

**Deliverables:**

1. **/document Command Enhancement**
   - [x] Use `ObsidianVault::search()` to find related notes
   - [x] Show search results with relevance scores
   - [x] Support creating new notes in suggested locations
   - [x] Support updating existing notes with diffs

2. **Note Template System**
   - [x] Generate structured note templates by topic
   - [x] Include metadata (date, tags, backlinks)
   - [x] Support different note types (meeting, concept, reference)

**Tests:**
| Test | What it verifies |
|------|------------------|
| `test_document_search_finds_related` | Finds notes by content |
| `test_document_creates_in_location` | Suggests correct subdirectory |
| `test_document_shows_diff` | Preview changes before applying |

**Stopping condition:**
```
✓ /document finds existing notes
✓ New notes created in appropriate subdirs
✓ Updates show diff preview
✓ Metadata included in new notes
```

---

#### 14.7: Comprehensive Testing

**Deliverables:**

1. **Build Verification**
   ```bash
   cargo build --all-targets
   cargo test --all
   cargo clippy --all-targets -- -D warnings
   cargo fmt --all -- --check
   ```

2. **Integration Test Suite**
   - [x] Test agent spawning and status
   - [x] Test tool execution with retries
   - [ ] Test auto-fix workflow end-to-end
   - [ ] Test permission prompts and config updates
   - [ ] Test multi-agent coordination

3. **Manual Workflow Tests**
   - [ ] Spawn multiple agents, check /status
   - [ ] Trigger permission prompt, select "Always"
   - [ ] Cause tool error, verify auto-fix attempt
   - [ ] Let long operation run, see fun fact
   - [ ] Commit with file grouping

**Test scenarios:**

```rust
#[tokio::test]
async fn test_end_to_end_auto_fix() {
    // 1. Set up project missing a dependency
    // 2. Ask Claude to use that dependency
    // 3. Tool execution fails
    // 4. Verify FixAgent spawned
    // 5. Verify dependency added
    // 6. Verify regression test created
    // 7. Verify retry succeeded
}

#[tokio::test]
async fn test_multi_agent_status_display() {
    // 1. Spawn 3 test agents
    // 2. Run /status
    // 3. Verify all 3 shown with progress
    // 4. Cancel one agent
    // 5. Run /status again
    // 6. Verify only 2 active
}

#[tokio::test]
async fn test_permission_prompt_workflow() {
    // 1. Attempt write to /tmp/test.txt
    // 2. Verify prompt appears
    // 3. Simulate "Always" response
    // 4. Verify config updated
    // 5. Attempt another write to /tmp/
    // 6. Verify no prompt (cached)
}
```

**Stopping condition:**
```
✓ cargo build produces NO warnings
✓ All 210+ tests pass
✓ Clippy produces no warnings
✓ Code formatted consistently
✓ All integration tests pass
✓ Manual workflows verified
```

---

#### 14.8: Documentation & Cleanup

**Deliverables:**

1. **Code Documentation**
   - [ ] Add doc comments to all public APIs
   - [ ] Document integration points between modules
   - [ ] Add examples to complex functions
   - [ ] Update README with new features

2. **Architecture Documentation**
   - [ ] Update `docs/CLI_ARCHITECTURE.md` with final structure
   - [ ] Document agent lifecycle and coordination
   - [ ] Document error recovery flow
   - [ ] Document tool execution pipeline

3. **Remove Temporary Allows**
   - [ ] Remove any `#[allow(dead_code)]` annotations added temporarily
   - [ ] Verify all code is actually used
   - [ ] Clean up any debug prints or temporary code

**Stopping condition:**
```
✓ All public APIs documented
✓ Architecture docs updated
✓ No #[allow(dead_code)] annotations
✓ No TODO or FIXME comments
✓ README reflects actual features
```

---

## Phase 14 Definition of Done

A complete integration when:

```
□ All modules wired into main execution flow
□ Zero compiler warnings on `cargo build`
□ Zero clippy warnings on `cargo clippy`
□ All 210+ tests pass
□ New integration tests added and passing
□ Manual workflow tests verified
□ Code formatted with rustfmt
□ Documentation updated
□ All advanced features functional:
  □ Self-healing error recovery
  □ Multi-agent coordination
  □ Permission prompts with config updates
  □ Tool execution with retries and spinners
  □ Smart git commit grouping
  □ Obsidian vault integration
  □ Status bar for agent progress
  □ Long-wait fun facts
```

---

## Edge Case Strategy

For each feature, handle edge cases with this priority:

1. **Prevent data loss** - Never lose user's work (always save before destructive operations)
2. **Graceful degradation** - Feature unavailable? Fall back to simpler version
3. **Clear communication** - Tell user what went wrong and how to fix it
4. **No crashes** - Catch all errors, log them, keep CLI running
5. **Recoverable state** - After any error, CLI should be usable

**Universal edge cases (handle in Phase 1-2):**
- Terminal resize → reflow UI
- SIGINT/SIGTERM → save state, exit cleanly
- Panic → restore terminal, save state, show error
- Disk full → warn, don't crash
- Permission denied → explain which permission, suggest fix

---

## Definition of Done (per Phase)

A phase is complete when:

```
□ All deliverables implemented
□ All tests written and passing
□ Snapshot tests reviewed and approved
□ Edge cases documented and handled
□ No compiler warnings
□ Code formatted with rustfmt
□ Documentation comments on public APIs
□ Manual testing completed (checklist above)
□ Changes committed with purpose-focused message
```

---

## Stretch Goals / Future Ideas

- **Split pane view** - Code preview alongside conversation
- **Vim keybindings** - For power users
- **Custom themes** - User-definable color schemes
- **Plugin system** - Custom slash commands via Lua/Rust
- **Fuzzy file picker** - Quick file selection with fzf-style interface
- **Web UI mode** - Optional browser-based interface
- **Voice input** - Whisper integration for voice commands
- **Snippets** - Save and reuse common prompts
- **Custom fun facts** - User-contributed facts/jokes
- **Session branching** - Fork conversations to explore alternatives

---

## Resolved Questions

- [x] Obsidian vault path: `~/Documents/Personal/`
- [x] Platform: Cross-platform (macOS primary)
- [x] Multi-line submit: Double-enter
- [x] Multi-agent status bar: Core feature
- [x] Tool verbosity: Standard (action + target)
- [x] Session persistence: SpecStory auto-save
- [x] `/commit` default: Agent decides, `--pick` for interactive
- [x] Token display: Cumulative session with context bar
- [x] Fun facts: Fetch from API (with local cache fallback)
- [x] Context bar position: Bottom of CLI
- [x] Cost display: Separate (`/cost` command), not in context bar
- [x] Startup: Welcome message with new/resume session options
- [x] `/clear` behavior: Resets both display AND context

---

## Recent Updates

### 2026-02-13: Claude API Integration & Terminal Fixes

**Claude API Integration** (Critical)
- [x] CLI now makes real API calls to Claude (was previously just echoing input)
- [x] Added `ureq` and `dotenvy` dependencies for HTTP requests and env loading
- [x] Loads `ANTHROPIC_API_KEY` from environment or `.env` file
- [x] Multi-turn conversation memory (Claude remembers context)
- [x] Shows "Thinking..." indicator during API calls

**Terminal Raw Mode Fix** (Bug Fix)
- [x] Fixed broken layout where ASCII logo and text drifted right
- [x] Root cause: In raw mode, `\n` only moves cursor down, doesn't return to column 0
- [x] Solution: Changed all newlines to `\r\n` in startup screen, REPL, and input handler
- [x] Added `print_line()` and `print_newline()` helpers to REPL for consistent handling

**Command Aliases** (UX Improvement)
- [x] Added `/quit` as alias for `/exit` (common user expectation)
- [x] Added `/q` as short alias for `/exit`
- [x] Added error message for bare `/` input (was being treated as regular message)

**Test Count:** 210 tests passing (206 unit + 3 integration + 1 doc test)

---

*Last updated: 2026-02-13*

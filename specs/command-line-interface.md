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
- [ ] Cost calculation based on model pricing

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
- [ ] Trusted paths configuration
- [ ] Path matching logic (supports `~`, globs)
- [ ] Permission check before file write/modify
- [ ] Permission prompt UI (Y/n/always/never)
- [ ] "Always" adds to trusted paths in config
- [ ] Read operations always allowed
- [ ] Permission decision caching (per-session)

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
- [ ] Tool executor framework
- [ ] Error categorization (code, permission, network, resource)
- [ ] Fix-agent spawning for code errors
- [ ] Diagnostic analysis (parse compiler errors)
- [ ] Auto-fix application
- [ ] Regression test generation
- [ ] Retry logic with backoff for network errors
- [ ] Tool execution spinner with status

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

### Phase 8: Git Integration

**Goal:** Smart commits with purpose-focused messages.

**Deliverables:**
- [ ] Git status reading with `git2`
- [ ] `/commit` command (agent decides what to commit)
- [ ] `/commit --pick` (interactive file picker)
- [ ] Smart file grouping (logically related changes)
- [ ] 3-sentence purpose-focused commit message generation
- [ ] Commit message preview and edit
- [ ] `/diff` command
- [ ] `/undo` command (revert last commit or file change)

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

### Phase 9: Workflow Commands

**Goal:** Full workflow support for specs, docs, and model switching.

**Deliverables:**
- [ ] `/spec <name>` - create spec file, enter planning mode
- [ ] Planning mode (different prompt behavior)
- [ ] Spec file template generation
- [ ] `/document <topic>` - Obsidian integration
- [ ] Vault search functionality
- [ ] Note creation and update
- [ ] `/model [name]` - switch AI model
- [ ] Model availability validation

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

### Phase 10: Fun Facts & Polish

**Goal:** Delightful experience with entertainment during waits.

**Deliverables:**
- [ ] Fun facts API integration (`reqwest`)
- [ ] Fact caching (local storage)
- [ ] Fallback to curated list
- [ ] Thinking messages rotation
- [ ] Long-wait detection (>10s)
- [ ] Fun fact display during long waits
- [ ] Configurable enable/disable
- [ ] `/status` command (active tasks, running agents)

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

### Phase 11: Multi-Agent Status Bar

**Goal:** Visual orchestration of parallel agent tasks.

**Deliverables:**
- [ ] Agent manager (spawn, track, cancel agents)
- [ ] Agent state machine (queued → running → complete/failed)
- [ ] Multi-agent status bar UI
- [ ] Progress tracking per agent
- [ ] Agent result aggregation
- [ ] Cancel individual agents
- [ ] Parallel execution with `tokio`

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

*Last updated: 2026-02-13*

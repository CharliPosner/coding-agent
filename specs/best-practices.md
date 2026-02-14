# Best Practices for AI Coding Agents

> Architectural patterns and improvements synthesized from Yegge's Gas Town/Beads and Huntley's Loom/Ralph, tailored for the coding-agent CLI.

---

## Overview

This spec captures transferable lessons from two production coding agent architectures:

- **Yegge's Gas Town / Beads** - Issue-oriented orchestration, session management, Zero Framework Cognition
- **Huntley's Loom / Ralph** - Context window management, backpressure, autonomous loops

**What this is:**

- Practical improvements for our existing architecture
- Prioritized by value/complexity ratio
- Simpler approaches where they work

**What this is NOT:**

- A replication of Gas Town or Loom
- Multi-agent orchestration (we're single-agent for now)
- Over-engineered solutions

---

## Core Principles

### 1. Zero Framework Cognition (ZFC)

> "All decision-making and reasoning is delegated to AI models. The application code is a thin, safe, deterministic shell for pure orchestration." — Yegge

**What it means:**

- No heuristics, regex, or pattern matching for decisions in the harness
- Don't parse or interpret agent output with brittle rules
- When you need a judgment call (is this done? what's next? is this safe?), send it to the model
- Your CLI is a dumb pipe that routes information to/from the model

**Current state:** Our state machine is pure (good). Watch for creeping heuristics.

**Action:** Before adding any if/else decision logic, ask: "Should the model decide this?"

### 2. Context Window as Memory

> "A context window is effectively just mallocing an array. There is no persistent server memory." — Huntley

**The numbers:**

- 200K advertised → ~176K usable (system prompt + harness overhead)
- **40-60% utilization is the "smart zone"** — beyond 60-70%, quality degrades
- Less is more: every token you allocate displaces reasoning capacity

**Key rules:**

1. One task per context window — don't let sessions sprawl
2. Clear context between activities — new topic = new session
3. Deterministically allocate files — start from known state
4. Minimize tool definitions — each costs tokens

**Current state:** We have token tracking and context bar. Missing: smart zone warnings.

### 3. Backpressure

> "Software engineering is now about preventing failure scenarios through back pressure to the generative function." — Huntley

**What it means:**

- Automated feedback that rejects invalid code generation
- Tests, type checking, linting, build success/failure
- Pre-commit hooks: annoying for humans, perfect for agents
- "Just enough" to reject invalid generations while keeping iteration speed high

**Forms of backpressure:**

| Type | Tool | When |
| ---- | ---- | ---- |
| Unit tests | `cargo test` | After changes |
| Type checking | `cargo check` | Before commit |
| Linting | `cargo clippy` | Before commit |
| Build | `cargo build` | Before commit |
| Pre-commit hooks | `prek` / hooks | Automatic |

**Current state:** Self-healing catches errors post-hoc. Missing: pre-commit prevention.

### 4. State Lives on Disk, Not in Context

**Why:**

- Context windows are ephemeral and disposable
- Git history persists progress
- Files persist across sessions
- Enables session interruption and resumption

**What should persist:**

| Data | Location | Format |
| ---- | -------- | ------ |
| Conversation history | `.specstory/history/` | Markdown |
| Current progress | `PROGRESS.md` or `.coding-agent/` | Markdown/JSON |
| Task tracking | `.coding-agent/tasks.json` | JSON |
| Handoff prompts | `.coding-agent/last-handoff.md` | Markdown |

**Current state:** SpecStory handles conversation persistence. Missing: progress files, task tracking.

### 5. One Task Per Context

**The anti-pattern:** A sprawling session that started with "fix the login bug" and is now "also let's refactor auth and maybe add 2FA."

**The fix:**

- Session scope reminder at start
- When scope drifts, suggest `/clear` or new session
- File discovered work for later, don't chase it now

---

## Prioritized Improvements

### Phase 0: PostToolsHook Infrastructure

**Priority:** Foundational — enables Phases A, B, and D features

**Why this matters:** Currently, when tools complete, the state machine immediately transitions to `CallingLlm`. This leaves no hook point for quality gates, context checking, progress tracking, or session management. Loom's architecture includes a `PostToolsHook` state specifically for this purpose.

**Current flow:**
```
ExecutingTools → [all done] → CallingLlm
```

**New flow:**
```
ExecutingTools → [all done] → PostToolsHook → [hooks complete] → CallingLlm
```

#### 0.1 State Machine Changes

**Location:** `crates/coding-agent-core/src/state.rs`

Add new state:
```rust
/// Post-tool-execution hook point for quality gates, context checks, etc.
PostToolsHook {
    conversation: Vec<Message>,
    pending_tool_results: Vec<Message>,  // Tool results ready to send
}
```

Add new event:
```rust
/// Post-tool hooks have completed
HooksCompleted {
    proceed: bool,  // false = stop loop, wait for user input
    warning: Option<String>,  // Optional warning to display
}
```

Add new actions:
```rust
/// Run post-tool hooks (caller decides what to run)
RunPostToolsHooks {
    tool_names: Vec<String>,  // Which tools just executed
}

/// Display a warning to the user (non-blocking)
DisplayWarning(String),
```

**Location:** `crates/coding-agent-core/src/machine.rs`

Change the "all tools done" branch (currently lines 170-197):

```rust
if all_done {
    // Build tool result messages (existing code)
    let mut tool_result_messages = Vec::new();
    for exec in &execs {
        // ... build messages ...
    }

    // Collect tool names for hook decision
    let tool_names: Vec<String> = execs.iter()
        .filter_map(|e| match e {
            ToolExecutionStatus::Completed { .. } => None,
            ToolExecutionStatus::Pending { tool_name, .. } => Some(tool_name.clone()),
            ToolExecutionStatus::Running { tool_name, .. } => Some(tool_name.clone()),
        })
        .collect();

    // NEW: Transition to PostToolsHook instead of CallingLlm
    self.state = AgentState::PostToolsHook {
        conversation: conversation.clone(),
        pending_tool_results: tool_result_messages,
    };

    AgentAction::RunPostToolsHooks { tool_names }
}
```

Add new transition handler:
```rust
// === PostToolsHook ===
(
    AgentState::PostToolsHook {
        conversation,
        pending_tool_results,
    },
    AgentEvent::HooksCompleted { proceed, warning },
) => {
    // Display warning if present
    let display_action = warning.map(|w| AgentAction::DisplayWarning(w));

    if proceed {
        // Continue to LLM with tool results
        let mut conv = conversation.clone();
        conv.extend(pending_tool_results.clone());

        self.state = AgentState::CallingLlm {
            conversation: conv.clone(),
            retries: 0,
        };

        // If there's a warning, caller handles both actions
        display_action.unwrap_or(AgentAction::SendLlmRequest { messages: conv })
    } else {
        // Stop loop, return to user
        self.state = AgentState::WaitingForUserInput {
            conversation: conversation.clone(),
        };

        display_action.unwrap_or(AgentAction::PromptForInput)
    }
}
```

#### 0.2 Minimal Hook Implementation (REPL)

**Location:** `crates/coding-agent-cli/src/cli/repl.rs`

The REPL handles `RunPostToolsHooks` by checking context budget:

```rust
AgentAction::RunPostToolsHooks { tool_names } => {
    // Minimal implementation: just check context budget
    let usage_percent = self.token_counter.usage_percentage();

    let (proceed, warning) = if usage_percent >= 70.0 {
        (true, Some(format!(
            "⚠ Context at {:.0}% — consider /clear or /land soon",
            usage_percent
        )))
    } else if usage_percent >= 60.0 {
        (true, Some(format!(
            "⚠ Context at {:.0}% — approaching limit",
            usage_percent
        )))
    } else {
        (true, None)
    };

    // Send HooksCompleted back to state machine
    let action = self.machine.handle_event(AgentEvent::HooksCompleted {
        proceed,
        warning,
    });

    self.handle_action(action).await?;
}
```

#### 0.3 Future Hook Expansion

Once the infrastructure exists, the REPL can add more sophisticated hooks:

| Hook | Trigger | Action |
|------|---------|--------|
| Context warning | Always | Check token budget, warn at 60%/70% |
| Lint check | After `edit_file` | Run `cargo clippy`, show errors |
| Test check | After `edit_file` | Run `cargo test`, show failures |
| Auto-format | After `edit_file` | Run `cargo fmt` |
| Progress update | After any tool | Write to `.coding-agent/progress.json` |
| Auto-commit | After edit + tests pass | `git add && git commit` |

These are configured per-project or via CLI flags, not hardcoded.

---

### Phase A: Context Intelligence

**Priority:** Immediate — high value, low effort

| Feature | Current | Improvement |
| ------- | ------- | ----------- |
| Smart zone warning | None | Warn at 60%, urge action at 70% |
| Budget display | Percentage only | Show tokens remaining for reasoning |
| Session scope | None | Remind: "One task per session" |

#### A.1 Smart Zone Warning

**Prerequisite:** Phase 0 (PostToolsHook)

**Note:** The minimal PostToolsHook (Phase 0.2) already provides context warnings after tool execution. This phase adds visual warnings to the context bar for continuous visibility.

**Location:** `crates/coding-agent-cli/src/ui/context_bar.rs`

```text
Current:
  Context: ████████████░░░░░░░░░░░░░░░░░░  38% │ 76k / 200k

Enhanced (60-70%):
  Context: ████████████████████░░░░░░░░░░  65% │ 130k / 200k
  ⚠ Approaching context limit — quality may degrade

Enhanced (>70%):
  Context: ██████████████████████████░░░░  78% │ 156k / 200k
  ⚠ Consider /clear or new session — reasoning space limited
```

**Implementation:**

- Add threshold constants: `SMART_ZONE_WARNING = 60`, `SMART_ZONE_CRITICAL = 70`
- In `ContextBar::render()`, check percentage and append warning text
- Use yellow for warning (60-70%), red for critical (>70%)
- This complements the PostToolsHook warning (which fires after tools) with always-visible status

#### A.2 Session Scope Guidance

**Trigger:** First user message of a new session

**Display:**

```text
💡 Tip: Focus on one task per session for best results.
   When you're done, use /land to safely close.
```

**Implementation:**

- Add `session_message_count` to REPL state
- On first message, display the tip (once per session)
- Make configurable in settings (can disable)

---

### Phase B: Session Lifecycle

**Priority:** Near-term

| Feature | Current | Improvement |
| ------- | ------- | ----------- |
| Session handoff | SpecStory saves history | `/land` command with handoff prompt |
| Session start | Resume loads history | Auto-check "what's ready?" |
| Progress tracking | None | Optional PROGRESS.md |

#### B.1 The `/land` Command (Interactive Mode)

> "When you tell the agent 'land the plane,' it executes a scripted checklist to safely close the session." — Yegge

**Prerequisite:** Phase 0 (PostToolsHook) — The quality gate infrastructure (test/lint) can reuse the hook machinery.

**Command:** `/land`

**Behavior:**

```text
/land

[1/5] Running tests...
  ✓ 759 tests passed

[2/5] Running linter...
  ✓ No warnings

[3/5] Checking for uncommitted changes...
  Found 3 modified files

[4/5] Creating commit...
  ✓ Committed: "Add auth validation (Phase 12.3)"

[5/5] Pushing to remote...
  ✓ Pushed to origin/phase-12-auth

──────────────────────────────────────────────
📋 Handoff for next session:

Next priority work from current state:
- [ ] Implement token refresh logic (Phase 12.4)
- [ ] Add logout endpoint (Phase 12.5)

Context: Working on auth flow. Login validation complete.
Branch: phase-12-auth
Last commit: Add auth validation (Phase 12.3)
──────────────────────────────────────────────

Session saved. Safe to close.
```

**Steps:**

1. **Run tests** — `cargo test` — abort if failing
2. **Run linter** — `cargo clippy` — abort if errors (warnings OK)
3. **Check changes** — `git status` — show what's uncommitted
4. **Commit** — If changes exist, prompt for commit or auto-commit
5. **Push** — Non-negotiable (Yegge: "NEVER stop before pushing")
6. **Generate handoff** — Ask model: "What should the next session focus on?"
7. **Save handoff** — Write to `.coding-agent/last-handoff.md`

**Location:** `src/cli/commands/land.rs`

#### B.2 Automation Script Enhancements

**File:** `phase-automation.sh`

**Current workflow:**

```text
test → commit → push → exit
```

**Enhanced workflow:**

```text
lint → test (fail = abort) → commit → push → generate handoff → exit
```

**Changes to add:**

```bash
# Before the commit step, add:
echo "🔍 Running clippy..."
if ! cargo clippy --package coding-agent-cli -- -D warnings 2>&1; then
    print_warning "Clippy failed - not committing"
    # Don't abort entirely, let retry logic handle
fi

# After the push step, add:
echo "📋 Generating handoff..."
HANDOFF=$(claude -p "Based on the work just completed and the spec at ${SPEC_FILE}, what should the next iteration focus on? Be specific. Output only the handoff text, no preamble.")
echo "$HANDOFF" > .coding-agent/last-handoff.md
echo "Handoff saved to .coding-agent/last-handoff.md"
```

#### B.3 Session-Start Onboarding

**Trigger:** When resuming a session or starting fresh

**Check for:**

1. `.coding-agent/last-handoff.md` — Show if exists
2. `PROGRESS.md` — Show summary if exists
3. `.coding-agent/tasks.json` — Show "ready" tasks if exists

**Display:**

```text
📋 Resuming session...

Last handoff (2 hours ago):
  Next: Implement token refresh logic (Phase 12.4)
  Branch: phase-12-auth

Ready tasks:
  1. [ ] Token refresh implementation
  2. [ ] Logout endpoint

Type your message or /help for commands.
```

#### B.4 Progress File (Optional)

**File:** `PROGRESS.md` (project root)

**Format:**

```markdown
# Project Progress

## Current Focus
Implementing authentication flow (Phase 12)

## Last Session
- Completed: Login validation
- Committed: Add auth validation (Phase 12.3)
- Branch: phase-12-auth

## Next Steps
1. Token refresh logic
2. Logout endpoint
3. Session timeout handling

## Blockers
None currently

## Decisions Made
- Using JWT for tokens (not sessions)
- Refresh tokens expire after 7 days
```

**Who updates it:**

- Agent updates at session end (via `/land` or automation)
- Human can edit directly for course corrections

---

### Phase C: Structured Task Tracking

**Priority:** Medium-term

| Feature | Current | Improvement |
| ------- | ------- | ----------- |
| Task tracking | None (markdown) | `/task` commands |
| Working set | N/A | Show only open tasks |
| Discovered work | Ad-hoc | Prompt to file issues |

#### C.1 Simple `/task` Commands

**Rationale:** Yegge's Beads shows that structured, queryable task tracking beats competing markdown files. But we don't need a full issue tracker — a simple JSON file suffices.

**Commands:**

| Command | Description |
| ------- | ----------- |
| `/task add "description"` | Create new task |
| `/task list` | Show all open tasks |
| `/task done <id>` | Mark task complete |
| `/task ready` | Show unblocked, prioritized tasks |
| `/task block <id> <blocker-id>` | Mark task as blocked by another |

**Storage:** `.coding-agent/tasks.json`

```json
{
  "tasks": [
    {
      "id": "t001",
      "title": "Implement token refresh",
      "status": "open",
      "priority": 1,
      "blocked_by": [],
      "created": "2024-02-13T10:00:00Z"
    },
    {
      "id": "t002",
      "title": "Add logout endpoint",
      "status": "open",
      "priority": 2,
      "blocked_by": ["t001"],
      "created": "2024-02-13T10:05:00Z"
    }
  ]
}
```

**Location:** `src/cli/commands/task.rs`

#### C.2 Discovered Work Prompting

**The pattern:** Agent finds a bug while implementing a feature. Instead of chasing it, file it and continue.

**Implementation:** Add to CLAUDE.md:

```markdown
## Discovered Work

When you discover an issue during implementation:
1. File it: `/task add "description"`
2. Note the source: "Discovered while implementing X"
3. Continue with your current task
4. Do NOT chase the discovered issue
```

#### C.3 Working Set Principle

> "Keep the number of active/open issues small. Agents get overwhelmed by large backlogs." — Yegge

**Rule:** `/task ready` shows at most 5 tasks. Others are "backlog."

**Implementation:**

- Tasks with `priority > 5` are backlog
- `/task list` shows all, `/task ready` shows top 5 unblocked

---

### Phase D: Autonomous Mode Enhancements

**Priority:** Future

| Feature | Current | Improvement |
| ------- | ------- | ----------- |
| Loop mode | Bash wrapper | `--loop` flag on CLI |
| Disk state | Session files | IMPLEMENTATION_PLAN.md |
| Quality gates | Self-healing | Pre-commit hooks |
| Parallel phases | Sequential | Git worktrees for parallel agents |

#### D.1 Native Loop Mode

**Current:** `phase-automation.sh` wraps the CLI in a bash loop.

**Future:** `coding-agent --loop --prompt PROMPT.md`

**Behavior:**

1. Read prompt file
2. Execute one task
3. Run quality gates (test, lint)
4. Commit and push
5. Update progress file
6. Exit (bash loop or internal loop restarts)

**Why native:**

- Better error handling
- Integrated with task tracking
- Can respect `/land` protocol internally

#### D.2 IMPLEMENTATION_PLAN.md Pattern

> "The implementation plan persists as a file. The context window is ephemeral." — Huntley

**Format:**

```markdown
# Implementation Plan

## Objective
Build authentication flow per specs/auth.md

## Tasks
- [x] Login endpoint
- [x] Password validation
- [ ] Token refresh ← CURRENT
- [ ] Logout endpoint
- [ ] Session timeout

## Notes
- Using JWT (decision made 2024-02-13)
- Refresh tokens: 7-day expiry
```

**Usage in loop:**

1. Read IMPLEMENTATION_PLAN.md
2. Find first unchecked `[ ]` item
3. Implement it
4. Mark `[x]`
5. Commit
6. Exit

#### D.3 Pre-Commit Hooks

**Tool:** Consider `prek` (Rust-based, fast) or standard git hooks.

**Hooks to run:**

```bash
# .coding-agent/pre-commit (or .git/hooks/pre-commit)
#!/bin/bash
set -e
cargo fmt --check
cargo clippy -- -D warnings
cargo test --package coding-agent-cli
```

**Why:**

- Rejects bad code before it enters the repo
- "Annoying for humans, perfect for agents" — Huntley
- Catches issues earlier than self-healing

#### D.4 Git Worktrees for Parallel Phases

> "Gas Town uses git worktrees so each agent works in its own directory." — Yegge

**Concept:** Multiple agents work on different phases simultaneously, each in its own worktree. All share the same `.git` database, but have independent working directories.

**Directory structure:**

```text
project/
├── .git/                    # Shared git database
├── main/                    # Primary worktree (or just project root)
└── .worktrees/
    ├── phase-12-auth/       # Agent 1: auth work
    ├── phase-13-ui/         # Agent 2: UI work
    └── phase-14-integration/# Agent 3: integration
```

**Parallel automation script:** `phase-automation-parallel.sh`

```bash
#!/bin/bash
# Parallel phase automation using git worktrees

set -e

SPEC_FILE="$(pwd)/specs/command-line-interface.md"
WORKTREE_DIR=".worktrees"
MAX_PARALLEL=3
DELAY=5

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Get phases that have unchecked deliverables
get_pending_phases() {
    # Parse spec file to find phases with [ ] items
    # Returns: "12 13 14" etc.
    grep -E "^### Phase [0-9]+" "$SPEC_FILE" | \
        while read -r line; do
            phase_num=$(echo "$line" | grep -oE "[0-9]+")
            # Check if phase has unchecked items
            if grep -A 100 "$line" "$SPEC_FILE" | grep -q "^\- \[ \]"; then
                echo "$phase_num"
            fi
        done | head -n "$MAX_PARALLEL"
}

# Create worktree for a phase
setup_worktree() {
    local phase=$1
    local branch="phase-$phase"
    local worktree_path="$WORKTREE_DIR/$branch"

    if [ ! -d "$worktree_path" ]; then
        echo -e "${BLUE}Creating worktree for phase $phase...${NC}"
        git worktree add "$worktree_path" -b "$branch" main 2>/dev/null || \
        git worktree add "$worktree_path" "$branch"
    fi

    echo "$worktree_path"
}

# Run agent in worktree
run_phase_agent() {
    local phase=$1
    local worktree_path=$2
    local log_file="$WORKTREE_DIR/phase-$phase.log"

    echo -e "${GREEN}Starting agent for phase $phase in $worktree_path${NC}"

    (
        cd "$worktree_path"

        claude --dangerously-skip-permissions -p "Read ${SPEC_FILE}.

WORKFLOW (Phase $phase only):
1. Find Phase $phase section
2. Find first [ ] deliverable in Phase $phase
3. Implement it FULLY
4. Write tests, run 'cargo test --package coding-agent-cli'
5. Run 'cargo build --package coding-agent-cli'
6. Mark item as [x] in spec
7. Git add, commit: '<action>: <what> (Phase $phase.X)'
8. Push to origin/phase-$phase
9. Output 'DELIVERABLE_COMPLETE' and exit

CRITICAL: Work ONLY on Phase $phase. One deliverable, then exit."
    ) > "$log_file" 2>&1

    local exit_code=$?
    echo -e "${BLUE}Phase $phase agent finished (exit: $exit_code)${NC}"
    return $exit_code
}

# Merge completed phase to main
merge_phase() {
    local phase=$1
    local branch="phase-$phase"

    echo -e "${GREEN}Merging $branch to main...${NC}"
    git checkout main
    git merge "$branch" --no-edit
    git push origin main
    git branch -d "$branch"
    git worktree remove "$WORKTREE_DIR/$branch" --force
}

# Main loop
main() {
    mkdir -p "$WORKTREE_DIR"

    while true; do
        phases=($(get_pending_phases))

        if [ ${#phases[@]} -eq 0 ]; then
            echo -e "${GREEN}All phases complete!${NC}"
            exit 0
        fi

        echo -e "${BLUE}━━━━━━━━━━ Starting parallel run [$(date +%H:%M:%S)] ━━━━━━━━━━${NC}"
        echo "Phases to work: ${phases[*]}"

        # Setup worktrees and launch agents in parallel
        pids=()
        for phase in "${phases[@]}"; do
            worktree=$(setup_worktree "$phase")
            run_phase_agent "$phase" "$worktree" &
            pids+=($!)
        done

        # Wait for all agents
        for pid in "${pids[@]}"; do
            wait "$pid" || true
        done

        # Check for completed phases and merge
        for phase in "${phases[@]}"; do
            # Check if all items in phase are [x]
            if ! grep -A 100 "### Phase $phase" "$SPEC_FILE" | \
                 grep -B 100 "### Phase" | grep -q "^\- \[ \]"; then
                merge_phase "$phase"
            fi
        done

        echo -e "${YELLOW}Waiting ${DELAY}s before next iteration...${NC}"
        sleep "$DELAY"
    done
}

main "$@"
```

**Key benefits:**

- **True parallelism**: Multiple phases progress simultaneously
- **Isolation**: Agent in phase-12 can't break phase-13's work
- **Clean merges**: Each phase is a branch, merges when complete
- **Shared git**: All worktrees share `.git`, efficient disk usage

**Conflict handling:**

- Phases should be independent (different files/modules)
- If phases touch same files, use sequential automation instead
- On merge conflict: pause, human resolves, then continue

**Cleanup:**

```bash
# Remove all worktrees and branches when done
git worktree list | grep -v "main" | awk '{print $1}' | xargs -I {} git worktree remove {}
git branch | grep "phase-" | xargs git branch -D
```

---

## Anti-Patterns to Avoid

| Anti-Pattern | Why It's Bad | Instead |
| ------------ | ------------ | ------- |
| Complex orchestration (Temporal) | Overkill for single agent | Simple state machine + model decisions |
| Multiple markdown files | Bit-rot: PLAN.md, TODO.md, TASKS.md contradict | Single source of truth |
| Hardcoded completion heuristics | Brittle, breaks on edge cases | Let model judge "is this done?" |
| Multi-agent before single-agent | Complexity explosion | Master single-agent first |
| Bloated CLAUDE.md | Wastes context every session | Keep <60 lines, operational only |
| Chasing discovered work | Session sprawl, lost focus | File it, continue current task |
| Context window sprawl | Quality degrades past 60% | One task per session, use `/land` |
| "I'm ready when you are" | Passive, wastes human time | Just do it (push, commit, act) |

---

## Implementation Order

**Recommended sequence:**

1. **0.1-0.2** PostToolsHook infrastructure — Foundational, enables everything else
2. **A.1** Smart zone warning — Now trivial (hook already checks budget)
3. **B.1** `/land` command — Core session management
4. **B.2** Automation enhancements — Improve existing workflow
5. **A.2** Session scope guidance — Polish
6. **B.3** Session-start onboarding — Ties together
7. **C.1** Task tracking — When markdown files become unwieldy
8. **D.1-D.3** Autonomous enhancements — When single-agent is mastered
9. **D.4** Parallel worktrees — When phases are independent and you want speed

**Note:** Once Phase 0 is complete, A.1 (Smart zone warning) is essentially done — the PostToolsHook minimal implementation already provides context warnings at 60%/70%.

---

## References

**Yegge's Work:**

- Beads repo: github.com/steveyegge/beads
- Gas Town repo: github.com/steveyegge/gastown
- VC repo: github.com/steveyegge/vc
- Blogs: "Zero Framework Cognition", "Beads Best Practices", "Gas Town Emergency User Manual"

**Huntley's Work:**

- Loom repo (loom-core, loom-cli, loom-thread)
- Ralph Playbook
- Workshop materials and podcast interviews

**Anthropic:**

- "Effective Harnesses for Long-Running Agents" (Nov 2025)

---

## Appendix: CLAUDE.md Best Practices

Keep CLAUDE.md **operational and concise** (<60 lines). Progress and status go elsewhere.

**Good structure:**

```markdown
## Build & Run
cargo build -p coding-agent-cli
cargo test -p coding-agent-cli

## Validation
- Tests: `make test`
- Lint: `make lint`

## Key Patterns
- State machine in coding-agent-core is pure (no I/O)
- Commands implement Command trait, register in mod.rs
- Tools defined in tools/definitions.rs

## Session Management
- Use /land to safely close sessions
- One task per session for best results
- File discovered work with /task add, don't chase it
```

**Bad patterns:**

- Progress notes ("we completed X yesterday")
- Long explanations of decisions
- Duplicating spec content
- Status updates (use PROGRESS.md instead)

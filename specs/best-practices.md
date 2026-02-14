# Best Practices for AI Coding Agents

> Focused patterns for the coding-agent CLI, synthesized from GSD, Yegge's Gas Town/Beads, and Huntley's Loom/Ralph.

---

## Part 1: Core Principles

### 1. Zero Framework Cognition (ZFC)

> "All decision-making and reasoning is delegated to AI models. The application code is a thin, safe, deterministic shell."

**Rules:**
- No heuristics, regex, or pattern matching for decisions in the harness
- Don't parse or interpret agent output with brittle rules
- When you need a judgment call, send it to the model
- The CLI is a dumb pipe that routes information to/from the model

**Test:** Before adding any if/else decision logic, ask: "Should the model decide this?"

### 2. Context Window as Memory

> "A context window is just mallocing an array. There is no persistent server memory."

**The numbers:**
- 200K advertised → ~176K usable (system prompt + overhead)
- **40-60% is the "smart zone"** — beyond 60-70%, quality degrades
- Less is more: every token displaces reasoning capacity

**Key rules:**
1. One task per context window
2. Clear context between activities
3. Deterministically allocate files
4. Minimize tool definitions

### 3. Backpressure

> "Software engineering is now about preventing failure scenarios through back pressure to the generative function."

**Forms:**

| Type | Tool | When |
|------|------|------|
| Unit tests | `cargo test` | After changes |
| Type checking | `cargo check` | Before commit |
| Linting | `cargo clippy` | Before commit |
| Build | `cargo build` | Before commit |
| Pre-commit | hooks | Automatic |

### 4. State Lives on Disk

Context windows are ephemeral. Persistence goes elsewhere:

| Data | Location | Format |
|------|----------|--------|
| Conversation history | `.specstory/history/` | Markdown |
| Current progress | `PROGRESS.md` | Markdown |
| Task tracking | `.coding-agent/tasks.json` | JSON |
| Handoff prompts | `.coding-agent/last-handoff.md` | Markdown |

### 5. One Task Per Context

**Anti-pattern:** A session that started with "fix the login bug" and sprawled to "also refactor auth and add 2FA."

**Fix:**
- Session scope reminder at start
- When scope drifts, suggest `/clear` or new session
- File discovered work for later, don't chase it now

### 6. Deviation Rules

Explicit autonomy boundaries for the agent.

**Auto-fix (proceed via FixAgent):**
- Rule 1: Code errors the agent introduced (compiler errors, type mismatches)
- Rule 2: Broken dependencies (missing crate in Cargo.toml that code needs)
- Rule 3: Test/lint failures from agent's changes

**Must ask user (blocks execution):**
- Rule 4: Architectural changes (new modules, schema changes, significant refactors)
- Rule 5: Adding dependencies not mentioned in task
- Rule 6: Deleting files

**Implementation:**
Extend `FixAgent::should_attempt_fix()` to check deviation category:
- `ErrorCategory::Code` + agent-introduced → auto-fix
- `ErrorCategory::Architecture` → prompt user

---

## Part 2: Implementation

### Phase 0: PostToolsHook Infrastructure

**Priority:** Foundational — enables context warnings, quality gates, and session management.

**Why:** Currently, when tools complete, the state machine immediately transitions to `CallingLlm`. This leaves no hook point for quality gates or context checking.

**Current flow:**
```
ExecutingTools → [all done] → CallingLlm
```

**New flow:**
```
ExecutingTools → [all done] → PostToolsHook → [hooks complete] → CallingLlm
```

#### 0.1 State Machine Changes

**File:** `crates/coding-agent-core/src/state.rs`

Add new state:
```rust
PostToolsHook {
    conversation: Vec<Message>,
    pending_tool_results: Vec<Message>,
}
```

Add new event:
```rust
HooksCompleted {
    proceed: bool,
    warning: Option<String>,
}
```

Add new actions:
```rust
RunPostToolsHooks {
    tool_names: Vec<String>,
}

DisplayWarning(String),
```

**File:** `crates/coding-agent-core/src/machine.rs`

Change the "all tools done" branch to transition to `PostToolsHook` instead of `CallingLlm`:

```rust
if all_done {
    // Collect tool names for hook decision
    let tool_names: Vec<String> = execs.iter()
        .filter_map(|e| match e {
            ToolExecutionStatus::Completed { .. } => None,
            ToolExecutionStatus::Pending { tool_name, .. } => Some(tool_name.clone()),
            ToolExecutionStatus::Running { tool_name, .. } => Some(tool_name.clone()),
        })
        .collect();

    self.state = AgentState::PostToolsHook {
        conversation: conversation.clone(),
        pending_tool_results: tool_result_messages,
    };

    AgentAction::RunPostToolsHooks { tool_names }
}
```

Add PostToolsHook handler:
```rust
(
    AgentState::PostToolsHook { conversation, pending_tool_results },
    AgentEvent::HooksCompleted { proceed, warning },
) => {
    let display_action = warning.map(|w| AgentAction::DisplayWarning(w));

    if proceed {
        let mut conv = conversation.clone();
        conv.extend(pending_tool_results.clone());
        self.state = AgentState::CallingLlm { conversation: conv.clone(), retries: 0 };
        display_action.unwrap_or(AgentAction::SendLlmRequest { messages: conv })
    } else {
        self.state = AgentState::WaitingForUserInput { conversation: conversation.clone() };
        display_action.unwrap_or(AgentAction::PromptForInput)
    }
}
```

#### 0.2 Minimal Hook Implementation (REPL)

**File:** `crates/coding-agent-cli/src/cli/repl.rs`

```rust
AgentAction::RunPostToolsHooks { tool_names } => {
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

    let action = self.machine.handle_event(AgentEvent::HooksCompleted {
        proceed,
        warning,
    });

    self.handle_action(action).await?;
}
```

#### 0.3 Future Hook Expansion

Once infrastructure exists, the REPL can add:

| Hook | Trigger | Action |
|------|---------|--------|
| Context warning | Always | Check token budget |
| Lint check | After `edit_file` | Run `cargo clippy` |
| Test check | After `edit_file` | Run `cargo test` |
| Auto-format | After `edit_file` | Run `cargo fmt` |
| Progress update | After any tool | Write to progress file |

---

### Phase A: Context Warnings

**File:** `crates/coding-agent-cli/src/ui/context_bar.rs`

Add thresholds:
```rust
pub const SMART_ZONE_WARNING: f64 = 60.0;
pub const SMART_ZONE_CRITICAL: f64 = 70.0;
```

Enhanced display:
```
Context: [====================----------]  65% | 130k / 200k
⚠ Approaching context limit — quality may degrade

Context: [==========================----]  78% | 156k / 200k
⚠ Consider /clear or new session — reasoning space limited
```

Add `warning_message()` method:
```rust
pub fn warning_message(&self) -> Option<String> {
    let pct = self.percent() as f64;
    if pct >= SMART_ZONE_CRITICAL {
        Some("⚠ Consider /clear or new session — reasoning space limited".to_string())
    } else if pct >= SMART_ZONE_WARNING {
        Some("⚠ Approaching context limit — quality may degrade".to_string())
    } else {
        None
    }
}
```

---

### Phase B: The `/land` Command

> "When you tell the agent 'land the plane,' it executes a scripted checklist to safely close the session."

**Command:** `/land`

**Steps:**
1. **Run tests** — `cargo test` — abort if failing
2. **Run linter** — `cargo clippy` — abort if errors
3. **Check changes** — `git status` — show uncommitted
4. **Commit** — If changes exist, prompt or auto-commit
5. **Push** — Non-negotiable ("NEVER stop before pushing")
6. **Generate handoff** — Ask model: "What should the next session focus on?"
7. **Save handoff** — Write to `.coding-agent/last-handoff.md`

**Output:**
```
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

Next priority work:
- [ ] Implement token refresh logic (Phase 12.4)
- [ ] Add logout endpoint (Phase 12.5)

Context: Working on auth flow. Login validation complete.
Branch: phase-12-auth
──────────────────────────────────────────────

Session saved. Safe to close.
```

**File:** `crates/coding-agent-cli/src/cli/commands/land.rs`

---

### Phase C: Session Guidance

**File:** `/Users/charliposner/coding-agent/CLAUDE.md`

Add section:
```markdown
## Session Discipline

- **One task per session** — don't let scope creep
- When you discover a bug while implementing a feature:
  1. Note it: "Discovered: [description]"
  2. Continue with current task
  3. File it at session end or suggest `/task add`
- When context reaches 60%+, suggest wrapping up via `/land`
```

---

## Part 3: Anti-Patterns

| Anti-Pattern | Why It's Bad | Instead |
|--------------|--------------|---------|
| Complex orchestration | Overkill for single agent | Simple state machine + model decisions |
| Multiple markdown files | Bit-rot, contradictions | Single source of truth |
| Hardcoded completion heuristics | Brittle | Let model judge "is this done?" |
| Multi-agent before single-agent | Complexity explosion | Master single-agent first |
| Bloated CLAUDE.md | Wastes context every session | Keep <60 lines, operational only |
| Chasing discovered work | Session sprawl | File it, continue current task |
| Context window sprawl | Quality degrades past 60% | One task per session, use `/land` |
| "I'm ready when you are" | Passive, wastes time | Just do it |

---

## Part 4: File Formats and Size Limits

### Size Limits

| File | Max Lines | Purpose |
|------|-----------|---------|
| `CLAUDE.md` | 60 | Operational instructions only |
| `STATE.md` | 300 | Current session state |
| `PROGRESS.md` | 300 | What happened |
| `PLAN.md` | 600 | Task breakdown |

### Passing Context (Future Subagents)

When a future subagent system exists:
- Pass file **paths**, not contents
- Subagent reads files in its own fresh context
- Return concise summaries (1-2k tokens), not transcripts

For now, this applies to human handoffs and `/land` output:
- Handoff lists relevant file paths, not contents
- Next session reads files fresh

### CLAUDE.md Structure

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

---

## References

**Yegge's Work:**
- Beads repo: github.com/steveyegge/beads
- Gas Town repo: github.com/steveyegge/gastown
- "Zero Framework Cognition" blog

**Huntley's Work:**
- Loom (loom-core, loom-cli, loom-thread)
- Ralph Playbook

**GSD:**
- Deviation rules and auto-fix boundaries

**Anthropic:**
- "Effective Harnesses for Long-Running Agents" (Nov 2025)

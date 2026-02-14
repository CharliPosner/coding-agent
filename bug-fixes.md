# Bug Fixes

Iterative bug fix list. Work through each deliverable in order.

Each deliverable contains multiple checklist items that should be completed together.

---

## Deliverable 1: Context Bar Display Mismatch
**Priority**: High
**Files**: `crates/coding-agent-cli/src/ui/context_bar.rs`, `crates/coding-agent-cli/src/cli/repl.rs`

**Symptoms**:
- Progress bar shows fully filled (solid green block)
- But percentage displays "0% used"
- Token count stays at "1k / 200k" and doesn't update after conversation exchanges

**Root Cause Investigation**:
- Check `render()` method in context_bar.rs - bar fill calculation may be inverted
- Check `update_context_tokens()` in repl.rs - tokens may not be accumulating
- The filled/empty character logic may be backwards

### Checklist
- [x] Identify the bar calculation bug in `context_bar.rs:render()`
- [x] Fix filled vs empty portion logic
- [x] Verify token count updates after each exchange
- [x] Test color transitions at 60% and 85% thresholds

### Tests

| Test | Type | What it verifies |
|------|------|------------------|
| `test_context_bar_empty_shows_empty` | Unit | 0 tokens = empty bar (all ░) |
| `test_context_bar_full_shows_full` | Unit | 200k tokens = full bar (all █) |
| `test_context_bar_percentage_matches_visual` | Unit | 50% usage = half filled bar |
| `test_context_bar_updates_on_exchange` | Integration | Bar updates after user/assistant messages |

**Stopping condition:**
```
✓ Empty context shows empty bar with 0%
✓ Bar fill matches percentage displayed
✓ Token count increments after each exchange
✓ All unit tests pass
```

---

## Deliverable 2: Read Tool Output Not Collapsible
**Priority**: Medium
**Files**: `crates/coding-agent-cli/src/ui/tool_result.rs`

**Symptoms**: Read tool dumps entire file content to terminal, unlike search/list tools which collapse.

### Checklist
- [x] Modify `format_result_collapsible()` case for read_file (lines 80-85)
- [x] Add line count check against `collapse_threshold`
- [x] Return `FormattedResult` with `collapsed_content: Some()` when exceeding threshold
- [x] Show summary: `"▸ file.rs (N lines, use /results to expand)"`
- [x] Verify `/results` command expands the collapsed content

### Tests

| Test | Type | What it verifies |
|------|------|------------------|
| `test_read_small_file_not_collapsed` | Unit | Files under threshold show full content |
| `test_read_large_file_collapsed` | Unit | Files over threshold show summary |
| `test_read_collapsed_has_content` | Unit | `collapsed_content` contains full file |
| `test_results_expands_read` | Integration | `/results` shows full file after collapse |

**Stopping condition:**
```
✓ Small files (<5 lines) display inline
✓ Large files show "▸ file.rs (N lines, use /results to expand)"
✓ /results displays the full file content
✓ All tests pass
```

---

## Deliverable 3: Code Blocks Need Syntax Highlighting
**Priority**: Medium
**Files**: `crates/coding-agent-cli/src/ui/markdown.rs`, `crates/coding-agent-cli/Cargo.toml`

**Symptoms**: Code blocks render in solid green. No syntax highlighting for keywords, strings, comments.

### Checklist
- [x] Add `syntect` dependency to Cargo.toml
- [x] Parse language from markdown fence (```rust, ```python, etc.)
- [x] Load syntax definitions for common languages
- [x] Apply syntax-aware coloring in code block rendering
- [x] Fallback to plain green for unknown languages

### Tests

| Test | Type | What it verifies |
|------|------|------------------|
| `test_code_block_rust_keywords` | Unit | `fn`, `let`, `impl` colored as keywords |
| `test_code_block_rust_strings` | Unit | String literals colored differently |
| `test_code_block_rust_comments` | Unit | Comments colored as muted |
| `test_code_block_unknown_lang` | Unit | Unknown language falls back gracefully |
| `test_code_block_no_lang` | Unit | Unfenced code still renders (green fallback) |

**Stopping condition:**
```
✓ Rust code has colored keywords, strings, comments
✓ Python code has colored keywords, strings, comments
✓ Unknown languages render without crash
✓ Visual inspection confirms readable colors
✓ All tests pass
```

---

## Deliverable 4: Maximum Tool Iterations Limit Too Low
**Priority**: Low
**Files**: `crates/coding-agent-cli/src/cli/repl.rs`, `crates/coding-agent-cli/src/config/settings.rs`

**Symptoms**: Error "Maximum tool iterations reached" after 10 tool calls.

**Current**: Hardcoded `MAX_TOOL_ITERATIONS: usize = 10` (line ~389)

### Checklist
- [x] Increase default limit to 50
- [x] Add `max_tool_iterations` to `BehaviorConfig` in settings.rs
- [x] Read config value in `process_conversation()`
- [x] Add iteration counter display during execution (e.g., "Tool call 5/50")
- [x] Warn user at 80% of limit before hard stop

### Tests

| Test | Type | What it verifies |
|------|------|------------------|
| `test_tool_iterations_default` | Unit | Default is 50 iterations |
| `test_tool_iterations_configurable` | Unit | Config value overrides default |
| `test_tool_iterations_warning` | Unit | Warning shown at 80% threshold |
| `test_tool_iterations_stops_at_limit` | Integration | Hard stop at configured limit |

**Stopping condition:**
```
✓ Default limit increased to 50
✓ Limit configurable via config.toml
✓ Warning displayed at 80% of limit
✓ All tests pass
```

---

## Deliverable 5: H1 Header Dark Blue Has Poor Contrast
**Priority**: Low
**Files**: `crates/coding-agent-cli/src/ui/markdown.rs`

**Symptoms**: Single `#` headers render in dark blue which is hard to read against black terminal background.

**Current**: Headers use Cyan -> Blue -> DarkCyan for h1 -> h2 -> h3

### Checklist
- [x] Change h1 color from dark blue to bright cyan or bright white
- [x] Ensure h1 > h2 > h3 visual hierarchy maintained
- [x] Test against common terminal backgrounds (black, dark grey)

### Tests

| Test | Type | What it verifies |
|------|------|------------------|
| `test_h1_uses_bright_color` | Unit | H1 color is bright cyan or white |
| `test_header_hierarchy` | Unit | h1 more prominent than h2/h3 |
| Snapshot: `header_rendering.snap` | UI | Headers render with good contrast |

**Stopping condition:**
```
✓ H1 headers clearly visible on dark backgrounds
✓ Visual hierarchy preserved (h1 > h2 > h3)
✓ Snapshot test approved
✓ All tests pass
```

# Unintegrated Features

This document tracks features that have been implemented but not yet fully integrated into the main execution flow. These generate compiler warnings about unused code, but are intentionally kept for future integration.

## Status: Implemented But Not Integrated

### Phase 14.3: Self-Healing Error Recovery

**Status**: Code complete, integration pending

**Modules:**
- `tools/auto_fix.rs` - Automatic fix application for various error types
  - Add missing dependencies to Cargo.toml/package.json
  - Add missing imports
  - Apply type fixes
  - Apply syntax fixes
- `tools/diagnostics.rs` - Compiler error parsing
  - Parse Rust, TypeScript, Go, and generic compiler output
  - Extract structured diagnostic information
  - Extract fix suggestions from error messages
- `tools/regression_tests.rs` - Automatic regression test generation
  - Generate tests after successful auto-fixes
  - Prevent fix regressions

**What needs to be done:**
1. Wire `FixAgent` into `ToolExecutor` error handling
2. On tool failure, check if error is auto-fixable
3. Spawn fix agent, apply fix, regenerate test
4. Retry original operation

### Phase 14.4: UI Enhancements

**Status**: Partially integrated

**Components fully integrated:**
- `ui/context_bar.rs` - Token usage visualization ✓
- `ui/theme.rs` - Color scheme ✓
- `ui/tool_spinner.rs` - Tool execution spinners ✓

**Components not yet integrated:**
- `ui/components.rs` - `MessageBox` component
- `ui/fun_facts.rs` - Fun fact fetching and caching (partially used)
- `ui/long_wait.rs` - Long wait detection (partially used)
- `ui/output.rs` - `StyledOutput` wrapper
- `ui/progress.rs` - Generic progress bar
- `ui/spinner.rs` - Generic spinner
- `ui/status_bar.rs` - Multi-agent status display
- `ui/syntax.rs` - Syntax highlighting (partially used)
- `ui/tool_result.rs` - Tool result formatting (partially used)

**What needs to be done:**
1. Use `StatusBar` in REPL when agents are active
2. Use `LongWaitDetector` more consistently for API calls
3. Use `MessageBox` for commit message previews
4. Apply syntax highlighting more consistently

### Phase 14.5: Advanced Git Features

**Status**: Partially integrated

**Integrated:**
- Basic git operations (status, commit, diff) ✓
- Commit message generation ✓

**Not integrated:**
- `integrations/git.rs` - Smart file grouping
  - `FileGrouper::group_files()` - Group related changes
  - `suggest_commit_splits()` - Suggest logical commit splits
  - `RepoStatus` helper methods - Filtering by file state

**What needs to be done:**
1. Call `FileGrouper::group_files()` in `/commit` command
2. Show grouping suggestions to user
3. Allow committing logical groups separately

### Phase 14.6: Obsidian Integration

**Status**: Basic implementation, not integrated

**Not integrated:**
- `integrations/obsidian.rs` - Most functionality
  - `ObsidianVault::search()` - Search notes by content
  - `ObsidianVault::create_note()` - Create new notes
  - `ObsidianVault::update_note()` - Update existing notes
  - `Note` metadata tracking
  - Diff generation for note updates

**What needs to be done:**
1. Implement `/document` command fully
2. Wire up vault search
3. Show search results and allow selection
4. Support create/update with previews

### Agents Infrastructure

**Status**: Partially integrated

**Integrated:**
- `AgentManager` created in REPL ✓
- Agent spawning infrastructure ✓

**Not fully utilized:**
- `agents/status.rs` - Agent state tracking
- `agents/manager.rs` - Progress reporting
- Agent names and descriptions (stored but not displayed)

**What needs to be done:**
1. Display agent progress in UI
2. Show agent names/descriptions in status bar
3. Use progress reporter in long-running operations

### Permissions System

**Status**: Core integrated, some features not used

**Integrated:**
- Basic permission checking ✓
- Trusted paths ✓

**Not integrated:**
- `permissions/prompt.rs` - Some helper functions
- `SessionPermissions` caching - Not fully utilized
- Config file updates from "Always" responses - Partially working

### Tokens & Pricing

**Status**: Core integrated, some models not configured

**Not integrated:**
- `tokens/pricing.rs`
  - Additional model constants (Sonnet, Haiku, etc.)
  - `available_models()` function
  - `calculate_total_cost()` method

**What needs to be done:**
1. Add all Claude model variants to pricing
2. Use `available_models()` in model switcher
3. Show total cost calculation in `/cost` command

### Config System

**Status**: Core working, advanced features not used

**Not integrated:**
- `config/settings.rs`
  - `load()`, `load_from()` methods - Manual config loading
  - `parse()`, `to_toml()` - Advanced serialization
  - `add_trusted_path_to()` - Programmatic config updates

**What needs to be done:**
1. Use programmatic config updates for "Always" permission responses
2. Implement config reload without restart

## Compiler Warnings Summary

As of 2026-02-14:
- ~170 warnings in bin target
- ~5 warnings in lib target
- Most are `dead_code` or `unused_imports` for unintegrated features

## Approach

Rather than removing this carefully implemented code, we've:

1. Added `#![allow(dead_code)]` to major unintegrated modules:
   - `tools/auto_fix.rs`
   - `tools/diagnostics.rs`
   - `tools/regression_tests.rs`

2. Added targeted `#[allow(dead_code)]` to specific structs/functions that are part of public APIs but not yet called

3. Documented unintegrated features in this file for future reference

## Next Steps

When resuming integration work:

1. Start with Phase 14.3 (Self-Healing) - highest value, most complex
2. Then Phase 14.4 (UI polish) - improves user experience
3. Then Phase 14.5 & 14.6 (Git & Obsidian) - workflow improvements
4. Finally, minor enhancements to agents, permissions, tokens

Each integration should:
- Remove corresponding `#[allow(dead_code)]` attributes
- Write integration tests
- Update this document
- Verify no new warnings introduced

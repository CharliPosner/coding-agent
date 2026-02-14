# Phase 4: Implementation Strategy & Sub-Agent Approach

## "Sub-Agent" Implementation Plan

Since I can't literally spawn sub-processes, I'll create a systematic analysis approach that simulates multiple focused agents:

### Agent 1: UI/Visual Test Auditor
**Mission: Eliminate visual test redundancy**
**Target: ui_visual_regression_tests.rs**

```bash
# Commands to execute:
# 1. Identify theme redundancy patterns
rg "test_theme_all_colors" --type rust
# 2. Count snapshot tests  
grep -c "assert_snapshot!" crates/coding-agent-cli/tests/ui_visual_regression_tests.rs
# 3. Identify consolidation candidates
rg "test_.*_snapshot" --type rust -A5 -B2
```

### Agent 2: Tool Test Deduplication Specialist  
**Mission: Eliminate tool testing duplication**
**Target: tests/ directory vs integration tests**

```bash
# Commands to execute:
# 1. Map tool test duplication
find tests/ -name "test_*.rs" -exec basename {} \; | sort
find crates/*/tests/integration/ -name "*test*.rs" -exec basename {} \; | sort
# 2. Count total tool tests
grep -r "fn test_" tests/ | wc -l
# 3. Identify overlap patterns  
rg "bash|read_file|list_files|edit_file|code_search" tests/ --type rust -l
```

### Agent 3: Integration Test Consolidator
**Mission: Merge overlapping integration tests** 
**Target: crates/coding-agent-cli/tests/integration/**

```bash
# Commands to execute:
# 1. Analyze integration test categories
ls -la crates/coding-agent-cli/tests/integration/
# 2. Count agent management tests
grep -c "fn test_agent" crates/coding-agent-cli/tests/integration/agent_integration_test.rs  
# 3. Identify workflow test overlap
rg "workflow|permission|retry" crates/coding-agent-cli/tests/integration/ --type rust -l
```

### Agent 4: E2E/Snapshot Overlap Eliminator
**Mission: Remove E2E/UI test duplication**
**Target: e2e/ vs ui_visual_regression_tests.rs**

```bash  
# Commands to execute:
# 1. Find snapshot test overlap
rg "assert_snapshot.*context_bar" --type rust
rg "assert_snapshot.*message" --type rust  
# 2. Count E2E snapshot tests
find crates/coding-agent-cli/tests/e2e/ -name "*.rs" -exec grep -c "assert_snapshot" {} \;
# 3. Map UI component overlap
rg "MessageBox|ProgressBar|Spinner|ToolResult" crates/coding-agent-cli/tests/ --type rust -l
```

## Implementation Commands

### Step 1: Execute Analysis Commands
Run the analysis commands above to generate specific consolidation targets.

### Step 2: Create Consolidated Test Files
Create new streamlined test files following the patterns from phases 1-3.

### Step 3: Remove Bloated Test Files  
Delete the identified redundant test files and excessive test functions.

### Step 4: Verification
```bash
# Before: ~2000 tests
cargo test --dry-run 2>&1 | grep "running" | tail -1

# After consolidation: Target ~400-600 tests (70% reduction)
# Verify all critical functionality still covered
cargo test
```

## Expected Results

**Before:** 2,000+ tests
**After:** ~500-600 tests  
**Reduction:** 70-75% fewer tests
**Benefit:** Faster CI, easier maintenance, focused on actual value
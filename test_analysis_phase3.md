# Phase 3: Integration & E2E Testing Consolidation

## Current Problem  
- Heavy overlap between integration tests, e2e tests, and snapshot tests
- Multiple test harnesses doing similar things
- Over-testing agent management scenarios

## Areas of Redundancy

### Agent Integration Tests (agent_integration_test.rs)
**Current: 12+ comprehensive async tests**
**Recommended: 4-5 core tests**

```rust
// KEEP - Core agent functionality
✅ test_agent_spawn_and_complete()
✅ test_multiple_agents_concurrent()  
✅ test_agent_failure()
✅ test_agent_cancellation()
✅ test_agent_cleanup()

// REMOVE - Over-detailed edge cases  
❌ test_agent_progress_reporting() // Too granular
❌ test_agent_wait_timeout() // Edge case
❌ test_async_agent_spawn() // Covered by core tests
❌ test_agent_state_transitions() // Implementation detail
❌ test_get_agent_status() // Utility function test
❌ test_agent_wait_for_completion() // Covered by spawn_and_complete
```

### E2E Snapshot Tests Overlap
**Current: snapshot_tests.rs has massive overlap with UI visual tests**

```rust
// REMOVE - Duplicates UI visual tests
❌ test_context_bar_render_snapshot_*() // Already in UI tests
❌ test_error_message_formatting_snapshot() // Already in UI tests  
❌ test_success_message_formatting_snapshot() // Already in UI tests
❌ test_warning_message_formatting_snapshot() // Already in UI tests

// KEEP - True E2E behavior
✅ test_startup_screen_snapshot() 
✅ test_help_command_snapshot()
✅ test_unknown_command_snapshot()
```

### Integration Test Categories to Consolidate

```rust
// Permission/Error Tests - consolidate multiple files into one
integration/permission_error_test.rs      } 
integration/resource_error_test.rs        } → integration/error_handling_test.rs
integration/tool_execution_retry_test.rs  }

// Workflow Tests - consolidate multiple complex scenarios  
integration/auto_fix_workflow_test.rs        }
integration/permission_prompt_workflow_test.rs } → integration/workflow_test.rs
integration/multi_agent_coordination_test.rs }

// Terminal/PTY - likely redundant with other integration tests
integration/terminal_test.rs  ← Remove if covered elsewhere
```

**Estimated Reduction: ~500 tests → ~150 tests (70% reduction)**
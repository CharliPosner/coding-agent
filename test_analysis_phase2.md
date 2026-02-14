# Phase 2: Tool Function Testing Consolidation

## Current Problem
- Duplicate testing: Root tests/ directory AND CLI integration tests
- Same tool functionality tested multiple times in different contexts
- Over-testing basic functionality

## Consolidation Strategy

### Current Duplication Pattern
```
tests/test_bash.rs           } Same bash tool
tests/test_read_file.rs      } Same read_file tool  
tests/test_list_files.rs     } Same list_files tool
tests/test_edit_file.rs      } Same edit_file tool
tests/test_code_search.rs    } Same code_search tool
    +
crates/.../integration/*/    } Testing same tools again
```

### Recommendation: Eliminate Root tests/ Directory

**Move essential tests to integration layer, delete root tests/**

```rust
// DELETE: tests/test_bash.rs (3 tests)
// DELETE: tests/test_read_file.rs (3 tests)  
// DELETE: tests/test_list_files.rs (3 tests)
// DELETE: tests/test_edit_file.rs (5 tests)
// DELETE: tests/test_code_search.rs (5 tests)

// KEEP: Integration tests in crates/.../integration/ 
// These test tools in realistic CLI context
```

### Keep One Comprehensive Tool Test Per Tool
```rust
// In integration tests - keep these patterns:
✅ test_tool_basic_success_case()
✅ test_tool_error_handling() 
✅ test_tool_integration_with_cli()

// Remove excessive edge case testing:
❌ test_tool_with_unicode_characters()
❌ test_tool_with_very_long_paths()
❌ test_tool_permission_variations()
```

**Estimated Reduction: ~300 tool tests → ~50 tool tests (83% reduction)**
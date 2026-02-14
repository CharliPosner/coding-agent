# Phase 1: UI/Visual Testing Consolidation

## Current Problem
- 35+ snapshot tests in ui_visual_regression_tests.rs
- Testing every theme variation for identical functionality
- Over-testing edge cases that don't provide value

## Consolidation Strategy

### Keep (Essential Tests - ~10 tests)
```rust
// Core component functionality
#[test] fn test_message_box_basic_functionality()
#[test] fn test_progress_bar_core_behavior()  
#[test] fn test_spinner_cycle_logic()
#[test] fn test_tool_result_formatting()

// Critical edge cases only
#[test] fn test_empty_content_handling()
#[test] fn test_very_long_content_truncation()

// One theme test (not all three)
#[test] fn test_theme_color_application()

// Critical integration
#[test] fn test_complete_tool_execution_sequence()
#[test] fn test_error_display_flow()
#[test] fn test_context_bar_thresholds() // Single test, key percentages only
```

### Remove (Bloat Tests - ~25+ tests)
```rust
// Theme variations - test ONE theme, not all three
❌ test_theme_all_colors_minimal()
❌ test_theme_all_colors_colorful()  
❌ test_theme_all_colors_monochrome()
// Keep: test_theme_color_application() - tests one theme thoroughly

// Excessive edge cases
❌ test_special_characters_display()
❌ test_unicode_emoji_display()
❌ test_very_long_line_no_newlines()

// Redundant snapshot variations  
❌ test_message_box_with_title()
❌ test_message_box_wide_content()
❌ test_message_box_commit_message_style()
// Keep: test_message_box_basic_functionality() 

// Over-granular progress bar tests
❌ test_progress_bar_percentages()
❌ test_progress_bar_increment()
// Keep: test_progress_bar_core_behavior()

// Redundant context bar snapshots
❌ test_context_bar_render_snapshot_25_percent()
❌ test_context_bar_render_snapshot_60_percent()  
❌ test_context_bar_render_snapshot_85_percent()
❌ test_context_bar_render_snapshot_100_percent()
❌ test_context_bar_compact_snapshot()
// Keep: test_context_bar_thresholds() - test key thresholds in one test
```

**Estimated Reduction: ~800 tests → ~150 tests (81% reduction)**
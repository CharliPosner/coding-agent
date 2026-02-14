//! Integration tests for resource error handling with alternative suggestions.

use coding_agent_cli::tools::{ErrorCategory, ToolError, ToolExecutionResult};
use std::time::Duration;

#[test]
fn test_resource_error_disk_full_alternatives() {
    // Create a tool error for disk full
    let error = ToolError::with_category(
        "No space left on device (ENOSPC)",
        ErrorCategory::Resource {
            resource_type: "disk_full".to_string(),
        },
    );

    // Verify error is categorized correctly
    assert!(matches!(
        error.category,
        ErrorCategory::Resource {
            resource_type: ref t
        } if t == "disk_full"
    ));

    // Verify it's not retriable
    assert!(!error.retriable);
}

#[test]
fn test_resource_error_out_of_memory_alternatives() {
    // Create a tool error for out of memory
    let error = ToolError::with_category(
        "Out of memory: cannot allocate",
        ErrorCategory::Resource {
            resource_type: "out_of_memory".to_string(),
        },
    );

    // Verify error is categorized correctly
    assert!(matches!(
        error.category,
        ErrorCategory::Resource {
            resource_type: ref t
        } if t == "out_of_memory"
    ));

    // Verify it's not retriable
    assert!(!error.retriable);
}

#[test]
fn test_resource_error_not_found_alternatives() {
    // Create a tool error for file not found
    let error = ToolError::with_category(
        "No such file or directory: '/tmp/missing.txt'",
        ErrorCategory::Resource {
            resource_type: "not_found".to_string(),
        },
    );

    // Verify error is categorized correctly
    assert!(matches!(
        error.category,
        ErrorCategory::Resource {
            resource_type: ref t
        } if t == "not_found"
    ));

    // Verify it's not retriable
    assert!(!error.retriable);
}

#[test]
fn test_resource_error_tool_not_found_alternatives() {
    // Create a tool error for tool not found
    let error = ToolError::with_category(
        "Unknown tool: nonexistent_tool",
        ErrorCategory::Resource {
            resource_type: "tool_not_found".to_string(),
        },
    );

    // Verify error is categorized correctly
    assert!(matches!(
        error.category,
        ErrorCategory::Resource {
            resource_type: ref t
        } if t == "tool_not_found"
    ));

    // Verify it's not retriable
    assert!(!error.retriable);
}

#[test]
fn test_tool_execution_result_resource_error() {
    // Create a tool execution result with a resource error
    let result = ToolExecutionResult {
        tool_name: "write_file".to_string(),
        call_id: "test_123".to_string(),
        result: Err(ToolError::with_category(
            "No space left on device",
            ErrorCategory::Resource {
                resource_type: "disk_full".to_string(),
            },
        )),
        duration: Duration::from_millis(100),
        retries: 0,
    };

    // Verify result failed
    assert!(!result.is_success());

    // Verify it's not auto-fixable (resource errors need user intervention)
    assert!(!result.is_auto_fixable());

    // Verify the error is available
    let error = result.error().unwrap();
    assert!(error.message.contains("No space left"));
    assert!(matches!(error.category, ErrorCategory::Resource { .. }));
}

#[test]
fn test_resource_error_alternative_suggestions_not_retriable() {
    // Resource errors should not be automatically retriable
    let disk_full = ToolError::new("No space left on device (ENOSPC)");
    assert!(!disk_full.retriable);

    let oom = ToolError::new("Out of memory: cannot allocate");
    assert!(!oom.retriable);

    let not_found = ToolError::new("No such file or directory: '/tmp/test.txt'");
    assert!(!not_found.retriable);
}

#[test]
fn test_resource_error_has_suggested_fix() {
    // Resource errors should have suggested fixes
    let disk_full = ToolError::new("No space left on device (ENOSPC)");
    assert!(disk_full.suggested_fix.is_some());
    assert_eq!(
        disk_full.suggested_fix.as_deref(),
        Some("Free up disk space")
    );

    let oom = ToolError::new("Out of memory: cannot allocate");
    assert!(oom.suggested_fix.is_some());
    assert_eq!(
        oom.suggested_fix.as_deref(),
        Some("Reduce memory usage or increase available memory")
    );

    let not_found = ToolError::new("No such file or directory: '/tmp/test.txt'");
    assert!(not_found.suggested_fix.is_some());
    assert!(not_found
        .suggested_fix
        .as_ref()
        .unwrap()
        .contains("does not exist"));
}

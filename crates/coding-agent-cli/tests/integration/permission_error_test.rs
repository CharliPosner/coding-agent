//! Integration tests for permission error recovery
//!
//! Tests the flow where a tool fails with a permission error,
//! the system prompts the user, and handles the response appropriately.

use coding_agent_cli::permissions::{
    OperationType, PermissionChecker, PermissionDecision, PermissionResponse, TrustedPaths,
};
use coding_agent_cli::tools::{ErrorCategory, ToolError};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_permission_error_categorization() {
    // Test that permission errors are correctly categorized
    let error = ToolError::new("Permission denied: '/etc/passwd'");

    assert!(matches!(
        error.category,
        ErrorCategory::Permission { ref resource } if resource == "/etc/passwd"
    ));
    assert!(!error.retriable);
    assert!(!error.is_auto_fixable());
}

#[test]
fn test_permission_checker_records_decisions() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let test_file = temp_dir.path().join("test.txt");

    let trusted = TrustedPaths::new(&[]).expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, true);

    // Initially needs prompt
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt
    );

    // Record an "allowed" decision
    checker.record_decision(
        &test_file,
        OperationType::Write,
        PermissionDecision::Allowed,
    );

    // Should now be allowed
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::Allowed
    );
}

#[test]
fn test_permission_checker_denied_persists() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let test_file = temp_dir.path().join("test.txt");

    let trusted = TrustedPaths::new(&[]).expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, true);

    // Record a "denied" decision
    checker.record_decision(&test_file, OperationType::Write, PermissionDecision::Denied);

    // Should be denied
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::Denied
    );
}

#[test]
fn test_permission_response_variants() {
    // Verify all permission response variants exist
    let _yes = PermissionResponse::Yes;
    let _no = PermissionResponse::No;
    let _always = PermissionResponse::Always;
    let _never = PermissionResponse::Never;

    // Verify they're distinct
    assert_ne!(
        PermissionResponse::Yes as i32,
        PermissionResponse::No as i32
    );
    assert_ne!(
        PermissionResponse::Always as i32,
        PermissionResponse::Never as i32
    );
}

#[test]
fn test_operation_types() {
    // Verify operation types are distinct
    assert_ne!(OperationType::Read, OperationType::Write);
    assert_ne!(OperationType::Write, OperationType::Modify);
    assert_ne!(OperationType::Modify, OperationType::Delete);
}

#[test]
fn test_permission_error_with_path_extraction() {
    let error1 = ToolError::new("Permission denied: '/tmp/test.txt'");
    match &error1.category {
        ErrorCategory::Permission { resource } => {
            assert_eq!(resource, "/tmp/test.txt");
        }
        _ => panic!("Expected Permission error category"),
    }

    let error2 = ToolError::new("Access denied for file '/home/user/file.txt'");
    match &error2.category {
        ErrorCategory::Permission { resource } => {
            assert!(
                resource.contains("/home/user/file.txt")
                    || resource.contains("'/home/user/file.txt'")
            );
        }
        _ => panic!("Expected Permission error category"),
    }
}

#[test]
fn test_permission_error_suggested_fix() {
    let error = ToolError::new("Permission denied: '/etc/shadow'");

    assert!(error.suggested_fix.is_some());
    let suggestion = error.suggested_fix.unwrap();
    assert!(
        suggestion.contains("permission") || suggestion.contains("access"),
        "Suggestion should mention permissions or access"
    );
}

#[test]
fn test_permission_checker_different_operations() {
    // Verify that decisions are operation-specific
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let test_file = temp_dir.path().join("test.txt");

    let trusted = TrustedPaths::new(&[]).expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, true);

    // Allow write
    checker.record_decision(
        &test_file,
        OperationType::Write,
        PermissionDecision::Allowed,
    );

    // Write should be allowed
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::Allowed
    );

    // But delete should still need prompt (different operation)
    assert_eq!(
        checker.check(&test_file, OperationType::Delete),
        PermissionDecision::NeedsPrompt
    );
}

#[test]
fn test_always_adds_to_trusted_paths() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let test_dir = temp_dir.path().join("trusted_dir");
    std::fs::create_dir(&test_dir).expect("Should create dir");

    let trusted = TrustedPaths::new(&[]).expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, true);

    // Initially not trusted
    let test_file = test_dir.join("file.txt");
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt
    );

    // Add to trusted paths
    let result = checker.add_trusted_path(&test_dir);
    assert!(result.is_ok(), "Should successfully add trusted path");

    // Now should be allowed
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::Allowed
    );
}

#[test]
fn test_permission_error_vs_other_errors() {
    // Permission error
    let perm_error = ToolError::new("Permission denied: '/etc/passwd'");
    assert!(matches!(
        perm_error.category,
        ErrorCategory::Permission { .. }
    ));
    assert!(!perm_error.is_auto_fixable());

    // Code error (should be auto-fixable)
    let code_error = ToolError::new("cannot find crate `serde_json`");
    assert!(matches!(code_error.category, ErrorCategory::Code { .. }));
    assert!(code_error.is_auto_fixable());

    // Network error (should be retriable)
    let net_error = ToolError::new("Connection timed out");
    assert!(matches!(net_error.category, ErrorCategory::Network { .. }));
    assert!(!net_error.is_auto_fixable());
    assert!(net_error.retriable);
}

#[test]
fn test_permission_error_not_retriable() {
    // Permission errors should not be automatically retried
    let error = ToolError::new("Permission denied: '/etc/shadow'");

    assert!(
        !error.retriable,
        "Permission errors should not be retriable"
    );
    assert!(
        !error.is_auto_fixable(),
        "Permission errors should not be auto-fixable"
    );
}

#[test]
fn test_session_cache_isolation() {
    // Verify that session cache doesn't leak between paths
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");

    let trusted = TrustedPaths::new(&[]).expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, true);

    // Allow file1
    checker.record_decision(&file1, OperationType::Write, PermissionDecision::Allowed);

    // file1 should be allowed
    assert_eq!(
        checker.check(&file1, OperationType::Write),
        PermissionDecision::Allowed
    );

    // file2 should still need prompt (different file)
    assert_eq!(
        checker.check(&file2, OperationType::Write),
        PermissionDecision::NeedsPrompt
    );
}

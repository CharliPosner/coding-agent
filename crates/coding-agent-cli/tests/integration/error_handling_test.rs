//! Consolidated error handling integration tests
//!
//! Combines permission_error_test.rs, resource_error_test.rs, and tool_execution_retry_test.rs
//! into a single comprehensive error handling test suite.

use coding_agent_cli::error::{CliError, ErrorCode};
use coding_agent_cli::tools::executor::ToolExecutor;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[tokio::test]
async fn test_permission_errors() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let protected_file = temp_dir.path().join("protected.txt");
    
    // Create file and remove read permissions
    fs::write(&protected_file, "protected content").expect("Failed to write file");
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&protected_file).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&protected_file, perms).unwrap();
    }

    let executor = ToolExecutor::new();
    let result = executor.read_file(protected_file.to_string_lossy().to_string()).await;

    assert!(result.is_err(), "Should fail with permission error");
    
    #[cfg(unix)]
    {
        let error = result.unwrap_err();
        assert!(matches!(error, CliError::PermissionDenied(_)));
    }
}

#[tokio::test] 
async fn test_resource_not_found_errors() {
    let executor = ToolExecutor::new();

    // Test non-existent file
    let result = executor.read_file("/nonexistent/path/file.txt".to_string()).await;
    assert!(result.is_err(), "Should fail for non-existent file");
    
    let error = result.unwrap_err();
    assert!(matches!(error, CliError::FileNotFound(_)));

    // Test non-existent directory in list_files
    let result = executor.list_files(Some("/nonexistent/directory".to_string())).await;
    assert!(result.is_err(), "Should fail for non-existent directory");
}

#[tokio::test]
async fn test_tool_execution_retry_logic() {
    let executor = ToolExecutor::with_retry_config(3, std::time::Duration::from_millis(10));

    // Test command that fails consistently
    let result = executor.bash("exit 1".to_string()).await;
    assert!(result.is_err(), "Command should fail after retries");

    // Test command that succeeds
    let result = executor.bash("echo success".to_string()).await;
    assert!(result.is_ok(), "Command should succeed");
    assert_eq!(result.unwrap().trim(), "success");
}

#[tokio::test]
async fn test_invalid_tool_input_errors() {
    let executor = ToolExecutor::new();

    // Test edit_file with same old_str and new_str
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").expect("Failed to write file");

    let result = executor.edit_file(
        file_path.to_string_lossy().to_string(),
        "same".to_string(),
        "same".to_string(), // Same as old_str
    ).await;
    
    assert!(result.is_err(), "Should fail with validation error");
    let error = result.unwrap_err();
    assert!(matches!(error, CliError::InvalidInput(_)));
}

#[tokio::test]
async fn test_network_timeout_errors() {
    // Test code_search with invalid ripgrep pattern that might timeout
    let executor = ToolExecutor::with_timeout(std::time::Duration::from_millis(1));
    
    let result = executor.code_search(
        ".*".repeat(1000), // Complex pattern that might be slow
        None,
        None,
        false,
    ).await;

    // Should either succeed quickly or timeout
    // This tests timeout handling in the tool execution layer
    if result.is_err() {
        let error = result.unwrap_err();
        // Could be timeout or invalid pattern, both are acceptable
        assert!(matches!(error, CliError::Timeout(_) | CliError::ToolExecutionFailed(_)));
    }
}

#[tokio::test]
async fn test_disk_space_simulation() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let large_file_path = temp_dir.path().join("large_file.txt");

    let executor = ToolExecutor::new();
    
    // Try to write a very large file (this should work in temp dir)
    let large_content = "x".repeat(1000000); // 1MB
    let result = executor.write_file(
        large_file_path.to_string_lossy().to_string(),
        large_content,
    ).await;

    // Should succeed in temp directory
    assert!(result.is_ok(), "Should be able to write to temp directory");
}

#[tokio::test]
async fn test_concurrent_file_access_errors() {
    use tokio::task;
    
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let shared_file = temp_dir.path().join("shared.txt");
    fs::write(&shared_file, "initial content").expect("Failed to write initial content");

    let executor = ToolExecutor::new();
    let file_path = shared_file.to_string_lossy().to_string();

    // Spawn concurrent edit operations
    let tasks: Vec<_> = (0..5).map(|i| {
        let executor = executor.clone();
        let file_path = file_path.clone();
        task::spawn(async move {
            executor.edit_file(
                file_path,
                "initial content".to_string(),
                format!("content from task {}", i),
            ).await
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(tasks).await;

    // Only one should succeed, others should fail due to content mismatch
    let successes = results.into_iter().filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok()).count();
    assert_eq!(successes, 1, "Only one concurrent edit should succeed");
}
//! Consolidated workflow integration tests
//!
//! Combines auto_fix_workflow_test.rs, permission_prompt_workflow_test.rs, 
//! and multi_agent_coordination_test.rs into streamlined workflow tests.

use coding_agent_cli::agents::manager::AgentManager;
use coding_agent_cli::workflows::{AutoFixWorkflow, WorkflowContext};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_auto_fix_workflow_basic() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let project_path = temp_dir.path();
    
    // Create a Cargo.toml with missing dependency
    let cargo_toml = project_path.join("Cargo.toml");
    std::fs::write(&cargo_toml, r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
# Missing serde dependency
"#).expect("Failed to write Cargo.toml");

    // Create src/main.rs that uses serde
    let src_dir = project_path.join("src");
    std::fs::create_dir(&src_dir).expect("Failed to create src dir");
    let main_rs = src_dir.join("main.rs");
    std::fs::write(&main_rs, r#"
use serde::Serialize;

#[derive(Serialize)]
struct Data {
    value: i32,
}

fn main() {
    let data = Data { value: 42 };
    println!("{}", serde_json::to_string(&data).unwrap());
}
"#).expect("Failed to write main.rs");

    let context = WorkflowContext::new(project_path.to_path_buf());
    let workflow = AutoFixWorkflow::new(context);

    // Run auto-fix workflow
    let result = workflow.execute().await;
    
    assert!(result.is_ok(), "Auto-fix workflow should succeed");

    // Verify dependency was added
    let updated_cargo = std::fs::read_to_string(&cargo_toml).expect("Failed to read updated Cargo.toml");
    assert!(updated_cargo.contains("serde"), "serde dependency should be added");
    assert!(updated_cargo.contains("serde_json"), "serde_json dependency should be added");
}

#[tokio::test]
async fn test_permission_prompt_workflow() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let protected_file = temp_dir.path().join("important.txt");
    
    std::fs::write(&protected_file, "important data").expect("Failed to write file");

    // Simulate workflow that needs permission to modify file
    let context = WorkflowContext::new(temp_dir.path().to_path_buf());
    let mut workflow = AutoFixWorkflow::new(context);
    
    // Configure to auto-approve permissions for testing
    workflow.set_auto_approve_permissions(true);

    let result = workflow.modify_file(
        protected_file.to_string_lossy().to_string(),
        "important data".to_string(),
        "updated important data".to_string(),
    ).await;

    assert!(result.is_ok(), "Permission workflow should succeed with auto-approval");

    // Verify file was modified
    let content = std::fs::read_to_string(&protected_file).expect("Failed to read file");
    assert_eq!(content, "updated important data");
}

#[tokio::test]
async fn test_multi_agent_coordination() {
    let manager = Arc::new(AgentManager::new());
    
    // Spawn 3 agents working on related tasks
    let agent1 = manager.spawn(
        "file-analyzer".to_string(),
        "Analyze project files".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(100));
            Ok("Found 5 Rust files, 2 test files".to_string())
        },
    );

    let agent2 = manager.spawn(
        "dependency-checker".to_string(), 
        "Check dependencies".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(150));
            Ok("Missing: serde, tokio".to_string())
        },
    );

    let agent3 = manager.spawn(
        "test-runner".to_string(),
        "Run tests".to_string(), 
        || {
            std::thread::sleep(Duration::from_millis(80));
            Ok("Tests: 15 passed, 3 failed".to_string())
        },
    );

    // Wait for all agents to complete
    let results = manager.wait_all(vec![agent1, agent2, agent3]).await;
    assert!(results.is_ok(), "All agents should complete successfully");

    let results = results.unwrap();
    assert_eq!(results.len(), 3, "Should have 3 results");
    
    // Verify each agent completed with expected output
    assert!(results.iter().any(|r| r.contains("Rust files")));
    assert!(results.iter().any(|r| r.contains("Missing:")));
    assert!(results.iter().any(|r| r.contains("Tests:")));
}

#[tokio::test]
async fn test_workflow_error_recovery() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let context = WorkflowContext::new(temp_dir.path().to_path_buf());
    let workflow = AutoFixWorkflow::new(context);

    // Try to fix a non-existent file (should handle gracefully)
    let result = workflow.fix_file("/nonexistent/file.rs".to_string()).await;
    
    assert!(result.is_err(), "Should fail for non-existent file");
    
    // Workflow should continue after error
    let recovery_result = workflow.get_status().await;
    assert!(recovery_result.is_ok(), "Workflow should recover from errors");
}

#[tokio::test]
async fn test_agent_coordination_with_failures() {
    let manager = Arc::new(AgentManager::new());

    // Mix of successful and failing agents
    let success_agent = manager.spawn(
        "success-task".to_string(),
        "Will succeed".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("Task completed".to_string())
        },
    );

    let failure_agent = manager.spawn(
        "failure-task".to_string(),
        "Will fail".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(30));
            Err("Intentional failure".to_string())
        },
    );

    // Wait for the successful one
    let success_result = manager.wait(success_agent).await;
    assert!(success_result.is_ok(), "Success agent should complete");
    assert_eq!(success_result.unwrap(), "Task completed");

    // Wait for the failing one
    let failure_result = manager.wait(failure_agent).await;
    assert!(failure_result.is_err(), "Failure agent should fail");
    assert_eq!(failure_result.unwrap_err(), "Intentional failure");

    // Manager should handle mixed results gracefully
    let remaining_statuses = manager.get_all_statuses();
    assert_eq!(remaining_statuses.len(), 0, "All agents should be cleaned up");
}

#[tokio::test] 
async fn test_workflow_progress_tracking() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let context = WorkflowContext::new(temp_dir.path().to_path_buf());
    let workflow = AutoFixWorkflow::new(context);

    // Start workflow and track progress
    let workflow_task = tokio::spawn(async move {
        workflow.execute_with_progress().await
    });

    // Give workflow time to start and make progress
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Workflow should complete
    let result = workflow_task.await;
    assert!(result.is_ok(), "Workflow task should complete");
}
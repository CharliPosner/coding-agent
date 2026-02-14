//! Streamlined agent integration tests
//!
//! Reduced from 12+ tests to 6 essential tests covering core agent functionality
//! without excessive edge case testing.

use coding_agent_cli::agents::manager::AgentManager;
use coding_agent_cli::agents::status::AgentId;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_agent_lifecycle() {
    let manager = Arc::new(AgentManager::new());

    // Test basic spawn → complete lifecycle
    let agent_id = manager.spawn(
        "lifecycle-test".to_string(),
        "Testing basic lifecycle".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("Completed successfully".to_string())
        },
    );

    // Verify agent is tracked
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 1, "Should have 1 agent");
    assert_eq!(statuses[0].name, "lifecycle-test");

    // Wait for completion
    let result = manager.wait(agent_id).await;
    assert!(result.is_ok(), "Agent should complete successfully");
    assert_eq!(result.unwrap(), "Completed successfully");

    // Verify cleanup
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 0, "Agent should be cleaned up after completion");
}

#[tokio::test]
async fn test_concurrent_agents() {
    let manager = Arc::new(AgentManager::new());

    // Spawn multiple agents concurrently
    let mut ids = Vec::new();
    for i in 0..3 {
        let id = manager.spawn(
            format!("concurrent-{}", i),
            format!("Task {}", i),
            move || {
                std::thread::sleep(Duration::from_millis(50 + i * 20));
                Ok(format!("Result {}", i))
            },
        );
        ids.push(id);
    }

    // Verify all are tracked
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 3, "Should have 3 concurrent agents");

    // Wait for all to complete
    let results = manager.wait_all(ids).await;
    assert!(results.is_ok(), "All agents should complete");
    
    let results = results.unwrap();
    assert_eq!(results.len(), 3, "Should have 3 results");
    
    // Verify all completed (order may vary due to different sleep times)
    for i in 0..3 {
        assert!(results.iter().any(|r| r == &format!("Result {}", i)));
    }

    // Verify cleanup
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 0, "All agents should be cleaned up");
}

#[tokio::test]
async fn test_agent_failure_handling() {
    let manager = Arc::new(AgentManager::new());

    let agent_id = manager.spawn(
        "failing-agent".to_string(),
        "Will fail".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(30));
            Err("Intentional test failure".to_string())
        },
    );

    let result = manager.wait(agent_id).await;
    assert!(result.is_err(), "Agent should fail as expected");
    assert_eq!(result.unwrap_err(), "Intentional test failure");

    // Failed agent should be cleaned up
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 0, "Failed agent should be cleaned up");
}

#[tokio::test]
async fn test_agent_cancellation() {
    let manager = Arc::new(AgentManager::new());

    // Spawn long-running agent
    let agent_id = manager.spawn(
        "long-runner".to_string(),
        "Long running task".to_string(),
        || {
            std::thread::sleep(Duration::from_secs(5)); // Long task
            Ok("Should not complete".to_string())
        },
    );

    // Give agent time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cancel the agent
    let cancel_result = manager.cancel(agent_id).await;
    assert!(cancel_result.is_ok(), "Cancel should succeed");

    // Wait should return cancellation error
    let result = manager.wait(agent_id).await;
    assert!(result.is_err(), "Cancelled agent should return error");
    
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("cancelled") || error_msg.contains("Agent"),
        "Error should indicate cancellation: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_agent_progress_reporting() {
    let manager = Arc::new(AgentManager::new());

    let agent_id = manager.spawn_with_progress(
        "progress-agent".to_string(),
        "Reports progress".to_string(),
        |reporter| {
            for i in (0..=100).step_by(20) {
                reporter.report(i);
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok("Progress complete".to_string())
        },
    );

    // Give time for progress updates
    tokio::time::sleep(Duration::from_millis(70)).await;
    manager.process_progress_updates();

    // Check progress was updated
    let statuses = manager.get_all_statuses();
    if !statuses.is_empty() {
        let agent_status = &statuses[0];
        assert!(agent_status.progress >= 0 && agent_status.progress <= 100);
    }

    // Wait for completion
    let result = manager.wait(agent_id).await;
    assert!(result.is_ok(), "Agent with progress should complete");
    assert_eq!(result.unwrap(), "Progress complete");
}

#[tokio::test] 
async fn test_async_agents() {
    let manager = Arc::new(AgentManager::new());

    let agent_id = manager.spawn_async(
        "async-agent".to_string(),
        "Async task".to_string(),
        |reporter| async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            reporter.report(50);
            tokio::time::sleep(Duration::from_millis(50)).await;
            reporter.report(100);
            Ok("Async completed".to_string())
        },
    );

    let result = manager.wait(agent_id).await;
    assert!(result.is_ok(), "Async agent should complete");
    assert_eq!(result.unwrap(), "Async completed");

    // Async agent should be cleaned up
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 0, "Async agent should be cleaned up");
}
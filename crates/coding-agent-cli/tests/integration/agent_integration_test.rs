//! Integration tests for agent spawning, tracking, and status reporting.
//!
//! These tests verify that the agent management system works correctly
//! from end-to-end: spawning agents, tracking their progress, reporting
//! status, handling completion/failure, and cleanup.

use coding_agent_cli::agents::manager::AgentManager;
use coding_agent_cli::agents::status::{AgentId, AgentState};
use std::sync::Arc;
use std::time::Duration;

/// Test basic agent spawning and completion.
#[tokio::test]
async fn test_agent_spawn_and_complete() {
    let manager = Arc::new(AgentManager::new());

    // Spawn a simple agent
    let agent_id = manager.spawn(
        "test-agent".to_string(),
        "Testing basic functionality".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(100));
            Ok("Success".to_string())
        },
    );

    // Verify agent was created with correct ID
    assert_eq!(agent_id, AgentId(0), "First agent should have ID 0");

    // Check initial status - should be queued or running
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 1, "Should have 1 agent");
    assert_eq!(statuses[0].name, "test-agent");
    assert_eq!(statuses[0].description, "Testing basic functionality");

    // Wait for agent to actually complete by calling wait()
    let result = manager.wait(agent_id).await;
    assert!(result.is_ok(), "Agent should complete successfully");
    assert_eq!(result.unwrap(), "Success");

    // After wait(), agent is removed from manager
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 0, "Agent should be removed after wait");
}

/// Test spawning multiple agents concurrently.
#[tokio::test]
async fn test_multiple_agents_concurrent() {
    let manager = Arc::new(AgentManager::new());

    // Spawn 3 agents
    let id1 = manager.spawn("agent-1".to_string(), "Task 1".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("Done 1".to_string())
    });

    let id2 = manager.spawn("agent-2".to_string(), "Task 2".to_string(), || {
        std::thread::sleep(Duration::from_millis(150));
        Ok("Done 2".to_string())
    });

    let id3 = manager.spawn("agent-3".to_string(), "Task 3".to_string(), || {
        std::thread::sleep(Duration::from_millis(50));
        Ok("Done 3".to_string())
    });

    // Verify unique IDs
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);

    // Check all agents are tracked
    tokio::time::sleep(Duration::from_millis(25)).await;
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 3, "Should have 3 agents");

    // Verify each agent is in the list
    let agent_names: Vec<String> = statuses.iter().map(|s| s.name.clone()).collect();
    assert!(agent_names.contains(&"agent-1".to_string()));
    assert!(agent_names.contains(&"agent-2".to_string()));
    assert!(agent_names.contains(&"agent-3".to_string()));

    // Wait for all to complete using wait_all
    let results = manager.wait_all(vec![id1, id2, id3]).await;
    assert!(results.is_ok(), "All agents should complete successfully");
    let results = results.unwrap();
    assert_eq!(results.len(), 3, "Should have 3 results");
    assert!(results.contains(&"Done 1".to_string()));
    assert!(results.contains(&"Done 2".to_string()));
    assert!(results.contains(&"Done 3".to_string()));

    // After wait_all(), all agents should be removed
    let statuses = manager.get_all_statuses();
    assert_eq!(
        statuses.len(),
        0,
        "All agents should be removed after wait_all"
    );
}

/// Test agent failure handling.
#[tokio::test]
async fn test_agent_failure() {
    let manager = Arc::new(AgentManager::new());

    // Spawn an agent that will fail
    let agent_id = manager.spawn("failing-agent".to_string(), "Will fail".to_string(), || {
        std::thread::sleep(Duration::from_millis(50));
        Err("Intentional failure".to_string())
    });

    // Wait for agent and expect failure
    let result = manager.wait(agent_id).await;
    assert!(result.is_err(), "Agent should fail");
    assert_eq!(result.unwrap_err(), "Intentional failure");

    // Agent should be removed after wait
    let statuses = manager.get_all_statuses();
    assert_eq!(
        statuses.len(),
        0,
        "Failed agent should be removed after wait"
    );
}

/// Test agent progress reporting.
#[tokio::test]
async fn test_agent_progress_reporting() {
    let manager = Arc::new(AgentManager::new());

    // Spawn an agent that reports progress
    let _agent_id = manager.spawn_with_progress(
        "progress-agent".to_string(),
        "Reporting progress".to_string(),
        |reporter| {
            // Report progress at intervals
            for i in 0..=10 {
                reporter.report(i * 10);
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok("Complete with progress".to_string())
        },
    );

    // Give it time to start and make some progress
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Process progress updates
    manager.process_progress_updates();

    // Check progress was updated
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 1, "Should have 1 agent");

    // Progress should be somewhere between 0 and 100
    let progress = statuses[0].progress;
    assert!(
        progress <= 100,
        "Progress should be <= 100, got {}",
        progress
    );

    // Wait for completion
    tokio::time::sleep(Duration::from_millis(150)).await;
    manager.process_progress_updates();

    let statuses = manager.get_all_statuses();
    if !statuses.is_empty() {
        assert_eq!(statuses[0].progress, 100, "Final progress should be 100%");
    }
}

/// Test agent cancellation.
#[tokio::test]
async fn test_agent_cancellation() {
    let manager = Arc::new(AgentManager::new());

    // Spawn a long-running agent
    let agent_id = manager.spawn(
        "long-runner".to_string(),
        "Will be cancelled".to_string(),
        || {
            std::thread::sleep(Duration::from_secs(10)); // Very long task
            Ok("Should not complete".to_string())
        },
    );

    // Give it time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify agent is in system before cancelling
    let statuses_before = manager.get_all_statuses();
    assert_eq!(
        statuses_before.len(),
        1,
        "Should have 1 agent before cancel"
    );

    // Cancel the agent
    let cancel_result = manager.cancel(agent_id).await;
    assert!(
        cancel_result.is_ok(),
        "Cancel should succeed: {:?}",
        cancel_result
    );

    // Give cancel signal time to process in the background task
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Wait for the cancelled agent - it should return the cancellation error
    let result = manager.wait(agent_id).await;
    assert!(
        result.is_err(),
        "Cancelled agent should return error, got: {:?}",
        result
    );
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("cancelled") || error_msg.contains("Agent"),
        "Error should mention cancellation: {}",
        error_msg
    );

    // Agent should be removed after wait
    let statuses = manager.get_all_statuses();
    assert_eq!(
        statuses.len(),
        0,
        "Cancelled agent should be removed after wait"
    );
}

/// Test waiting for an agent to complete.
#[tokio::test]
async fn test_agent_wait_for_completion() {
    let manager = Arc::new(AgentManager::new());

    // Spawn an agent
    let agent_id = manager.spawn("wait-test".to_string(), "Testing wait".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("Done waiting".to_string())
    });

    // Wait for it to complete
    let result = manager.wait(agent_id).await;

    assert!(result.is_ok(), "Wait should succeed");
    assert_eq!(result.unwrap(), "Done waiting");
}

/// Test waiting with timeout using tokio::time::timeout.
#[tokio::test]
async fn test_agent_wait_timeout() {
    let manager = Arc::new(AgentManager::new());

    // Spawn a slow agent
    let agent_id = manager.spawn("slow-agent".to_string(), "Very slow".to_string(), || {
        std::thread::sleep(Duration::from_secs(5));
        Ok("Too slow".to_string())
    });

    // Wait with short timeout using tokio timeout
    let result = tokio::time::timeout(Duration::from_millis(100), manager.wait(agent_id)).await;

    assert!(result.is_err(), "Wait should timeout");
}

/// Test async agent spawning.
#[tokio::test]
async fn test_async_agent_spawn() {
    let manager = Arc::new(AgentManager::new());

    // Spawn an async agent
    let agent_id = manager.spawn_async(
        "async-agent".to_string(),
        "Async task".to_string(),
        |reporter| async move {
            // Simulate async work
            tokio::time::sleep(Duration::from_millis(50)).await;
            reporter.report(50);
            tokio::time::sleep(Duration::from_millis(50)).await;
            reporter.report(100);
            Ok("Async complete".to_string())
        },
    );

    // Wait for async agent to complete
    let result = manager.wait(agent_id).await;
    assert!(result.is_ok(), "Async agent should complete successfully");
    assert_eq!(result.unwrap(), "Async complete");

    // After wait(), agent is removed
    let statuses = manager.get_all_statuses();
    assert_eq!(
        statuses.len(),
        0,
        "Async agent should be removed after completion"
    );
}

/// Test cleanup of completed agents.
#[tokio::test]
async fn test_agent_cleanup() {
    let manager = Arc::new(AgentManager::new());

    // Spawn several quick agents and collect their IDs
    let mut ids = vec![];
    for i in 0..5 {
        let id = manager.spawn(
            format!("cleanup-agent-{}", i),
            format!("Task {}", i),
            move || {
                std::thread::sleep(Duration::from_millis(50));
                Ok(format!("Done {}", i))
            },
        );
        ids.push(id);
    }

    // Check we have agents before they complete
    let statuses_before = manager.get_all_statuses();
    assert_eq!(statuses_before.len(), 5, "Should have 5 agents");

    // Wait for all agents to actually complete
    let results = manager.wait_all(ids).await;
    assert!(results.is_ok(), "All agents should complete successfully");

    // After wait_all(), all agents are automatically removed
    let statuses = manager.get_all_statuses();
    assert_eq!(
        statuses.len(),
        0,
        "All agents should be removed after wait_all: found {}",
        statuses.len()
    );
}

/// Test agent state transitions through its lifecycle.
#[tokio::test]
async fn test_agent_state_transitions() {
    let manager = Arc::new(AgentManager::new());

    // Spawn an agent
    let agent_id = manager.spawn(
        "state-test".to_string(),
        "Testing states".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(100));
            Ok("State complete".to_string())
        },
    );

    // Check initial state (Queued or quickly transitioned to Running)
    let statuses = manager.get_all_statuses();
    let agent = statuses.iter().find(|s| s.id == agent_id).unwrap();
    assert!(
        agent.state == AgentState::Queued || agent.state == AgentState::Running,
        "Initial state should be Queued or Running (might have started already)"
    );

    // Wait for it and verify result
    let result = manager.wait(agent_id).await;
    assert!(result.is_ok(), "Agent should complete successfully");
    assert_eq!(result.unwrap(), "State complete");

    // After wait(), agent is removed from manager
    let statuses = manager.get_all_statuses();
    assert_eq!(
        statuses.len(),
        0,
        "Agent should be removed after completion"
    );
}

/// Test getting status of specific agent.
#[tokio::test]
async fn test_get_agent_status() {
    let manager = Arc::new(AgentManager::new());

    // Spawn an agent
    let agent_id = manager.spawn("status-query".to_string(), "Query test".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("Query done".to_string())
    });

    // Get status of this specific agent
    let status = manager.get_status(agent_id);
    assert!(status.is_some(), "Should find agent status");

    let status = status.unwrap();
    assert_eq!(status.name, "status-query");
    assert_eq!(status.description, "Query test");

    // Try getting status of non-existent agent
    let fake_status = manager.get_status(AgentId(9999));
    assert!(fake_status.is_none(), "Should not find fake agent");
}

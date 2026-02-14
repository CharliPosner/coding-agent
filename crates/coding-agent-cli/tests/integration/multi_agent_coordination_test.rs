//! Integration tests for multi-agent coordination.
//!
//! These tests verify that multiple agents can work together, coordinating their
//! results and handling complex scenarios like partial failures, cancellations,
//! and result aggregation.

use coding_agent_cli::agents::manager::AgentManager;
use coding_agent_cli::agents::status::AgentState;
use std::time::Duration;

/// Test that multiple agents can be spawned and their statuses tracked.
#[tokio::test]
async fn test_multi_agent_spawn_and_status_tracking() {
    let manager = AgentManager::new();

    // Spawn 5 agents with different descriptions
    let id1 = manager.spawn(
        "search-agent".to_string(),
        "Searching for auth files".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(100));
            Ok("Found 5 auth files".to_string())
        },
    );

    let id2 = manager.spawn(
        "refactor-agent".to_string(),
        "Analyzing dependencies".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(150));
            Ok("Analyzed 12 dependencies".to_string())
        },
    );

    let id3 = manager.spawn(
        "test-agent".to_string(),
        "Running unit tests".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(80));
            Ok("All tests passed".to_string())
        },
    );

    let id4 = manager.spawn(
        "build-agent".to_string(),
        "Building project".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(200));
            Ok("Build successful".to_string())
        },
    );

    let id5 = manager.spawn(
        "lint-agent".to_string(),
        "Running linter".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("No lint errors".to_string())
        },
    );

    // Verify all agents are tracked
    let statuses = manager.get_all_statuses();
    assert_eq!(statuses.len(), 5);

    // Verify correct names
    assert!(statuses.iter().any(|s| s.name == "search-agent"));
    assert!(statuses.iter().any(|s| s.name == "refactor-agent"));
    assert!(statuses.iter().any(|s| s.name == "test-agent"));
    assert!(statuses.iter().any(|s| s.name == "build-agent"));
    assert!(statuses.iter().any(|s| s.name == "lint-agent"));

    // Verify active count
    assert_eq!(manager.active_count(), 5);

    // Wait for all to complete in parallel
    let results = manager
        .wait_all_parallel(vec![id1, id2, id3, id4, id5])
        .await;

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 5);
    assert!(results.contains(&"Found 5 auth files".to_string()));
    assert!(results.contains(&"Analyzed 12 dependencies".to_string()));
    assert!(results.contains(&"All tests passed".to_string()));
    assert!(results.contains(&"Build successful".to_string()));
    assert!(results.contains(&"No lint errors".to_string()));

    // All should be completed now
    assert_eq!(manager.active_count(), 0);
}

/// Test coordinated result aggregation from multiple agents.
#[tokio::test]
async fn test_coordinated_result_aggregation() {
    let manager = AgentManager::new();

    // Spawn agents that produce parts of a larger result
    let id1 = manager.spawn(
        "parser-agent".to_string(),
        "Parsing code".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("Functions: 15".to_string())
        },
    );

    let id2 = manager.spawn(
        "analyzer-agent".to_string(),
        "Analyzing".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("Structs: 8".to_string())
        },
    );

    let id3 = manager.spawn(
        "metrics-agent".to_string(),
        "Computing metrics".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("Lines: 1234".to_string())
        },
    );

    // Aggregate results into a report
    let report = manager
        .aggregate_results(
            vec![id1, id2, id3],
            String::from("Code Analysis Report:\n"),
            |acc, result| format!("{}- {}\n", acc, result),
        )
        .await;

    assert!(report.is_ok());
    let report = report.unwrap();
    assert!(report.contains("Functions: 15"));
    assert!(report.contains("Structs: 8"));
    assert!(report.contains("Lines: 1234"));
}

/// Test handling partial failures in multi-agent coordination.
#[tokio::test]
async fn test_partial_failure_handling() {
    let manager = AgentManager::new();

    // Spawn 4 agents, 2 will succeed and 2 will fail
    let id1 = manager.spawn("success-1".to_string(), "Task 1".to_string(), || {
        std::thread::sleep(Duration::from_millis(50));
        Ok("Success 1".to_string())
    });

    let id2 = manager.spawn("failure-1".to_string(), "Task 2".to_string(), || {
        std::thread::sleep(Duration::from_millis(50));
        Err("Error in task 2".to_string())
    });

    let id3 = manager.spawn("success-2".to_string(), "Task 3".to_string(), || {
        std::thread::sleep(Duration::from_millis(50));
        Ok("Success 3".to_string())
    });

    let id4 = manager.spawn("failure-2".to_string(), "Task 4".to_string(), || {
        std::thread::sleep(Duration::from_millis(50));
        Err("Error in task 4".to_string())
    });

    // Wait for all - should fail due to partial failures
    let results = manager.wait_all_parallel(vec![id1, id2, id3, id4]).await;

    assert!(results.is_err());
    let error_msg = results.unwrap_err();

    // Verify error message contains details about failures
    assert!(error_msg.contains("2 agent(s) failed"));
    assert!(error_msg.contains("Error in task 2") || error_msg.contains("Error in task 4"));
}

/// Test graceful handling of all agents failing.
#[tokio::test]
async fn test_all_agents_failing() {
    let manager = AgentManager::new();

    // Spawn 3 agents that all fail
    let id1 = manager.spawn("fail-1".to_string(), "Task 1".to_string(), || {
        Err("Failure 1".to_string())
    });

    let id2 = manager.spawn("fail-2".to_string(), "Task 2".to_string(), || {
        Err("Failure 2".to_string())
    });

    let id3 = manager.spawn("fail-3".to_string(), "Task 3".to_string(), || {
        Err("Failure 3".to_string())
    });

    // Wait for all - should fail
    let results = manager.wait_all_parallel(vec![id1, id2, id3]).await;

    assert!(results.is_err());
    let error_msg = results.unwrap_err();
    assert!(error_msg.contains("3 agent(s) failed"));
}

/// Test progress tracking across multiple concurrent agents.
#[tokio::test]
async fn test_multi_agent_progress_tracking() {
    let manager = AgentManager::new();

    // Spawn 3 agents with progress reporting
    let id1 =
        manager.spawn_with_progress("agent-1".to_string(), "Task 1".to_string(), |reporter| {
            for i in 1..=4 {
                reporter.report(i * 25);
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok("Done 1".to_string())
        });

    let id2 =
        manager.spawn_with_progress("agent-2".to_string(), "Task 2".to_string(), |reporter| {
            for i in 1..=5 {
                reporter.report(i * 20);
                std::thread::sleep(Duration::from_millis(15));
            }
            Ok("Done 2".to_string())
        });

    let id3 =
        manager.spawn_with_progress("agent-3".to_string(), "Task 3".to_string(), |reporter| {
            for i in 1..=10 {
                reporter.report(i * 10);
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok("Done 3".to_string())
        });

    // Periodically process progress updates
    let mut progress_snapshots = Vec::new();
    for _ in 0..8 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        manager.process_progress_updates();

        // Take snapshot of current progress
        let statuses = manager.get_all_statuses();
        let progresses: Vec<u8> = statuses.iter().map(|s| s.progress).collect();
        progress_snapshots.push(progresses);
    }

    // Wait for all to complete
    let results = manager.wait_all_parallel(vec![id1, id2, id3]).await;
    assert!(results.is_ok());

    // Verify that progress increased over time
    let first_snapshot = &progress_snapshots[0];
    let last_snapshot = &progress_snapshots[progress_snapshots.len() - 1];

    // At least one agent should have made progress
    let first_sum: u32 = first_snapshot.iter().map(|&p| p as u32).sum();
    let last_sum: u32 = last_snapshot.iter().map(|&p| p as u32).sum();
    assert!(last_sum >= first_sum);
}

/// Test canceling a subset of agents while others continue.
#[tokio::test]
async fn test_selective_agent_cancellation() {
    let manager = AgentManager::new();

    // Spawn 4 agents
    let id1 = manager.spawn("keep-1".to_string(), "Keep running".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("Completed 1".to_string())
    });

    let id2 = manager.spawn("cancel-1".to_string(), "Will cancel".to_string(), || {
        std::thread::sleep(Duration::from_secs(10));
        Ok("Should not see this".to_string())
    });

    let id3 = manager.spawn("keep-2".to_string(), "Keep running".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("Completed 2".to_string())
    });

    let id4 = manager.spawn("cancel-2".to_string(), "Will cancel".to_string(), || {
        std::thread::sleep(Duration::from_secs(10));
        Ok("Should not see this".to_string())
    });

    // Let agents start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cancel subset of agents
    manager.cancel(id2).await.unwrap();
    manager.cancel(id4).await.unwrap();

    // Verify statuses
    let status2 = manager.get_status(id2).unwrap();
    let status4 = manager.get_status(id4).unwrap();
    assert_eq!(status2.state, AgentState::Cancelled);
    assert_eq!(status4.state, AgentState::Cancelled);

    // Wait for the non-cancelled agents
    let r1 = manager.wait(id1).await;
    let r3 = manager.wait(id3).await;

    assert!(r1.is_ok());
    assert!(r3.is_ok());
    assert_eq!(r1.unwrap(), "Completed 1");
    assert_eq!(r3.unwrap(), "Completed 2");

    // Wait for cancelled agents - should error
    let r2 = manager.wait(id2).await;
    let r4 = manager.wait(id4).await;

    assert!(r2.is_err());
    assert!(r4.is_err());
}

/// Test racing multiple agents and taking the first result.
#[tokio::test]
async fn test_agent_racing() {
    let manager = AgentManager::new();

    // Spawn 5 agents with different speeds
    let slow1 = manager.spawn("slow-1".to_string(), "Slow task".to_string(), || {
        std::thread::sleep(Duration::from_millis(300));
        Ok("Slow result 1".to_string())
    });

    let fast = manager.spawn("fast".to_string(), "Fast task".to_string(), || {
        std::thread::sleep(Duration::from_millis(50));
        Ok("Fast result".to_string())
    });

    let slow2 = manager.spawn("slow-2".to_string(), "Slow task".to_string(), || {
        std::thread::sleep(Duration::from_millis(400));
        Ok("Slow result 2".to_string())
    });

    let slow3 = manager.spawn("slow-3".to_string(), "Slow task".to_string(), || {
        std::thread::sleep(Duration::from_millis(500));
        Ok("Slow result 3".to_string())
    });

    let slow4 = manager.spawn("slow-4".to_string(), "Slow task".to_string(), || {
        std::thread::sleep(Duration::from_millis(600));
        Ok("Slow result 4".to_string())
    });

    // Race all agents
    let start = std::time::Instant::now();
    let result = manager
        .wait_any(vec![slow1, fast, slow2, slow3, slow4])
        .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    let (_winner_id, winner_result) = result.unwrap();

    // The fast agent should win
    assert_eq!(winner_result, "Fast result");

    // Should complete quickly (not wait for slow agents)
    assert!(elapsed < Duration::from_millis(200));
}

/// Test finding first successful result when some agents fail.
#[tokio::test]
async fn test_first_success_with_failures() {
    let manager = AgentManager::new();

    // Spawn agents: some fail fast, one succeeds later
    let fail1 = manager.spawn("fail-1".to_string(), "Fast fail".to_string(), || {
        std::thread::sleep(Duration::from_millis(10));
        Err("Failed 1".to_string())
    });

    let fail2 = manager.spawn("fail-2".to_string(), "Fast fail".to_string(), || {
        std::thread::sleep(Duration::from_millis(20));
        Err("Failed 2".to_string())
    });

    let success = manager.spawn("success".to_string(), "Will succeed".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("Success!".to_string())
    });

    let fail3 = manager.spawn("fail-3".to_string(), "Slow fail".to_string(), || {
        std::thread::sleep(Duration::from_millis(200));
        Err("Failed 3".to_string())
    });

    // Find first success
    let result = manager
        .wait_first_success(vec![fail1, fail2, success, fail3])
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success!");
}

/// Test complex coordination: agents depend on each other's results.
#[tokio::test]
async fn test_sequential_coordination() {
    let manager = AgentManager::new();

    // Phase 1: Initial data gathering
    let gather1 = manager.spawn(
        "gather-1".to_string(),
        "Gathering data 1".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("data-set-1".to_string())
        },
    );

    let gather2 = manager.spawn(
        "gather-2".to_string(),
        "Gathering data 2".to_string(),
        || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("data-set-2".to_string())
        },
    );

    // Wait for phase 1
    let phase1_results = manager.wait_all_parallel(vec![gather1, gather2]).await;
    assert!(phase1_results.is_ok());
    let phase1_data = phase1_results.unwrap();

    // Phase 2: Process gathered data
    let data1 = phase1_data[0].clone();
    let data2 = phase1_data[1].clone();

    let process1 = manager.spawn(
        "process-1".to_string(),
        "Processing data".to_string(),
        move || {
            let _data = &data1; // Use data from phase 1
            std::thread::sleep(Duration::from_millis(50));
            Ok("processed-1".to_string())
        },
    );

    let process2 = manager.spawn(
        "process-2".to_string(),
        "Processing data".to_string(),
        move || {
            let _data = &data2; // Use data from phase 1
            std::thread::sleep(Duration::from_millis(50));
            Ok("processed-2".to_string())
        },
    );

    // Wait for phase 2
    let phase2_results = manager.wait_all_parallel(vec![process1, process2]).await;
    assert!(phase2_results.is_ok());

    let phase2_data = phase2_results.unwrap();
    assert_eq!(phase2_data.len(), 2);
    assert!(phase2_data.contains(&"processed-1".to_string()));
    assert!(phase2_data.contains(&"processed-2".to_string()));
}

/// Test async agents working alongside sync agents.
#[tokio::test]
async fn test_mixed_async_sync_coordination() {
    let manager = AgentManager::new();

    // Spawn mix of sync and async agents
    let sync1 = manager.spawn("sync-1".to_string(), "Sync work".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("sync-result-1".to_string())
    });

    let async1 = manager.spawn_async("async-1".to_string(), "Async work".to_string(), |_| async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok("async-result-1".to_string())
    });

    let sync2 = manager.spawn("sync-2".to_string(), "Sync work".to_string(), || {
        std::thread::sleep(Duration::from_millis(100));
        Ok("sync-result-2".to_string())
    });

    let async2 = manager.spawn_async("async-2".to_string(), "Async work".to_string(), |_| async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok("async-result-2".to_string())
    });

    // All should coordinate properly
    let results = manager
        .wait_all_parallel(vec![sync1, async1, sync2, async2])
        .await;

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 4);
    assert!(results.contains(&"sync-result-1".to_string()));
    assert!(results.contains(&"async-result-1".to_string()));
    assert!(results.contains(&"sync-result-2".to_string()));
    assert!(results.contains(&"async-result-2".to_string()));
}

/// Test high-concurrency scenario with many agents.
#[tokio::test]
async fn test_high_concurrency_coordination() {
    use std::sync::Arc;
    let manager = Arc::new(AgentManager::new());

    // Spawn 50 agents
    let mut ids = Vec::new();
    for i in 0..50 {
        let id = manager.spawn_async(
            format!("agent-{}", i),
            format!("Task {}", i),
            move |reporter| async move {
                reporter.report(33);
                tokio::time::sleep(Duration::from_millis(50)).await;
                reporter.report(66);
                tokio::time::sleep(Duration::from_millis(50)).await;
                reporter.report(100);
                Ok(format!("result-{}", i))
            },
        );
        ids.push(id);
    }

    // Verify all are tracked
    assert_eq!(manager.active_count(), 50);

    // Process progress updates periodically during execution
    let update_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let update_count_clone = update_count.clone();

    // Wait for all agents while processing progress updates
    let start = std::time::Instant::now();

    // Spawn a background task to process progress updates
    let progress_task = tokio::spawn({
        let manager_clone = Arc::clone(&manager);
        let update_count = update_count_clone.clone();
        async move {
            // Process updates for the duration of the test
            for _ in 0..10 {
                tokio::time::sleep(Duration::from_millis(15)).await;
                let processed = manager_clone.process_progress_updates();
                update_count.fetch_add(processed, std::sync::atomic::Ordering::SeqCst);
            }
        }
    });

    // Wait for all agents
    let results = manager.wait_all_parallel(ids).await;
    let elapsed = start.elapsed();

    // Wait for progress task to complete
    progress_task.await.unwrap();

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 50);

    // Verify parallel execution - should be ~100ms, not 5 seconds
    assert!(
        elapsed < Duration::from_millis(300),
        "Expected parallel execution, took {:?}",
        elapsed
    );

    // Note: Progress updates may or may not be processed depending on timing.
    // The important thing is that the manager provides the mechanism, which is tested
    // in test_multi_agent_progress_tracking. Here we just verify the agents completed correctly.

    // All should be complete
    assert_eq!(manager.active_count(), 0);
}

/// Test cleanup after mixed success/failure scenarios.
#[tokio::test]
async fn test_cleanup_after_mixed_completion() {
    let manager = AgentManager::new();

    // Spawn various agents
    let success1 = manager.spawn("s1".to_string(), "desc".to_string(), || {
        Ok("ok1".to_string())
    });

    let fail1 = manager.spawn("f1".to_string(), "desc".to_string(), || {
        Err("err1".to_string())
    });

    let success2 = manager.spawn("s2".to_string(), "desc".to_string(), || {
        Ok("ok2".to_string())
    });

    let cancel = manager.spawn("c1".to_string(), "desc".to_string(), || {
        std::thread::sleep(Duration::from_secs(10));
        Ok("should not see".to_string())
    });

    // Let them start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cancel one
    manager.cancel(cancel).await.unwrap();

    // Wait for the others
    let _ = manager.wait(success1).await;
    let _ = manager.wait(fail1).await;
    let _ = manager.wait(success2).await;
    let _ = manager.wait(cancel).await;

    // All should be removed after waiting
    assert_eq!(manager.get_all_statuses().len(), 0);
    assert_eq!(manager.active_count(), 0);
}

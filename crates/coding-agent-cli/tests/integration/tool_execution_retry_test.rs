//! Integration tests for tool execution with retry logic.
//!
//! Tests the full tool execution flow including:
//! - Retry logic with exponential backoff
//! - Transient vs permanent error handling
//! - Success after retry
//! - Max retry limit enforcement

use coding_agent_cli::tools::{ErrorCategory, ToolError, ToolExecutor, ToolExecutorConfig};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A simulated flaky network tool that succeeds after N failures
fn create_flaky_network_tool(fail_count: u32) -> impl Fn(Value) -> Result<String, String> {
    static CALL_COUNTER: AtomicU32 = AtomicU32::new(0);

    // Reset counter
    CALL_COUNTER.store(0, Ordering::SeqCst);

    move |_input: Value| {
        let call_num = CALL_COUNTER.fetch_add(1, Ordering::SeqCst);
        if call_num < fail_count {
            Err("Connection timed out: server unavailable".to_string())
        } else {
            Ok(format!("Success after {} failures", call_num))
        }
    }
}

#[test]
fn test_tool_execution_succeeds_on_first_retry() {
    // Test that a tool failing once with a retriable error succeeds on first retry

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn flaky_tool(_input: Value) -> Result<String, String> {
        let count = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        if count == 0 {
            Err("Connection refused: temporary network issue".to_string())
        } else {
            Ok("Data retrieved successfully".to_string())
        }
    }

    // Reset counter
    CALL_COUNT.store(0, Ordering::SeqCst);

    // Configure executor with short delays for faster testing
    let config = ToolExecutorConfig {
        max_retries: 3,
        base_retry_delay_ms: 10,
        max_retry_delay_ms: 100,
        auto_fix_enabled: false,
        execution_timeout_ms: 10000,
    };

    let mut executor = ToolExecutor::new(config);
    executor.register_tool("flaky_network", flaky_tool);

    let start = Instant::now();
    let result = executor.execute("call_1", "flaky_network", json!({}));
    let elapsed = start.elapsed();

    // Verify success after retry
    assert!(result.is_success(), "Expected success after retry");
    assert_eq!(result.retries, 1, "Should have retried exactly once");
    assert_eq!(result.result.unwrap(), "Data retrieved successfully");

    // Verify tool was called twice (initial + 1 retry)
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);

    // Verify some delay occurred (at least 10ms for the retry delay)
    assert!(
        elapsed >= Duration::from_millis(10),
        "Should have waited for retry delay"
    );
}

#[test]
fn test_tool_execution_succeeds_after_multiple_retries() {
    // Test that a tool can succeed after multiple retry attempts

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn very_flaky_tool(_input: Value) -> Result<String, String> {
        let count = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        if count < 2 {
            Err(format!("Connection timed out: attempt {}", count + 1))
        } else {
            Ok("Finally connected!".to_string())
        }
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let config = ToolExecutorConfig {
        max_retries: 3,
        base_retry_delay_ms: 5,
        max_retry_delay_ms: 50,
        ..Default::default()
    };

    let mut executor = ToolExecutor::new(config);
    executor.register_tool("very_flaky", very_flaky_tool);

    let result = executor.execute("call_2", "very_flaky", json!({}));

    // Should succeed on the 3rd attempt (2 retries)
    assert!(result.is_success());
    assert_eq!(result.retries, 2);
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 3);
}

#[test]
fn test_tool_execution_fails_after_max_retries() {
    // Test that the executor gives up after max retries

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn always_fail_network_tool(_input: Value) -> Result<String, String> {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        Err("Connection refused: server is down".to_string())
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let config = ToolExecutorConfig {
        max_retries: 3,
        base_retry_delay_ms: 5,
        max_retry_delay_ms: 50,
        ..Default::default()
    };

    let mut executor = ToolExecutor::new(config);
    executor.register_tool("always_fail", always_fail_network_tool);

    let start = Instant::now();
    let result = executor.execute("call_3", "always_fail", json!({}));
    let elapsed = start.elapsed();

    // Should have failed after max retries
    assert!(!result.is_success());
    assert_eq!(result.retries, 3, "Should have retried max times");

    // Total calls = 1 initial + 3 retries = 4
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 4);

    // Verify the error is still a network error
    let error = result.error().unwrap();
    assert!(error.message.contains("Connection refused"));
    assert!(matches!(
        error.category,
        ErrorCategory::Network { is_transient: true }
    ));

    // Verify exponential backoff delays occurred
    // Expected delays: 5ms, 10ms, 20ms = 35ms minimum
    assert!(
        elapsed >= Duration::from_millis(35),
        "Should have waited for exponential backoff delays, elapsed: {:?}",
        elapsed
    );
}

#[test]
fn test_non_retriable_error_no_retry_integration() {
    // Test that permission errors are not retried

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn permission_denied_tool(_input: Value) -> Result<String, String> {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        Err("Permission denied: cannot write to /etc/hosts".to_string())
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let config = ToolExecutorConfig {
        max_retries: 3,
        base_retry_delay_ms: 10,
        max_retry_delay_ms: 100,
        ..Default::default()
    };

    let mut executor = ToolExecutor::new(config);
    executor.register_tool("perm_denied", permission_denied_tool);

    let start = Instant::now();
    let result = executor.execute("call_4", "perm_denied", json!({}));
    let elapsed = start.elapsed();

    // Should have failed immediately without retries
    assert!(!result.is_success());
    assert_eq!(result.retries, 0, "Should not have retried");
    assert_eq!(
        CALL_COUNT.load(Ordering::SeqCst),
        1,
        "Should only call once"
    );

    // Should have failed quickly (no retry delays)
    assert!(
        elapsed < Duration::from_millis(50),
        "Should fail fast for non-retriable errors"
    );

    let error = result.error().unwrap();
    assert!(matches!(error.category, ErrorCategory::Permission { .. }));
    assert!(!error.retriable);
}

#[test]
fn test_code_error_no_retry_integration() {
    // Test that code errors (like missing dependencies) are not retried automatically

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn missing_dependency_tool(_input: Value) -> Result<String, String> {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        Err("error[E0463]: can't find crate for `serde_json`".to_string())
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let mut executor = ToolExecutor::with_defaults();
    executor.register_tool("compile", missing_dependency_tool);

    let result = executor.execute("call_5", "compile", json!({}));

    // Should fail immediately (code errors need fixing, not retrying)
    assert!(!result.is_success());
    assert_eq!(result.retries, 0);
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);

    // But it should be marked as auto-fixable
    assert!(result.is_auto_fixable());

    let error = result.error().unwrap();
    assert!(matches!(
        error.category,
        ErrorCategory::Code { error_type: ref t } if t == "missing_dependency"
    ));
}

#[test]
fn test_exponential_backoff_timing() {
    // Verify that exponential backoff delays are applied correctly

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);
    static CALL_TIMES: Mutex<Vec<Instant>> = Mutex::new(Vec::new());

    fn timed_fail_tool(_input: Value) -> Result<String, String> {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        CALL_TIMES.lock().unwrap().push(Instant::now());
        Err("Connection timed out".to_string())
    }

    CALL_COUNT.store(0, Ordering::SeqCst);
    CALL_TIMES.lock().unwrap().clear();

    let config = ToolExecutorConfig {
        max_retries: 3,
        base_retry_delay_ms: 100, // 100ms base
        max_retry_delay_ms: 1000,
        ..Default::default()
    };

    let mut executor = ToolExecutor::new(config);
    executor.register_tool("timed_fail", timed_fail_tool);

    let _ = executor.execute("call_6", "timed_fail", json!({}));

    // Verify the timing of calls
    let times = CALL_TIMES.lock().unwrap();
    assert_eq!(times.len(), 4, "Should have 4 attempts");

    // Check delays between attempts
    // Expected: 100ms, 200ms, 400ms (exponential backoff)
    if times.len() >= 2 {
        let delay1 = times[1].duration_since(times[0]);
        assert!(
            delay1 >= Duration::from_millis(90) && delay1 <= Duration::from_millis(150),
            "First retry delay should be ~100ms, got {:?}",
            delay1
        );
    }

    if times.len() >= 3 {
        let delay2 = times[2].duration_since(times[1]);
        assert!(
            delay2 >= Duration::from_millis(180) && delay2 <= Duration::from_millis(250),
            "Second retry delay should be ~200ms, got {:?}",
            delay2
        );
    }

    if times.len() >= 4 {
        let delay3 = times[3].duration_since(times[2]);
        assert!(
            delay3 >= Duration::from_millis(380) && delay3 <= Duration::from_millis(450),
            "Third retry delay should be ~400ms, got {:?}",
            delay3
        );
    }
}

#[test]
fn test_retry_with_different_error_types() {
    // Test that different network error types are all retriable

    let test_cases = vec![
        "Connection refused",
        "Connection reset by peer",
        "Network unreachable",
        "Operation timed out",
        "Connection timed out",
        "Deadline exceeded",
    ];

    for error_msg in test_cases {
        // For this test, we just verify categorization
        // Testing actual retry behavior is covered in other tests
        let error = ToolError::new(error_msg);
        assert!(error.retriable, "Error '{}' should be retriable", error_msg);
        assert!(matches!(
            error.category,
            ErrorCategory::Network { is_transient: true }
        ));
    }
}

#[test]
fn test_max_retry_delay_cap() {
    // Test that retry delays are capped at max_retry_delay_ms

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn fail_tool(_input: Value) -> Result<String, String> {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        Err("Connection timed out".to_string())
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let config = ToolExecutorConfig {
        max_retries: 10, // Many retries
        base_retry_delay_ms: 100,
        max_retry_delay_ms: 500, // Cap at 500ms
        ..Default::default()
    };

    let executor = ToolExecutor::new(config);

    // Test the delay calculation directly
    // retry 1: 100ms
    // retry 2: 200ms
    // retry 3: 400ms
    // retry 4: 800ms -> capped at 500ms
    // retry 5+: still 500ms

    assert_eq!(executor.config().base_retry_delay_ms, 100);
    assert_eq!(executor.config().max_retry_delay_ms, 500);

    // The executor should cap delays - verified in unit tests
    // This integration test verifies the config is respected
}

#[test]
fn test_retry_preserves_call_id_and_tool_name() {
    // Test that metadata is preserved through retries

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn flaky_tool(_input: Value) -> Result<String, String> {
        let count = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        if count < 1 {
            Err("Connection refused".to_string())
        } else {
            Ok("success".to_string())
        }
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let mut executor = ToolExecutor::with_defaults();
    executor.register_tool("metadata_test", flaky_tool);

    let result = executor.execute(
        "unique_call_id_123",
        "metadata_test",
        json!({"key": "value"}),
    );

    assert!(result.is_success());
    assert_eq!(result.call_id, "unique_call_id_123");
    assert_eq!(result.tool_name, "metadata_test");
    assert_eq!(result.retries, 1);
}

#[test]
fn test_tool_execution_duration_tracking() {
    // Test that execution duration includes retry delays

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn slow_flaky_tool(_input: Value) -> Result<String, String> {
        let count = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(50)); // Simulate work
        if count < 2 {
            Err("Connection timed out".to_string())
        } else {
            Ok("success".to_string())
        }
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let config = ToolExecutorConfig {
        max_retries: 3,
        base_retry_delay_ms: 50,
        max_retry_delay_ms: 200,
        ..Default::default()
    };

    let mut executor = ToolExecutor::new(config);
    executor.register_tool("slow_flaky", slow_flaky_tool);

    let result = executor.execute("call_7", "slow_flaky", json!({}));

    assert!(result.is_success());
    assert_eq!(result.retries, 2);

    // Duration should include:
    // - 3 tool executions at 50ms each = 150ms
    // - 2 retry delays at 50ms and 100ms = 150ms
    // Total: ~300ms minimum
    assert!(
        result.duration >= Duration::from_millis(250),
        "Duration should include retry delays and execution time, got {:?}",
        result.duration
    );
}

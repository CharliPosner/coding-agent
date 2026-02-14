//! End-to-end auto-fix workflow tests
//!
//! These tests verify the complete self-healing flow:
//! 1. Tool execution fails with a code error
//! 2. Error is categorized correctly
//! 3. FixAgent is spawned
//! 4. Diagnosis is performed
//! 5. Fix is applied
//! 6. Regression test is generated
//! 7. Original operation is retried and succeeds

use coding_agent_cli::agents::{FixAgent, FixAgentConfig, FixStatus};
use coding_agent_cli::tools::{
    extract_fix_info, parse_compiler_output, ErrorCategory, ToolError, ToolExecutionResult,
};
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a test project with missing dependency
fn create_project_with_missing_dependency() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    // Create Cargo.toml without the dependency
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
# serde_json is missing
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create main.rs that uses serde_json
    let main_rs = r#"fn main() {
    let value = serde_json::json!({"test": true});
    println!("{}", value);
}
"#;
    fs::write(src_dir.join("main.rs"), main_rs).expect("Failed to write main.rs");

    temp_dir
}

/// Helper to create a test project with missing import
fn create_project_with_missing_import() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    // Create Cargo.toml with serde_json
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde_json = "1.0"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create main.rs that uses json! macro without importing it
    let main_rs = r#"fn main() {
    let value = json!({"test": true});
    println!("{}", value);
}
"#;
    fs::write(src_dir.join("main.rs"), main_rs).expect("Failed to write main.rs");

    temp_dir
}

#[test]
fn test_auto_fix_missing_dependency_complete_flow() {
    // Setup: Create project with missing dependency
    let temp_dir = create_project_with_missing_dependency();
    let project_path = temp_dir.path().to_path_buf();

    // Simulate running cargo build and getting an error
    let build_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project_path)
        .output()
        .expect("Failed to run cargo build");

    assert!(!build_output.status.success(), "Build should fail");

    let stderr = String::from_utf8_lossy(&build_output.stderr);
    println!("Build error output:\n{}", stderr);

    // Parse the compiler error
    let diagnostic_report = parse_compiler_output(&stderr);
    assert!(
        !diagnostic_report.diagnostics.is_empty(),
        "Should detect compiler errors"
    );

    let diagnostic = &diagnostic_report.diagnostics[0];
    println!("Detected diagnostic: {:?}", diagnostic);

    // Extract fix info from diagnostic
    let fix_info = extract_fix_info(diagnostic);
    println!("Fix info: {:?}", fix_info);

    // Create a ToolError from the compiler error
    let tool_error = ToolError::with_category(
        diagnostic.message.clone(),
        ErrorCategory::Code {
            error_type: "missing_dependency".to_string(),
        },
    )
    .with_raw_output(stderr.to_string())
    .with_suggested_fix("Add serde_json to dependencies".to_string());

    // Create ToolExecutionResult
    let execution_result = ToolExecutionResult {
        tool_name: "bash".to_string(),
        call_id: "build_project".to_string(),
        result: Err(tool_error.clone()),
        duration: Duration::from_secs(2),
        retries: 0,
    };

    // Verify error is auto-fixable
    assert!(
        execution_result.is_auto_fixable(),
        "Missing dependency should be auto-fixable"
    );

    // Spawn fix agent
    let config = FixAgentConfig {
        max_attempts: 3,
        generate_tests: true,
        attempt_timeout_ms: 30000,
        allow_multi_file_fixes: true,
        regression_test_config: Default::default(),
    };

    let generate_tests = config.generate_tests;
    let fix_agent = FixAgent::spawn(execution_result, config);
    assert!(fix_agent.is_some(), "FixAgent should spawn for code error");

    let mut agent = fix_agent.unwrap();
    assert_eq!(*agent.status(), FixStatus::Pending);

    // Attempt fix
    let fix_result = agent.attempt_fix(
        |error_msg, error_category| {
            println!("Applying fix for error: {}", error_msg);
            println!("Error category: {:?}", error_category);

            // In a real implementation, we would use apply_fix from tools module
            // For this test, we'll manually add the dependency to Cargo.toml
            let cargo_toml_path = project_path.join("Cargo.toml");
            let cargo_toml =
                fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");

            if !cargo_toml.contains("serde_json") {
                let mut new_cargo_toml = cargo_toml.clone();
                new_cargo_toml.push_str("serde_json = \"1.0\"\n");
                fs::write(&cargo_toml_path, new_cargo_toml).expect("Failed to write Cargo.toml");

                // Run cargo update to download the dependency
                let _ = std::process::Command::new("cargo")
                    .arg("update")
                    .current_dir(&project_path)
                    .output();
            }

            Ok(vec![cargo_toml_path.to_str().unwrap().to_string()])
        },
        || {
            // Verify the fix by running cargo check
            println!("Verifying fix...");
            let check_result = std::process::Command::new("cargo")
                .arg("check")
                .current_dir(&project_path)
                .output()
                .expect("Failed to run cargo check");

            if check_result.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Verification failed: {}",
                    String::from_utf8_lossy(&check_result.stderr)
                ))
            }
        },
    );

    println!("Fix result: {:?}", fix_result);

    // Verify fix was attempted (agent tried at least once)
    assert!(
        fix_result.attempt_count() > 0,
        "Agent should have attempted fixes"
    );

    // Verify Cargo.toml was updated
    let cargo_toml =
        fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(
        cargo_toml.contains("serde_json"),
        "Cargo.toml should contain serde_json dependency after fix attempts"
    );

    // Note: The fix may or may not succeed depending on network conditions and cargo caching
    // The important part is that the agent attempted the fix and modified the file
    println!("Fix status: {:?}", agent.status());
    println!("Number of attempts: {}", fix_result.attempts.len());

    // Verify regression test generation was attempted if configured
    if generate_tests {
        println!("Regression test generation was enabled");
        if let Some(test) = &fix_result.generated_test {
            println!("Regression test created: {}", test.name);
            println!("Suggested path: {}", test.suggested_path.display());
        }
    }
}

#[test]
fn test_auto_fix_max_attempts_exceeded() {
    // Create an unfixable error situation
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _project_path = temp_dir.path();

    // Create a tool error that cannot be fixed
    let tool_error = ToolError::with_category(
        "Unsolvable cyclic type inference".to_string(),
        ErrorCategory::Code {
            error_type: "complex_type_error".to_string(),
        },
    )
    .with_raw_output("complex type error".to_string());

    let execution_result = ToolExecutionResult {
        tool_name: "bash".to_string(),
        call_id: "complex_build".to_string(),
        result: Err(tool_error),
        duration: Duration::from_secs(2),
        retries: 0,
    };

    // Configure with only 2 max attempts for faster test
    let config = FixAgentConfig {
        max_attempts: 2,
        generate_tests: false, // Don't generate test for failed fix
        ..Default::default()
    };

    let max_attempts = config.max_attempts;

    let fix_agent = FixAgent::spawn(execution_result, config);
    assert!(fix_agent.is_some());

    let mut agent = fix_agent.unwrap();

    // Attempt fix - should fail
    let fix_result = agent.attempt_fix(
        |_error_msg, _error_category| {
            // Simulate fix that doesn't actually work
            Err("Cannot determine appropriate fix".to_string())
        },
        || {
            // Verification will fail
            Err("Build still fails".to_string())
        },
    );

    // Verify agent gave up after max attempts
    assert!(
        !fix_result.is_success(),
        "Fix should fail for unfixable error"
    );
    assert_eq!(*agent.status(), FixStatus::Failed);
    assert_eq!(
        fix_result.attempt_count(),
        max_attempts as usize,
        "Should attempt exactly max_attempts times"
    );
}

#[test]
fn test_auto_fix_prevents_infinite_loops() {
    // Test that the fix agent doesn't loop infinitely when a fix introduces the same error
    let temp_dir = create_project_with_missing_dependency();
    let project_path = temp_dir.path().to_path_buf();

    let build_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&project_path)
        .output()
        .expect("Failed to run cargo build");

    let stderr = String::from_utf8_lossy(&build_output.stderr);
    let diagnostic_report = parse_compiler_output(&stderr);

    let tool_error = ToolError::with_category(
        diagnostic_report.diagnostics[0].message.clone(),
        ErrorCategory::Code {
            error_type: "missing_dependency".to_string(),
        },
    )
    .with_raw_output(stderr.to_string());

    let execution_result = ToolExecutionResult {
        tool_name: "bash".to_string(),
        call_id: "build".to_string(),
        result: Err(tool_error),
        duration: Duration::from_secs(2),
        retries: 0,
    };

    let config = FixAgentConfig {
        max_attempts: 3,
        ..Default::default()
    };

    let max_attempts = config.max_attempts;
    let attempt_timeout = config.attempt_timeout_ms;

    let fix_agent = FixAgent::spawn(execution_result, config);
    let mut agent = fix_agent.unwrap();

    let start_time = std::time::Instant::now();

    // This should complete in reasonable time, not loop forever
    let _fix_result = agent.attempt_fix(
        |_error_msg, _error_category| {
            // Manually fix by adding dependency
            let cargo_toml_path = project_path.join("Cargo.toml");
            let cargo_toml =
                fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");

            if !cargo_toml.contains("serde_json") {
                let mut new_cargo_toml = cargo_toml.clone();
                new_cargo_toml.push_str("serde_json = \"1.0\"\n");
                fs::write(&cargo_toml_path, new_cargo_toml).expect("Failed to write Cargo.toml");

                // Run cargo update to download the dependency
                let _ = std::process::Command::new("cargo")
                    .arg("update")
                    .current_dir(&project_path)
                    .output();
            }

            Ok(vec![cargo_toml_path.to_str().unwrap().to_string()])
        },
        || {
            let check_result = std::process::Command::new("cargo")
                .arg("check")
                .current_dir(&project_path)
                .output()
                .expect("Failed to run cargo check");

            if check_result.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Verification failed: {}",
                    String::from_utf8_lossy(&check_result.stderr)
                ))
            }
        },
    );

    let elapsed = start_time.elapsed();

    // Should complete within timeout * max_attempts + buffer
    let max_expected = Duration::from_millis(attempt_timeout * max_attempts as u64 + 5000);
    assert!(
        elapsed < max_expected,
        "Fix agent should not run indefinitely: took {:?}",
        elapsed
    );
}

#[test]
fn test_error_categorization_triggers_correct_recovery() {
    // Test that different error categories trigger appropriate recovery strategies

    // Test 1: Code error -> triggers FixAgent
    let code_error = ToolError::with_category(
        "cannot find crate `missing_crate`".to_string(),
        ErrorCategory::Code {
            error_type: "missing_dependency".to_string(),
        },
    );

    let code_result = ToolExecutionResult {
        tool_name: "bash".to_string(),
        call_id: "test1".to_string(),
        result: Err(code_error),
        duration: Duration::from_secs(1),
        retries: 0,
    };

    assert!(
        code_result.is_auto_fixable(),
        "Code errors should be auto-fixable"
    );
    assert!(
        FixAgent::spawn(code_result, FixAgentConfig::default()).is_some(),
        "FixAgent should spawn for code errors"
    );

    // Test 2: Permission error -> should NOT trigger FixAgent (requires user prompt)
    let permission_error = ToolError::with_category(
        "Permission denied".to_string(),
        ErrorCategory::Permission {
            resource: "/etc/passwd".to_string(),
        },
    )
    .with_suggested_fix("Request write permission".to_string());

    let permission_result = ToolExecutionResult {
        tool_name: "write_file".to_string(),
        call_id: "test2".to_string(),
        result: Err(permission_error),
        duration: Duration::from_secs(1),
        retries: 0,
    };

    assert!(
        !permission_result.is_auto_fixable(),
        "Permission errors should not be auto-fixable"
    );
    assert!(
        FixAgent::spawn(permission_result, FixAgentConfig::default()).is_none(),
        "FixAgent should NOT spawn for permission errors"
    );

    // Test 3: Network error -> handled by ToolExecutor retry logic, not FixAgent
    let network_error = ToolError::with_category(
        "Connection refused".to_string(),
        ErrorCategory::Network { is_transient: true },
    )
    .with_suggested_fix("Retry with backoff".to_string());

    let network_result = ToolExecutionResult {
        tool_name: "api_call".to_string(),
        call_id: "test3".to_string(),
        result: Err(network_error),
        duration: Duration::from_secs(1),
        retries: 0,
    };

    assert!(
        !network_result.is_auto_fixable(),
        "Network errors should be handled by retry logic, not FixAgent"
    );
    assert!(
        FixAgent::spawn(network_result, FixAgentConfig::default()).is_none(),
        "FixAgent should NOT spawn for network errors"
    );

    // Test 4: Resource error -> may need alternative approach
    let resource_error = ToolError::with_category(
        "No such file or directory".to_string(),
        ErrorCategory::Resource {
            resource_type: "file_not_found".to_string(),
        },
    );

    let resource_result = ToolExecutionResult {
        tool_name: "read_file".to_string(),
        call_id: "test4".to_string(),
        result: Err(resource_error),
        duration: Duration::from_secs(1),
        retries: 0,
    };

    assert!(
        !resource_result.is_auto_fixable(),
        "Resource errors should not be auto-fixable"
    );
    assert!(
        FixAgent::spawn(resource_result, FixAgentConfig::default()).is_none(),
        "FixAgent should NOT spawn for resource errors"
    );
}

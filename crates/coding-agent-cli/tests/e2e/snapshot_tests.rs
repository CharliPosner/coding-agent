//! Essential E2E snapshot tests
//!
//! Focused on true end-to-end CLI behavior rather than duplicating 
//! UI component tests that are covered in ui_visual_regression_tests.rs

use crate::e2e::harness::CliTestSession;
use crate::e2e::mock_claude::MockClaudeServer;
use insta::assert_snapshot;
use std::time::Duration;

/// Helper to strip ANSI escape codes from terminal output
fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").to_string()
}

/// Helper to normalize whitespace for consistent snapshots
fn normalize_whitespace(s: &str) -> String {
    s.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Helper to capture terminal output with ANSI codes stripped
fn capture_without_ansi(output: &str) -> String {
    normalize_whitespace(&strip_ansi(output))
}

// ============================================================================
// TRUE E2E TESTS - CLI Interaction & Startup Behavior
// ============================================================================

#[test]
#[ignore] // Requires building the binary first
fn test_startup_screen_snapshot() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");

    // Wait for startup screen to render
    std::thread::sleep(Duration::from_millis(500));

    let result = session.expect_startup_screen();
    assert!(result.is_ok(), "Failed to see startup screen");

    // This test verifies the startup screen renders without panicking
    // Full snapshot testing would require term-transcript integration
}

#[test]
#[ignore] 
fn test_help_command_snapshot() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Run /help command
    let output = session.run_command("/help").expect("Failed to run /help");

    // Strip ANSI for consistent snapshots
    let clean_output = capture_without_ansi(&output);

    // Snapshot the help output
    assert_snapshot!("help_output", clean_output);
}

#[test]
#[ignore]
fn test_unknown_command_snapshot() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Run unknown command
    let output = session
        .run_command("/foobar")
        .expect("Failed to run command");
    let clean_output = capture_without_ansi(&output);

    assert_snapshot!("unknown_command_error", clean_output);
}

#[tokio::test]
#[ignore] // Requires mock API server
async fn test_full_conversation_flow() {
    // Test complete conversation: user input → API call → tool execution → response
    let _mock_server = MockClaudeServer::start().await;

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.select_new_session().expect("Failed to select session");

    // Simulate user asking for file read
    let output = session
        .run_command("Please read the file fixtures/fizzbuzz.js")
        .expect("Failed to run command");

    let clean_output = capture_without_ansi(&output);
    
    // Should contain tool execution and file content
    assert!(clean_output.contains("fizzbuzz") || clean_output.contains("function"));
    assert_snapshot!("full_conversation_flow", clean_output);
}

// Note: Removed redundant tests that duplicate UI component testing:
// - context_bar_render_snapshot_* (covered in ui_visual_regression_tests.rs)  
// - error/success/warning_message_formatting_* (covered in ui_visual_regression_tests.rs)
// - context_bar_color_transitions (covered in ui_visual_regression_tests.rs)
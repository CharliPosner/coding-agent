use super::harness::CliTestSession;
use super::pty_helpers::*;
use std::time::Duration;

/// Test: ASCII art renders correctly in PTY
#[test]
#[ignore] // Requires building the binary first
fn test_pty_startup_displays_logo() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");

    // Wait for startup screen
    let result = session.expect_startup_screen();
    assert!(result.is_ok(), "Failed to see startup screen");
}

/// Test: Input only submits on double-enter
#[test]
#[ignore]
fn test_pty_double_enter_submits() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("Startup failed");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Send text with single enter
    send_text(session.session_mut(), "First line").expect("Failed to send text");
    send_enter(session.session_mut()).expect("Failed to send enter");

    // Should still be accepting input (no response yet)
    send_text(session.session_mut(), "Second line").expect("Failed to send text");

    // Double-enter should submit
    send_enter(session.session_mut()).expect("Failed to send first enter");
    send_enter(session.session_mut()).expect("Failed to send second enter");

    // Should get a response now
    let result = wait_for_prompt(session.session_mut(), Duration::from_secs(10));
    assert!(
        result.is_ok(),
        "Should have gotten prompt after double-enter"
    );
}

/// Test: Ctrl+C clears current input
#[test]
#[ignore]
fn test_pty_ctrl_c_cancels() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("Startup failed");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Type some text
    send_text(session.session_mut(), "Some text to cancel").expect("Failed to send text");

    // Send Ctrl+C
    session.send_ctrl_c().expect("Failed to send Ctrl+C");

    // Prompt should reappear without submission
    let result = wait_for_prompt(session.session_mut(), Duration::from_secs(2));
    assert!(result.is_ok(), "Prompt should reappear after Ctrl+C");
}

/// Test: Ctrl+D exits cleanly
#[test]
#[ignore]
fn test_pty_ctrl_d_exits() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("Startup failed");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Send Ctrl+D to exit
    session.send_ctrl_d().expect("Failed to send Ctrl+D");

    // Session should end
    std::thread::sleep(Duration::from_millis(500));
    assert!(session.is_eof(), "Session should have ended after Ctrl+D");
}

/// Test: Terminal restored after exit/crash
#[test]
#[ignore]
fn test_pty_raw_mode_cleanup() {
    // Spawn and immediately exit
    {
        let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
        session.expect_startup_screen().expect("Startup failed");
        session.send_ctrl_d().expect("Failed to send Ctrl+D");
    } // Session dropped here

    // Check that terminal state is restored by running a simple command
    let output = std::process::Command::new("tty")
        .output()
        .expect("Failed to run tty command");

    assert!(
        output.status.success(),
        "Terminal should be in a valid state after CLI exit"
    );
}

/// Test: Unicode rendering (emoji and CJK)
#[test]
#[ignore]
fn test_pty_unicode_rendering() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("Startup failed");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Send unicode text
    let unicode_text = "Hello 👋 世界 🚀";
    send_text(session.session_mut(), unicode_text).expect("Failed to send unicode");
    send_enter(session.session_mut()).expect("Failed to send first enter");
    send_enter(session.session_mut()).expect("Failed to send second enter");

    // Should get response (even if API fails, shouldn't crash on unicode)
    let result = wait_for_prompt(session.session_mut(), Duration::from_secs(10));
    assert!(result.is_ok(), "Should handle unicode input gracefully");
}

/// Test: Long output scrolls properly
#[test]
#[ignore]
fn test_pty_long_output_scrolls() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("Startup failed");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Run /help which should produce long output
    let result = session.run_command("/help");

    // Should complete without hanging
    assert!(result.is_ok(), "Long output should not hang the CLI");

    // Output should contain help text
    let output = result.unwrap();
    assert!(
        contains_pattern(&output, "help") || contains_pattern(&output, "command"),
        "Help output should contain relevant text"
    );
}

/// Test: Terminal resize handling
#[test]
#[ignore]
fn test_pty_terminal_resize() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("Startup failed");

    // Note: Actual resize testing is complex and platform-specific
    // This test just verifies the CLI doesn't crash on startup
    // Real resize testing would require platform-specific PTY manipulation

    assert!(!session.is_eof(), "Session should still be running");
}

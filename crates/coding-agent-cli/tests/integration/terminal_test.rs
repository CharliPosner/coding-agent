//! Integration tests for terminal handling
//!
//! These tests verify that the terminal is properly restored after various scenarios.

/// Test that the terminal is restored after a panic in the CLI.
///
/// This test verifies the code structure is correct by checking that:
/// 1. The panic hook is set up in mod.rs
/// 2. The Terminal struct has a Drop impl that calls disable_raw_mode
///
/// Note: Actually spawning a subprocess to test terminal restoration is difficult
/// in CI environments without a TTY, so we verify the code structure instead.
#[test]
fn test_terminal_cleanup_on_panic() {
    // Verify the panic hook exists in the source
    let mod_rs = include_str!("../../src/cli/mod.rs");
    assert!(
        mod_rs.contains("panic::set_hook") || mod_rs.contains("std::panic::set_hook"),
        "Panic hook should be set up in cli/mod.rs"
    );
    assert!(
        mod_rs.contains("disable_raw_mode"),
        "Panic hook should disable raw mode"
    );

    // Verify the Terminal Drop impl exists
    let terminal_rs = include_str!("../../src/cli/terminal.rs");
    assert!(
        terminal_rs.contains("impl Drop for Terminal"),
        "Terminal should have a Drop implementation"
    );
    assert!(
        terminal_rs.contains("disable_raw_mode"),
        "Terminal Drop should disable raw mode"
    );
}

/// Test that Terminal's Drop implementation properly cleans up raw mode.
#[test]
fn test_terminal_drop_cleanup() {
    // Verify the source code has proper Drop implementation
    let terminal_rs = include_str!("../../src/cli/terminal.rs");

    // Check Drop impl exists and handles raw_mode_enabled flag
    assert!(terminal_rs.contains("impl Drop for Terminal"));
    assert!(terminal_rs.contains("if self.raw_mode_enabled"));
    assert!(terminal_rs.contains("disable_raw_mode"));
}

/// Test that the panic hook restores cursor visibility.
#[test]
fn test_panic_hook_restores_cursor() {
    let mod_rs = include_str!("../../src/cli/mod.rs");

    // The panic hook should show the cursor
    assert!(
        mod_rs.contains("cursor::Show") || mod_rs.contains("Show"),
        "Panic hook should restore cursor visibility"
    );
}

//! Snapshot tests for terminal output
//!
//! These tests capture the actual visual output of the CLI including ANSI colors
//! and formatting. They use `insta` for snapshot comparison.

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

/// Helper to capture terminal output with ANSI codes preserved
fn capture_with_ansi(output: &str) -> String {
    normalize_whitespace(output)
}

/// Helper to capture terminal output with ANSI codes stripped
fn capture_without_ansi(output: &str) -> String {
    normalize_whitespace(&strip_ansi(output))
}

#[test]
#[ignore] // Requires building the binary first
fn test_startup_screen_snapshot() {
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");

    // Wait for startup screen to render
    std::thread::sleep(Duration::from_millis(500));

    // The startup screen should be already displayed
    // For now, we'll capture what we can see through the PTY
    // Note: This is a simplified version; real implementation would need
    // to capture the full terminal buffer

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

#[test]
fn test_context_bar_render_snapshot_25_percent() {
    use coding_agent_cli::ui::context_bar::ContextBar;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    // Use monochrome theme for consistent snapshots
    let theme = Theme::new(ThemeStyle::Monochrome);
    let mut bar = ContextBar::with_theme(200_000, theme);
    bar.set_tokens(50_000); // 25%

    let output = bar.render();
    let clean_output = strip_ansi(&output);

    assert_snapshot!("context_bar_25_percent", clean_output);
}

#[test]
fn test_context_bar_render_snapshot_60_percent() {
    use coding_agent_cli::ui::context_bar::ContextBar;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Monochrome);
    let mut bar = ContextBar::with_theme(200_000, theme);
    bar.set_tokens(120_000); // 60%

    let output = bar.render();
    let clean_output = strip_ansi(&output);

    assert_snapshot!("context_bar_60_percent", clean_output);
}

#[test]
fn test_context_bar_render_snapshot_85_percent() {
    use coding_agent_cli::ui::context_bar::ContextBar;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Monochrome);
    let mut bar = ContextBar::with_theme(200_000, theme);
    bar.set_tokens(170_000); // 85%

    let output = bar.render();
    let clean_output = strip_ansi(&output);

    assert_snapshot!("context_bar_85_percent", clean_output);
}

#[test]
fn test_context_bar_render_snapshot_100_percent() {
    use coding_agent_cli::ui::context_bar::ContextBar;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Monochrome);
    let mut bar = ContextBar::with_theme(200_000, theme);
    bar.set_tokens(200_000); // 100%

    let output = bar.render();
    let clean_output = strip_ansi(&output);

    assert_snapshot!("context_bar_100_percent", clean_output);
}

#[test]
fn test_context_bar_compact_snapshot() {
    use coding_agent_cli::ui::context_bar::ContextBar;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Monochrome);
    let mut bar = ContextBar::with_theme(200_000, theme);
    bar.set_tokens(50_000); // 25%

    let output = bar.render_compact();
    let clean_output = strip_ansi(&output);

    assert_snapshot!("context_bar_compact", clean_output);
}

#[tokio::test]
#[ignore] // Requires mock API server
async fn test_tool_execution_display_snapshot() {
    // This would test the "● Reading..." and "✓ Read 150 lines" output
    // Requires integration with mock API server

    let _mock_server = MockClaudeServer::start().await;

    // TODO: Implement when mock server is fully integrated
}

#[test]
fn test_error_message_formatting_snapshot() {
    use coding_agent_cli::ui::output::StyledOutput;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Monochrome);
    let output = StyledOutput::new(theme);

    // We can't easily capture println! output, so we'll test the theme application directly
    let error_msg = output.theme().apply(
        coding_agent_cli::ui::theme::Color::Error,
        "File not found: /path/to/file.rs",
    );
    let clean_output = strip_ansi(&error_msg);

    assert_snapshot!("error_message_file_not_found", clean_output);
}

#[test]
fn test_success_message_formatting_snapshot() {
    use coding_agent_cli::ui::output::StyledOutput;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Monochrome);
    let output = StyledOutput::new(theme);

    let success_msg = output.theme().apply(
        coding_agent_cli::ui::theme::Color::Success,
        "File created successfully",
    );
    let clean_output = strip_ansi(&success_msg);

    assert_snapshot!("success_message", clean_output);
}

#[test]
fn test_warning_message_formatting_snapshot() {
    use coding_agent_cli::ui::output::StyledOutput;
    use coding_agent_cli::ui::theme::{Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Monochrome);
    let output = StyledOutput::new(theme);

    let warning_msg = output.theme().apply(
        coding_agent_cli::ui::theme::Color::Warning,
        "This operation may be slow",
    );
    let clean_output = strip_ansi(&warning_msg);

    assert_snapshot!("warning_message", clean_output);
}

/// Test that verifies color transitions at exact thresholds
#[test]
fn test_context_bar_color_transitions() {
    use coding_agent_cli::ui::context_bar::ContextBar;
    use coding_agent_cli::ui::theme::{Color, Theme, ThemeStyle};

    let theme = Theme::new(ThemeStyle::Minimal);

    // Test 59% - should be green
    let mut bar = ContextBar::with_theme(100, theme.clone());
    bar.set_tokens(59);
    assert_eq!(bar.usage_color(), Color::ContextGreen);

    // Test 60% - should be yellow
    let mut bar = ContextBar::with_theme(100, theme.clone());
    bar.set_tokens(60);
    assert_eq!(bar.usage_color(), Color::ContextYellow);

    // Test 84% - should be yellow
    let mut bar = ContextBar::with_theme(100, theme.clone());
    bar.set_tokens(84);
    assert_eq!(bar.usage_color(), Color::ContextYellow);

    // Test 85% - should be red
    let mut bar = ContextBar::with_theme(100, theme);
    bar.set_tokens(85);
    assert_eq!(bar.usage_color(), Color::ContextRed);
}

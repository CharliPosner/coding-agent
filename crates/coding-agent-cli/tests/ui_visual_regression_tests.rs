//! Visual regression tests for UI components
//!
//! These tests create snapshots of all UI components to detect visual changes.
//! They test components in isolation to ensure consistent rendering across changes.

use coding_agent_cli::ui::components::MessageBox;
use coding_agent_cli::ui::progress::ProgressBar;
use coding_agent_cli::ui::spinner::Spinner;
use coding_agent_cli::ui::theme::{Color, Theme, ThemeStyle};
use coding_agent_cli::ui::tool_result::ToolResultFormatter;
use insta::assert_snapshot;

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

// ============================================================================
// Spinner Component Tests
// ============================================================================

#[test]
fn test_spinner_message_cycle() {
    let messages = vec!["Message 1", "Message 2", "Message 3"];
    let mut spinner = Spinner::with_messages(messages);

    // Capture several message transitions
    let mut result = String::new();
    for i in 0..5 {
        result.push_str(&format!("Frame {}: {}\n", i, spinner.current_message()));
        spinner.next_message();
    }

    assert_snapshot!("spinner_message_cycle", normalize_whitespace(&result));
}

#[test]
fn test_spinner_default_messages() {
    let spinner = Spinner::new();
    let current = spinner.current_message();

    assert!(!current.is_empty(), "Default spinner should have a message");
    assert_snapshot!("spinner_default_first_message", current);
}

#[test]
fn test_spinner_with_static_message() {
    let spinner = Spinner::with_message("Processing files...");
    let message = spinner.current_message();

    assert_snapshot!("spinner_static_message", message);
}

#[test]
fn test_spinner_empty_messages() {
    let spinner = Spinner::with_messages(vec![]);
    let message = spinner.current_message();

    assert_snapshot!("spinner_empty_messages", message);
}

// ============================================================================
// Progress Bar Component Tests
// ============================================================================

#[test]
fn test_progress_bar_percentages() {
    let bar = ProgressBar::new(100);

    let mut result = String::new();
    for percent in &[0, 25, 50, 75, 100] {
        bar.set_position(*percent);
        result.push_str(&format!("{}%: position={}\n", percent, bar.position()));
    }

    assert_snapshot!("progress_bar_percentages", normalize_whitespace(&result));
}

#[test]
fn test_progress_bar_increment() {
    let bar = ProgressBar::new(100);

    let mut result = String::new();
    result.push_str(&format!("Initial: {}\n", bar.position()));

    bar.inc(10);
    result.push_str(&format!("After +10: {}\n", bar.position()));

    bar.inc(25);
    result.push_str(&format!("After +25: {}\n", bar.position()));

    bar.inc(65);
    result.push_str(&format!("After +65: {} (percent: {}%)\n", bar.position(), bar.percent()));

    assert_snapshot!("progress_bar_increment", normalize_whitespace(&result));
}

#[test]
fn test_progress_bar_context_style() {
    let bar = ProgressBar::context_bar(200_000);

    let mut result = String::new();
    result.push_str(&format!("Total: {:?}\n", bar.length()));

    // Test at key thresholds
    bar.set_position(50_000); // 25%
    result.push_str(&format!("At 50k tokens: {}%\n", bar.percent()));

    bar.set_position(120_000); // 60%
    result.push_str(&format!("At 120k tokens: {}%\n", bar.percent()));

    bar.set_position(170_000); // 85%
    result.push_str(&format!("At 170k tokens: {}%\n", bar.percent()));

    assert_snapshot!("progress_bar_context_style", normalize_whitespace(&result));
}

// ============================================================================
// Message Box Component Tests
// ============================================================================

#[test]
fn test_message_box_simple() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let output = message_box.info("Simple message");
    let clean = strip_ansi(&output);
    assert_snapshot!("message_box_simple", normalize_whitespace(&clean));
}

#[test]
fn test_message_box_multi_line() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let content = "Line 1: First line of message\nLine 2: Second line\nLine 3: Third line";
    let output = message_box.info(content);
    let clean = strip_ansi(&output);
    assert_snapshot!("message_box_multi_line", normalize_whitespace(&clean));
}

#[test]
fn test_message_box_with_title() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let output = message_box.render(Some("Important"), "This is the content");
    let clean = strip_ansi(&output);
    assert_snapshot!("message_box_with_title", normalize_whitespace(&clean));
}

#[test]
fn test_message_box_wide_content() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let wide_content = "This is a very long message that should wrap properly or be displayed in a box that accommodates it";
    let output = message_box.info(wide_content);
    let clean = strip_ansi(&output);
    assert_snapshot!("message_box_wide_content", normalize_whitespace(&clean));
}

#[test]
fn test_message_box_commit_message_style() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let output = message_box.commit_message(
        "Add user authentication flow",
        "This enables users to securely log in before accessing\nsensitive data. Implements JWT-based session management."
    );
    let clean = strip_ansi(&output);
    assert_snapshot!("message_box_commit_style", normalize_whitespace(&clean));
}

#[test]
fn test_message_box_error() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let output = message_box.error("File not found", "/path/to/file.rs");
    let clean = strip_ansi(&output);
    assert_snapshot!("message_box_error", normalize_whitespace(&clean));
}

// ============================================================================
// Tool Result Formatter Tests
// ============================================================================

#[test]
fn test_tool_result_file_content_short() {
    let formatter = ToolResultFormatter::new();
    let content = "fn main() {\n    println!(\"Hello, World!\");\n}";
    let result = formatter.format_result("read_file", content);
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_file_content_short", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_file_content_truncated() {
    use coding_agent_cli::ui::tool_result::ToolResultConfig;

    let config = ToolResultConfig {
        max_display_lines: 3,
        enable_highlighting: false,
        show_line_numbers: false,
    };
    let formatter = ToolResultFormatter::with_config(config);

    let content = (0..10).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    let result = formatter.format_result("read_file", &content);
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_file_content_truncated", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_write_success() {
    let formatter = ToolResultFormatter::new();
    let result = formatter.format_result("write_file", "File written successfully");
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_write_success", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_edit_ok() {
    let formatter = ToolResultFormatter::new();
    let result = formatter.format_result("edit_file", "OK");
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_edit_ok", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_file_list() {
    let formatter = ToolResultFormatter::new();
    let files = r#"["src/", "tests/", "Cargo.toml", "README.md"]"#;
    let result = formatter.format_result("list_files", files);
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_file_list", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_bash_output_empty() {
    let formatter = ToolResultFormatter::new();
    let result = formatter.format_result("bash", "");
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_bash_empty", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_bash_output_multi_line() {
    let formatter = ToolResultFormatter::new();
    let output = "Compiling coding-agent v0.1.0\nFinished dev [unoptimized + debuginfo] target(s) in 2.3s";
    let result = formatter.format_result("bash", output);
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_bash_multi_line", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_search_no_matches() {
    let formatter = ToolResultFormatter::new();
    let result = formatter.format_result("code_search", "No matches found");
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_search_no_matches", normalize_whitespace(&clean));
}

#[test]
fn test_tool_result_search_with_matches() {
    let formatter = ToolResultFormatter::new();
    let output = "src/main.rs:10:fn main()\nsrc/lib.rs:25:pub fn test()\ntests/integration.rs:5:fn integration_test()";
    let result = formatter.format_result("code_search", output);
    let clean = strip_ansi(&result);
    assert_snapshot!("tool_result_search_with_matches", normalize_whitespace(&clean));
}

// ============================================================================
// Themed Output Tests - Various Color Combinations
// ============================================================================

#[test]
fn test_theme_all_colors_minimal() {
    let theme = Theme::new(ThemeStyle::Minimal);
    let mut output = String::new();

    output.push_str(&format!("User: {}\n", theme.apply(Color::UserInput, "User input text")));
    output.push_str(&format!("Agent: {}\n", theme.apply(Color::Agent, "Agent response text")));
    output.push_str(&format!("Tool: {}\n", theme.apply(Color::Tool, "Tool execution")));
    output.push_str(&format!("Success: {}\n", theme.apply(Color::Success, "Operation succeeded")));
    output.push_str(&format!("Error: {}\n", theme.apply(Color::Error, "Error occurred")));
    output.push_str(&format!("Warning: {}\n", theme.apply(Color::Warning, "Warning message")));
    output.push_str(&format!("Muted: {}\n", theme.apply(Color::Muted, "Secondary info")));
    output.push_str(&format!("Cost: {}\n", theme.apply(Color::Cost, "$1.50")));

    let clean = strip_ansi(&output);
    assert_snapshot!("theme_all_colors_minimal", normalize_whitespace(&clean));
}

#[test]
fn test_theme_all_colors_colorful() {
    let theme = Theme::new(ThemeStyle::Colorful);
    let mut output = String::new();

    output.push_str(&format!("User: {}\n", theme.apply(Color::UserInput, "User input text")));
    output.push_str(&format!("Agent: {}\n", theme.apply(Color::Agent, "Agent response text")));
    output.push_str(&format!("Tool: {}\n", theme.apply(Color::Tool, "Tool execution")));
    output.push_str(&format!("Success: {}\n", theme.apply(Color::Success, "Operation succeeded")));
    output.push_str(&format!("Error: {}\n", theme.apply(Color::Error, "Error occurred")));
    output.push_str(&format!("Warning: {}\n", theme.apply(Color::Warning, "Warning message")));
    output.push_str(&format!("Muted: {}\n", theme.apply(Color::Muted, "Secondary info")));
    output.push_str(&format!("Cost: {}\n", theme.apply(Color::Cost, "$1.50")));

    let clean = strip_ansi(&output);
    assert_snapshot!("theme_all_colors_colorful", normalize_whitespace(&clean));
}

#[test]
fn test_theme_all_colors_monochrome() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let mut output = String::new();

    output.push_str(&format!("User: {}\n", theme.apply(Color::UserInput, "User input text")));
    output.push_str(&format!("Agent: {}\n", theme.apply(Color::Agent, "Agent response text")));
    output.push_str(&format!("Tool: {}\n", theme.apply(Color::Tool, "Tool execution")));
    output.push_str(&format!("Success: {}\n", theme.apply(Color::Success, "Operation succeeded")));
    output.push_str(&format!("Error: {}\n", theme.apply(Color::Error, "Error occurred")));
    output.push_str(&format!("Warning: {}\n", theme.apply(Color::Warning, "Warning message")));
    output.push_str(&format!("Muted: {}\n", theme.apply(Color::Muted, "Secondary info")));
    output.push_str(&format!("Cost: {}\n", theme.apply(Color::Cost, "$1.50")));

    let clean = strip_ansi(&output);
    assert_snapshot!("theme_all_colors_monochrome", normalize_whitespace(&clean));
}

// ============================================================================
// Edge Case Visual Tests
// ============================================================================

#[test]
fn test_empty_content_handling() {
    let theme = Theme::new(ThemeStyle::Monochrome);

    // Empty message box
    let message_box = MessageBox::new(theme.clone());
    let box_output = strip_ansi(&message_box.info(""));

    // Empty tool result
    let formatter = ToolResultFormatter::new();
    let result_output = strip_ansi(&formatter.format_result("bash", ""));

    let combined = format!(
        "Message Box:\n{}\n\nTool Result:\n{}",
        box_output, result_output
    );

    assert_snapshot!("empty_content_handling", normalize_whitespace(&combined));
}

#[test]
fn test_special_characters_display() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);

    let special_chars = "Testing: <>&\"'`$()[]{}|\\~*?#";
    let output = message_box.info(special_chars);
    let clean = strip_ansi(&output);

    assert_snapshot!("special_characters_display", normalize_whitespace(&clean));
}

#[test]
fn test_unicode_emoji_display() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);

    let unicode_text = "Testing Unicode: 你好 🚀 👍 ✓ ✗ → ← ↑ ↓ 💻 🔥 ⚡ 🎯";
    let output = message_box.info(unicode_text);
    let clean = strip_ansi(&output);

    assert_snapshot!("unicode_emoji_display", normalize_whitespace(&clean));
}

#[test]
fn test_very_long_line_no_newlines() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);

    let long_line = "a".repeat(150);
    let output = message_box.info(&long_line);
    let clean = strip_ansi(&output);

    assert_snapshot!("very_long_line_no_newlines", normalize_whitespace(&clean));
}

// ============================================================================
// Integration Visual Tests - Combined Components
// ============================================================================

#[test]
fn test_complete_tool_execution_sequence() {
    let formatter = ToolResultFormatter::new();
    let mut output = String::new();

    // Simulate a file read operation
    let file_content = "fn hello() {\n    println!(\"Hello, world!\");\n}\n\nfn main() {\n    hello();\n}";
    let result = formatter.format_result("read_file", file_content);
    output.push_str(&result);

    let clean = strip_ansi(&output);
    assert_snapshot!("complete_tool_execution_sequence", normalize_whitespace(&clean));
}

#[test]
fn test_error_display_flow() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);

    let mut output = String::new();

    // Show error
    let error = message_box.error(
        "Build failed",
        "missing dependency 'serde'\n\nHelp: Add serde = \"1.0\" to Cargo.toml"
    );
    output.push_str(&error);
    output.push_str("\n\n");

    // Show fix message
    let fix_msg = message_box.render(
        Some("Auto-Fix"),
        "→ Diagnosing issue...\n→ Found: Cargo.toml missing serde dependency\n→ Applying fix..."
    );
    output.push_str(&fix_msg);

    let clean = strip_ansi(&output);
    assert_snapshot!("error_display_flow", normalize_whitespace(&clean));
}

#[test]
fn test_multiple_file_operations() {
    let formatter = ToolResultFormatter::new();
    let mut output = String::new();

    // Read operation
    output.push_str("=== READ ===\n");
    let read_result = formatter.format_result("read_file", "fn main() {\n    println!(\"test\");\n}");
    output.push_str(&read_result);
    output.push_str("\n");

    // Write operation
    output.push_str("=== WRITE ===\n");
    let write_result = formatter.format_result("write_file", "File written: src/new.rs");
    output.push_str(&write_result);
    output.push_str("\n");

    // Edit operation
    output.push_str("=== EDIT ===\n");
    let edit_result = formatter.format_result("edit_file", "OK");
    output.push_str(&edit_result);

    let clean = strip_ansi(&output);
    assert_snapshot!("multiple_file_operations", normalize_whitespace(&clean));
}

#[test]
fn test_theme_formatting_consistency() {
    let themes = vec![
        ("minimal", ThemeStyle::Minimal),
        ("colorful", ThemeStyle::Colorful),
        ("monochrome", ThemeStyle::Monochrome),
    ];

    let mut output = String::new();

    for (name, style) in themes {
        let theme = Theme::new(style);
        output.push_str(&format!("=== {} ===\n", name));

        let msg_box = MessageBox::new(theme.clone());
        let formatted = msg_box.info("Test message");
        let clean = strip_ansi(&formatted);

        output.push_str(&clean);
        output.push_str("\n\n");
    }

    assert_snapshot!("theme_formatting_consistency", normalize_whitespace(&output));
}

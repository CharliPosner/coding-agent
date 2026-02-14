//! Streamlined visual regression tests for UI components
//!
//! Consolidated from 33 tests to 8 essential tests that provide maximum coverage
//! with minimum redundancy. Focus on core functionality rather than exhaustive edge cases.

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
// Core Component Tests (Essential Functionality Only)
// ============================================================================

#[test]
fn test_spinner_core_functionality() {
    let messages = vec!["Processing...", "Almost done...", "Complete"];
    let mut spinner = Spinner::with_messages(messages);

    // Test message cycling
    let mut result = String::new();
    for i in 0..4 {
        result.push_str(&format!("Step {}: {}\n", i, spinner.current_message()));
        spinner.next_message();
    }

    assert_snapshot!("spinner_core_functionality", normalize_whitespace(&result));
}

#[test]
fn test_progress_bar_core_behavior() {
    let bar = ProgressBar::new(100);

    let mut result = String::new();
    // Test key percentage points
    for percent in &[0, 25, 50, 75, 100] {
        bar.set_position(*percent);
        result.push_str(&format!("{}%: {}\n", percent, bar.position()));
    }

    // Test increment behavior
    bar.set_position(0);
    bar.inc(30);
    result.push_str(&format!("After +30: {}\n", bar.position()));

    assert_snapshot!("progress_bar_core_behavior", normalize_whitespace(&result));
}

#[test]
fn test_message_box_essential_cases() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);

    let mut result = String::new();
    
    // Basic message
    let basic = message_box.info("Simple information message");
    result.push_str("=== BASIC ===\n");
    result.push_str(&strip_ansi(&basic));
    result.push_str("\n\n");

    // Error message
    let error = message_box.error("Operation failed", "Details about the failure");
    result.push_str("=== ERROR ===\n");
    result.push_str(&strip_ansi(&error));
    result.push_str("\n\n");

    // Multi-line content
    let multiline = message_box.info("Line 1\nLine 2\nLine 3");
    result.push_str("=== MULTILINE ===\n");
    result.push_str(&strip_ansi(&multiline));

    assert_snapshot!("message_box_essential_cases", normalize_whitespace(&result));
}

#[test]
fn test_tool_result_formatting() {
    let formatter = ToolResultFormatter::new();
    let mut result = String::new();

    // File content
    let file_content = "fn main() {\n    println!(\"Hello, World!\");\n}";
    let formatted = formatter.format_result("read_file", file_content);
    result.push_str("=== FILE CONTENT ===\n");
    result.push_str(&strip_ansi(&formatted));
    result.push_str("\n\n");

    // Success operation
    let success = formatter.format_result("write_file", "File written successfully");
    result.push_str("=== SUCCESS ===\n");
    result.push_str(&strip_ansi(&success));
    result.push_str("\n\n");

    // Search results
    let search = formatter.format_result("code_search", "src/main.rs:10:fn main()\nsrc/lib.rs:25:pub fn test()");
    result.push_str("=== SEARCH ===\n");
    result.push_str(&strip_ansi(&search));

    assert_snapshot!("tool_result_formatting", normalize_whitespace(&result));
}

#[test]
fn test_context_bar_key_thresholds() {
    use coding_agent_cli::ui::context_bar::ContextBar;
    
    let theme = Theme::new(ThemeStyle::Monochrome);
    let mut result = String::new();

    // Test color transition thresholds: 60% (yellow) and 85% (red)
    let test_cases = vec![
        (50_000, 200_000, "25% - Green"),
        (120_000, 200_000, "60% - Yellow"), 
        (170_000, 200_000, "85% - Red"),
        (200_000, 200_000, "100% - Red"),
    ];

    for (tokens, max_tokens, description) in test_cases {
        let mut bar = ContextBar::with_theme(max_tokens, theme.clone());
        bar.set_tokens(tokens);
        let output = bar.render();
        result.push_str(&format!("{}: {}\n", description, strip_ansi(&output)));
    }

    assert_snapshot!("context_bar_key_thresholds", normalize_whitespace(&result));
}

#[test]
fn test_theme_color_application() {
    // Test ONE theme thoroughly rather than all three themes redundantly
    let theme = Theme::new(ThemeStyle::Colorful);
    let mut result = String::new();

    let test_cases = vec![
        (Color::UserInput, "User input text"),
        (Color::Agent, "Agent response"),
        (Color::Tool, "Tool execution"),
        (Color::Success, "Success message"),
        (Color::Error, "Error message"),
        (Color::Warning, "Warning message"),
        (Color::Muted, "Secondary info"),
        (Color::Cost, "$1.50"),
    ];

    for (color, text) in test_cases {
        let styled = theme.apply(color, text);
        result.push_str(&format!("{:?}: {}\n", color, strip_ansi(&styled)));
    }

    assert_snapshot!("theme_color_application", normalize_whitespace(&result));
}

#[test]
fn test_error_display_workflow() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let mut result = String::new();

    // Simulate error → diagnosis → fix workflow
    let error = message_box.error(
        "Build failed",
        "missing dependency 'serde'\n\nHelp: Add serde = \"1.0\" to Cargo.toml",
    );
    result.push_str("=== ERROR ===\n");
    result.push_str(&strip_ansi(&error));
    result.push_str("\n\n");

    let fix = message_box.render(
        Some("Auto-Fix"),
        "→ Diagnosing issue...\n→ Found: Cargo.toml missing serde dependency\n→ Applying fix...",
    );
    result.push_str("=== AUTO-FIX ===\n");
    result.push_str(&strip_ansi(&fix));

    assert_snapshot!("error_display_workflow", normalize_whitespace(&result));
}

#[test]
fn test_edge_case_handling() {
    let theme = Theme::new(ThemeStyle::Monochrome);
    let message_box = MessageBox::new(theme);
    let formatter = ToolResultFormatter::new();
    
    let mut result = String::new();

    // Empty content
    let empty_msg = message_box.info("");
    result.push_str("=== EMPTY MESSAGE ===\n");
    result.push_str(&strip_ansi(&empty_msg));
    result.push_str("\n\n");

    // Empty tool result
    let empty_tool = formatter.format_result("bash", "");
    result.push_str("=== EMPTY TOOL RESULT ===\n");
    result.push_str(&strip_ansi(&empty_tool));
    result.push_str("\n\n");

    // Long line (test truncation behavior)
    let long_line = "x".repeat(120);
    let long_msg = message_box.info(&long_line);
    result.push_str("=== LONG LINE ===\n");
    result.push_str(&strip_ansi(&long_msg));

    assert_snapshot!("edge_case_handling", normalize_whitespace(&result));
}
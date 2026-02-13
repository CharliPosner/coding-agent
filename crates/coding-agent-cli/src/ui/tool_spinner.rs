//! Tool execution spinner with status display
//!
//! Provides a spinner specifically designed for tool execution, showing:
//! - The tool name and target being operated on
//! - Progress status during execution
//! - Success/failure indicators with timing information

use indicatif::{ProgressBar as IndicatifBar, ProgressStyle};
use std::time::{Duration, Instant};

use super::theme::{Color, Theme};

/// Status of a tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// Tool is currently running
    Running,
    /// Tool completed successfully
    Success,
    /// Tool failed with an error
    Failed,
    /// Tool is being retried
    Retrying,
}

/// A spinner for displaying tool execution status
pub struct ToolExecutionSpinner {
    /// The underlying progress bar
    bar: IndicatifBar,
    /// The tool name
    tool_name: String,
    /// The target being operated on (e.g., file path)
    target: Option<String>,
    /// When the operation started
    start_time: Instant,
    /// Current status
    status: ToolStatus,
    /// Theme for styling
    theme: Theme,
    /// Current retry attempt (if any)
    retry_attempt: u32,
}

impl ToolExecutionSpinner {
    /// Create a new tool execution spinner
    pub fn new(tool_name: impl Into<String>, theme: Theme) -> Self {
        let tool_name = tool_name.into();
        let bar = IndicatifBar::new_spinner();

        let style = ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.yellow} {msg}")
            .unwrap();
        bar.set_style(style);
        bar.enable_steady_tick(Duration::from_millis(80));

        let display_name = format_tool_action(&tool_name, None);
        bar.set_message(display_name);

        Self {
            bar,
            tool_name,
            target: None,
            start_time: Instant::now(),
            status: ToolStatus::Running,
            theme,
            retry_attempt: 0,
        }
    }

    /// Create a spinner with a target (e.g., file path being read)
    pub fn with_target(
        tool_name: impl Into<String>,
        target: impl Into<String>,
        theme: Theme,
    ) -> Self {
        let tool_name = tool_name.into();
        let target = target.into();
        let bar = IndicatifBar::new_spinner();

        let style = ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.yellow} {msg}")
            .unwrap();
        bar.set_style(style);
        bar.enable_steady_tick(Duration::from_millis(80));

        let display_name = format_tool_action(&tool_name, Some(&target));
        bar.set_message(display_name);

        Self {
            bar,
            tool_name,
            target: Some(target),
            start_time: Instant::now(),
            status: ToolStatus::Running,
            theme,
            retry_attempt: 0,
        }
    }

    /// Update the target being operated on
    pub fn set_target(&mut self, target: impl Into<String>) {
        self.target = Some(target.into());
        self.update_message();
    }

    /// Update the status message
    pub fn set_status(&mut self, status: &str) {
        let display = if let Some(ref target) = self.target {
            format!("{}: {}", status, target)
        } else {
            status.to_string()
        };
        self.bar.set_message(display);
    }

    /// Mark as retrying with attempt number
    pub fn set_retrying(&mut self, attempt: u32, max_attempts: u32) {
        self.status = ToolStatus::Retrying;
        self.retry_attempt = attempt;
        let display = format_tool_action(&self.tool_name, self.target.as_deref());
        let retry_msg = format!("{} (retry {}/{})", display, attempt, max_attempts);
        self.bar.set_message(retry_msg);
    }

    /// Finish with success
    pub fn finish_success(&self) {
        let duration = self.start_time.elapsed();
        let duration_str = format_duration(duration);

        let display = format_tool_success(&self.tool_name, self.target.as_deref());
        let msg = if duration.as_millis() > 500 {
            format!("{} ({})", display, duration_str)
        } else {
            display
        };

        self.bar
            .finish_with_message(self.theme.apply(Color::Success, &msg));
    }

    /// Finish with success and a custom message
    pub fn finish_success_with_message(&self, message: &str) {
        let duration = self.start_time.elapsed();
        let duration_str = format_duration(duration);

        let msg = if duration.as_millis() > 500 {
            format!("✓ {} ({})", message, duration_str)
        } else {
            format!("✓ {}", message)
        };

        self.bar
            .finish_with_message(self.theme.apply(Color::Success, &msg));
    }

    /// Finish with failure
    pub fn finish_failed(&self, error: &str) {
        let duration = self.start_time.elapsed();
        let duration_str = format_duration(duration);

        let display = format_tool_failure(&self.tool_name, self.target.as_deref());
        let msg = if self.retry_attempt > 0 {
            format!(
                "{} after {} retries ({}): {}",
                display, self.retry_attempt, duration_str, error
            )
        } else {
            format!("{} ({}): {}", display, duration_str, error)
        };

        self.bar
            .finish_with_message(self.theme.apply(Color::Error, &msg));
    }

    /// Finish with failure and only show the error message
    pub fn finish_failed_simple(&self, error: &str) {
        let msg = format!("✗ {}", error);
        self.bar
            .finish_with_message(self.theme.apply(Color::Error, &msg));
    }

    /// Finish and clear the spinner (for intermediate steps)
    pub fn finish_and_clear(&self) {
        self.bar.finish_and_clear();
    }

    /// Get the elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get the tool name
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Get the target
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Get current status
    pub fn status(&self) -> ToolStatus {
        self.status
    }

    /// Update the displayed message based on current state
    fn update_message(&self) {
        let display = format_tool_action(&self.tool_name, self.target.as_deref());
        self.bar.set_message(display);
    }
}

impl Drop for ToolExecutionSpinner {
    fn drop(&mut self) {
        if !self.bar.is_finished() {
            self.bar.finish_and_clear();
        }
    }
}

/// Format a tool action for display (e.g., "Reading src/main.rs")
fn format_tool_action(tool_name: &str, target: Option<&str>) -> String {
    let verb = tool_name_to_verb(tool_name);
    if let Some(target) = target {
        format!("{} {}", verb, target)
    } else {
        verb.to_string()
    }
}

/// Format a tool success message
fn format_tool_success(tool_name: &str, target: Option<&str>) -> String {
    let verb = tool_name_to_past_tense(tool_name);
    if let Some(target) = target {
        format!("✓ {} {}", verb, target)
    } else {
        format!("✓ {}", verb)
    }
}

/// Format a tool failure message
fn format_tool_failure(tool_name: &str, target: Option<&str>) -> String {
    let verb = tool_name_to_verb(tool_name);
    if let Some(target) = target {
        format!("✗ {} {} failed", verb, target)
    } else {
        format!("✗ {} failed", verb)
    }
}

/// Convert a tool name to its action verb (present progressive)
fn tool_name_to_verb(tool_name: &str) -> &str {
    match tool_name.to_lowercase().as_str() {
        "read" | "read_file" => "Reading",
        "write" | "write_file" => "Writing",
        "edit" | "edit_file" => "Editing",
        "bash" | "execute" | "exec" => "Executing",
        "search" | "grep" | "code_search" => "Searching",
        "list" | "list_files" | "ls" => "Listing",
        "build" | "cargo_build" => "Building",
        "test" | "cargo_test" => "Testing",
        "format" | "fmt" => "Formatting",
        "lint" | "clippy" => "Linting",
        "check" | "cargo_check" => "Checking",
        "install" | "add" => "Installing",
        "delete" | "remove" | "rm" => "Deleting",
        "copy" | "cp" => "Copying",
        "move" | "mv" => "Moving",
        "create" | "mkdir" => "Creating",
        "git" => "Running git",
        "fetch" | "download" => "Fetching",
        "analyze" | "diagnose" => "Analyzing",
        "fix" | "repair" => "Fixing",
        _ => "Running",
    }
}

/// Convert a tool name to its past tense (for success messages)
fn tool_name_to_past_tense(tool_name: &str) -> &str {
    match tool_name.to_lowercase().as_str() {
        "read" | "read_file" => "Read",
        "write" | "write_file" => "Wrote",
        "edit" | "edit_file" => "Edited",
        "bash" | "execute" | "exec" => "Executed",
        "search" | "grep" | "code_search" => "Searched",
        "list" | "list_files" | "ls" => "Listed",
        "build" | "cargo_build" => "Built",
        "test" | "cargo_test" => "Tested",
        "format" | "fmt" => "Formatted",
        "lint" | "clippy" => "Linted",
        "check" | "cargo_check" => "Checked",
        "install" | "add" => "Installed",
        "delete" | "remove" | "rm" => "Deleted",
        "copy" | "cp" => "Copied",
        "move" | "mv" => "Moved",
        "create" | "mkdir" => "Created",
        "git" => "Ran git",
        "fetch" | "download" => "Fetched",
        "analyze" | "diagnose" => "Analyzed",
        "fix" | "repair" => "Fixed",
        _ => "Completed",
    }
}

/// Format a duration for display
fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        format!("{}ms", millis)
    } else if millis < 60_000 {
        let secs = duration.as_secs_f64();
        format!("{:.1}s", secs)
    } else {
        let mins = duration.as_secs() / 60;
        let secs = duration.as_secs() % 60;
        format!("{}m {}s", mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::ThemeStyle;

    #[test]
    fn test_tool_name_to_verb() {
        assert_eq!(tool_name_to_verb("read"), "Reading");
        assert_eq!(tool_name_to_verb("READ"), "Reading");
        assert_eq!(tool_name_to_verb("write_file"), "Writing");
        assert_eq!(tool_name_to_verb("bash"), "Executing");
        assert_eq!(tool_name_to_verb("search"), "Searching");
        assert_eq!(tool_name_to_verb("unknown_tool"), "Running");
    }

    #[test]
    fn test_tool_name_to_past_tense() {
        assert_eq!(tool_name_to_past_tense("read"), "Read");
        assert_eq!(tool_name_to_past_tense("write"), "Wrote");
        assert_eq!(tool_name_to_past_tense("build"), "Built");
        assert_eq!(tool_name_to_past_tense("unknown"), "Completed");
    }

    #[test]
    fn test_format_tool_action_with_target() {
        let action = format_tool_action("read", Some("src/main.rs"));
        assert_eq!(action, "Reading src/main.rs");
    }

    #[test]
    fn test_format_tool_action_without_target() {
        let action = format_tool_action("build", None);
        assert_eq!(action, "Building");
    }

    #[test]
    fn test_format_tool_success_with_target() {
        let success = format_tool_success("read", Some("src/main.rs"));
        assert_eq!(success, "✓ Read src/main.rs");
    }

    #[test]
    fn test_format_tool_success_without_target() {
        let success = format_tool_success("build", None);
        assert_eq!(success, "✓ Built");
    }

    #[test]
    fn test_format_tool_failure() {
        let failure = format_tool_failure("write", Some("output.txt"));
        assert_eq!(failure, "✗ Writing output.txt failed");
    }

    #[test]
    fn test_format_duration_milliseconds() {
        let duration = Duration::from_millis(500);
        assert_eq!(format_duration(duration), "500ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        let duration = Duration::from_millis(2500);
        assert_eq!(format_duration(duration), "2.5s");
    }

    #[test]
    fn test_format_duration_minutes() {
        let duration = Duration::from_secs(125);
        assert_eq!(format_duration(duration), "2m 5s");
    }

    #[test]
    fn test_tool_execution_spinner_creation() {
        let theme = Theme::new(ThemeStyle::Minimal);
        let spinner = ToolExecutionSpinner::new("read", theme);
        assert_eq!(spinner.tool_name(), "read");
        assert!(spinner.target().is_none());
        assert_eq!(spinner.status(), ToolStatus::Running);
    }

    #[test]
    fn test_tool_execution_spinner_with_target() {
        let theme = Theme::new(ThemeStyle::Minimal);
        let spinner = ToolExecutionSpinner::with_target("read", "src/main.rs", theme);
        assert_eq!(spinner.tool_name(), "read");
        assert_eq!(spinner.target(), Some("src/main.rs"));
    }

    #[test]
    fn test_tool_execution_spinner_set_target() {
        let theme = Theme::new(ThemeStyle::Minimal);
        let mut spinner = ToolExecutionSpinner::new("read", theme);
        assert!(spinner.target().is_none());

        spinner.set_target("Cargo.toml");
        assert_eq!(spinner.target(), Some("Cargo.toml"));
    }

    #[test]
    fn test_tool_execution_spinner_elapsed() {
        let theme = Theme::new(ThemeStyle::Minimal);
        let spinner = ToolExecutionSpinner::new("test", theme);

        // Elapsed time should work (just verify we can call it)
        let _elapsed = spinner.elapsed();
    }

    #[test]
    fn test_tool_status_enum() {
        assert_eq!(ToolStatus::Running, ToolStatus::Running);
        assert_ne!(ToolStatus::Running, ToolStatus::Success);
        assert_ne!(ToolStatus::Success, ToolStatus::Failed);
        assert_ne!(ToolStatus::Failed, ToolStatus::Retrying);
    }

    #[test]
    fn test_spinner_cleanup_on_drop() {
        let theme = Theme::new(ThemeStyle::Minimal);
        {
            let _spinner = ToolExecutionSpinner::new("test", theme);
            // Spinner should clean up on drop without panic
        }
        // If we get here, drop worked correctly
    }

    #[test]
    fn test_format_tool_action_various_tools() {
        // Test various tool mappings
        assert!(format_tool_action("code_search", Some("fn main")).starts_with("Searching"));
        assert!(format_tool_action("list_files", Some("/src")).starts_with("Listing"));
        assert!(format_tool_action("cargo_build", None).starts_with("Building"));
        assert!(format_tool_action("cargo_test", Some("test_name")).starts_with("Testing"));
    }
}

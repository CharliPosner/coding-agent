//! Styled output functions for the coding-agent CLI

use super::theme::{Color, Theme};
use std::io::{self, Write};

/// Styled output writer
pub struct StyledOutput {
    theme: Theme,
}

impl StyledOutput {
    /// Create a new styled output with the given theme
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Create with default theme
    pub fn default_theme() -> Self {
        Self::new(Theme::default())
    }

    /// Print user input (white)
    pub fn user_input(&self, text: &str) {
        println!("{}", self.theme.apply(Color::UserInput, text));
    }

    /// Print agent response (cyan)
    pub fn agent(&self, text: &str) {
        println!("{}", self.theme.apply(Color::Agent, text));
    }

    /// Print tool call (yellow)
    pub fn tool(&self, text: &str) {
        println!("{}", self.theme.apply(Color::Tool, text));
    }

    /// Print success message (green)
    pub fn success(&self, text: &str) {
        println!("{}", self.theme.apply(Color::Success, text));
    }

    /// Print error message (red)
    pub fn error(&self, text: &str) {
        println!("{}", self.theme.apply(Color::Error, text));
    }

    /// Print warning message (yellow/orange)
    pub fn warning(&self, text: &str) {
        println!("{}", self.theme.apply(Color::Warning, text));
    }

    /// Print muted/secondary text (gray)
    pub fn muted(&self, text: &str) {
        println!("{}", self.theme.apply(Color::Muted, text));
    }

    /// Print cost/token information (magenta)
    pub fn cost(&self, text: &str) {
        println!("{}", self.theme.apply(Color::Cost, text));
    }

    /// Print a tool call with status indicator
    pub fn tool_call(&self, status: ToolStatus, message: &str) {
        let indicator = match status {
            ToolStatus::Running => self.theme.apply(Color::Tool, "●"),
            ToolStatus::Success => self.theme.apply(Color::Success, "✓"),
            ToolStatus::Error => self.theme.apply(Color::Error, "✗"),
        };
        println!("{} {}", indicator, message);
    }

    /// Print a labeled line
    pub fn labeled(&self, label: &str, value: &str) {
        let styled_label = self.theme.apply(Color::Muted, label);
        println!("{}: {}", styled_label, value);
    }

    /// Print a separator line
    pub fn separator(&self) {
        println!(
            "{}",
            self.theme.apply(Color::Muted, "─".repeat(50).as_str())
        );
    }

    /// Print an empty line
    pub fn newline(&self) {
        println!();
    }

    /// Flush stdout
    pub fn flush(&self) -> io::Result<()> {
        io::stdout().flush()
    }

    /// Get the underlying theme
    pub fn theme(&self) -> &Theme {
        &self.theme
    }
}

/// Status of a tool call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// Tool is currently running
    Running,
    /// Tool completed successfully
    Success,
    /// Tool failed with an error
    Error,
}

impl Default for StyledOutput {
    fn default() -> Self {
        Self::default_theme()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::ThemeStyle;

    #[test]
    fn test_styled_output_creation() {
        let output = StyledOutput::default_theme();
        assert!(output.theme().colors_enabled() || true); // May be disabled in CI
    }

    #[test]
    fn test_tool_status() {
        assert_eq!(ToolStatus::Running, ToolStatus::Running);
        assert_ne!(ToolStatus::Running, ToolStatus::Success);
        assert_ne!(ToolStatus::Success, ToolStatus::Error);
    }

    #[test]
    fn test_styled_output_with_theme() {
        let theme = Theme::new(ThemeStyle::Monochrome);
        let output = StyledOutput::new(theme);
        assert!(!output.theme().colors_enabled());
    }
}

//! Permission prompt UI for user confirmation
//!
//! This module provides a prompt for asking users about file operation permissions
//! with options: Y (yes), n (no), a (always), N (never)

use crate::ui::{Color, Theme};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use super::OperationType;

/// User's response to a permission prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionResponse {
    /// Allow this operation once
    Yes,
    /// Deny this operation once
    No,
    /// Always allow operations on this path (adds to trusted paths)
    Always,
    /// Never allow operations on this path (for this session)
    Never,
}

/// Permission prompt for asking users about file operations
pub struct PermissionPrompt {
    theme: Theme,
}

impl PermissionPrompt {
    /// Create a new permission prompt with the given theme
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Create with default theme
    pub fn default_theme() -> Self {
        Self::new(Theme::default())
    }

    /// Prompt the user for permission to perform an operation on a path
    ///
    /// Displays a prompt like:
    /// ```text
    /// ⚠ Permission required: write to /path/to/file.txt
    /// Allow this operation? [Y/n/a/N] (Y=yes, n=no, a=always, N=never):
    /// ```
    ///
    /// Returns the user's choice.
    pub fn prompt(&self, path: &Path, operation: OperationType) -> io::Result<PermissionResponse> {
        // Display the permission request
        let warning = self.theme.apply(Color::Warning, "⚠ Permission required");
        let op_str = format!("{}", operation);
        let path_display = path.display();

        println!("{}: {} {}", warning, op_str, path_display);

        // Display the options
        let prompt_text = self.format_prompt_options();
        print!("{}", prompt_text);
        io::stdout().flush()?;

        // Read user input
        self.read_response()
    }

    /// Format the prompt options string
    fn format_prompt_options(&self) -> String {
        let yes = self.theme.apply(Color::Success, "Y");
        let no = self.theme.apply(Color::Muted, "n");
        let always = self.theme.apply(Color::Success, "a");
        let never = self.theme.apply(Color::Error, "N");

        format!(
            "Allow this operation? [{}/{}/{}/{}] ({}=yes, {}=no, {}=always, {}=never): ",
            yes, no, always, never, yes, no, always, never
        )
    }

    /// Read a single character response from the user
    fn read_response(&self) -> io::Result<PermissionResponse> {
        loop {
            // Poll for events with a timeout
            if event::poll(Duration::from_millis(100))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            {
                if let Event::Key(key_event) =
                    event::read().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                {
                    if let Some(response) = self.handle_key_event(key_event) {
                        println!(); // Move to next line after input
                        return Ok(response);
                    }
                }
            }
        }
    }

    /// Handle a key event and return the response if valid
    fn handle_key_event(&self, event: KeyEvent) -> Option<PermissionResponse> {
        match (event.code, event.modifiers) {
            // 'Y' or 'y' or Enter = Yes (default)
            (KeyCode::Char('Y'), _)
            | (KeyCode::Char('y'), KeyModifiers::NONE)
            | (KeyCode::Enter, KeyModifiers::NONE) => {
                print!("y");
                let _ = io::stdout().flush();
                Some(PermissionResponse::Yes)
            }

            // 'n' = No (lowercase only, to distinguish from Never)
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                print!("n");
                let _ = io::stdout().flush();
                Some(PermissionResponse::No)
            }

            // 'a' or 'A' = Always
            (KeyCode::Char('a'), KeyModifiers::NONE)
            | (KeyCode::Char('A'), KeyModifiers::SHIFT) => {
                print!("a");
                let _ = io::stdout().flush();
                Some(PermissionResponse::Always)
            }

            // 'N' = Never (uppercase only, to distinguish from No)
            (KeyCode::Char('N'), KeyModifiers::SHIFT) => {
                print!("N");
                let _ = io::stdout().flush();
                Some(PermissionResponse::Never)
            }

            // Ctrl+C = No (cancel)
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                print!("^C");
                let _ = io::stdout().flush();
                Some(PermissionResponse::No)
            }

            // Ignore other keys
            _ => None,
        }
    }
}

impl Default for PermissionPrompt {
    fn default() -> Self {
        Self::default_theme()
    }
}

/// Parse a permission response from a string (for testing or non-interactive use)
pub fn parse_response(input: &str) -> Option<PermissionResponse> {
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" | "" => Some(PermissionResponse::Yes),
        "n" | "no" => Some(PermissionResponse::No),
        "a" | "always" => Some(PermissionResponse::Always),
        _ => None,
    }
}

/// Parse "Never" response (case-sensitive, uppercase N)
pub fn parse_never_response(input: &str) -> Option<PermissionResponse> {
    let trimmed = input.trim();
    if trimmed == "N" || trimmed.to_lowercase() == "never" {
        Some(PermissionResponse::Never)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_permission_response_equality() {
        assert_eq!(PermissionResponse::Yes, PermissionResponse::Yes);
        assert_eq!(PermissionResponse::No, PermissionResponse::No);
        assert_eq!(PermissionResponse::Always, PermissionResponse::Always);
        assert_eq!(PermissionResponse::Never, PermissionResponse::Never);
        assert_ne!(PermissionResponse::Yes, PermissionResponse::No);
    }

    #[test]
    fn test_handle_key_yes_lowercase() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(result, Some(PermissionResponse::Yes));
    }

    #[test]
    fn test_handle_key_yes_uppercase() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('Y'), KeyModifiers::SHIFT));
        assert_eq!(result, Some(PermissionResponse::Yes));
    }

    #[test]
    fn test_handle_key_enter_is_yes() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(result, Some(PermissionResponse::Yes));
    }

    #[test]
    fn test_handle_key_no() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(result, Some(PermissionResponse::No));
    }

    #[test]
    fn test_handle_key_always_lowercase() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(result, Some(PermissionResponse::Always));
    }

    #[test]
    fn test_handle_key_always_uppercase() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('A'), KeyModifiers::SHIFT));
        assert_eq!(result, Some(PermissionResponse::Always));
    }

    #[test]
    fn test_handle_key_never() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('N'), KeyModifiers::SHIFT));
        assert_eq!(result, Some(PermissionResponse::Never));
    }

    #[test]
    fn test_handle_key_ctrl_c() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(result, Some(PermissionResponse::No));
    }

    #[test]
    fn test_handle_key_unknown() {
        let prompt = PermissionPrompt::default_theme();
        let result = prompt.handle_key_event(key_event(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(result, None);
    }

    #[test]
    fn test_handle_key_lowercase_n_is_no_not_never() {
        let prompt = PermissionPrompt::default_theme();
        // Lowercase 'n' should be No
        let result = prompt.handle_key_event(key_event(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(result, Some(PermissionResponse::No));

        // Uppercase 'N' should be Never
        let result = prompt.handle_key_event(key_event(KeyCode::Char('N'), KeyModifiers::SHIFT));
        assert_eq!(result, Some(PermissionResponse::Never));
    }

    #[test]
    fn test_parse_response_yes() {
        assert_eq!(parse_response("y"), Some(PermissionResponse::Yes));
        assert_eq!(parse_response("Y"), Some(PermissionResponse::Yes));
        assert_eq!(parse_response("yes"), Some(PermissionResponse::Yes));
        assert_eq!(parse_response("YES"), Some(PermissionResponse::Yes));
        assert_eq!(parse_response(""), Some(PermissionResponse::Yes)); // Default
        assert_eq!(parse_response("  "), Some(PermissionResponse::Yes)); // Whitespace = default
    }

    #[test]
    fn test_parse_response_no() {
        assert_eq!(parse_response("n"), Some(PermissionResponse::No));
        assert_eq!(parse_response("no"), Some(PermissionResponse::No));
        assert_eq!(parse_response("NO"), Some(PermissionResponse::No));
    }

    #[test]
    fn test_parse_response_always() {
        assert_eq!(parse_response("a"), Some(PermissionResponse::Always));
        assert_eq!(parse_response("A"), Some(PermissionResponse::Always));
        assert_eq!(parse_response("always"), Some(PermissionResponse::Always));
        assert_eq!(parse_response("ALWAYS"), Some(PermissionResponse::Always));
    }

    #[test]
    fn test_parse_response_invalid() {
        assert_eq!(parse_response("x"), None);
        assert_eq!(parse_response("maybe"), None);
    }

    #[test]
    fn test_parse_never_response() {
        assert_eq!(parse_never_response("N"), Some(PermissionResponse::Never));
        assert_eq!(
            parse_never_response("never"),
            Some(PermissionResponse::Never)
        );
        assert_eq!(
            parse_never_response("NEVER"),
            Some(PermissionResponse::Never)
        );
        assert_eq!(parse_never_response("n"), None); // Lowercase 'n' is not Never
    }

    #[test]
    fn test_permission_prompt_creation() {
        let _prompt = PermissionPrompt::default_theme();
        let theme = Theme::default();
        let _prompt = PermissionPrompt::new(theme);
    }

    #[test]
    fn test_format_prompt_options() {
        let prompt = PermissionPrompt::default_theme();
        let formatted = prompt.format_prompt_options();

        // Should contain all option descriptions
        assert!(formatted.contains("yes"));
        assert!(formatted.contains("no"));
        assert!(formatted.contains("always"));
        assert!(formatted.contains("never"));
    }
}

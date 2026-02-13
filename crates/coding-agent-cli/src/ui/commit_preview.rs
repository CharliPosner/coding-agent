//! Commit message preview and edit component
//!
//! This component displays a generated commit message and allows
//! the user to preview, edit, or cancel the commit.

use super::components::MessageBox;
use super::theme::{Color, Theme};
use crossterm::{
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{stdout, Write};

/// Result of the commit preview interaction
#[derive(Debug, Clone, PartialEq)]
pub enum CommitPreviewResult {
    /// User confirmed the commit with the message (possibly edited)
    Confirmed(String),
    /// User wants to edit the message
    Edit,
    /// User cancelled the commit
    Cancelled,
}

/// Interactive commit message preview
pub struct CommitPreview {
    /// The commit message to preview
    message: String,
    /// Files that will be committed
    files: Vec<String>,
    /// Theme for styling
    theme: Theme,
}

impl CommitPreview {
    /// Create a new commit preview
    pub fn new(message: String, files: Vec<String>) -> Self {
        Self {
            message,
            files,
            theme: Theme::default(),
        }
    }

    /// Set the theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Get the current message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Set a new message (after editing)
    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }

    /// Run the interactive preview
    pub fn run(&self) -> std::io::Result<CommitPreviewResult> {
        enable_raw_mode()?;
        let result = self.run_inner();
        disable_raw_mode()?;

        // Ensure cursor is visible
        execute!(stdout(), Show)?;

        result
    }

    fn run_inner(&self) -> std::io::Result<CommitPreviewResult> {
        let mut stdout = stdout();

        // Hide cursor during preview
        execute!(stdout, Hide)?;

        // Render the preview
        let lines_rendered = self.render(&mut stdout)?;

        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Enter => {
                            self.clear_display(&mut stdout, lines_rendered)?;
                            return Ok(CommitPreviewResult::Confirmed(self.message.clone()));
                        }
                        KeyCode::Char('e') | KeyCode::Char('E') => {
                            self.clear_display(&mut stdout, lines_rendered)?;
                            return Ok(CommitPreviewResult::Edit);
                        }
                        KeyCode::Esc => {
                            self.clear_display(&mut stdout, lines_rendered)?;
                            return Ok(CommitPreviewResult::Cancelled);
                        }
                        KeyCode::Char('c')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            self.clear_display(&mut stdout, lines_rendered)?;
                            return Ok(CommitPreviewResult::Cancelled);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn render(&self, stdout: &mut impl Write) -> std::io::Result<usize> {
        let mut lines = 0;

        // Render header
        let header = self.theme.bold(Color::Agent).apply_to("Commit Preview");
        execute!(stdout, Print(format!("{}\n\n", header)))?;
        lines += 2;

        // Render commit message in a box
        let msg_box = MessageBox::new(self.theme.clone());

        // Split message into title and body
        let message_parts: Vec<&str> = self.message.splitn(2, "\n\n").collect();
        let title = message_parts.first().unwrap_or(&"");
        let body = message_parts.get(1).unwrap_or(&"");

        let box_output = msg_box.commit_message(title, body);
        for line in box_output.lines() {
            execute!(stdout, Print(format!("{}\n", line)))?;
            lines += 1;
        }

        // Blank line
        execute!(stdout, Print("\n"))?;
        lines += 1;

        // Render files to be committed
        let files_header = self.theme.apply(
            Color::Muted,
            &format!("Files to commit ({}):", self.files.len()),
        );
        execute!(stdout, Print(format!("{}\n", files_header)))?;
        lines += 1;

        for file in &self.files {
            let file_line = format!("  • {}", file);
            let styled = self.theme.apply(Color::UserInput, &file_line);
            execute!(stdout, Print(format!("{}\n", styled)))?;
            lines += 1;
        }

        // Blank line
        execute!(stdout, Print("\n"))?;
        lines += 1;

        // Render footer with options
        let footer = self.theme.apply(
            Color::Muted,
            "[Enter] Commit • [e] Edit message • [Esc] Cancel",
        );
        execute!(stdout, Print(footer))?;
        lines += 1;

        stdout.flush()?;
        Ok(lines)
    }

    fn clear_display(&self, stdout: &mut impl Write, lines: usize) -> std::io::Result<()> {
        execute!(stdout, MoveToColumn(0))?;

        for _ in 0..lines {
            execute!(stdout, Clear(ClearType::CurrentLine), MoveUp(1))?;
        }
        execute!(stdout, Clear(ClearType::CurrentLine))?;

        stdout.flush()?;
        Ok(())
    }
}

/// Edit a commit message using the user's preferred editor
///
/// Opens the message in $EDITOR (or vim as fallback) and returns
/// the edited message. Returns None if the edit was cancelled or failed.
pub fn edit_commit_message(message: &str) -> std::io::Result<Option<String>> {
    use std::env;
    use std::fs;
    use std::process::Command;

    // Create a temporary file with the commit message
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join("COMMIT_EDITMSG");

    // Write the message with instructions
    let content = format!(
        "{}\n\n# Edit the commit message above.\n# Lines starting with '#' will be ignored.\n# An empty message will abort the commit.\n",
        message
    );
    fs::write(&temp_file, &content)?;

    // Determine the editor to use
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    // Run the editor
    let status = Command::new(&editor).arg(&temp_file).status()?;

    if !status.success() {
        // Editor exited with error
        fs::remove_file(&temp_file).ok();
        return Ok(None);
    }

    // Read the edited content
    let edited_content = fs::read_to_string(&temp_file)?;

    // Clean up
    fs::remove_file(&temp_file).ok();

    // Process the content: remove comment lines and trim
    let processed: Vec<&str> = edited_content
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();

    let final_message = processed.join("\n").trim().to_string();

    if final_message.is_empty() {
        // Empty message means abort
        return Ok(None);
    }

    Ok(Some(final_message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_preview_new() {
        let preview = CommitPreview::new(
            "Add new feature\n\nThis adds a feature.".to_string(),
            vec!["src/main.rs".to_string()],
        );
        assert_eq!(preview.message, "Add new feature\n\nThis adds a feature.");
        assert_eq!(preview.files.len(), 1);
    }

    #[test]
    fn test_commit_preview_message_getter() {
        let preview = CommitPreview::new("Test message".to_string(), vec![]);
        assert_eq!(preview.message(), "Test message");
    }

    #[test]
    fn test_commit_preview_set_message() {
        let mut preview = CommitPreview::new("Original".to_string(), vec![]);
        preview.set_message("Updated".to_string());
        assert_eq!(preview.message(), "Updated");
    }

    #[test]
    fn test_commit_preview_with_theme() {
        let theme = Theme::default();
        let preview = CommitPreview::new("Test".to_string(), vec![]).with_theme(theme);
        // Just verify it doesn't panic
        assert_eq!(preview.message(), "Test");
    }

    #[test]
    fn test_commit_preview_result_variants() {
        let confirmed = CommitPreviewResult::Confirmed("message".to_string());
        let edit = CommitPreviewResult::Edit;
        let cancelled = CommitPreviewResult::Cancelled;

        assert!(matches!(confirmed, CommitPreviewResult::Confirmed(_)));
        assert!(matches!(edit, CommitPreviewResult::Edit));
        assert!(matches!(cancelled, CommitPreviewResult::Cancelled));
    }

    #[test]
    fn test_commit_preview_result_equality() {
        let a = CommitPreviewResult::Confirmed("msg".to_string());
        let b = CommitPreviewResult::Confirmed("msg".to_string());
        let c = CommitPreviewResult::Confirmed("other".to_string());

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(CommitPreviewResult::Edit, CommitPreviewResult::Edit);
        assert_eq!(
            CommitPreviewResult::Cancelled,
            CommitPreviewResult::Cancelled
        );
        assert_ne!(CommitPreviewResult::Edit, CommitPreviewResult::Cancelled);
    }

    #[test]
    fn test_commit_preview_multiple_files() {
        let files = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "tests/test.rs".to_string(),
        ];
        let preview = CommitPreview::new("Test".to_string(), files);
        assert_eq!(preview.files.len(), 3);
    }
}

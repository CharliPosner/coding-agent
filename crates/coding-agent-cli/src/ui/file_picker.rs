//! Interactive file picker component for selecting files
//!
//! This component provides a checkbox-based file selection UI
//! that allows users to navigate with arrow keys and toggle
//! selections with space.

use super::theme::{Color, Theme};
use crossterm::{
    cursor::{Hide, MoveDown, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{stdout, Write};

/// A file entry in the picker
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Path to display
    pub path: String,
    /// Status indicator (e.g., "M ", "??")
    pub status: String,
    /// Whether this file is currently selected
    pub selected: bool,
}

impl FileEntry {
    /// Create a new file entry
    pub fn new(path: String, status: String) -> Self {
        Self {
            path,
            status,
            selected: false,
        }
    }

    /// Create a new file entry that's pre-selected
    pub fn new_selected(path: String, status: String) -> Self {
        Self {
            path,
            status,
            selected: true,
        }
    }
}

/// Result of the file picker interaction
#[derive(Debug)]
pub enum FilePickerResult {
    /// User confirmed selection
    Selected(Vec<String>),
    /// User cancelled
    Cancelled,
}

/// Interactive file picker with checkboxes
pub struct FilePicker {
    /// Files to display
    entries: Vec<FileEntry>,
    /// Current cursor position
    cursor: usize,
    /// Theme for styling
    theme: Theme,
    /// Header text
    header: String,
    /// Footer help text
    footer: String,
}

impl FilePicker {
    /// Create a new file picker
    pub fn new(entries: Vec<FileEntry>) -> Self {
        Self {
            entries,
            cursor: 0,
            theme: Theme::default(),
            header: "Select files to commit:".to_string(),
            footer: "[Space] toggle • [↑/↓] navigate • [Enter] confirm • [Esc] cancel • [a] all • [n] none".to_string(),
        }
    }

    /// Set the header text
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header = header.into();
        self
    }

    /// Set the footer help text
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = footer.into();
        self
    }

    /// Set the theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Get the number of selected entries
    fn selected_count(&self) -> usize {
        self.entries.iter().filter(|e| e.selected).count()
    }

    /// Run the interactive picker
    pub fn run(&mut self) -> std::io::Result<FilePickerResult> {
        if self.entries.is_empty() {
            return Ok(FilePickerResult::Selected(vec![]));
        }

        enable_raw_mode()?;
        let result = self.run_inner();
        disable_raw_mode()?;

        // Ensure cursor is visible
        execute!(stdout(), Show)?;

        result
    }

    fn run_inner(&mut self) -> std::io::Result<FilePickerResult> {
        let mut stdout = stdout();

        // Hide cursor during selection
        execute!(stdout, Hide)?;

        // Initial render
        self.render(&mut stdout)?;

        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.cursor > 0 {
                                self.cursor -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.cursor < self.entries.len() - 1 {
                                self.cursor += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            // Toggle selection
                            self.entries[self.cursor].selected =
                                !self.entries[self.cursor].selected;
                        }
                        KeyCode::Char('a') => {
                            // Select all
                            for entry in &mut self.entries {
                                entry.selected = true;
                            }
                        }
                        KeyCode::Char('n') => {
                            // Deselect all
                            for entry in &mut self.entries {
                                entry.selected = false;
                            }
                        }
                        KeyCode::Enter => {
                            // Clear the picker display
                            self.clear_display(&mut stdout)?;

                            // Return selected files
                            let selected: Vec<String> = self
                                .entries
                                .iter()
                                .filter(|e| e.selected)
                                .map(|e| e.path.clone())
                                .collect();
                            return Ok(FilePickerResult::Selected(selected));
                        }
                        KeyCode::Esc => {
                            // Clear the picker display
                            self.clear_display(&mut stdout)?;
                            return Ok(FilePickerResult::Cancelled);
                        }
                        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Ctrl+C also cancels
                            self.clear_display(&mut stdout)?;
                            return Ok(FilePickerResult::Cancelled);
                        }
                        _ => {}
                    }

                    // Re-render after each key press
                    self.render(&mut stdout)?;
                }
            }
        }
    }

    fn render(&self, stdout: &mut impl Write) -> std::io::Result<()> {
        // Move cursor to beginning and clear
        execute!(stdout, MoveToColumn(0))?;

        // Calculate lines to clear (header + entries + status + footer + spacing)
        let total_lines = self.entries.len() + 4;
        for _ in 0..total_lines {
            execute!(stdout, Clear(ClearType::CurrentLine), MoveDown(1))?;
        }

        // Move back up
        for _ in 0..total_lines {
            execute!(stdout, MoveUp(1))?;
        }

        // Render header
        let header = self.theme.bold(Color::Agent).apply_to(&self.header);
        execute!(stdout, Print(format!("{}\n\n", header)))?;

        // Render entries
        for (i, entry) in self.entries.iter().enumerate() {
            let is_current = i == self.cursor;

            // Checkbox
            let checkbox = if entry.selected {
                self.theme.apply(Color::Success, "[✓]")
            } else {
                self.theme.apply(Color::Muted, "[ ]")
            };

            // Status indicator
            let status = self.theme.apply(Color::Tool, &entry.status);

            // Path
            let path = if is_current {
                self.theme.bold(Color::UserInput).apply_to(&entry.path)
            } else {
                self.theme.style(Color::UserInput).apply_to(&entry.path)
            };

            // Cursor indicator
            let cursor = if is_current {
                self.theme.apply(Color::Agent, "❯")
            } else {
                " ".to_string()
            };

            execute!(
                stdout,
                Print(format!("{} {} {} {}\n", cursor, checkbox, status, path))
            )?;
        }

        // Status line
        let selected = self.selected_count();
        let total = self.entries.len();
        let status_text = format!("\n{}/{} selected", selected, total);
        let status = self.theme.apply(Color::Muted, &status_text);
        execute!(stdout, Print(format!("{}\n", status)))?;

        // Footer
        let footer = self.theme.apply(Color::Muted, &self.footer);
        execute!(stdout, Print(footer))?;

        stdout.flush()?;
        Ok(())
    }

    fn clear_display(&self, stdout: &mut impl Write) -> std::io::Result<()> {
        // Move to column 0
        execute!(stdout, MoveToColumn(0))?;

        // Clear all lines we rendered
        let total_lines = self.entries.len() + 4;
        for _ in 0..total_lines {
            execute!(stdout, Clear(ClearType::CurrentLine), MoveDown(1))?;
        }

        // Move back to start
        for _ in 0..total_lines {
            execute!(stdout, MoveUp(1))?;
        }

        stdout.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry_new() {
        let entry = FileEntry::new("src/main.rs".to_string(), " M".to_string());
        assert_eq!(entry.path, "src/main.rs");
        assert_eq!(entry.status, " M");
        assert!(!entry.selected);
    }

    #[test]
    fn test_file_entry_new_selected() {
        let entry = FileEntry::new_selected("src/main.rs".to_string(), " M".to_string());
        assert!(entry.selected);
    }

    #[test]
    fn test_file_picker_selected_count() {
        let entries = vec![
            FileEntry::new_selected("a.rs".to_string(), " M".to_string()),
            FileEntry::new("b.rs".to_string(), " M".to_string()),
            FileEntry::new_selected("c.rs".to_string(), "??".to_string()),
        ];
        let picker = FilePicker::new(entries);
        assert_eq!(picker.selected_count(), 2);
    }

    #[test]
    fn test_file_picker_with_header() {
        let picker = FilePicker::new(vec![]).with_header("Custom Header");
        assert_eq!(picker.header, "Custom Header");
    }

    #[test]
    fn test_file_picker_with_footer() {
        let picker = FilePicker::new(vec![]).with_footer("Custom Footer");
        assert_eq!(picker.footer, "Custom Footer");
    }

    #[test]
    fn test_file_picker_empty_returns_empty() {
        let mut picker = FilePicker::new(vec![]);
        // Empty picker should immediately return empty selection without blocking
        let result = picker.run();
        assert!(result.is_ok());
        match result.unwrap() {
            FilePickerResult::Selected(files) => assert!(files.is_empty()),
            FilePickerResult::Cancelled => panic!("Expected Selected, got Cancelled"),
        }
    }

    #[test]
    fn test_pick_mode_ui_construction() {
        // Test that the picker can be constructed with various file statuses
        let entries = vec![
            FileEntry::new("src/auth/login.rs".to_string(), " M".to_string()),
            FileEntry::new("src/auth/logout.rs".to_string(), " M".to_string()),
            FileEntry::new("tests/auth_test.rs".to_string(), "??".to_string()),
            FileEntry::new("README.md".to_string(), "A ".to_string()),
        ];

        let picker = FilePicker::new(entries)
            .with_header("Select files to stage and commit:")
            .with_theme(Theme::default());

        assert_eq!(picker.entries.len(), 4);
        assert_eq!(picker.cursor, 0);
    }
}

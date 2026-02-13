//! Reusable UI components like message boxes

use super::theme::{Color, Theme};

/// A styled message box for displaying important content
pub struct MessageBox {
    theme: Theme,
}

impl MessageBox {
    /// Create a new message box with the given theme
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Create with default theme
    pub fn default_theme() -> Self {
        Self::new(Theme::default())
    }

    /// Render a simple box around text
    pub fn render(&self, title: Option<&str>, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let max_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let width = max_width.max(title.map(|t| t.len()).unwrap_or(0)).max(20);

        let border_color = Color::Muted;
        let top_border = self
            .theme
            .apply(border_color, &format!("┌{}┐", "─".repeat(width + 2)));
        let bottom_border = self
            .theme
            .apply(border_color, &format!("└{}┘", "─".repeat(width + 2)));

        let mut result = vec![top_border];

        // Add title if present
        if let Some(title) = title {
            let title_line = format!(
                "{} {}{} {}",
                self.theme.apply(border_color, "│"),
                self.theme.bold(Color::Agent).apply_to(title),
                " ".repeat(width - title.len()),
                self.theme.apply(border_color, "│")
            );
            result.push(title_line);

            // Separator after title
            let sep = self
                .theme
                .apply(border_color, &format!("├{}┤", "─".repeat(width + 2)));
            result.push(sep);
        }

        // Add content lines
        for line in lines {
            let padding = " ".repeat(width - line.chars().count());
            let content_line = format!(
                "{} {}{} {}",
                self.theme.apply(border_color, "│"),
                line,
                padding,
                self.theme.apply(border_color, "│")
            );
            result.push(content_line);
        }

        result.push(bottom_border);
        result.join("\n")
    }

    /// Render a commit message box
    pub fn commit_message(&self, title: &str, body: &str) -> String {
        let mut content = String::new();
        content.push_str(&self.theme.bold(Color::Agent).apply_to(title).to_string());
        if !body.is_empty() {
            content.push_str("\n\n");
            content.push_str(body);
        }

        self.render(Some("Commit Message"), &content)
    }

    /// Render a simple info box
    pub fn info(&self, message: &str) -> String {
        self.render(None, message)
    }

    /// Render an error box
    pub fn error(&self, title: &str, message: &str) -> String {
        let content = format!("{}\n\n{}", title, message);
        self.render(Some("Error"), &content)
    }
}

impl Default for MessageBox {
    fn default() -> Self {
        Self::default_theme()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_box_simple() {
        let box_comp = MessageBox::default_theme();
        let output = box_comp.info("Hello, World!");

        assert!(output.contains("Hello, World!"));
        assert!(output.contains("┌"));
        assert!(output.contains("└"));
        assert!(output.contains("│"));
    }

    #[test]
    fn test_message_box_with_title() {
        let box_comp = MessageBox::default_theme();
        let output = box_comp.render(Some("Title"), "Content here");

        assert!(output.contains("Title"));
        assert!(output.contains("Content here"));
        assert!(output.contains("├")); // Title separator
    }

    #[test]
    fn test_message_box_multiline() {
        let box_comp = MessageBox::default_theme();
        let output = box_comp.info("Line 1\nLine 2\nLine 3");

        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 2"));
        assert!(output.contains("Line 3"));
    }

    #[test]
    fn test_commit_message_box() {
        let box_comp = MessageBox::default_theme();
        let output = box_comp.commit_message(
            "Add new feature",
            "This implements the new feature\nwith proper error handling.",
        );

        assert!(output.contains("Commit Message"));
        assert!(output.contains("Add new feature"));
        assert!(output.contains("proper error handling"));
    }

    #[test]
    fn test_error_box() {
        let box_comp = MessageBox::default_theme();
        let output = box_comp.error("File not found", "The file 'test.txt' does not exist.");

        assert!(output.contains("Error"));
        assert!(output.contains("File not found"));
        assert!(output.contains("test.txt"));
    }
}

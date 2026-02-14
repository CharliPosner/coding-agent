//! Markdown rendering for terminal output
//!
//! Uses termimad to render markdown with proper formatting:
//! - Headers with colors
//! - Bold and italic text
//! - Code blocks with syntax highlighting
//! - Lists and blockquotes
//! - Tables

use termimad::{crossterm::style::Color, MadSkin};

/// Markdown renderer for terminal output
pub struct MarkdownRenderer {
    skin: MadSkin,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    /// Create a new markdown renderer with default styling
    pub fn new() -> Self {
        let mut skin = MadSkin::default();

        // Customize colors for better readability
        // Headers - cyan/blue gradient
        skin.headers[0].set_fg(Color::Cyan);
        skin.headers[1].set_fg(Color::Blue);
        skin.headers[2].set_fg(Color::DarkCyan);

        // Bold text - white/bright
        skin.bold.set_fg(Color::White);

        // Italic text - slightly dimmer
        skin.italic.set_fg(Color::Grey);

        // Inline code - yellow on dark background
        skin.inline_code.set_fg(Color::Yellow);

        // Code blocks - keep default but ensure visible
        skin.code_block.set_fg(Color::Green);

        // Blockquotes - dimmer
        skin.quote_mark.set_fg(Color::DarkGrey);

        // Links - blue underlined (termimad handles this)
        skin.paragraph.set_fg(Color::Reset);

        Self { skin }
    }

    /// Render markdown text for terminal display
    ///
    /// Returns the formatted string ready for printing.
    /// The output includes ANSI escape codes for colors and formatting.
    pub fn render(&self, markdown: &str) -> String {
        // termimad's text() method renders to a string with ANSI codes
        self.skin.text(markdown, None).to_string()
    }

    /// Render markdown and print directly to stdout
    ///
    /// This handles proper line endings for raw mode terminals.
    pub fn print(&self, markdown: &str) {
        let rendered = self.render(markdown);
        // Replace \n with \r\n for raw mode terminal compatibility
        for line in rendered.lines() {
            print!("{}\r\n", line);
        }
    }

    /// Render a single line of markdown (inline formatting only)
    ///
    /// Useful for rendering text that should stay on one line.
    pub fn render_inline(&self, text: &str) -> String {
        self.skin.inline(text).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = MarkdownRenderer::new();
        // Just verify it can be created
        assert!(!renderer.render("test").is_empty());
    }

    #[test]
    fn test_render_plain_text() {
        let renderer = MarkdownRenderer::new();
        let output = renderer.render("Hello, world!");
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_render_bold() {
        let renderer = MarkdownRenderer::new();
        let output = renderer.render("This is **bold** text");
        // Output should contain the word "bold" (formatting codes may vary)
        assert!(output.contains("bold"));
    }

    #[test]
    fn test_render_code_block() {
        let renderer = MarkdownRenderer::new();
        let output = renderer.render("```rust\nfn main() {}\n```");
        assert!(output.contains("fn main()"));
    }

    #[test]
    fn test_render_header() {
        let renderer = MarkdownRenderer::new();
        let output = renderer.render("# Header");
        assert!(output.contains("Header"));
    }

    #[test]
    fn test_render_list() {
        let renderer = MarkdownRenderer::new();
        let output = renderer.render("- Item 1\n- Item 2");
        assert!(output.contains("Item 1"));
        assert!(output.contains("Item 2"));
    }

    #[test]
    fn test_render_inline() {
        let renderer = MarkdownRenderer::new();
        let output = renderer.render_inline("**bold** and *italic*");
        assert!(output.contains("bold"));
        assert!(output.contains("italic"));
    }
}

//! Markdown rendering for terminal output
//!
//! Uses termimad to render markdown with proper formatting:
//! - Headers with colors
//! - Bold and italic text
//! - Code blocks with syntax highlighting
//! - Lists and blockquotes
//! - Tables

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use termimad::{crossterm::style::Color, MadSkin};

/// Markdown renderer for terminal output
pub struct MarkdownRenderer {
    skin: MadSkin,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
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

        // Load syntax definitions
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        Self {
            skin,
            syntax_set,
            theme_set,
        }
    }

    /// Highlight code with syntax-aware coloring
    fn highlight_code(&self, code: &str, language: &str) -> String {
        // Try to find syntax definition for the language
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language));

        // If no syntax found, return plain green text (fallback)
        let Some(syntax) = syntax else {
            return format!("\x1b[32m{}\x1b[0m", code);
        };

        // Use a dark theme for terminal
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut highlighted = String::new();
        for line in LinesWithEndings::from(code) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            for (style, text) in ranges {
                highlighted.push_str(&self.style_to_ansi(style));
                highlighted.push_str(text);
            }
            highlighted.push_str("\x1b[0m"); // Reset after each line
        }

        highlighted
    }

    /// Convert syntect Style to ANSI escape codes
    fn style_to_ansi(&self, style: Style) -> String {
        let fg = style.foreground;
        format!("\x1b[38;2;{};{};{}m", fg.r, fg.g, fg.b)
    }

    /// Render markdown text for terminal display
    ///
    /// Returns the formatted string ready for printing.
    /// The output includes ANSI escape codes for colors and formatting.
    pub fn render(&self, markdown: &str) -> String {
        // Pre-process to extract and highlight code blocks
        let processed = self.preprocess_code_blocks(markdown);
        // termimad's text() method renders to a string with ANSI codes
        self.skin.text(&processed, None).to_string()
    }

    /// Pre-process markdown to highlight code blocks
    fn preprocess_code_blocks(&self, markdown: &str) -> String {
        let mut result = String::new();
        let mut in_code_block = false;
        let mut code_buffer = String::new();
        let mut language = String::new();
        let lines: Vec<&str> = markdown.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if let Some(lang) = line.strip_prefix("```") {
                if in_code_block {
                    // End of code block - highlight and append
                    let highlighted = self.highlight_code(&code_buffer, &language);
                    result.push_str("```\n");
                    result.push_str(&highlighted);
                    if i < lines.len() - 1 {
                        result.push_str("\n```\n");
                    } else {
                        result.push_str("```\n");
                    }
                    code_buffer.clear();
                    language.clear();
                    in_code_block = false;
                } else {
                    // Start of code block
                    language = lang.trim().to_string();
                    in_code_block = true;
                }
            } else if in_code_block {
                code_buffer.push_str(line);
                code_buffer.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Handle unclosed code block
        if in_code_block {
            let highlighted = self.highlight_code(&code_buffer, &language);
            result.push_str("```\n");
            result.push_str(&highlighted);
            result.push_str("```\n");
        }

        result
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
        // The output contains ANSI codes, so check for individual parts
        assert!(output.contains("fn"));
        assert!(output.contains("main"));
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

    #[test]
    fn test_code_block_rust_keywords() {
        let renderer = MarkdownRenderer::new();
        let markdown = "```rust\nfn main() {\n    let x = 5;\n}\n```";
        let output = renderer.render(markdown);

        // Check that keywords are present in output
        assert!(output.contains("fn"));
        assert!(output.contains("let"));

        // Check for ANSI color codes (syntax highlighting applied)
        assert!(output.contains("\x1b[38;2;"));
    }

    #[test]
    fn test_code_block_rust_strings() {
        let renderer = MarkdownRenderer::new();
        let markdown = r#"```rust
let s = "hello";
```"#;
        let output = renderer.render(markdown);

        // Check that string literal is present
        assert!(output.contains("hello"));

        // Check for ANSI color codes
        assert!(output.contains("\x1b[38;2;"));
    }

    #[test]
    fn test_code_block_rust_comments() {
        let renderer = MarkdownRenderer::new();
        let markdown = "```rust\n// This is a comment\nfn main() {}\n```";
        let output = renderer.render(markdown);

        // Check that comment is present
        assert!(output.contains("This is a comment"));

        // Check for ANSI color codes
        assert!(output.contains("\x1b[38;2;"));
    }

    #[test]
    fn test_code_block_unknown_lang() {
        let renderer = MarkdownRenderer::new();
        let markdown = "```unknownlang\nsome code here\n```";
        let output = renderer.render(markdown);

        // Should contain the code
        assert!(output.contains("some code here"));

        // Should fallback to green (ANSI code 32)
        assert!(output.contains("\x1b[32m"));
    }

    #[test]
    fn test_code_block_no_lang() {
        let renderer = MarkdownRenderer::new();
        let markdown = "```\nplain code\n```";
        let output = renderer.render(markdown);

        // Should contain the code
        assert!(output.contains("plain code"));

        // Should fallback to green since no language specified
        assert!(output.contains("\x1b[32m"));
    }
}

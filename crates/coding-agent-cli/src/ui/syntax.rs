//! Syntax highlighting for code blocks using syntect

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

/// Syntax highlighter for code blocks
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with default settings
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Set the highlighting theme
    pub fn set_theme(&mut self, name: &str) {
        if self.theme_set.themes.contains_key(name) {
            self.theme_name = name.to_string();
        }
    }

    /// Get available theme names
    pub fn available_themes(&self) -> Vec<&str> {
        self.theme_set.themes.keys().map(|s| s.as_str()).collect()
    }

    /// Highlight code with the given language
    pub fn highlight(&self, code: &str, language: &str) -> String {
        // Try to find syntax for the language
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(language)
            .or_else(|| self.syntax_set.find_syntax_by_name(language))
            .or_else(|| self.syntax_set.find_syntax_by_token(language));

        match syntax {
            Some(syntax) => {
                let theme = &self.theme_set.themes[&self.theme_name];
                let mut highlighter = HighlightLines::new(syntax, theme);
                let mut output = String::new();

                for line in LinesWithEndings::from(code) {
                    let ranges: Vec<(Style, &str)> =
                        highlighter.highlight_line(line, &self.syntax_set).unwrap();
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                    output.push_str(&escaped);
                }

                // Reset terminal colors at the end
                output.push_str("\x1b[0m");
                output
            }
            None => {
                // Unknown language, return code as-is
                code.to_string()
            }
        }
    }

    /// Check if a language is supported
    pub fn is_supported(&self, language: &str) -> bool {
        self.syntax_set.find_syntax_by_extension(language).is_some()
            || self.syntax_set.find_syntax_by_name(language).is_some()
            || self.syntax_set.find_syntax_by_token(language).is_some()
    }

    /// Get list of supported language extensions
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.syntax_set
            .syntaxes()
            .iter()
            .flat_map(|s| s.file_extensions.iter())
            .map(|s| s.as_str())
            .collect()
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlight_rust() {
        let highlighter = SyntaxHighlighter::new();
        let code = r#"fn main() {
    println!("Hello, world!");
}"#;

        let highlighted = highlighter.highlight(code, "rs");

        // Should contain escape codes for coloring
        assert!(highlighted.contains("\x1b["));
        // Should end with reset sequence
        assert!(highlighted.ends_with("\x1b[0m"));
        // Should contain the keywords (potentially with escape sequences between chars)
        assert!(highlighted.contains("fn"));
        assert!(highlighted.contains("main"));
        assert!(highlighted.contains("println"));
    }

    #[test]
    fn test_syntax_highlight_unknown() {
        let highlighter = SyntaxHighlighter::new();
        let code = "some unknown code";

        let highlighted = highlighter.highlight(code, "unknown_language_xyz");

        // Should fall back to plain text
        assert_eq!(highlighted, code);
    }

    #[test]
    fn test_syntax_is_supported() {
        let highlighter = SyntaxHighlighter::new();

        assert!(highlighter.is_supported("rs"));
        assert!(highlighter.is_supported("rust"));
        assert!(highlighter.is_supported("py"));
        assert!(highlighter.is_supported("python"));
        assert!(highlighter.is_supported("js"));
        assert!(highlighter.is_supported("javascript"));

        assert!(!highlighter.is_supported("unknown_language_xyz"));
    }

    #[test]
    fn test_available_themes() {
        let highlighter = SyntaxHighlighter::new();
        let themes = highlighter.available_themes();

        assert!(!themes.is_empty());
        assert!(themes.contains(&"base16-ocean.dark"));
    }

    #[test]
    fn test_set_theme() {
        let mut highlighter = SyntaxHighlighter::new();

        // Valid theme should be set
        highlighter.set_theme("base16-mocha.dark");
        assert_eq!(highlighter.theme_name, "base16-mocha.dark");

        // Invalid theme should be ignored
        highlighter.set_theme("nonexistent-theme");
        assert_eq!(highlighter.theme_name, "base16-mocha.dark");
    }

    #[test]
    fn test_highlight_multiline() {
        let highlighter = SyntaxHighlighter::new();
        let code = r#"def hello():
    print("Hello")
    return True"#;

        let highlighted = highlighter.highlight(code, "py");

        // Should contain escape codes
        assert!(highlighted.contains("\x1b["));
        // Should preserve newlines
        assert!(highlighted.contains('\n'));
    }
}

//! Tool result formatting and display
//!
//! This module provides formatted display of tool execution results,
//! including syntax highlighting for code content.

use super::syntax::SyntaxHighlighter;
use super::theme::{Color, Theme};

/// Configuration for tool result display
#[derive(Debug, Clone)]
pub struct ToolResultConfig {
    /// Maximum lines to display before truncation
    pub max_display_lines: usize,
    /// Whether to enable syntax highlighting
    pub enable_highlighting: bool,
    /// Whether to show line numbers for code
    pub show_line_numbers: bool,
}

impl Default for ToolResultConfig {
    fn default() -> Self {
        Self {
            max_display_lines: 50,
            enable_highlighting: true,
            show_line_numbers: false,
        }
    }
}

/// Tool result formatter with syntax highlighting
pub struct ToolResultFormatter {
    highlighter: SyntaxHighlighter,
    theme: Theme,
    config: ToolResultConfig,
}

impl ToolResultFormatter {
    /// Create a new formatter with default settings
    pub fn new() -> Self {
        Self {
            highlighter: SyntaxHighlighter::new(),
            theme: Theme::default(),
            config: ToolResultConfig::default(),
        }
    }

    /// Create a formatter with custom configuration
    pub fn with_config(config: ToolResultConfig) -> Self {
        Self {
            highlighter: SyntaxHighlighter::new(),
            theme: Theme::default(),
            config,
        }
    }

    /// Format a tool result for display
    pub fn format_result(&self, tool_name: &str, output: &str) -> String {
        match tool_name {
            "read_file" => self.format_file_content(output),
            "write_file" => self.format_write_result(output),
            "edit_file" => self.format_edit_result(output),
            "list_files" => self.format_file_list(output),
            "bash" => self.format_bash_output(output),
            "code_search" => self.format_search_results(output),
            _ => self.format_generic(output),
        }
    }

    /// Format file content with syntax highlighting
    fn format_file_content(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let mut result = String::new();

        // Add header
        result.push_str(&format!(
            "  {}\r\n",
            self.theme
                .apply(Color::Muted, &format!("({} lines)", total_lines))
        ));

        // Determine if we need to truncate
        let (display_lines, truncated) = if total_lines > self.config.max_display_lines {
            (&lines[..self.config.max_display_lines], true)
        } else {
            (&lines[..], false)
        };

        // Try to detect language from content (simple heuristics)
        let language = self.detect_language(content);

        // Display content
        if self.config.enable_highlighting && language.is_some() {
            let highlighted = self.highlighter.highlight(content, language.unwrap());
            for line in highlighted.lines().take(self.config.max_display_lines) {
                result.push_str(&format!("  {}\r\n", line));
            }
        } else {
            for line in display_lines {
                result.push_str(&format!("  {}\r\n", line));
            }
        }

        // Add truncation notice if needed
        if truncated {
            result.push_str(&format!(
                "  {}\r\n",
                self.theme.apply(
                    Color::Muted,
                    &format!(
                        "... ({} more lines)",
                        total_lines - self.config.max_display_lines
                    )
                )
            ));
        }

        result
    }

    /// Format write result
    fn format_write_result(&self, output: &str) -> String {
        format!("  {}\r\n", self.theme.apply(Color::Success, output))
    }

    /// Format edit result
    fn format_edit_result(&self, output: &str) -> String {
        if output == "OK" {
            format!(
                "  {}\r\n",
                self.theme
                    .apply(Color::Success, "Edit applied successfully")
            )
        } else {
            format!("  {}\r\n", output)
        }
    }

    /// Format file list
    fn format_file_list(&self, output: &str) -> String {
        if let Ok(files) = serde_json::from_str::<Vec<String>>(output) {
            let mut result = String::new();
            result.push_str(&format!(
                "  {}\r\n",
                self.theme
                    .apply(Color::Muted, &format!("({} items)", files.len()))
            ));

            let display_count = self.config.max_display_lines.min(files.len());
            for file in &files[..display_count] {
                // Color directories differently
                if file.ends_with('/') {
                    result.push_str(&format!("  {}\r\n", self.theme.apply(Color::Agent, file)));
                } else {
                    result.push_str(&format!("  {}\r\n", file));
                }
            }

            if files.len() > display_count {
                result.push_str(&format!(
                    "  {}\r\n",
                    self.theme.apply(
                        Color::Muted,
                        &format!("... ({} more)", files.len() - display_count)
                    )
                ));
            }

            result
        } else {
            format!("  {}\r\n", output)
        }
    }

    /// Format bash command output
    fn format_bash_output(&self, output: &str) -> String {
        if output.is_empty() {
            return format!("  {}\r\n", self.theme.apply(Color::Muted, "(no output)"));
        }

        let lines: Vec<&str> = output.lines().collect();
        let total_lines = lines.len();

        let mut result = String::new();

        if total_lines > 1 {
            result.push_str(&format!(
                "  {}\r\n",
                self.theme
                    .apply(Color::Muted, &format!("({} lines)", total_lines))
            ));
        }

        let display_count = self.config.max_display_lines.min(total_lines);
        for line in &lines[..display_count] {
            result.push_str(&format!("  {}\r\n", line));
        }

        if total_lines > display_count {
            result.push_str(&format!(
                "  {}\r\n",
                self.theme.apply(
                    Color::Muted,
                    &format!("... ({} more lines)", total_lines - display_count)
                )
            ));
        }

        result
    }

    /// Format code search results
    fn format_search_results(&self, output: &str) -> String {
        if output == "No matches found" {
            return format!("  {}\r\n", self.theme.apply(Color::Muted, output));
        }

        let lines: Vec<&str> = output.lines().collect();
        let total_lines = lines.len();

        let mut result = String::new();
        result.push_str(&format!(
            "  {}\r\n",
            self.theme
                .apply(Color::Muted, &format!("({} matches)", total_lines))
        ));

        let display_count = self.config.max_display_lines.min(total_lines);
        for line in &lines[..display_count] {
            // Highlight file paths
            if line.contains(':') {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    result.push_str(&format!(
                        "  {}: {}\r\n",
                        self.theme.apply(Color::Agent, parts[0]),
                        parts[1]
                    ));
                    continue;
                }
            }
            result.push_str(&format!("  {}\r\n", line));
        }

        if total_lines > display_count {
            result.push_str(&format!(
                "  {}\r\n",
                self.theme.apply(
                    Color::Muted,
                    &format!("... ({} more matches)", total_lines - display_count)
                )
            ));
        }

        result
    }

    /// Format generic output
    fn format_generic(&self, output: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();
        let total_lines = lines.len();

        let mut result = String::new();
        let display_count = self.config.max_display_lines.min(total_lines);

        for line in &lines[..display_count] {
            result.push_str(&format!("  {}\r\n", line));
        }

        if total_lines > display_count {
            result.push_str(&format!(
                "  {}\r\n",
                self.theme.apply(
                    Color::Muted,
                    &format!("... ({} more lines)", total_lines - display_count)
                )
            ));
        }

        result
    }

    /// Detect programming language from content (simple heuristics)
    fn detect_language(&self, content: &str) -> Option<&str> {
        // Check for common language markers
        if content.contains("fn main()")
            || content.contains("impl ")
            || content.contains("pub struct")
        {
            return Some("rs");
        }
        if content.contains("def ") || content.contains("import ") || content.contains("class ") {
            return Some("py");
        }
        if content.contains("function ") || content.contains("const ") || content.contains("let ") {
            return Some("js");
        }
        if content.contains("package ") || content.contains("import java.") {
            return Some("java");
        }
        if content.contains("#include") || content.contains("int main(") {
            return Some("cpp");
        }

        None
    }
}

impl Default for ToolResultFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_content_short() {
        // Disable highlighting for predictable testing
        let config = ToolResultConfig {
            enable_highlighting: false,
            ..Default::default()
        };
        let formatter = ToolResultFormatter::with_config(config);
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let result = formatter.format_file_content(content);

        assert!(result.contains("3 lines"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_format_file_content_truncated() {
        let mut config = ToolResultConfig::default();
        config.max_display_lines = 3;
        let formatter = ToolResultFormatter::with_config(config);

        let content = (0..10)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = formatter.format_file_content(&content);

        assert!(result.contains("10 lines"));
        assert!(result.contains("... (7 more lines)"));
    }

    #[test]
    fn test_format_write_result() {
        let formatter = ToolResultFormatter::new();
        let result = formatter.format_write_result("File written successfully");

        assert!(result.contains("File written successfully"));
    }

    #[test]
    fn test_format_edit_result_ok() {
        let formatter = ToolResultFormatter::new();
        let result = formatter.format_edit_result("OK");

        assert!(result.contains("Edit applied successfully"));
    }

    #[test]
    fn test_format_file_list() {
        let formatter = ToolResultFormatter::new();
        let files = r#"["file1.txt", "dir1/", "file2.rs"]"#;
        let result = formatter.format_file_list(files);

        assert!(result.contains("3 items"));
        assert!(result.contains("file1.txt"));
        assert!(result.contains("dir1/"));
        assert!(result.contains("file2.rs"));
    }

    #[test]
    fn test_format_file_list_truncated() {
        let mut config = ToolResultConfig::default();
        config.max_display_lines = 2;
        let formatter = ToolResultFormatter::with_config(config);

        let files = r#"["file1.txt", "file2.txt", "file3.txt", "file4.txt"]"#;
        let result = formatter.format_file_list(files);

        assert!(result.contains("4 items"));
        assert!(result.contains("... (2 more)"));
    }

    #[test]
    fn test_format_bash_output_empty() {
        let formatter = ToolResultFormatter::new();
        let result = formatter.format_bash_output("");

        assert!(result.contains("(no output)"));
    }

    #[test]
    fn test_format_bash_output_single_line() {
        let formatter = ToolResultFormatter::new();
        let result = formatter.format_bash_output("Hello, world!");

        assert!(result.contains("Hello, world!"));
        assert!(!result.contains("lines")); // Single line shouldn't show line count
    }

    #[test]
    fn test_format_bash_output_multi_line() {
        let formatter = ToolResultFormatter::new();
        let output = "line 1\nline 2\nline 3";
        let result = formatter.format_bash_output(output);

        assert!(result.contains("3 lines"));
        assert!(result.contains("line 1"));
    }

    #[test]
    fn test_format_search_results_no_matches() {
        let formatter = ToolResultFormatter::new();
        let result = formatter.format_search_results("No matches found");

        assert!(result.contains("No matches found"));
    }

    #[test]
    fn test_format_search_results_with_matches() {
        let formatter = ToolResultFormatter::new();
        let output = "src/main.rs:10:fn main()\nsrc/lib.rs:25:pub fn test()";
        let result = formatter.format_search_results(output);

        assert!(result.contains("2 matches"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_detect_language_rust() {
        let formatter = ToolResultFormatter::new();
        let content = "fn main() { println!(\"Hello\"); }";

        assert_eq!(formatter.detect_language(content), Some("rs"));
    }

    #[test]
    fn test_detect_language_python() {
        let formatter = ToolResultFormatter::new();
        let content = "def hello():\n    print(\"Hello\")";

        assert_eq!(formatter.detect_language(content), Some("py"));
    }

    #[test]
    fn test_detect_language_javascript() {
        let formatter = ToolResultFormatter::new();
        let content = "function hello() { console.log(\"Hello\"); }";

        assert_eq!(formatter.detect_language(content), Some("js"));
    }

    #[test]
    fn test_detect_language_unknown() {
        let formatter = ToolResultFormatter::new();
        let content = "some random text without code markers";

        assert_eq!(formatter.detect_language(content), None);
    }

    #[test]
    fn test_format_generic() {
        let formatter = ToolResultFormatter::new();
        let output = "Generic output\nLine 2\nLine 3";
        let result = formatter.format_generic(output);

        assert!(result.contains("Generic output"));
        assert!(result.contains("Line 2"));
        assert!(result.contains("Line 3"));
    }

    #[test]
    fn test_config_max_display_lines() {
        let config = ToolResultConfig {
            max_display_lines: 5,
            enable_highlighting: true,
            show_line_numbers: false,
        };

        assert_eq!(config.max_display_lines, 5);
    }

    #[test]
    fn test_config_enable_highlighting() {
        let config = ToolResultConfig {
            max_display_lines: 50,
            enable_highlighting: false,
            show_line_numbers: false,
        };

        assert!(!config.enable_highlighting);
    }
}

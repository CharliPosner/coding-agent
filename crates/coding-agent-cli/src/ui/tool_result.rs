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
    /// Threshold for collapsing results (0 = never collapse)
    pub collapse_threshold: usize,
}

impl Default for ToolResultConfig {
    fn default() -> Self {
        Self {
            max_display_lines: 50,
            enable_highlighting: true,
            show_line_numbers: false,
            collapse_threshold: 5,
        }
    }
}

/// Result of formatting a tool output, potentially collapsible
#[derive(Debug, Clone)]
pub struct FormattedResult {
    /// The formatted output to display (may be collapsed summary)
    pub display: String,
    /// If collapsed, contains the full output for later viewing
    pub collapsed_content: Option<String>,
    /// Number of items that were collapsed (for display purposes)
    pub collapsed_count: usize,
    /// The tool name that produced this result
    pub tool_name: String,
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

    /// Format a tool result for display (simple string output, no collapsing)
    pub fn format_result(&self, tool_name: &str, output: &str) -> String {
        self.format_result_collapsible(tool_name, output).display
    }

    /// Format a tool result with collapsible support
    pub fn format_result_collapsible(&self, tool_name: &str, output: &str) -> FormattedResult {
        match tool_name {
            "read_file" => FormattedResult {
                display: self.format_file_content(output),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            },
            "write_file" => FormattedResult {
                display: self.format_write_result(output),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            },
            "edit_file" => FormattedResult {
                display: self.format_edit_result(output),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            },
            "list_files" => self.format_file_list_collapsible(output, tool_name),
            "bash" => FormattedResult {
                display: self.format_bash_output(output),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            },
            "code_search" => self.format_search_results_collapsible(output, tool_name),
            _ => FormattedResult {
                display: self.format_generic(output),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            },
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

        // Calculate line number width for padding
        let line_num_width = if self.config.show_line_numbers {
            total_lines.to_string().len()
        } else {
            0
        };

        // Display content
        if self.config.enable_highlighting && language.is_some() {
            let highlighted = self.highlighter.highlight(content, language.unwrap());
            for (idx, line) in highlighted
                .lines()
                .take(self.config.max_display_lines)
                .enumerate()
            {
                if self.config.show_line_numbers {
                    let line_num = idx + 1;
                    result.push_str(&format!(
                        "  {} │ {}\r\n",
                        self.theme.apply(
                            Color::Muted,
                            &format!("{:>width$}", line_num, width = line_num_width)
                        ),
                        line
                    ));
                } else {
                    result.push_str(&format!("  {}\r\n", line));
                }
            }
        } else {
            for (idx, line) in display_lines.iter().enumerate() {
                if self.config.show_line_numbers {
                    let line_num = idx + 1;
                    result.push_str(&format!(
                        "  {} │ {}\r\n",
                        self.theme.apply(
                            Color::Muted,
                            &format!("{:>width$}", line_num, width = line_num_width)
                        ),
                        line
                    ));
                } else {
                    result.push_str(&format!("  {}\r\n", line));
                }
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

    /// Format file list with collapsible support
    fn format_file_list_collapsible(&self, output: &str, tool_name: &str) -> FormattedResult {
        if let Ok(files) = serde_json::from_str::<Vec<String>>(output) {
            let total_files = files.len();
            let threshold = self.config.collapse_threshold;

            // If under threshold, show all files normally
            if threshold == 0 || total_files <= threshold {
                return FormattedResult {
                    display: self.format_file_list(output),
                    collapsed_content: None,
                    collapsed_count: 0,
                    tool_name: tool_name.to_string(),
                };
            }

            // Collapse: show summary only
            let display = format!(
                "  {} {}\r\n",
                self.theme.apply(Color::Muted, "▸"),
                self.theme.apply(
                    Color::Muted,
                    &format!("{} files found (use /results to expand)", total_files)
                )
            );

            // Build full output for later viewing
            let mut full_output = String::new();
            full_output.push_str(&format!(
                "  {}\r\n",
                self.theme
                    .apply(Color::Muted, &format!("({} items)", total_files))
            ));

            for file in &files {
                if file.ends_with('/') {
                    full_output
                        .push_str(&format!("  {}\r\n", self.theme.apply(Color::Agent, file)));
                } else {
                    full_output.push_str(&format!("  {}\r\n", file));
                }
            }

            FormattedResult {
                display,
                collapsed_content: Some(full_output),
                collapsed_count: total_files,
                tool_name: tool_name.to_string(),
            }
        } else {
            FormattedResult {
                display: format!("  {}\r\n", output),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            }
        }
    }

    /// Format code search results with collapsible support
    fn format_search_results_collapsible(&self, output: &str, tool_name: &str) -> FormattedResult {
        if output == "No matches found" {
            return FormattedResult {
                display: format!("  {}\r\n", self.theme.apply(Color::Muted, output)),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            };
        }

        let lines: Vec<&str> = output.lines().collect();
        let total_lines = lines.len();
        let threshold = self.config.collapse_threshold;

        // If under threshold, show all results normally
        if threshold == 0 || total_lines <= threshold {
            return FormattedResult {
                display: self.format_search_results(output),
                collapsed_content: None,
                collapsed_count: 0,
                tool_name: tool_name.to_string(),
            };
        }

        // Collapse: show summary only
        let display = format!(
            "  {} {}\r\n",
            self.theme.apply(Color::Muted, "▸"),
            self.theme.apply(
                Color::Muted,
                &format!("{} matches found (use /results to expand)", total_lines)
            )
        );

        // Build full output for later viewing
        let mut full_output = String::new();
        full_output.push_str(&format!(
            "  {}\r\n",
            self.theme
                .apply(Color::Muted, &format!("({} matches)", total_lines))
        ));

        for line in &lines {
            if line.contains(':') {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    full_output.push_str(&format!(
                        "  {}: {}\r\n",
                        self.theme.apply(Color::Agent, parts[0]),
                        parts[1]
                    ));
                    continue;
                }
            }
            full_output.push_str(&format!("  {}\r\n", line));
        }

        FormattedResult {
            display,
            collapsed_content: Some(full_output),
            collapsed_count: total_lines,
            tool_name: tool_name.to_string(),
        }
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
            collapse_threshold: 5,
        };

        assert_eq!(config.max_display_lines, 5);
    }

    #[test]
    fn test_config_enable_highlighting() {
        let config = ToolResultConfig {
            max_display_lines: 50,
            enable_highlighting: false,
            show_line_numbers: false,
            collapse_threshold: 5,
        };

        assert!(!config.enable_highlighting);
    }

    #[test]
    fn test_format_file_content_with_line_numbers() {
        let config = ToolResultConfig {
            max_display_lines: 50,
            enable_highlighting: false,
            show_line_numbers: true,
            collapse_threshold: 5,
        };
        let formatter = ToolResultFormatter::with_config(config);
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let result = formatter.format_file_content(content);

        // Should contain line numbers
        assert!(result.contains("1 │"));
        assert!(result.contains("2 │"));
        assert!(result.contains("3 │"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_format_file_content_without_line_numbers() {
        let config = ToolResultConfig {
            max_display_lines: 50,
            enable_highlighting: false,
            show_line_numbers: false,
            collapse_threshold: 5,
        };
        let formatter = ToolResultFormatter::with_config(config);
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let result = formatter.format_file_content(content);

        // Should NOT contain line numbers
        assert!(!result.contains("1 │"));
        assert!(!result.contains("2 │"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_line_numbers_proper_padding() {
        let config = ToolResultConfig {
            max_display_lines: 50,
            enable_highlighting: false,
            show_line_numbers: true,
            collapse_threshold: 5,
        };
        let formatter = ToolResultFormatter::with_config(config);

        // Create content with 100 lines to test padding
        let content = (1..=100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = formatter.format_file_content(&content);

        // Should have consistent width for all line numbers
        // Line 1 should be padded to match line 100's width (3 digits)
        assert!(result.contains("  1 │"));
        assert!(result.contains(" 10 │"));
        assert!(result.contains(" 50 │"));
    }

    #[test]
    fn test_collapsible_search_under_threshold() {
        // With default threshold of 5, 3 results should NOT collapse
        let formatter = ToolResultFormatter::new();
        let output = "src/a.rs:1:match\nsrc/b.rs:2:match\nsrc/c.rs:3:match";
        let result = formatter.format_result_collapsible("code_search", output);

        assert!(result.collapsed_content.is_none());
        assert_eq!(result.collapsed_count, 0);
        assert!(result.display.contains("3 matches"));
    }

    #[test]
    fn test_collapsible_search_over_threshold() {
        // With default threshold of 5, 10 results should collapse
        let formatter = ToolResultFormatter::new();
        let output = (1..=10)
            .map(|i| format!("src/file{}.rs:{}:match", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = formatter.format_result_collapsible("code_search", &output);

        assert!(result.collapsed_content.is_some());
        assert_eq!(result.collapsed_count, 10);
        assert!(result.display.contains("10 matches found"));
        assert!(result.display.contains("/results"));
    }

    #[test]
    fn test_collapsible_file_list_over_threshold() {
        // With default threshold of 5, 10 files should collapse
        let formatter = ToolResultFormatter::new();
        let files: Vec<String> = (1..=10).map(|i| format!("file{}.txt", i)).collect();
        let output = serde_json::to_string(&files).unwrap();
        let result = formatter.format_result_collapsible("list_files", &output);

        assert!(result.collapsed_content.is_some());
        assert_eq!(result.collapsed_count, 10);
        assert!(result.display.contains("10 files found"));
        assert!(result.display.contains("/results"));
    }

    #[test]
    fn test_collapsible_disabled_with_zero_threshold() {
        let config = ToolResultConfig {
            collapse_threshold: 0, // Disable collapsing
            ..Default::default()
        };
        let formatter = ToolResultFormatter::with_config(config);

        let output = (1..=100)
            .map(|i| format!("src/file{}.rs:{}:match", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = formatter.format_result_collapsible("code_search", &output);

        // Should NOT collapse even with 100 results
        assert!(result.collapsed_content.is_none());
        assert_eq!(result.collapsed_count, 0);
    }

    #[test]
    fn test_formatted_result_tool_name() {
        let formatter = ToolResultFormatter::new();
        let result = formatter.format_result_collapsible("code_search", "No matches found");

        assert_eq!(result.tool_name, "code_search");
    }
}

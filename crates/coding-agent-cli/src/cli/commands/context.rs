//! The /context command - shows current context (loaded files, working dir, token usage)

use super::{Command, CommandContext, CommandResult};
use crate::tokens::CostTracker;
use std::path::PathBuf;

pub struct ContextCommand;

impl Command for ContextCommand {
    fn name(&self) -> &'static str {
        "context"
    }

    fn description(&self) -> &'static str {
        "Show current context (loaded files, working directory, token usage)"
    }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandResult {
        let output = render_context_info(&ctx.cost_tracker);
        CommandResult::Output(output)
    }
}

/// Render the context information display.
fn render_context_info(cost_tracker: &CostTracker) -> String {
    let separator = "──────────────────────────────────────────────";

    let mut output = String::new();
    output.push_str("Current Context\n");
    output.push_str(separator);
    output.push_str("\n\n");

    // Working directory
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    output.push_str(&format!("Working directory:\n  {}\n\n", cwd));

    // Loaded files section
    output.push_str("Loaded files:\n");
    // For now, we don't track loaded files yet - this will be implemented
    // when the agent integration is added. Show a placeholder message.
    output.push_str("  (No files loaded in this session)\n\n");

    // Token usage summary
    output.push_str("Token usage:\n");
    output.push_str(&format!(
        "  Input:   {:>10} tokens\n",
        CostTracker::format_tokens(cost_tracker.input_tokens())
    ));
    output.push_str(&format!(
        "  Output:  {:>10} tokens\n",
        CostTracker::format_tokens(cost_tracker.output_tokens())
    ));
    output.push_str(separator);
    output.push_str("\n");
    output.push_str(&format!(
        "  Total:   {:>10} tokens ({:.0}% of context window)\n",
        CostTracker::format_tokens(cost_tracker.total_tokens()),
        cost_tracker.context_percent()
    ));

    output
}

/// Represents a file that has been loaded into the context.
/// This will be used when file loading is implemented.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LoadedFile {
    /// Path to the file (relative to working directory if possible)
    pub path: PathBuf,
    /// Number of tokens in this file
    pub tokens: u64,
    /// Whether the file was explicitly loaded or auto-included
    pub explicit: bool,
}

impl LoadedFile {
    /// Create a new LoadedFile entry
    pub fn new(path: PathBuf, tokens: u64, explicit: bool) -> Self {
        Self {
            path,
            tokens,
            explicit,
        }
    }

    /// Get the file path as a display string, relative to cwd if possible
    pub fn display_path(&self) -> String {
        // Try to make path relative to current directory for cleaner display
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(relative) = self.path.strip_prefix(&cwd) {
                return relative.display().to_string();
            }
        }
        self.path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::ModelPricing;

    #[test]
    fn test_context_command_name() {
        let cmd = ContextCommand;
        assert_eq!(cmd.name(), "context");
    }

    #[test]
    fn test_context_command_description() {
        let cmd = ContextCommand;
        assert!(!cmd.description().is_empty());
        assert!(cmd.description().contains("context"));
    }

    #[test]
    fn test_context_command_output_contains_working_dir() {
        let cmd = ContextCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Current Context"));
            assert!(output.contains("Working directory:"));
            // The working directory should be a path (may vary by system)
            // but should be present
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_context_command_output_contains_loaded_files() {
        let cmd = ContextCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Loaded files:"));
            // For now, shows no files loaded
            assert!(output.contains("No files loaded"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_context_command_output_contains_token_usage() {
        let cmd = ContextCommand;
        let registry = CommandRegistry::with_defaults();
        let mut cost_tracker = CostTracker::with_default_model();
        cost_tracker.add_input_tokens(1000);
        cost_tracker.add_output_tokens(500);

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Token usage:"));
            assert!(output.contains("Input:"));
            assert!(output.contains("Output:"));
            assert!(output.contains("Total:"));
            assert!(output.contains("1,000")); // Input tokens formatted
            assert!(output.contains("500")); // Output tokens
            assert!(output.contains("1,500")); // Total tokens
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_context_command_shows_context_percentage() {
        let cmd = ContextCommand;
        let registry = CommandRegistry::with_defaults();
        let mut cost_tracker = CostTracker::new(ModelPricing::CLAUDE_3_OPUS);
        // Add 100k tokens (50% of 200k context window)
        cost_tracker.add_input_tokens(50_000);
        cost_tracker.add_output_tokens(50_000);

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("% of context window"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_loaded_file_new() {
        let file = LoadedFile::new(PathBuf::from("/test/file.rs"), 100, true);
        assert_eq!(file.path, PathBuf::from("/test/file.rs"));
        assert_eq!(file.tokens, 100);
        assert!(file.explicit);
    }

    #[test]
    fn test_loaded_file_display_path() {
        let file = LoadedFile::new(PathBuf::from("/some/absolute/path.rs"), 100, false);
        // Display path should at least contain the filename
        let display = file.display_path();
        assert!(display.contains("path.rs"));
    }

    #[test]
    fn test_context_command_zero_tokens() {
        let cmd = ContextCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            // Should show 0 tokens without errors
            assert!(output.contains("0 tokens"));
            assert!(output.contains("0% of context window"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }
}

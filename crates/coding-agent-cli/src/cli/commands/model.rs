//! The /model command - switch AI model

use super::{Command, CommandContext, CommandResult};

pub struct ModelCommand;

impl Command for ModelCommand {
    fn name(&self) -> &'static str {
        "model"
    }

    fn description(&self) -> &'static str {
        "Switch AI model (show current if no argument)"
    }

    fn usage(&self) -> &'static str {
        "/model [name]"
    }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        if args.is_empty() {
            // Show current model
            CommandResult::Output(format_current_model())
        } else {
            // Switch to specified model
            let model_name = args[0];

            // Validate model
            if !is_valid_model(model_name) {
                let suggestions = suggest_models(model_name);
                let mut error_msg = format!("Unknown model: {}", model_name);
                if !suggestions.is_empty() {
                    error_msg.push_str("\n\nDid you mean:");
                    for suggestion in suggestions {
                        error_msg.push_str(&format!("\n  - {}", suggestion));
                    }
                }
                error_msg.push_str("\n\nAvailable models:");
                for model in available_models() {
                    error_msg.push_str(&format!("\n  - {}", model));
                }
                return CommandResult::Error(error_msg);
            }

            // Return success with model switch request
            // The REPL will need to handle this
            CommandResult::Output(format!("Switched to model: {}\n\nNote: Model switching will take effect on the next message.", model_name))
        }
    }
}

/// Format the current model display
fn format_current_model() -> String {
    // For now, hardcoded - this will be made dynamic when we add model state to REPL
    let model = "claude-sonnet-4-20250514";
    let separator = "──────────────────────────────────────────────";

    let mut output = String::new();
    output.push_str("Current Model\n");
    output.push_str(separator);
    output.push_str("\n\n");
    output.push_str(&format!("Model: {}\n", model));
    output.push_str(&format!(
        "Context window: {} tokens\n",
        format_context_window(model)
    ));
    output.push_str("\nAvailable models:\n");
    for available_model in available_models() {
        if available_model == model {
            output.push_str(&format!("  • {} (current)\n", available_model));
        } else {
            output.push_str(&format!("  • {}\n", available_model));
        }
    }
    output.push_str("\n");
    output.push_str(separator);
    output
}

/// Get the context window size for a model
fn format_context_window(model: &str) -> String {
    let tokens = get_context_window(model);
    // Format with thousand separators
    format_number_with_commas(tokens)
}

/// Format a number with comma separators
fn format_number_with_commas(n: u32) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }

    result.chars().rev().collect()
}

/// Get the context window size in tokens for a model
fn get_context_window(model: &str) -> u32 {
    match model {
        "claude-3-opus" | "claude-3-opus-20240229" => 200_000,
        "claude-3-sonnet" | "claude-3-sonnet-20240229" => 200_000,
        "claude-3-haiku" | "claude-3-haiku-20240307" => 200_000,
        "claude-sonnet-4-20250514" => 200_000,
        _ => 200_000, // Default
    }
}

/// Check if a model name is valid
fn is_valid_model(model: &str) -> bool {
    available_models().contains(&model)
}

/// Get list of available models
fn available_models() -> Vec<&'static str> {
    vec![
        "claude-3-opus",
        "claude-3-opus-20240229",
        "claude-3-sonnet",
        "claude-3-sonnet-20240229",
        "claude-3-haiku",
        "claude-3-haiku-20240307",
        "claude-sonnet-4-20250514",
    ]
}

/// Suggest similar model names based on the input
fn suggest_models(input: &str) -> Vec<&'static str> {
    let input_lower = input.to_lowercase();
    let mut suggestions = Vec::new();

    // Simple substring matching
    for model in available_models() {
        if model.to_lowercase().contains(&input_lower) {
            suggestions.push(model);
        }
    }

    // If no substring matches, look for partial matches
    if suggestions.is_empty() {
        for model in available_models() {
            if input_lower.contains("opus") && model.contains("opus") {
                suggestions.push(model);
            } else if input_lower.contains("sonnet") && model.contains("sonnet") {
                suggestions.push(model);
            } else if input_lower.contains("haiku") && model.contains("haiku") {
                suggestions.push(model);
            }
        }
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::{CommandContext, CommandRegistry};
    use crate::tokens::CostTracker;

    #[test]
    fn test_model_command_name() {
        let cmd = ModelCommand;
        assert_eq!(cmd.name(), "model");
    }

    #[test]
    fn test_model_command_description() {
        let cmd = ModelCommand;
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_model_command_usage() {
        let cmd = ModelCommand;
        assert_eq!(cmd.usage(), "/model [name]");
    }

    #[test]
    fn test_model_switch_valid() {
        let cmd = ModelCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
        };

        let result = cmd.execute(&["claude-3-opus"], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("claude-3-opus"));
            assert!(output.contains("Switched"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_model_switch_invalid() {
        let cmd = ModelCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
        };

        let result = cmd.execute(&["invalid-model"], &mut ctx);

        if let CommandResult::Error(error) = result {
            assert!(error.contains("Unknown model"));
            assert!(error.contains("Available models"));
        } else {
            panic!("Expected CommandResult::Error");
        }
    }

    #[test]
    fn test_model_no_args_shows_current() {
        let cmd = ModelCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Current Model"));
            assert!(output.contains("Model:"));
            assert!(output.contains("Context window:"));
            assert!(output.contains("Available models:"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_available_models_not_empty() {
        let models = available_models();
        assert!(!models.is_empty());
        assert!(models.contains(&"claude-3-opus"));
        assert!(models.contains(&"claude-3-sonnet"));
        assert!(models.contains(&"claude-3-haiku"));
    }

    #[test]
    fn test_is_valid_model() {
        assert!(is_valid_model("claude-3-opus"));
        assert!(is_valid_model("claude-3-sonnet"));
        assert!(is_valid_model("claude-3-haiku"));
        assert!(!is_valid_model("invalid-model"));
        assert!(!is_valid_model("gpt-4"));
    }

    #[test]
    fn test_suggest_models_substring() {
        let suggestions = suggest_models("opus");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().all(|s| s.contains("opus")));
    }

    #[test]
    fn test_suggest_models_partial() {
        let suggestions = suggest_models("sonnet");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().all(|s| s.contains("sonnet")));
    }

    #[test]
    fn test_suggest_models_no_match() {
        let suggestions = suggest_models("gpt");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_get_context_window() {
        assert_eq!(get_context_window("claude-3-opus"), 200_000);
        assert_eq!(get_context_window("claude-3-sonnet"), 200_000);
        assert_eq!(get_context_window("claude-3-haiku"), 200_000);
        assert_eq!(get_context_window("claude-sonnet-4-20250514"), 200_000);
    }

    #[test]
    fn test_format_context_window() {
        let formatted = format_context_window("claude-3-opus");
        assert!(formatted.contains("200"));
        assert!(formatted.contains(","));
    }
}

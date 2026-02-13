//! The /cost command - shows detailed token usage and cost breakdown

use super::{Command, CommandContext, CommandResult};

pub struct CostCommand;

impl Command for CostCommand {
    fn name(&self) -> &'static str {
        "cost"
    }

    fn description(&self) -> &'static str {
        "Show detailed token usage and cost breakdown"
    }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandResult {
        // Get the cost tracker from the context and render the breakdown
        let breakdown = ctx.cost_tracker.render_breakdown();
        CommandResult::Output(breakdown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::{CostTracker, ModelPricing};

    #[test]
    fn test_cost_command_name() {
        let cmd = CostCommand;
        assert_eq!(cmd.name(), "cost");
    }

    #[test]
    fn test_cost_command_description() {
        let cmd = CostCommand;
        assert!(!cmd.description().is_empty());
        assert!(cmd.description().contains("token") || cmd.description().contains("cost"));
    }

    #[test]
    fn test_cost_command_output_contains_breakdown() {
        let cmd = CostCommand;
        let registry = CommandRegistry::with_defaults();
        let mut cost_tracker = CostTracker::new(ModelPricing::CLAUDE_3_OPUS);
        cost_tracker.add_input_tokens(45230);
        cost_tracker.add_output_tokens(12450);
        cost_tracker.add_message();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Session Cost Breakdown"));
            assert!(output.contains("Model:"));
            assert!(output.contains("Input tokens:"));
            assert!(output.contains("Output tokens:"));
            assert!(output.contains("Total:"));
            assert!(output.contains("Context used:"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_cost_command_shows_correct_model() {
        let cmd = CostCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::new(ModelPricing::CLAUDE_3_SONNET);

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("claude-3-sonnet"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_cost_command_shows_zero_when_empty() {
        let cmd = CostCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker,
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("0 messages"));
            // Should show $0.00 or similar for zero tokens
            assert!(output.contains("$0"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }
}

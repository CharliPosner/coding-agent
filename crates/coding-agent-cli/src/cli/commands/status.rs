//! The /status command - shows active tasks and running agents

use super::{Command, CommandContext, CommandResult};

pub struct StatusCommand;

impl Command for StatusCommand {
    fn name(&self) -> &'static str {
        "status"
    }

    fn description(&self) -> &'static str {
        "Show agent status, active tasks, running agents"
    }

    fn execute(&self, _args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        let mut output = String::from("Agent Status:\n\n");

        // For Phase 11, we don't have a multi-agent system yet
        // This is a placeholder implementation that shows the current state
        output.push_str("  Current session: Active\n");
        output.push_str("  Active tasks: None\n");
        output.push_str("  Running agents: None\n");
        output.push_str("\n");
        output.push_str("Note: Multi-agent orchestration will be available in Phase 12.\n");

        CommandResult::Output(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::CostTracker;

    #[test]
    fn test_status_command_name() {
        let cmd = StatusCommand;
        assert_eq!(cmd.name(), "status");
    }

    #[test]
    fn test_status_command_description() {
        let cmd = StatusCommand;
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_status_shows_session_active() {
        let cmd = StatusCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            // Should show session is active
            assert!(
                output.contains("Current session: Active"),
                "Status output should show active session: {}",
                output
            );
            // Should mention tasks
            assert!(
                output.contains("Active tasks"),
                "Status output should mention active tasks: {}",
                output
            );
            // Should mention agents
            assert!(
                output.contains("Running agents"),
                "Status output should mention running agents: {}",
                output
            );
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_status_shows_no_tasks_initially() {
        let cmd = StatusCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            // Should show no tasks since we don't have multi-agent yet
            assert!(
                output.contains("Active tasks: None") || output.contains("None"),
                "Status output should show no tasks: {}",
                output
            );
        } else {
            panic!("Expected CommandResult::Output");
        }
    }
}

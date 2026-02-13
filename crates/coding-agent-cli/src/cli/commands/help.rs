//! The /help command - lists all available commands

use super::{Command, CommandContext, CommandResult};

pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandResult {
        let mut output = String::from("Available commands:\n\n");

        let mut commands: Vec<_> = ctx.registry.commands().collect();
        commands.sort_by_key(|cmd| cmd.name());

        for cmd in commands {
            output.push_str(&format!("  /{:<12} {}\n", cmd.name(), cmd.description()));
        }

        output.push_str("\nType a command to execute it.");

        CommandResult::Output(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::CostTracker;

    #[test]
    fn test_help_command_name() {
        let cmd = HelpCommand;
        assert_eq!(cmd.name(), "help");
    }

    #[test]
    fn test_help_command_description() {
        let cmd = HelpCommand;
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_help_lists_all_commands() {
        let cmd = HelpCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            // Should list all registered commands
            for name in registry.command_names() {
                assert!(
                    output.contains(&format!("/{}", name)),
                    "Help output should contain /{}: {}",
                    name,
                    output
                );
            }
        } else {
            panic!("Expected CommandResult::Output");
        }
    }
}

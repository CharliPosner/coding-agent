//! The /exit command - exits the CLI

use super::{Command, CommandContext, CommandResult};

pub struct ExitCommand;

impl Command for ExitCommand {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn description(&self) -> &'static str {
        "Exit the CLI"
    }

    fn execute(&self, _args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::Exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::CostTracker;

    #[test]
    fn test_exit_command_name() {
        let cmd = ExitCommand;
        assert_eq!(cmd.name(), "exit");
    }

    #[test]
    fn test_exit_command_returns_exit() {
        let cmd = ExitCommand;
        let mut ctx = CommandContext {
            registry: CommandRegistry::with_defaults(),
            cost_tracker: CostTracker::with_default_model(),
        };

        let result = cmd.execute(&[], &mut ctx);
        assert_eq!(result, CommandResult::Exit);
    }
}

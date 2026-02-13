//! The /clear command - clears the terminal screen

use super::{Command, CommandContext, CommandResult};

pub struct ClearCommand;

impl Command for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        "Clear the screen"
    }

    fn execute(&self, _args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::Cleared
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;

    #[test]
    fn test_clear_command_name() {
        let cmd = ClearCommand;
        assert_eq!(cmd.name(), "clear");
    }

    #[test]
    fn test_clear_command_returns_cleared() {
        let cmd = ClearCommand;
        let mut ctx = CommandContext {
            registry: CommandRegistry::with_defaults(),
        };

        let result = cmd.execute(&[], &mut ctx);
        assert_eq!(result, CommandResult::Cleared);
    }
}

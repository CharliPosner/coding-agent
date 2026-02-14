//! The /results command - shows collapsed tool results

use super::{Command, CommandContext, CommandResult};

pub struct ResultsCommand;

impl Command for ResultsCommand {
    fn name(&self) -> &'static str {
        "results"
    }

    fn description(&self) -> &'static str {
        "Show the last collapsed tool results"
    }

    fn usage(&self) -> &'static str {
        "/results"
    }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandResult {
        let collapsed = ctx.collapsed_results.lock().unwrap();

        if collapsed.content.is_none() {
            return CommandResult::Output(
                "No collapsed results to show. Results are automatically collapsed when more than 5 items are returned.".to_string()
            );
        }

        let content = collapsed.content.as_ref().unwrap();
        let header = format!(
            "Showing {} {} from {}:\n\n",
            collapsed.count,
            if collapsed.count == 1 {
                "result"
            } else {
                "results"
            },
            collapsed.tool_name
        );

        CommandResult::Output(format!("{}{}", header, content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::{CollapsedResults, CommandRegistry};
    use crate::tokens::CostTracker;
    use std::sync::{Arc, Mutex};

    fn create_test_context() -> CommandContext {
        CommandContext {
            registry: CommandRegistry::with_defaults(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
            config: Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        }
    }

    #[test]
    fn test_results_command_name() {
        let cmd = ResultsCommand;
        assert_eq!(cmd.name(), "results");
    }

    #[test]
    fn test_results_command_no_collapsed_results() {
        let cmd = ResultsCommand;
        let mut ctx = create_test_context();

        let result = cmd.execute(&[], &mut ctx);
        match result {
            CommandResult::Output(msg) => {
                assert!(msg.contains("No collapsed results"));
            }
            _ => panic!("Expected Output result"),
        }
    }

    #[test]
    fn test_results_command_with_collapsed_results() {
        let cmd = ResultsCommand;
        let mut ctx = create_test_context();

        // Set up collapsed results
        {
            let mut collapsed = ctx.collapsed_results.lock().unwrap();
            collapsed.content = Some("  file1.rs\n  file2.rs\n  file3.rs\n".to_string());
            collapsed.tool_name = "code_search".to_string();
            collapsed.count = 3;
        }

        let result = cmd.execute(&[], &mut ctx);
        match result {
            CommandResult::Output(msg) => {
                assert!(msg.contains("3 results"));
                assert!(msg.contains("code_search"));
                assert!(msg.contains("file1.rs"));
            }
            _ => panic!("Expected Output result"),
        }
    }
}

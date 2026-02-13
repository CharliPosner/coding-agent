//! The /status command - shows active tasks and running agents

use super::{Command, CommandContext, CommandResult};

pub struct StatusCommand;

impl Command for StatusCommand {
    fn name(&self) -> &'static str {
        "status"
    }

    fn description(&self) -> &'static str {
        "Show agent status, active tasks, running agents. Use /cancel <id> to cancel an agent."
    }

    fn execute(&self, _args: &[&str], ctx: &mut CommandContext) -> CommandResult {
        let mut output = String::from("Agent Status:\n\n");

        // Check if agent manager is available
        if let Some(agent_manager) = &ctx.agent_manager {
            let statuses = agent_manager.get_all_statuses();

            if statuses.is_empty() {
                output.push_str("  No active agents.\n");
            } else {
                output.push_str(&format!("  Active agents: {}\n\n", statuses.len()));

                for status in &statuses {
                    let state_symbol = status.state.symbol();

                    output.push_str(&format!(
                        "  {} [{:?}] {} - {} ({}%)\n",
                        state_symbol, status.id, status.name, status.description, status.progress
                    ));
                }

                // Show instructions for cancellation
                output.push_str("\nTo cancel an agent, use: /cancel <id>\n");
            }
        } else {
            output.push_str("  Agent manager not available.\n");
        }

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
    fn test_status_shows_no_agents_when_none_active() {
        let cmd = StatusCommand;
        let registry = CommandRegistry::with_defaults();
        let manager = std::sync::Arc::new(crate::agents::manager::AgentManager::new());
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: Some(manager),
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            // Should show no active agents
            assert!(
                output.contains("No active agents"),
                "Status output should show no active agents: {}",
                output
            );
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_status_without_agent_manager() {
        let cmd = StatusCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            // Should show message about manager not being available
            assert!(
                output.contains("Agent manager not available"),
                "Status output should show manager not available: {}",
                output
            );
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[tokio::test]
    async fn test_status_shows_active_agents() {
        use crate::agents::manager::AgentManager;
        use std::sync::Arc;
        use std::time::Duration;

        let cmd = StatusCommand;
        let registry = CommandRegistry::with_defaults();
        let manager = Arc::new(AgentManager::new());

        // Spawn a test agent
        let _agent_id = manager.spawn(
            "test-agent".to_string(),
            "Testing something".to_string(),
            || {
                std::thread::sleep(Duration::from_millis(500));
                Ok("done".to_string())
            },
        );

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: Some(manager),
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            // Should show active agents count
            assert!(
                output.contains("Active agents: 1"),
                "Status output should show 1 active agent: {}",
                output
            );
            // Should show agent name
            assert!(
                output.contains("test-agent"),
                "Status output should show agent name: {}",
                output
            );
            // Should show description
            assert!(
                output.contains("Testing something"),
                "Status output should show description: {}",
                output
            );
            // Should show cancel instructions
            assert!(
                output.contains("/cancel"),
                "Status output should show cancel instructions: {}",
                output
            );
        } else {
            panic!("Expected CommandResult::Output");
        }
    }
}

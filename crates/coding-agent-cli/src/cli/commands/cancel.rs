//! The /cancel command - cancels a running agent by ID

use super::{Command, CommandContext, CommandResult};
use crate::agents::status::AgentId;

pub struct CancelCommand;

impl Command for CancelCommand {
    fn name(&self) -> &'static str {
        "cancel"
    }

    fn description(&self) -> &'static str {
        "Cancel a running agent by ID (e.g., /cancel AgentId(3))"
    }

    fn execute(&self, args: &[&str], ctx: &mut CommandContext) -> CommandResult {
        // Check if agent manager is available
        let agent_manager = match &ctx.agent_manager {
            Some(manager) => manager,
            None => {
                return CommandResult::Output("Agent manager not available.".to_string());
            }
        };

        // Parse agent ID from arguments
        if args.is_empty() {
            return CommandResult::Output(
                "Usage: /cancel <agent_id>\nExample: /cancel AgentId(3) or /cancel 3\n\nUse /status to see active agents.".to_string()
            );
        }

        let agent_id = match parse_agent_id(args[0]) {
            Some(id) => id,
            None => {
                return CommandResult::Output(format!(
                    "Invalid agent ID: {}\nUse /status to see active agents.",
                    args[0]
                ));
            }
        };

        // Check if agent exists
        if agent_manager.get_status(agent_id).is_none() {
            return CommandResult::Output(format!(
                "Agent {:?} not found. Use /status to see active agents.",
                agent_id
            ));
        }

        // Cancel the agent (this is synchronous, but the cancel operation uses async internally)
        // We need to handle the async operation
        let manager_clone = agent_manager.clone();
        let cancel_result = std::thread::spawn(move || {
            // Create a new runtime in this thread
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { manager_clone.cancel(agent_id).await })
        })
        .join()
        .unwrap();

        match cancel_result {
            Ok(()) => {
                CommandResult::Output(format!("Successfully cancelled agent {:?}.", agent_id))
            }
            Err(e) => {
                CommandResult::Output(format!("Failed to cancel agent {:?}: {}", agent_id, e))
            }
        }
    }
}

/// Parse an agent ID from a string.
/// Accepts formats: "AgentId(3)", "3"
fn parse_agent_id(s: &str) -> Option<AgentId> {
    // Try to parse "AgentId(N)" format
    if s.starts_with("AgentId(") && s.ends_with(')') {
        let num_str = &s[8..s.len() - 1];
        if let Ok(id) = num_str.parse::<u64>() {
            return Some(AgentId(id));
        }
    }

    // Try to parse just the number
    if let Ok(id) = s.parse::<u64>() {
        return Some(AgentId(id));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::manager::AgentManager;
    use crate::cli::commands::{CollapsedResults, CommandRegistry};
    use crate::tokens::CostTracker;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn test_cancel_command_name() {
        let cmd = CancelCommand;
        assert_eq!(cmd.name(), "cancel");
    }

    #[test]
    fn test_cancel_command_description() {
        let cmd = CancelCommand;
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_cancel_without_args() {
        let cmd = CancelCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: Some(Arc::new(AgentManager::new())),
            config: std::sync::Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        };

        let result = cmd.execute(&[], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Usage"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_agent() {
        let cmd = CancelCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: Some(Arc::new(AgentManager::new())),
            config: std::sync::Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        };

        let result = cmd.execute(&["999"], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("not found"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[tokio::test]
    async fn test_cancel_existing_agent() {
        let cmd = CancelCommand;
        let registry = CommandRegistry::with_defaults();
        let manager = Arc::new(AgentManager::new());

        // Spawn a long-running agent
        let id = manager.spawn(
            "test-agent".to_string(),
            "Long running task".to_string(),
            || {
                std::thread::sleep(Duration::from_secs(10));
                Ok("should not see this".to_string())
            },
        );

        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: Some(manager.clone()),
            config: std::sync::Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        };

        // Give the agent time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel it
        let id_str = format!("{}", id.0);
        let result = cmd.execute(&[&id_str], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Successfully cancelled"));
        } else {
            panic!("Expected CommandResult::Output");
        }

        // Wait for it to finish
        let wait_result = manager.wait(id).await;
        assert!(wait_result.is_err());
        assert!(wait_result.unwrap_err().contains("cancelled"));
    }

    #[test]
    fn test_parse_agent_id_number() {
        assert_eq!(parse_agent_id("42"), Some(AgentId(42)));
        assert_eq!(parse_agent_id("0"), Some(AgentId(0)));
        assert_eq!(parse_agent_id("999"), Some(AgentId(999)));
    }

    #[test]
    fn test_parse_agent_id_formatted() {
        assert_eq!(parse_agent_id("AgentId(42)"), Some(AgentId(42)));
        assert_eq!(parse_agent_id("AgentId(0)"), Some(AgentId(0)));
        assert_eq!(parse_agent_id("AgentId(999)"), Some(AgentId(999)));
    }

    #[test]
    fn test_parse_agent_id_invalid() {
        assert_eq!(parse_agent_id("abc"), None);
        assert_eq!(parse_agent_id("AgentId(abc)"), None);
        assert_eq!(parse_agent_id("AgentId(42"), None);
        assert_eq!(parse_agent_id("42)"), None);
        assert_eq!(parse_agent_id(""), None);
    }

    #[test]
    fn test_cancel_command_no_manager() {
        let cmd = CancelCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        };

        let result = cmd.execute(&["1"], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("not available"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }

    #[test]
    fn test_cancel_invalid_id_format() {
        let cmd = CancelCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry: registry.clone(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: Some(Arc::new(AgentManager::new())),
            config: std::sync::Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        };

        let result = cmd.execute(&["invalid"], &mut ctx);

        if let CommandResult::Output(output) = result {
            assert!(output.contains("Invalid agent ID"));
        } else {
            panic!("Expected CommandResult::Output");
        }
    }
}

//! The /history command - browse past conversation sessions

use super::{Command, CommandContext, CommandResult};
use crate::integrations::SessionManager;
use std::path::PathBuf;

pub struct HistoryCommand;

impl Command for HistoryCommand {
    fn name(&self) -> &'static str {
        "history"
    }

    fn description(&self) -> &'static str {
        "Browse conversation history (SpecStory)"
    }

    fn execute(&self, _args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        // Get session manager for the current working directory
        let base_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".specstory/history");

        let manager = SessionManager::new(base_dir);

        match manager.list_sessions() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    return CommandResult::Output(
                        "No conversation history found.\nStart a new session and your conversations will be saved automatically."
                            .to_string(),
                    );
                }

                let mut output = String::from("Conversation History\n");
                output.push_str(&"─".repeat(50));
                output.push('\n');
                output.push('\n');

                for (i, session) in sessions.iter().enumerate() {
                    // Format: [1] "Session Title" (2 hours ago)
                    //         └─ 12 messages
                    output.push_str(&format!(
                        "  [{}] \"{}\"\n",
                        i + 1,
                        truncate_title(&session.title, 40)
                    ));
                    output.push_str(&format!(
                        "      └─ {} messages, {}\n",
                        session.message_count,
                        session.time_ago()
                    ));
                    output.push('\n');
                }

                output.push_str(&"─".repeat(50));
                output.push('\n');
                output.push_str(&format!("Total: {} sessions\n", sessions.len()));
                output.push_str(
                    "\nTip: Use the startup screen [r] option to resume the last session.",
                );

                CommandResult::Output(output)
            }
            Err(e) => CommandResult::Error(format!("Failed to list history: {}", e)),
        }
    }
}

/// Truncate a title to a maximum length, adding "..." if needed
fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &title[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::integrations::Session;
    use crate::tokens::CostTracker;
    use tempfile::TempDir;

    #[test]
    fn test_history_command_name() {
        let cmd = HistoryCommand;
        assert_eq!(cmd.name(), "history");
    }

    #[test]
    fn test_history_command_description() {
        let cmd = HistoryCommand;
        assert!(!cmd.description().is_empty());
        assert!(cmd.description().to_lowercase().contains("history"));
    }

    #[test]
    fn test_truncate_title_short() {
        assert_eq!(truncate_title("Hello", 10), "Hello");
        assert_eq!(truncate_title("Hello", 5), "Hello");
    }

    #[test]
    fn test_truncate_title_long() {
        assert_eq!(truncate_title("Hello World!", 8), "Hello...");
        assert_eq!(
            truncate_title("This is a very long title", 15),
            "This is a ve..."
        );
    }

    #[test]
    fn test_truncate_title_edge_cases() {
        assert_eq!(truncate_title("Hi", 2), "Hi");
        assert_eq!(truncate_title("Hi", 3), "Hi");
        assert_eq!(truncate_title("", 10), "");
        assert_eq!(truncate_title("Test", 3), "...");
    }

    #[test]
    fn test_history_empty() {
        // Create a temp directory with no sessions
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

        let cmd = HistoryCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry,
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
        };

        let result = cmd.execute(&[], &mut ctx);

        match result {
            CommandResult::Output(output) => {
                assert!(output.contains("No conversation history"));
            }
            _ => panic!("Expected Output result for empty history"),
        }
    }

    #[test]
    fn test_history_lists_sessions() {
        // Create a temp directory and add some sessions
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let history_dir = temp_dir.path().join(".specstory/history");
        std::fs::create_dir_all(&history_dir).expect("Failed to create history dir");

        // Create a session
        let manager = SessionManager::new(history_dir);
        let mut session = Session::new();
        session.add_user_message("Test message");
        session.add_agent_message("Test response");
        manager.save(&mut session).expect("Failed to save session");

        // Change to temp dir
        std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

        let cmd = HistoryCommand;
        let registry = CommandRegistry::with_defaults();
        let mut ctx = CommandContext {
            registry,
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
        };

        let result = cmd.execute(&[], &mut ctx);

        match result {
            CommandResult::Output(output) => {
                assert!(output.contains("Conversation History"));
                assert!(output.contains("[1]"));
                assert!(output.contains("messages"));
                assert!(output.contains("Total: 1 sessions"));
            }
            _ => panic!("Expected Output result"),
        }
    }
}

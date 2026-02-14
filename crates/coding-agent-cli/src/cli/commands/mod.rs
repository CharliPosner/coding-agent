//! Command system for the coding-agent CLI
//!
//! This module provides a command registry pattern for slash commands.
//! Commands are registered by name and can be looked up and executed.

mod cancel;
mod clear;
mod commit;
pub mod config;
mod context;
mod cost;
mod diff;
mod document;
mod exit;
mod help;
mod history;
mod model;
mod spec;
mod status;
mod undo;

use crate::cli::Mode;
use crate::tokens::CostTracker;
use std::collections::HashMap;

/// Result of executing a command
#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    /// Command executed successfully, continue REPL
    Continue,
    /// Command requests exit
    Exit,
    /// Command cleared the screen
    Cleared,
    /// Command produced output to display
    Output(String),
    /// Command failed with an error
    Error(String),
    /// Command requests a mode change with optional output
    ModeChange { mode: Mode, output: Option<String> },
}

/// Trait for implementing commands
pub trait Command: Send + Sync {
    /// The name of the command (without the leading slash)
    fn name(&self) -> &'static str;

    /// Short description for the help listing
    fn description(&self) -> &'static str;

    /// Usage string (e.g., "/command \[args\]")
    fn usage(&self) -> &'static str {
        self.name()
    }

    /// Execute the command with the given arguments
    fn execute(&self, args: &[&str], ctx: &mut CommandContext) -> CommandResult;
}

/// Context available to commands during execution
pub struct CommandContext {
    /// Reference to the command registry for introspection
    pub registry: CommandRegistry,
    /// Cost tracker for token usage and cost calculations
    pub cost_tracker: CostTracker,
    /// Agent manager for tracking and controlling agents
    pub agent_manager: Option<std::sync::Arc<crate::agents::manager::AgentManager>>,
    /// Configuration settings
    pub config: std::sync::Arc<crate::config::Config>,
}

/// Registry of available commands
#[derive(Clone)]
pub struct CommandRegistry {
    commands: HashMap<&'static str, &'static dyn Command>,
}

impl CommandRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Create a registry with all default commands
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(&help::HelpCommand);
        registry.register(&cancel::CancelCommand);
        registry.register(&clear::ClearCommand);
        registry.register(&commit::CommitCommand);
        registry.register(&config::ConfigCommand);
        registry.register(&context::ContextCommand);
        registry.register(&cost::CostCommand);
        registry.register(&diff::DiffCommand);
        registry.register(&document::DocumentCommand);
        registry.register(&exit::ExitCommand);
        registry.register(&exit::QuitCommand);
        registry.register(&exit::QCommand);
        registry.register(&history::HistoryCommand);
        registry.register(&model::ModelCommand);
        registry.register(&spec::SpecCommand);
        registry.register(&status::StatusCommand);
        registry.register(&undo::UndoCommand);
        registry
    }

    /// Register a command
    pub fn register(&mut self, command: &'static dyn Command) {
        self.commands.insert(command.name(), command);
    }

    /// Look up a command by name
    pub fn get(&self, name: &str) -> Option<&&'static dyn Command> {
        self.commands.get(name)
    }

    /// Get all registered commands
    pub fn commands(&self) -> impl Iterator<Item = &&'static dyn Command> {
        self.commands.values()
    }

    /// Get all command names, sorted alphabetically
    pub fn command_names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.commands.keys().copied().collect();
        names.sort();
        names
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Parse a line of input to determine if it's a command
///
/// Returns None if the input is not a command (doesn't start with '/')
/// Returns Some((command_name, args)) if it is a command
pub fn parse_command(input: &str) -> Option<(&str, Vec<&str>)> {
    let input = input.trim();

    if !input.starts_with('/') {
        return None;
    }

    let input = &input[1..]; // Remove leading slash
    let mut parts = input.split_whitespace();

    let command_name = parts.next()?;
    let args: Vec<&str> = parts.collect();

    Some((command_name, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_command_parsing() {
        // Basic command
        let result = parse_command("/help");
        assert_eq!(result, Some(("help", vec![])));

        // Command with arguments
        let result = parse_command("/commit -m \"message\"");
        assert_eq!(result, Some(("commit", vec!["-m", "\"message\""])));

        // Command with multiple arguments
        let result = parse_command("/cmd arg1 arg2 arg3");
        assert_eq!(result, Some(("cmd", vec!["arg1", "arg2", "arg3"])));

        // Command with leading/trailing whitespace
        let result = parse_command("  /help  ");
        assert_eq!(result, Some(("help", vec![])));

        // Not a command (no slash)
        let result = parse_command("hello world");
        assert_eq!(result, None);

        // Empty input
        let result = parse_command("");
        assert_eq!(result, None);

        // Just a slash
        let result = parse_command("/");
        assert_eq!(result, None);
    }

    #[test]
    fn test_command_registry_lookup() {
        let registry = CommandRegistry::with_defaults();

        // Help command exists
        assert!(registry.get("help").is_some());

        // Clear command exists
        assert!(registry.get("clear").is_some());

        // Config command exists
        assert!(registry.get("config").is_some());

        // Context command exists
        assert!(registry.get("context").is_some());

        // Cost command exists
        assert!(registry.get("cost").is_some());

        // Diff command exists
        assert!(registry.get("diff").is_some());

        // Exit command exists
        assert!(registry.get("exit").is_some());

        // Quit command (alias for exit) exists
        assert!(registry.get("quit").is_some());

        // Q command (short alias for exit) exists
        assert!(registry.get("q").is_some());

        // History command exists
        assert!(registry.get("history").is_some());

        // Model command exists
        assert!(registry.get("model").is_some());

        // Spec command exists
        assert!(registry.get("spec").is_some());

        // Undo command exists
        assert!(registry.get("undo").is_some());

        // Unknown command doesn't exist
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_unknown_command_error() {
        let registry = CommandRegistry::with_defaults();

        // Looking up an unknown command returns None
        let result = registry.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_command_names_sorted() {
        let registry = CommandRegistry::with_defaults();
        let names = registry.command_names();

        // Should be sorted alphabetically
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }

    #[test]
    fn test_parse_command_edge_cases() {
        // Multiple spaces between args
        let result = parse_command("/cmd   arg1    arg2");
        assert_eq!(result, Some(("cmd", vec!["arg1", "arg2"])));

        // Tab characters
        let result = parse_command("/cmd\targ1");
        assert_eq!(result, Some(("cmd", vec!["arg1"])));

        // Command with numbers
        let result = parse_command("/test123");
        assert_eq!(result, Some(("test123", vec![])));
    }
}

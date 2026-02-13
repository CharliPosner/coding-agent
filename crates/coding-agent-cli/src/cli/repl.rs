//! REPL (Read-Eval-Print Loop) for the coding-agent CLI
//!
//! This module implements the main loop: read → parse → execute → display → repeat

use super::commands::{parse_command, CommandContext, CommandRegistry, CommandResult};
use super::input::{InputHandler, InputResult};
use super::terminal::Terminal;
use crate::config::Config;
use crate::integrations::{Session, SessionManager};
use std::io::Write;
use std::path::PathBuf;

/// REPL configuration
pub struct ReplConfig {
    /// Whether to show verbose output
    pub verbose: bool,
    /// Path for session history (relative to project root)
    pub history_path: Option<PathBuf>,
    /// Whether session persistence is enabled
    pub persistence_enabled: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            history_path: None,
            persistence_enabled: true,
        }
    }
}

impl ReplConfig {
    /// Create a ReplConfig from the application config
    pub fn from_config(config: &Config, verbose: bool) -> Self {
        Self {
            verbose,
            history_path: Some(PathBuf::from(&config.persistence.path)),
            persistence_enabled: config.persistence.enabled,
        }
    }
}

/// The main REPL loop
pub struct Repl {
    config: ReplConfig,
    registry: CommandRegistry,
    input_handler: InputHandler,
    session: Session,
    session_manager: Option<SessionManager>,
}

impl Repl {
    /// Create a new REPL with the given configuration
    pub fn new(config: ReplConfig) -> Self {
        // Initialize session manager if persistence is enabled
        let session_manager = if config.persistence_enabled {
            let base_dir = config
                .history_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(".specstory/history"));

            // Use current working directory as base
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Some(SessionManager::new(cwd.join(base_dir)))
        } else {
            None
        };

        Self {
            config,
            registry: CommandRegistry::with_defaults(),
            input_handler: InputHandler::new(),
            session: Session::new(),
            session_manager,
        }
    }

    /// Get the current session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get a mutable reference to the current session
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Save the current session to disk
    pub fn save_session(&mut self) -> Result<(), String> {
        if let Some(ref manager) = self.session_manager {
            if !self.session.is_empty() {
                manager.save(&mut self.session).map_err(|e| e.to_string())?;
                if self.config.verbose {
                    if let Some(ref path) = self.session.file_path {
                        eprintln!("[verbose] Session saved to: {:?}", path);
                    }
                }
            }
        }
        Ok(())
    }

    /// Start a new session, optionally saving the current one first
    pub fn new_session(&mut self, save_current: bool) -> Result<(), String> {
        if save_current {
            self.save_session()?;
        }
        self.session = Session::new();
        Ok(())
    }

    /// Load a session by filename
    pub fn load_session(&mut self, filename: &str) -> Result<(), String> {
        if let Some(ref manager) = self.session_manager {
            self.session = manager.load(filename).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Session persistence is disabled".to_string())
        }
    }

    /// Get the session manager
    pub fn session_manager(&self) -> Option<&SessionManager> {
        self.session_manager.as_ref()
    }

    /// Run the REPL loop
    pub async fn run(&mut self, _terminal: &mut Terminal) -> Result<(), String> {
        self.print_welcome();

        loop {
            print!("> ");
            std::io::stdout().flush().map_err(|e| e.to_string())?;

            match self.input_handler.read_input().await {
                Ok(InputResult::Submitted(text)) => {
                    if text.is_empty() {
                        continue;
                    }

                    if self.config.verbose {
                        eprintln!("[verbose] Input: {:?}", text);
                    }

                    match self.process_input(&text) {
                        ReplAction::Continue => continue,
                        ReplAction::Exit => {
                            // Save session before exiting
                            if let Err(e) = self.save_session() {
                                eprintln!("Warning: Failed to save session: {}", e);
                            }
                            println!("\nGoodbye!\n");
                            break;
                        }
                        ReplAction::Clear => {
                            // Save session before clearing
                            if let Err(e) = self.save_session() {
                                eprintln!("Warning: Failed to save session: {}", e);
                            }
                            // Start a new session
                            self.session = Session::new();
                            Terminal::clear().map_err(|e| e.to_string())?;
                            self.print_welcome();
                        }
                        ReplAction::Output(output) => {
                            println!("\n{}\n", output);
                        }
                        ReplAction::Error(error) => {
                            println!("\nError: {}\n", error);
                        }
                        ReplAction::Message(input) => {
                            // Record the user message
                            self.session.add_user_message(&input);

                            // For now, just echo regular messages
                            // In the future, this will send to the AI agent
                            let response = format!("You entered:\n{}", input);
                            println!("\n{}\n", response);

                            // Record the agent response (placeholder for now)
                            self.session.add_agent_message(&response);

                            // Auto-save after each exchange
                            if let Err(e) = self.save_session() {
                                if self.config.verbose {
                                    eprintln!(
                                        "[verbose] Warning: Failed to auto-save session: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(InputResult::Cancelled) => {
                    println!("\n[Input cleared]\n");
                }
                Ok(InputResult::Exit) => {
                    // Save session before exiting
                    if let Err(e) = self.save_session() {
                        eprintln!("Warning: Failed to save session: {}", e);
                    }
                    println!("\nGoodbye!\n");
                    break;
                }
                Err(e) => {
                    eprintln!("\nError reading input: {}\n", e);
                }
            }
        }

        Ok(())
    }

    /// Print the welcome message
    fn print_welcome(&self) {
        println!("coding-agent v0.1.0");
        println!("Type your message and press Enter twice to submit.");
        println!("Use /help for available commands.\n");
    }

    /// Process a line of input, returning the action to take
    fn process_input(&self, input: &str) -> ReplAction {
        // Check if this is a command
        if let Some((cmd_name, args)) = parse_command(input) {
            return self.execute_command(cmd_name, &args);
        }

        // Regular message
        ReplAction::Message(input.to_string())
    }

    /// Execute a slash command
    fn execute_command(&self, name: &str, args: &[&str]) -> ReplAction {
        let mut ctx = CommandContext {
            registry: self.registry.clone(),
        };

        match self.registry.get(name) {
            Some(cmd) => match cmd.execute(args, &mut ctx) {
                CommandResult::Continue => ReplAction::Continue,
                CommandResult::Exit => ReplAction::Exit,
                CommandResult::Cleared => ReplAction::Clear,
                CommandResult::Output(output) => ReplAction::Output(output),
                CommandResult::Error(error) => ReplAction::Error(error),
            },
            None => ReplAction::Error(format!(
                "Unknown command: /{}. Try /help for available commands.",
                name
            )),
        }
    }
}

/// Action to take after processing input
enum ReplAction {
    /// Continue the REPL loop
    Continue,
    /// Exit the REPL
    Exit,
    /// Clear the screen
    Clear,
    /// Display output
    Output(String),
    /// Display an error
    Error(String),
    /// A regular message (not a command)
    Message(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_input_command() {
        let repl = Repl::new(ReplConfig::default());

        // Help command
        let action = repl.process_input("/help");
        assert!(matches!(action, ReplAction::Output(_)));

        // Exit command
        let action = repl.process_input("/exit");
        assert!(matches!(action, ReplAction::Exit));

        // Clear command
        let action = repl.process_input("/clear");
        assert!(matches!(action, ReplAction::Clear));

        // Unknown command
        let action = repl.process_input("/unknown");
        assert!(matches!(action, ReplAction::Error(_)));
    }

    #[test]
    fn test_process_input_message() {
        let repl = Repl::new(ReplConfig::default());

        // Regular message (not a command)
        let action = repl.process_input("Hello, world!");
        match action {
            ReplAction::Message(msg) => assert_eq!(msg, "Hello, world!"),
            _ => panic!("Expected Message action"),
        }
    }

    #[test]
    fn test_execute_unknown_command() {
        let repl = Repl::new(ReplConfig::default());

        let action = repl.execute_command("nonexistent", &[]);
        match action {
            ReplAction::Error(msg) => {
                assert!(msg.contains("Unknown command"));
                assert!(msg.contains("/help"));
            }
            _ => panic!("Expected Error action"),
        }
    }
}

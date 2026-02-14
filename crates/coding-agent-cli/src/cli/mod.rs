//! CLI module for the coding-agent
//!
//! This module provides the main entry point for the CLI, including
//! terminal handling, input processing, and the REPL loop.

pub mod commands;
mod input;
pub mod modes;
mod repl;
mod startup;
mod terminal;

use commands::Command;

pub use input::InputHandler;
pub use modes::Mode;
pub use repl::{Repl, ReplConfig};
pub use startup::{StartupOption, StartupScreen};
pub use terminal::Terminal;

/// Run the CLI application
pub async fn run(verbose: bool) -> Result<(), String> {
    run_with_startup(verbose, true).await
}

/// Run the CLI application with optional startup screen
pub async fn run_with_startup(verbose: bool, show_startup: bool) -> Result<(), String> {
    use crate::integrations::SessionManager;
    use std::path::PathBuf;

    let mut terminal = Terminal::new().map_err(|e| e.to_string())?;

    // Set up panic handler to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Attempt to restore terminal on panic
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::cursor::Show,
            crossterm::terminal::LeaveAlternateScreen
        );
        original_hook(panic_info);
    }));

    terminal.enable_raw_mode().map_err(|e| e.to_string())?;

    if verbose {
        eprintln!("[verbose] Terminal initialized in raw mode");
    }

    // Create session manager for startup screen
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let session_manager = SessionManager::new(cwd.join(".specstory/history"));

    // Show startup screen if enabled
    let startup_result = if show_startup {
        let startup_screen = StartupScreen::new(Some(session_manager.clone()));
        Some(startup_screen.show().map_err(|e| e.to_string())?)
    } else {
        None
    };

    // Handle startup option
    match startup_result.as_ref().map(|r| &r.option) {
        Some(StartupOption::Exit) => {
            terminal.disable_raw_mode().map_err(|e| e.to_string())?;
            return Ok(());
        }
        Some(StartupOption::Help) => {
            // Show help and exit
            terminal.disable_raw_mode().map_err(|e| e.to_string())?;
            println!("\ncoding-agent - AI coding assistant\n");
            println!("Usage: code [OPTIONS]\n");
            println!("Options:");
            println!("  -v, --verbose    Enable verbose output");
            println!("  -h, --help       Show this help message\n");
            println!("Commands (in REPL):");
            println!("  /help            Show available commands");
            println!("  /clear           Clear screen and reset context");
            println!("  /history         Browse conversation history");
            println!("  /config          Open config in editor");
            println!("  /exit            Exit the CLI\n");
            return Ok(());
        }
        Some(StartupOption::Config) => {
            // Open config and exit
            terminal.disable_raw_mode().map_err(|e| e.to_string())?;
            let config_cmd = commands::config::ConfigCommand;
            let mut ctx = commands::CommandContext {
                registry: commands::CommandRegistry::with_defaults(),
                cost_tracker: crate::tokens::CostTracker::with_default_model(),
                agent_manager: None,
                config: std::sync::Arc::new(crate::config::Config::default()),
            };
            match config_cmd.execute(&[], &mut ctx) {
                commands::CommandResult::Output(msg) => println!("{}", msg),
                commands::CommandResult::Error(msg) => eprintln!("Error: {}", msg),
                _ => {}
            }
            return Ok(());
        }
        _ => {}
    }

    let config = ReplConfig {
        verbose,
        ..ReplConfig::default()
    };
    let mut repl = Repl::new(config);

    // Load session if resuming
    if let Some(StartupOption::ResumeSession(filename)) = startup_result.as_ref().map(|r| &r.option)
    {
        if let Err(e) = repl.load_session(filename) {
            eprintln!("Warning: Failed to load session: {}", e);
        } else if verbose {
            eprintln!("[verbose] Resumed session from: {}", filename);
        }
    }

    let result = repl.run(&mut terminal).await;

    terminal.disable_raw_mode().map_err(|e| e.to_string())?;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_handler_new() {
        let handler = InputHandler::new();
        assert_eq!(handler.buffer(), "");
    }

    #[test]
    fn test_terminal_new() {
        let terminal = Terminal::new();
        assert!(terminal.is_ok());
    }
}

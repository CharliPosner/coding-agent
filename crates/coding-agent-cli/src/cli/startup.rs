//! Startup screen for the coding-agent CLI
//!
//! This module provides the welcome screen displayed when the CLI starts,
//! including the ASCII logo and session selection options.

use crate::integrations::{SessionInfo, SessionManager};
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};

/// The ASCII art logo for the CLI
const ASCII_LOGO: &str = r#"
   ██████╗ ██████╗ ██████╗ ███████╗
  ██╔════╝██╔═══██╗██╔══██╗██╔════╝
  ██║     ██║   ██║██║  ██║█████╗
  ██║     ██║   ██║██║  ██║██╔══╝
  ╚██████╗╚██████╔╝██████╔╝███████╗
   ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝
"#;

/// Startup screen options
#[derive(Debug, Clone, PartialEq)]
pub enum StartupOption {
    /// Start a new session
    NewSession,
    /// Resume the last session
    ResumeSession(String), // filename
    /// Show help
    Help,
    /// Open config
    Config,
    /// Exit
    Exit,
}

/// Result of showing the startup screen
pub struct StartupResult {
    /// The option selected by the user
    pub option: StartupOption,
}

/// Display the startup screen and get user selection
pub struct StartupScreen {
    /// Session manager for accessing history
    session_manager: Option<SessionManager>,
}

impl StartupScreen {
    /// Create a new startup screen
    pub fn new(session_manager: Option<SessionManager>) -> Self {
        Self { session_manager }
    }

    /// Display the startup screen and return the selected option
    pub fn show(&self) -> io::Result<StartupResult> {
        let mut stdout = io::stdout();

        // Clear screen and move cursor to top
        execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        // Print logo in cyan
        // Note: In raw mode, \n only moves down, \r\n is needed for proper newline
        execute!(stdout, SetForegroundColor(Color::Cyan))?;
        for line in ASCII_LOGO.lines() {
            execute!(stdout, Print(format!("   {}\r\n", line)))?;
        }
        execute!(stdout, ResetColor)?;

        // Print version
        execute!(
            stdout,
            Print("\r\n   coding-agent v0.1.0\r\n\r\n"),
            SetForegroundColor(Color::Reset)
        )?;

        // Get last session info
        let last_session = self.get_last_session();

        // Print options
        // Note: In raw mode, \n only moves down, \r\n is needed for proper newline
        execute!(
            stdout,
            SetForegroundColor(Color::Yellow),
            Print("   [n]"),
            ResetColor,
            Print(" New session\r\n")
        )?;

        if let Some(ref info) = last_session {
            execute!(
                stdout,
                SetForegroundColor(Color::Yellow),
                Print("   [r]"),
                ResetColor,
                Print(" Resume last session\r\n")
            )?;

            // Show session preview with proper indentation
            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!(
                    "       └─ \"{}\" ({})\r\n",
                    truncate_title(&info.title, 40),
                    info.time_ago()
                )),
                ResetColor
            )?;
        }

        execute!(stdout, Print("\r\n"))?;

        execute!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("   [h]"),
            ResetColor,
            SetForegroundColor(Color::DarkGrey),
            Print(" Help  "),
            Print("[c]"),
            Print(" Config\r\n\r\n"),
            ResetColor
        )?;

        stdout.flush()?;

        // Read user selection
        let option = self.read_selection(&last_session)?;

        Ok(StartupResult { option })
    }

    /// Get the last session info, if any
    fn get_last_session(&self) -> Option<SessionInfo> {
        self.session_manager
            .as_ref()
            .and_then(|manager| manager.list_sessions().ok())
            .and_then(|sessions| sessions.into_iter().next())
    }

    /// Read user selection from keyboard
    fn read_selection(&self, last_session: &Option<SessionInfo>) -> io::Result<StartupOption> {
        use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

        loop {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                // Handle Ctrl+C and Ctrl+D
                if modifiers.contains(KeyModifiers::CONTROL) {
                    match code {
                        KeyCode::Char('c') | KeyCode::Char('d') => {
                            return Ok(StartupOption::Exit);
                        }
                        _ => continue,
                    }
                }

                match code {
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        return Ok(StartupOption::NewSession);
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if let Some(ref info) = last_session {
                            return Ok(StartupOption::ResumeSession(info.filename.clone()));
                        }
                        // If no last session, do nothing (ignore keypress)
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        return Ok(StartupOption::Help);
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        return Ok(StartupOption::Config);
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        return Ok(StartupOption::Exit);
                    }
                    KeyCode::Enter => {
                        // Enter defaults to new session
                        return Ok(StartupOption::NewSession);
                    }
                    _ => continue,
                }
            }
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
    }

    #[test]
    fn test_startup_option_variants() {
        let new = StartupOption::NewSession;
        let resume = StartupOption::ResumeSession("test.md".to_string());
        let help = StartupOption::Help;
        let config = StartupOption::Config;
        let exit = StartupOption::Exit;

        assert_eq!(new, StartupOption::NewSession);
        assert_eq!(resume, StartupOption::ResumeSession("test.md".to_string()));
        assert_eq!(help, StartupOption::Help);
        assert_eq!(config, StartupOption::Config);
        assert_eq!(exit, StartupOption::Exit);
    }

    #[test]
    fn test_startup_screen_new_without_manager() {
        let screen = StartupScreen::new(None);
        assert!(screen.session_manager.is_none());
    }

    #[test]
    fn test_startup_screen_get_last_session_no_manager() {
        let screen = StartupScreen::new(None);
        assert!(screen.get_last_session().is_none());
    }
}

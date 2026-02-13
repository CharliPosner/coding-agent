use crossterm::terminal;
use std::io::{self, Write};

/// Manages terminal state (raw mode, cleanup)
pub struct Terminal {
    raw_mode_enabled: bool,
}

impl Terminal {
    pub fn new() -> Result<Self, TerminalError> {
        Ok(Self {
            raw_mode_enabled: false,
        })
    }

    /// Enable raw mode for character-by-character input
    pub fn enable_raw_mode(&mut self) -> Result<(), TerminalError> {
        if !self.raw_mode_enabled {
            terminal::enable_raw_mode().map_err(TerminalError::CrosstermError)?;
            self.raw_mode_enabled = true;
        }
        Ok(())
    }

    /// Disable raw mode and restore terminal to normal state
    pub fn disable_raw_mode(&mut self) -> Result<(), TerminalError> {
        if self.raw_mode_enabled {
            terminal::disable_raw_mode().map_err(TerminalError::CrosstermError)?;
            self.raw_mode_enabled = false;
        }
        Ok(())
    }

    /// Check if raw mode is currently enabled
    pub fn is_raw_mode(&self) -> bool {
        self.raw_mode_enabled
    }

    /// Get the terminal size (columns, rows)
    pub fn size() -> Result<(u16, u16), TerminalError> {
        terminal::size().map_err(TerminalError::CrosstermError)
    }

    /// Clear the terminal screen
    pub fn clear() -> Result<(), TerminalError> {
        crossterm::execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )
        .map_err(TerminalError::CrosstermError)
    }

    /// Flush stdout
    pub fn flush() -> Result<(), TerminalError> {
        io::stdout().flush().map_err(TerminalError::IoError)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Always try to restore terminal state on drop
        if self.raw_mode_enabled {
            let _ = terminal::disable_raw_mode();
        }
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new().expect("Failed to create terminal")
    }
}

/// Errors that can occur during terminal operations
#[derive(Debug)]
pub enum TerminalError {
    CrosstermError(io::Error),
    IoError(io::Error),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalError::CrosstermError(e) => write!(f, "Terminal error: {}", e),
            TerminalError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for TerminalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TerminalError::CrosstermError(e) => Some(e),
            TerminalError::IoError(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_creation() {
        let terminal = Terminal::new();
        assert!(terminal.is_ok());
        let terminal = terminal.unwrap();
        assert!(!terminal.is_raw_mode());
    }

    #[test]
    fn test_terminal_size() {
        // This may fail in CI environments without a terminal
        // but should work in normal development
        let result = Terminal::size();
        // Just check it doesn't panic - actual size depends on environment
        // Result is OK in terminal environment, Err in CI without TTY
        let _ = result;
    }

    #[test]
    fn test_terminal_default() {
        let terminal = Terminal::default();
        assert!(!terminal.is_raw_mode());
    }

    // Note: We don't test enable_raw_mode/disable_raw_mode in unit tests
    // because they affect the actual terminal and can interfere with test runners.
    // These are tested via integration tests instead.
}

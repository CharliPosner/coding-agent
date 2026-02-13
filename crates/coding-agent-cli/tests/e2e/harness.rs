use expectrl::{Eof, Session};
use std::time::Duration;

/// Test harness for CLI PTY-based interactive testing
pub struct CliTestSession {
    session: Session,
}

impl CliTestSession {
    /// Spawn a new CLI test session
    pub fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let session = expectrl::spawn("cargo run -p coding-agent-cli")?;
        Ok(Self { session })
    }

    /// Wait for the startup screen to display
    pub fn expect_startup_screen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Wait for ASCII logo
        self.session.expect("CODE")?;
        self.session.expect("[n] New session")?;
        self.session.expect("[r] Resume last session")?;
        Ok(())
    }

    /// Select new session option
    pub fn select_new_session(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.session.send_line("n")?;
        self.session.expect(">")?; // Wait for prompt
        Ok(())
    }

    /// Send a message in the REPL
    pub fn send_message(&mut self, msg: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.session.send_line(msg)?;
        self.session.send_line("")?; // Double-enter to submit
        Ok(())
    }

    /// Wait for and capture response
    pub fn expect_response(
        &mut self,
        timeout: Duration,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.session.set_expect_timeout(Some(timeout));
        let output = self.session.expect(">")?; // Wait for next prompt
        Ok(String::from_utf8_lossy(output.as_bytes()).to_string())
    }

    /// Run a slash command
    pub fn run_command(&mut self, cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.session.send_line(cmd)?;
        self.session.send_line("")?;
        self.expect_response(Duration::from_secs(5))
    }

    /// Send Ctrl+C
    pub fn send_ctrl_c(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.session.send("\x03")?;
        Ok(())
    }

    /// Send Ctrl+D
    pub fn send_ctrl_d(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.session.send("\x04")?;
        Ok(())
    }

    /// Check if the session has exited
    /// Note: This is a simple check that may not work in all cases
    pub fn is_eof(&mut self) -> bool {
        // Try to check if session is still alive
        // For now, just return false as expectrl doesn't expose a direct is_eof method
        // Real test should use expect() with timeout to detect EOF
        false
    }

    /// Get the underlying session for advanced operations
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

impl Drop for CliTestSession {
    fn drop(&mut self) {
        // Try to exit cleanly
        let _ = self.session.send("\x04"); // Ctrl+D
        let _ = self.session.expect(Eof);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires building the binary first
    fn test_harness_spawn() {
        let result = CliTestSession::spawn();
        assert!(result.is_ok(), "Failed to spawn CLI session");
    }

    #[test]
    #[ignore]
    fn test_harness_startup_screen() {
        let mut session = CliTestSession::spawn().expect("Failed to spawn");
        let result = session.expect_startup_screen();
        assert!(result.is_ok(), "Failed to see startup screen");
    }
}

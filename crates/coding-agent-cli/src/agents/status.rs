//! Agent status tracking and state machine.
//!
//! This module defines the state machine for agents and provides status tracking.

/// Unique identifier for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentId(pub u64);

/// Agent state machine: Queued → Running → Complete/Failed/Cancelled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is queued but not yet running.
    Queued,
    /// Agent is currently running.
    Running,
    /// Agent completed successfully.
    Complete,
    /// Agent failed with an error.
    Failed,
    /// Agent was cancelled by user or system.
    Cancelled,
}

impl AgentState {
    /// Returns true if the agent is in a terminal state (complete, failed, or cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Complete | AgentState::Failed | AgentState::Cancelled)
    }

    /// Returns true if the agent is active (queued or running).
    pub fn is_active(&self) -> bool {
        matches!(self, AgentState::Queued | AgentState::Running)
    }

    /// Returns a symbol representing the state for display.
    pub fn symbol(&self) -> &'static str {
        match self {
            AgentState::Queued => "○",
            AgentState::Running => "●",
            AgentState::Complete => "✓",
            AgentState::Failed => "✗",
            AgentState::Cancelled => "⊘",
        }
    }

    /// Returns a color code for the state (ANSI escape).
    pub fn color(&self) -> &'static str {
        match self {
            AgentState::Queued => "\x1b[90m",      // Gray
            AgentState::Running => "\x1b[33m",     // Yellow
            AgentState::Complete => "\x1b[32m",    // Green
            AgentState::Failed => "\x1b[31m",      // Red
            AgentState::Cancelled => "\x1b[90m",   // Gray
        }
    }
}

/// Status information for an agent.
#[derive(Debug, Clone)]
pub struct AgentStatus {
    /// Unique identifier for this agent.
    pub id: AgentId,
    /// Human-readable name of the agent.
    pub name: String,
    /// Description of what the agent is doing.
    pub description: String,
    /// Current state of the agent.
    pub state: AgentState,
    /// Progress percentage (0-100).
    pub progress: u8,
}

impl AgentStatus {
    /// Creates a new agent status in the Queued state.
    pub fn new(id: AgentId, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            state: AgentState::Queued,
            progress: 0,
        }
    }

    /// Transitions to the Running state.
    pub fn start(&mut self) {
        if self.state == AgentState::Queued {
            self.state = AgentState::Running;
        }
    }

    /// Transitions to the Complete state with 100% progress.
    pub fn complete(&mut self) {
        self.state = AgentState::Complete;
        self.progress = 100;
    }

    /// Transitions to the Failed state.
    pub fn fail(&mut self) {
        self.state = AgentState::Failed;
    }

    /// Transitions to the Cancelled state.
    pub fn cancel(&mut self) {
        self.state = AgentState::Cancelled;
    }

    /// Updates the progress (clamped to 0-100).
    pub fn update_progress(&mut self, progress: u8) {
        self.progress = progress.min(100);
    }

    /// Returns a formatted status line for display.
    pub fn format_line(&self) -> String {
        let reset = "\x1b[0m";
        let progress_bar = self.format_progress_bar();

        format!(
            "{}{}{} {}  {}  {}{}",
            self.state.color(),
            self.state.symbol(),
            reset,
            self.name,
            self.description,
            progress_bar,
            reset
        )
    }

    /// Formats a simple progress bar for display.
    fn format_progress_bar(&self) -> String {
        if self.state.is_terminal() {
            // Don't show progress bar for completed agents
            return String::new();
        }

        let total_blocks = 20;
        let filled_blocks = (self.progress as usize * total_blocks) / 100;
        let empty_blocks = total_blocks - filled_blocks;

        format!(
            "[{}{}] {}%",
            "█".repeat(filled_blocks),
            "░".repeat(empty_blocks),
            self.progress
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_is_terminal() {
        assert!(!AgentState::Queued.is_terminal());
        assert!(!AgentState::Running.is_terminal());
        assert!(AgentState::Complete.is_terminal());
        assert!(AgentState::Failed.is_terminal());
        assert!(AgentState::Cancelled.is_terminal());
    }

    #[test]
    fn test_agent_state_is_active() {
        assert!(AgentState::Queued.is_active());
        assert!(AgentState::Running.is_active());
        assert!(!AgentState::Complete.is_active());
        assert!(!AgentState::Failed.is_active());
        assert!(!AgentState::Cancelled.is_active());
    }

    #[test]
    fn test_agent_state_symbol() {
        assert_eq!(AgentState::Queued.symbol(), "○");
        assert_eq!(AgentState::Running.symbol(), "●");
        assert_eq!(AgentState::Complete.symbol(), "✓");
        assert_eq!(AgentState::Failed.symbol(), "✗");
        assert_eq!(AgentState::Cancelled.symbol(), "⊘");
    }

    #[test]
    fn test_agent_status_new() {
        let status = AgentStatus::new(
            AgentId(1),
            "test-agent".to_string(),
            "Testing".to_string()
        );

        assert_eq!(status.id, AgentId(1));
        assert_eq!(status.name, "test-agent");
        assert_eq!(status.description, "Testing");
        assert_eq!(status.state, AgentState::Queued);
        assert_eq!(status.progress, 0);
    }

    #[test]
    fn test_agent_status_transitions() {
        let mut status = AgentStatus::new(
            AgentId(1),
            "test".to_string(),
            "desc".to_string()
        );

        // Start
        status.start();
        assert_eq!(status.state, AgentState::Running);

        // Complete
        status.complete();
        assert_eq!(status.state, AgentState::Complete);
        assert_eq!(status.progress, 100);
    }

    #[test]
    fn test_agent_status_fail() {
        let mut status = AgentStatus::new(
            AgentId(1),
            "test".to_string(),
            "desc".to_string()
        );

        status.start();
        status.fail();
        assert_eq!(status.state, AgentState::Failed);
    }

    #[test]
    fn test_agent_status_cancel() {
        let mut status = AgentStatus::new(
            AgentId(1),
            "test".to_string(),
            "desc".to_string()
        );

        status.start();
        status.cancel();
        assert_eq!(status.state, AgentState::Cancelled);
    }

    #[test]
    fn test_update_progress() {
        let mut status = AgentStatus::new(
            AgentId(1),
            "test".to_string(),
            "desc".to_string()
        );

        status.update_progress(50);
        assert_eq!(status.progress, 50);

        // Should clamp to 100
        status.update_progress(150);
        assert_eq!(status.progress, 100);
    }

    #[test]
    fn test_progress_bar_formatting() {
        let mut status = AgentStatus::new(
            AgentId(1),
            "test".to_string(),
            "desc".to_string()
        );

        status.start();
        status.update_progress(50);

        let bar = status.format_progress_bar();
        assert!(bar.contains("["));
        assert!(bar.contains("]"));
        assert!(bar.contains("50%"));
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_progress_bar_complete_no_bar() {
        let mut status = AgentStatus::new(
            AgentId(1),
            "test".to_string(),
            "desc".to_string()
        );

        status.complete();
        let bar = status.format_progress_bar();
        assert_eq!(bar, ""); // No bar for completed agents
    }

    #[test]
    fn test_format_line() {
        let mut status = AgentStatus::new(
            AgentId(1),
            "test-agent".to_string(),
            "Testing something".to_string()
        );

        status.start();
        status.update_progress(75);

        let line = status.format_line();
        assert!(line.contains("●")); // Running symbol
        assert!(line.contains("test-agent"));
        assert!(line.contains("Testing something"));
        assert!(line.contains("75%"));
    }

    #[test]
    fn test_agent_id_equality() {
        let id1 = AgentId(42);
        let id2 = AgentId(42);
        let id3 = AgentId(43);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}

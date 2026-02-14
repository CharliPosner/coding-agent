//! Multi-agent status bar for displaying parallel agent progress.
//!
//! This module provides a visual component for showing the status of multiple
//! agents running concurrently, including their progress, state, and descriptions.

use crate::agents::status::{AgentState, AgentStatus};
use crate::ui::theme::{Color, Theme};
use std::io::{self, Write};

/// Multi-agent status bar component.
///
/// Displays a visual representation of all active agents with their current
/// state, progress, and description.
pub struct StatusBar {
    theme: Theme,
    max_agents_shown: usize,
}

impl StatusBar {
    /// Creates a new status bar with the default theme.
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
            max_agents_shown: 5,
        }
    }

    /// Creates a new status bar with a custom theme.
    pub fn with_theme(theme: Theme) -> Self {
        Self {
            theme,
            max_agents_shown: 5,
        }
    }

    /// Sets the maximum number of agents to show at once.
    pub fn max_agents(&mut self, max: usize) -> &mut Self {
        self.max_agents_shown = max;
        self
    }

    /// Renders the status bar to stdout.
    ///
    /// Returns the number of lines rendered.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::ui::StatusBar;
    /// use coding_agent_cli::agents::status::{AgentStatus, AgentState, AgentId};
    ///
    /// let status_bar = StatusBar::new();
    ///
    /// let agents = vec![
    ///     AgentStatus {
    ///         id: AgentId(1),
    ///         name: "fix-agent".to_string(),
    ///         description: "Fixing missing dependency".to_string(),
    ///         state: AgentState::Running,
    ///         progress: 65,
    ///     },
    ///     AgentStatus {
    ///         id: AgentId(2),
    ///         name: "test-runner".to_string(),
    ///         description: "Running test suite".to_string(),
    ///         state: AgentState::Running,
    ///         progress: 40,
    ///     },
    /// ];
    ///
    /// // Render status bar to show agent progress
    /// let lines = status_bar.render(&agents).unwrap();
    /// println!("Rendered {} lines", lines);
    /// ```
    pub fn render(&self, agents: &[AgentStatus]) -> io::Result<usize> {
        if agents.is_empty() {
            return Ok(0);
        }

        let mut lines = 0;
        let mut stdout = io::stdout();

        // Header
        let header = self.format_header();
        writeln!(stdout, "{}", header)?;
        lines += 1;

        // Agent lines (limit to max_agents_shown)
        let agents_to_show = agents.iter().take(self.max_agents_shown);
        for agent in agents_to_show {
            let line = self.format_agent_line(agent);
            writeln!(stdout, "{}", line)?;
            lines += 1;
        }

        // Show "and N more..." if there are additional agents
        if agents.len() > self.max_agents_shown {
            let remaining = agents.len() - self.max_agents_shown;
            let more_line = self.theme.apply(
                Color::Muted,
                &format!(
                    "  ... and {} more agent{}",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                ),
            );
            writeln!(stdout, "{}", more_line)?;
            lines += 1;
        }

        stdout.flush()?;
        Ok(lines)
    }

    /// Renders the status bar to a string (for testing/snapshots).
    pub fn render_to_string(&self, agents: &[AgentStatus]) -> String {
        if agents.is_empty() {
            return String::new();
        }

        let mut result = String::new();

        // Header
        result.push_str(&self.format_header());
        result.push('\n');

        // Agent lines
        let agents_to_show = agents.iter().take(self.max_agents_shown);
        for agent in agents_to_show {
            result.push_str(&self.format_agent_line(agent));
            result.push('\n');
        }

        // "and N more..." line
        if agents.len() > self.max_agents_shown {
            let remaining = agents.len() - self.max_agents_shown;
            let more_line = self.theme.apply(
                Color::Muted,
                &format!(
                    "  ... and {} more agent{}",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                ),
            );
            result.push_str(&more_line);
            result.push('\n');
        }

        result
    }

    /// Formats the header line.
    fn format_header(&self) -> String {
        self.theme
            .apply(Color::Agent, &format!("┌─ AGENTS {}", "─".repeat(60)))
    }

    /// Formats a single agent status line.
    fn format_agent_line(&self, agent: &AgentStatus) -> String {
        let symbol_color = match agent.state {
            AgentState::Queued => Color::Muted,
            AgentState::Running => Color::Tool,
            AgentState::Complete => Color::Success,
            AgentState::Failed => Color::Error,
            AgentState::Cancelled => Color::Muted,
        };

        let symbol = agent.state.symbol();

        // Format: "│ ● agent-name    Description...    [progress bar]"
        let name_width = 20;
        let desc_width = 30;

        let name = format!("{:width$}", agent.name, width = name_width);
        let desc = if agent.description.len() > desc_width {
            format!("{}...", &agent.description[..desc_width - 3])
        } else {
            format!("{:width$}", agent.description, width = desc_width)
        };

        let progress_bar = if agent.state.is_active() {
            self.format_progress_bar(agent.progress)
        } else {
            String::new()
        };

        let pipe = self.theme.apply(Color::Agent, "│");
        let colored_symbol = self.theme.apply(symbol_color, symbol);

        format!(
            "{} {} {} {} {}",
            pipe, colored_symbol, name, desc, progress_bar
        )
    }

    /// Formats a progress bar for the given percentage.
    fn format_progress_bar(&self, progress: u8) -> String {
        let total_blocks = 10;
        let filled_blocks = (progress as usize * total_blocks) / 100;
        let empty_blocks = total_blocks - filled_blocks;

        let bar = format!("{}{}", "█".repeat(filled_blocks), "░".repeat(empty_blocks));

        self.theme.apply(Color::Success, &bar)
    }

    /// Clears the status bar by moving the cursor up and clearing lines.
    ///
    /// Call this before re-rendering to avoid accumulating output.
    pub fn clear(&self, line_count: usize) -> io::Result<()> {
        if line_count == 0 {
            return Ok(());
        }

        let mut stdout = io::stdout();

        // Move cursor up and clear each line
        for _ in 0..line_count {
            write!(stdout, "\x1b[1A")?; // Move up one line
            write!(stdout, "\x1b[2K")?; // Clear entire line
        }

        stdout.flush()?;
        Ok(())
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::status::AgentId;

    fn create_test_agent(
        id: u64,
        name: &str,
        desc: &str,
        state: AgentState,
        progress: u8,
    ) -> AgentStatus {
        let mut status = AgentStatus::new(AgentId(id), name.to_string(), desc.to_string());
        match state {
            AgentState::Running => status.start(),
            AgentState::Complete => status.complete(),
            AgentState::Failed => status.fail(),
            AgentState::Cancelled => status.cancel(),
            AgentState::Queued => {}
        }
        if state.is_active() {
            status.update_progress(progress);
        }
        status
    }

    #[test]
    fn test_status_bar_single_agent() {
        let bar = StatusBar::new();
        let agents = vec![create_test_agent(
            1,
            "test-agent",
            "Testing something",
            AgentState::Running,
            50,
        )];

        let output = bar.render_to_string(&agents);

        assert!(output.contains("AGENTS"));
        assert!(output.contains("test-agent"));
        assert!(output.contains("Testing something"));
        assert!(output.contains("●")); // Running symbol
        assert!(output.contains("█")); // Progress bar
    }

    #[test]
    fn test_status_bar_multiple_agents() {
        let bar = StatusBar::new();
        let agents = vec![
            create_test_agent(1, "agent-1", "First task", AgentState::Running, 25),
            create_test_agent(2, "agent-2", "Second task", AgentState::Running, 75),
            create_test_agent(3, "agent-3", "Third task", AgentState::Queued, 0),
        ];

        let output = bar.render_to_string(&agents);

        assert!(output.contains("agent-1"));
        assert!(output.contains("agent-2"));
        assert!(output.contains("agent-3"));
        assert!(output.contains("First task"));
        assert!(output.contains("Second task"));
        assert!(output.contains("Third task"));
    }

    #[test]
    fn test_status_bar_empty() {
        let bar = StatusBar::new();
        let agents = vec![];

        let output = bar.render_to_string(&agents);
        assert_eq!(output, "");
    }

    #[test]
    fn test_status_bar_max_agents_limit() {
        let mut bar = StatusBar::new();
        bar.max_agents(3);

        let agents = vec![
            create_test_agent(1, "a1", "d1", AgentState::Running, 10),
            create_test_agent(2, "a2", "d2", AgentState::Running, 20),
            create_test_agent(3, "a3", "d3", AgentState::Running, 30),
            create_test_agent(4, "a4", "d4", AgentState::Running, 40),
            create_test_agent(5, "a5", "d5", AgentState::Running, 50),
        ];

        let output = bar.render_to_string(&agents);

        assert!(output.contains("a1"));
        assert!(output.contains("a2"));
        assert!(output.contains("a3"));
        assert!(!output.contains("a4"));
        assert!(!output.contains("a5"));
        assert!(output.contains("and 2 more agents"));
    }

    #[test]
    fn test_status_bar_completed_no_progress() {
        let bar = StatusBar::new();
        let agents = vec![create_test_agent(
            1,
            "done-agent",
            "Completed",
            AgentState::Complete,
            100,
        )];

        let output = bar.render_to_string(&agents);

        assert!(output.contains("done-agent"));
        assert!(output.contains("✓")); // Complete symbol
                                       // Progress bar should not be shown for completed agents
        let lines: Vec<&str> = output.lines().collect();
        let agent_line = lines
            .iter()
            .find(|l| l.contains("done-agent"))
            .expect("Should have agent line");
        // Completed agents shouldn't have progress bars
        assert!(!agent_line.contains("█") || agent_line.matches("█").count() == 0);
    }

    #[test]
    fn test_status_bar_failed_state() {
        let bar = StatusBar::new();
        let agents = vec![create_test_agent(
            1,
            "failed-agent",
            "Something went wrong",
            AgentState::Failed,
            0,
        )];

        let output = bar.render_to_string(&agents);

        assert!(output.contains("failed-agent"));
        assert!(output.contains("✗")); // Failed symbol
    }

    #[test]
    fn test_status_bar_cancelled_state() {
        let bar = StatusBar::new();
        let agents = vec![create_test_agent(
            1,
            "cancelled-agent",
            "User cancelled",
            AgentState::Cancelled,
            0,
        )];

        let output = bar.render_to_string(&agents);

        assert!(output.contains("cancelled-agent"));
        assert!(output.contains("⊘")); // Cancelled symbol
    }

    #[test]
    fn test_progress_bar_formatting() {
        let bar = StatusBar::new();

        // 0% progress
        let bar_0 = bar.format_progress_bar(0);
        assert!(bar_0.contains("░"));
        assert!(!bar_0.contains("█"));

        // 50% progress
        let bar_50 = bar.format_progress_bar(50);
        assert!(bar_50.contains("█"));
        assert!(bar_50.contains("░"));

        // 100% progress
        let bar_100 = bar.format_progress_bar(100);
        assert!(bar_100.contains("█"));
        assert!(!bar_100.contains("░"));
    }

    #[test]
    fn test_long_description_truncation() {
        let bar = StatusBar::new();
        let long_desc = "This is a very long description that should be truncated to fit";
        let agents = vec![create_test_agent(
            1,
            "agent",
            long_desc,
            AgentState::Running,
            50,
        )];

        let output = bar.render_to_string(&agents);

        // Should contain "..." for truncation
        let lines: Vec<&str> = output.lines().collect();
        let agent_line = lines
            .iter()
            .find(|l| l.contains("agent"))
            .expect("Should have agent line");
        assert!(agent_line.contains("..."));
    }

    #[test]
    fn test_header_format() {
        let bar = StatusBar::new();
        let header = bar.format_header();

        assert!(header.contains("AGENTS"));
        assert!(header.contains("─")); // Box drawing character
    }

    #[test]
    fn test_agent_line_format() {
        let bar = StatusBar::new();
        let agent = create_test_agent(1, "test", "description", AgentState::Running, 50);

        let line = bar.format_agent_line(&agent);

        assert!(line.contains("│")); // Box drawing character
        assert!(line.contains("●")); // Running symbol
        assert!(line.contains("test"));
        assert!(line.contains("description"));
        assert!(line.contains("█")); // Progress bar
    }

    #[test]
    fn test_queued_agent_with_progress() {
        let bar = StatusBar::new();
        let agents = vec![create_test_agent(
            1,
            "queued-agent",
            "Waiting to start",
            AgentState::Queued,
            0,
        )];

        let output = bar.render_to_string(&agents);

        assert!(output.contains("queued-agent"));
        assert!(output.contains("○")); // Queued symbol
                                       // Queued agents should show progress bar (they are active)
        assert!(output.contains("░")); // Empty progress bar
    }
}

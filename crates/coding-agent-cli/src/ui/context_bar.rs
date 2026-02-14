//! Context bar component for showing token usage at the bottom of the screen.
//!
//! The context bar displays:
//! - Visual progress bar of context window usage
//! - Percentage used
//! - Current tokens / max tokens
//!
//! Color coding:
//! - Green: 0-60% usage
//! - Yellow: 60-85% usage
//! - Red: 85%+ usage

use crate::ui::theme::{Color, Theme};
use std::io::{self, Write};

/// Threshold percentages for color changes.
const YELLOW_THRESHOLD: u64 = 60;
const RED_THRESHOLD: u64 = 85;

/// Default context bar width (in characters).
const DEFAULT_BAR_WIDTH: usize = 30;

/// Context bar state and rendering.
#[derive(Debug, Clone)]
pub struct ContextBar {
    /// Current token count.
    current_tokens: u64,
    /// Maximum token count (context window size).
    max_tokens: u64,
    /// Width of the progress bar in characters.
    bar_width: usize,
    /// Theme for styling.
    theme: Theme,
}

impl ContextBar {
    /// Create a new context bar with the given maximum tokens.
    ///
    /// # Arguments
    ///
    /// * `max_tokens` - The context window size in tokens
    pub fn new(max_tokens: u64) -> Self {
        Self {
            current_tokens: 0,
            max_tokens,
            bar_width: DEFAULT_BAR_WIDTH,
            theme: Theme::default(),
        }
    }

    /// Create a context bar with a custom theme.
    pub fn with_theme(max_tokens: u64, theme: Theme) -> Self {
        Self {
            current_tokens: 0,
            max_tokens,
            bar_width: DEFAULT_BAR_WIDTH,
            theme,
        }
    }

    /// Set the bar width in characters.
    pub fn set_bar_width(&mut self, width: usize) {
        self.bar_width = width;
    }

    /// Update the current token count.
    pub fn set_tokens(&mut self, tokens: u64) {
        self.current_tokens = tokens;
    }

    /// Add tokens to the current count.
    pub fn add_tokens(&mut self, tokens: u64) {
        self.current_tokens = self.current_tokens.saturating_add(tokens);
    }

    /// Reset the token count to zero.
    pub fn reset(&mut self) {
        self.current_tokens = 0;
    }

    /// Get the current token count.
    pub fn current_tokens(&self) -> u64 {
        self.current_tokens
    }

    /// Get the maximum token count.
    pub fn max_tokens(&self) -> u64 {
        self.max_tokens
    }

    /// Update the maximum tokens (when switching models).
    pub fn set_max_tokens(&mut self, max_tokens: u64) {
        self.max_tokens = max_tokens;
    }

    /// Calculate the percentage of context used.
    ///
    /// Returns a value between 0 and 100.
    pub fn percent(&self) -> u64 {
        if self.max_tokens == 0 {
            return 0;
        }
        ((self.current_tokens as f64 / self.max_tokens as f64) * 100.0).min(100.0) as u64
    }

    /// Get the appropriate color for the current usage level.
    ///
    /// - Green: 0-60%
    /// - Yellow: 60-85%
    /// - Red: 85%+
    pub fn usage_color(&self) -> Color {
        let pct = self.percent();
        if pct >= RED_THRESHOLD {
            Color::ContextRed
        } else if pct >= YELLOW_THRESHOLD {
            Color::ContextYellow
        } else {
            Color::ContextGreen
        }
    }

    /// Render the progress bar portion as a string.
    ///
    /// Returns a string like "████████████░░░░░░░░░░░░░░░░░░"
    fn render_bar(&self) -> String {
        let filled = if self.max_tokens == 0 {
            0
        } else {
            ((self.current_tokens as f64 / self.max_tokens as f64) * self.bar_width as f64) as usize
        };
        let filled = filled.min(self.bar_width);
        let empty = self.bar_width - filled;

        let filled_char = '█';
        let empty_char = '░';

        format!(
            "{}{}",
            filled_char.to_string().repeat(filled),
            empty_char.to_string().repeat(empty)
        )
    }

    /// Format token count for display (e.g., "76k" or "200k").
    fn format_tokens(tokens: u64) -> String {
        if tokens >= 1000 {
            format!("{}k", tokens / 1000)
        } else {
            tokens.to_string()
        }
    }

    /// Render the context bar as a styled string.
    ///
    /// Output format: "Context: [████████████░░░░░░░░░░░░░░░░░░]  38% used | 76k / 200k tokens"
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::ui::ContextBar;
    ///
    /// let mut bar = ContextBar::new(200_000);
    /// bar.set_tokens(76_000);
    ///
    /// // Render the context bar
    /// let output = bar.render();
    /// println!("{}", output);
    /// // => "Context: [████████████░░░░░░░░░░░░░░░░░░]  38% used | 76k / 200k tokens"
    ///
    /// // Check usage level
    /// if bar.percent() > 80 {
    ///     eprintln!("Warning: Context usage is high!");
    /// }
    /// ```
    pub fn render(&self) -> String {
        let bar = self.render_bar();
        let color = self.usage_color();
        let pct = self.percent();

        let current_str = Self::format_tokens(self.current_tokens);
        let max_str = Self::format_tokens(self.max_tokens);

        let bar_styled = self.theme.apply(color, &bar);

        format!(
            "Context: [{}] {:>3}% used | {} / {} tokens",
            bar_styled, pct, current_str, max_str
        )
    }

    /// Render a compact version for narrow terminals.
    ///
    /// Output format: "Context: [████████░░░░░░░░░░░░] 25% | 50k"
    pub fn render_compact(&self) -> String {
        let bar = self.render_bar();
        let color = self.usage_color();
        let pct = self.percent();
        let current_str = Self::format_tokens(self.current_tokens);

        let bar_styled = self.theme.apply(color, &bar);

        format!("Context: [{}] {}% | {}", bar_styled, pct, current_str)
    }

    /// Print the context bar to stdout.
    pub fn print(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        writeln!(stdout, "{}", self.render())?;
        stdout.flush()
    }

    /// Print without a trailing newline (for status line updates).
    pub fn print_inline(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        write!(stdout, "\r{}", self.render())?;
        stdout.flush()
    }
}

impl Default for ContextBar {
    fn default() -> Self {
        // Default to Claude's context window size
        Self::new(200_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::ThemeStyle;

    #[test]
    fn test_context_bar_new() {
        let bar = ContextBar::new(200_000);
        assert_eq!(bar.current_tokens(), 0);
        assert_eq!(bar.max_tokens(), 200_000);
        assert_eq!(bar.percent(), 0);
    }

    #[test]
    fn test_context_bar_set_tokens() {
        let mut bar = ContextBar::new(100_000);
        bar.set_tokens(50_000);
        assert_eq!(bar.current_tokens(), 50_000);
        assert_eq!(bar.percent(), 50);
    }

    #[test]
    fn test_context_bar_add_tokens() {
        let mut bar = ContextBar::new(100_000);
        bar.add_tokens(10_000);
        bar.add_tokens(20_000);
        assert_eq!(bar.current_tokens(), 30_000);
    }

    #[test]
    fn test_context_bar_reset() {
        let mut bar = ContextBar::new(100_000);
        bar.set_tokens(50_000);
        bar.reset();
        assert_eq!(bar.current_tokens(), 0);
    }

    #[test]
    fn test_context_bar_color_green() {
        let mut bar = ContextBar::new(100);

        // 0% - green
        bar.set_tokens(0);
        assert_eq!(bar.usage_color(), Color::ContextGreen);

        // 30% - green
        bar.set_tokens(30);
        assert_eq!(bar.usage_color(), Color::ContextGreen);

        // 59% - green
        bar.set_tokens(59);
        assert_eq!(bar.usage_color(), Color::ContextGreen);
    }

    #[test]
    fn test_context_bar_color_yellow() {
        let mut bar = ContextBar::new(100);

        // 60% - yellow
        bar.set_tokens(60);
        assert_eq!(bar.usage_color(), Color::ContextYellow);

        // 70% - yellow
        bar.set_tokens(70);
        assert_eq!(bar.usage_color(), Color::ContextYellow);

        // 84% - yellow
        bar.set_tokens(84);
        assert_eq!(bar.usage_color(), Color::ContextYellow);
    }

    #[test]
    fn test_context_bar_color_red() {
        let mut bar = ContextBar::new(100);

        // 85% - red
        bar.set_tokens(85);
        assert_eq!(bar.usage_color(), Color::ContextRed);

        // 90% - red
        bar.set_tokens(90);
        assert_eq!(bar.usage_color(), Color::ContextRed);

        // 100% - red
        bar.set_tokens(100);
        assert_eq!(bar.usage_color(), Color::ContextRed);
    }

    #[test]
    fn test_context_bar_percent_zero_max() {
        let bar = ContextBar::new(0);
        assert_eq!(bar.percent(), 0);
    }

    #[test]
    fn test_context_bar_percent_overflow() {
        let mut bar = ContextBar::new(100);
        bar.set_tokens(150); // Over 100%
        assert_eq!(bar.percent(), 100); // Capped at 100
    }

    #[test]
    fn test_context_bar_at_100_percent() {
        let mut bar = ContextBar::new(200_000);
        bar.set_tokens(200_000);
        assert_eq!(bar.percent(), 100);
        assert_eq!(bar.usage_color(), Color::ContextRed);

        // Render should not panic
        let rendered = bar.render();
        assert!(rendered.contains("100%"));
    }

    #[test]
    fn test_context_bar_render_bar() {
        let mut bar = ContextBar::new(100);
        bar.set_bar_width(10); // 10 chars for easy testing

        // 0%
        bar.set_tokens(0);
        let rendered = bar.render_bar();
        assert_eq!(rendered.chars().filter(|&c| c == '█').count(), 0);
        assert_eq!(rendered.chars().filter(|&c| c == '░').count(), 10);

        // 50%
        bar.set_tokens(50);
        let rendered = bar.render_bar();
        assert_eq!(rendered.chars().filter(|&c| c == '█').count(), 5);
        assert_eq!(rendered.chars().filter(|&c| c == '░').count(), 5);

        // 100%
        bar.set_tokens(100);
        let rendered = bar.render_bar();
        assert_eq!(rendered.chars().filter(|&c| c == '█').count(), 10);
        assert_eq!(rendered.chars().filter(|&c| c == '░').count(), 0);
    }

    #[test]
    fn test_context_bar_format_tokens() {
        assert_eq!(ContextBar::format_tokens(500), "500");
        assert_eq!(ContextBar::format_tokens(1000), "1k");
        assert_eq!(ContextBar::format_tokens(50_000), "50k");
        assert_eq!(ContextBar::format_tokens(200_000), "200k");
    }

    #[test]
    fn test_context_bar_render_contains_expected_parts() {
        let mut bar = ContextBar::new(200_000);
        bar.set_tokens(76_000);

        let rendered = bar.render();
        assert!(rendered.contains("Context:"));
        assert!(rendered.contains("38%")); // 76000/200000 = 38%
        assert!(rendered.contains("76k"));
        assert!(rendered.contains("200k"));
        assert!(rendered.contains("tokens"));
    }

    #[test]
    fn test_context_bar_render_compact() {
        let mut bar = ContextBar::new(200_000);
        bar.set_tokens(50_000);

        let rendered = bar.render_compact();
        assert!(rendered.contains("Context:"));
        assert!(rendered.contains("25%"));
        assert!(rendered.contains("50k"));
        // Compact version doesn't include "tokens"
        assert!(!rendered.contains("tokens"));
    }

    #[test]
    fn test_context_bar_with_theme() {
        let theme = Theme::new(ThemeStyle::Monochrome);
        let bar = ContextBar::with_theme(200_000, theme);
        assert_eq!(bar.max_tokens(), 200_000);

        // Should render without colors
        let rendered = bar.render();
        assert!(rendered.contains("Context:"));
    }

    #[test]
    fn test_context_bar_set_max_tokens() {
        let mut bar = ContextBar::new(200_000);
        bar.set_tokens(100_000);
        assert_eq!(bar.percent(), 50);

        // Switch to a smaller context window
        bar.set_max_tokens(100_000);
        assert_eq!(bar.percent(), 100);
    }

    #[test]
    fn test_context_bar_default() {
        let bar = ContextBar::default();
        assert_eq!(bar.max_tokens(), 200_000);
        assert_eq!(bar.current_tokens(), 0);
    }

    #[test]
    fn test_context_bar_add_tokens_saturating() {
        let mut bar = ContextBar::new(u64::MAX);
        bar.set_tokens(u64::MAX - 10);
        bar.add_tokens(100); // Would overflow
        assert_eq!(bar.current_tokens(), u64::MAX); // Saturated
    }
}

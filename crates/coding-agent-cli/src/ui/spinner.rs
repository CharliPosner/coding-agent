//! Spinner component for showing progress during operations

use indicatif::{ProgressBar as IndicatifBar, ProgressStyle};
use std::time::Duration;

/// Spinner for showing activity during long-running operations
pub struct Spinner {
    bar: IndicatifBar,
    messages: Vec<&'static str>,
    message_index: usize,
}

/// Default thinking messages
const DEFAULT_MESSAGES: &[&str] = &[
    "Pondering...",
    "Percolating...",
    "Cogitating...",
    "Mulling it over...",
    "Connecting dots...",
];

impl Spinner {
    /// Create a new spinner with default messages
    pub fn new() -> Self {
        Self::with_messages(DEFAULT_MESSAGES.to_vec())
    }

    /// Create a spinner with custom messages
    pub fn with_messages(messages: Vec<&'static str>) -> Self {
        let bar = IndicatifBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.yellow} {msg}")
                .unwrap(),
        );
        bar.enable_steady_tick(Duration::from_millis(80));

        let spinner = Self {
            bar,
            messages,
            message_index: 0,
        };

        // Set initial message
        if !spinner.messages.is_empty() {
            spinner.bar.set_message(spinner.messages[0]);
        }

        spinner
    }

    /// Create a spinner with a single static message
    pub fn with_message(message: &'static str) -> Self {
        let bar = IndicatifBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.yellow} {msg}")
                .unwrap(),
        );
        bar.set_message(message);
        bar.enable_steady_tick(Duration::from_millis(80));

        Self {
            bar,
            messages: vec![message],
            message_index: 0,
        }
    }

    /// Set the current message
    pub fn set_message(&mut self, message: &str) {
        self.bar.set_message(message.to_string());
    }

    /// Cycle to the next message
    pub fn next_message(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        self.message_index = (self.message_index + 1) % self.messages.len();
        self.bar.set_message(self.messages[self.message_index]);
    }

    /// Get the current message
    pub fn current_message(&self) -> &str {
        if self.messages.is_empty() {
            return "";
        }
        self.messages[self.message_index]
    }

    /// Stop the spinner with a success message
    pub fn finish_with_message(&self, message: &str) {
        self.bar.finish_with_message(message.to_string());
    }

    /// Stop the spinner and clear it from the terminal
    pub fn finish_and_clear(&self) {
        self.bar.finish_and_clear();
    }

    /// Stop the spinner
    pub fn finish(&self) {
        self.bar.finish();
    }

    /// Check if the spinner is finished
    pub fn is_finished(&self) -> bool {
        self.bar.is_finished()
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        // Ensure spinner is cleaned up
        if !self.bar.is_finished() {
            self.bar.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_messages_cycle() {
        let messages = vec!["Message 1", "Message 2", "Message 3"];
        let mut spinner = Spinner::with_messages(messages);

        assert_eq!(spinner.current_message(), "Message 1");

        spinner.next_message();
        assert_eq!(spinner.current_message(), "Message 2");

        spinner.next_message();
        assert_eq!(spinner.current_message(), "Message 3");

        // Should wrap around
        spinner.next_message();
        assert_eq!(spinner.current_message(), "Message 1");
    }

    #[test]
    fn test_spinner_stops_cleanly() {
        let spinner = Spinner::new();
        assert!(!spinner.is_finished());

        spinner.finish_and_clear();
        assert!(spinner.is_finished());
    }

    #[test]
    fn test_spinner_with_single_message() {
        let spinner = Spinner::with_message("Loading...");
        assert_eq!(spinner.current_message(), "Loading...");
    }

    #[test]
    fn test_spinner_default_messages() {
        let spinner = Spinner::new();

        // Should have default messages
        assert!(!spinner.messages.is_empty());
        assert_eq!(spinner.messages.len(), DEFAULT_MESSAGES.len());
    }

    #[test]
    fn test_spinner_empty_messages() {
        let spinner = Spinner::with_messages(vec![]);
        assert_eq!(spinner.current_message(), "");

        // Should not panic on next_message
        let mut spinner = spinner;
        spinner.next_message();
    }
}

//! Thinking messages for AI agent waiting periods
//!
//! Provides rotating contextual messages to display while the agent is processing.

use std::time::{Duration, Instant};

/// A manager for rotating thinking messages
pub struct ThinkingMessages {
    messages: Vec<&'static str>,
    current_index: usize,
    last_rotation: Instant,
    rotation_interval: Duration,
}

impl ThinkingMessages {
    /// Create a new ThinkingMessages with default messages
    pub fn new() -> Self {
        Self::with_messages(vec![
            "Pondering...",
            "Percolating...",
            "Cogitating...",
            "Mulling it over...",
            "Connecting dots...",
        ])
    }

    /// Create a new ThinkingMessages with custom messages
    pub fn with_messages(messages: Vec<&'static str>) -> Self {
        Self {
            messages,
            current_index: 0,
            last_rotation: Instant::now(),
            rotation_interval: Duration::from_secs(3),
        }
    }

    /// Set the rotation interval (how often messages change)
    pub fn with_rotation_interval(mut self, interval: Duration) -> Self {
        self.rotation_interval = interval;
        self
    }

    /// Get the current thinking message
    pub fn current(&self) -> &'static str {
        if self.messages.is_empty() {
            "Thinking..."
        } else {
            self.messages[self.current_index]
        }
    }

    /// Advance to the next message if enough time has passed
    /// Returns true if the message changed
    pub fn tick(&mut self) -> bool {
        if self.messages.is_empty() {
            return false;
        }

        if self.last_rotation.elapsed() >= self.rotation_interval {
            self.current_index = (self.current_index + 1) % self.messages.len();
            self.last_rotation = Instant::now();
            true
        } else {
            false
        }
    }

    /// Force advance to the next message
    pub fn next(&mut self) -> &'static str {
        if !self.messages.is_empty() {
            self.current_index = (self.current_index + 1) % self.messages.len();
            self.last_rotation = Instant::now();
        }
        self.current()
    }

    /// Reset to the first message
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.last_rotation = Instant::now();
    }

    /// Get the number of available messages
    pub fn count(&self) -> usize {
        self.messages.len()
    }
}

impl Default for ThinkingMessages {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_default_messages() {
        let thinking = ThinkingMessages::new();
        let expected = vec![
            "Pondering...",
            "Percolating...",
            "Cogitating...",
            "Mulling it over...",
            "Connecting dots...",
        ];
        assert_eq!(thinking.count(), expected.len());
        assert_eq!(thinking.current(), expected[0]);
    }

    #[test]
    fn test_custom_messages() {
        let messages = vec!["Thinking...", "Processing...", "Working..."];
        let thinking = ThinkingMessages::with_messages(messages.clone());
        assert_eq!(thinking.count(), 3);
        assert_eq!(thinking.current(), "Thinking...");
    }

    #[test]
    fn test_next_cycles_messages() {
        let mut thinking = ThinkingMessages::with_messages(vec!["A", "B", "C"]);
        assert_eq!(thinking.current(), "A");
        assert_eq!(thinking.next(), "B");
        assert_eq!(thinking.next(), "C");
        assert_eq!(thinking.next(), "A"); // Cycles back
    }

    #[test]
    fn test_tick_changes_after_interval() {
        let mut thinking = ThinkingMessages::with_messages(vec!["A", "B"])
            .with_rotation_interval(Duration::from_millis(50));

        assert_eq!(thinking.current(), "A");
        assert!(!thinking.tick()); // Too soon

        thread::sleep(Duration::from_millis(60));
        assert!(thinking.tick()); // Enough time passed
        assert_eq!(thinking.current(), "B");
    }

    #[test]
    fn test_tick_no_change_before_interval() {
        let mut thinking = ThinkingMessages::with_messages(vec!["A", "B"])
            .with_rotation_interval(Duration::from_secs(10));

        assert_eq!(thinking.current(), "A");
        assert!(!thinking.tick());
        assert_eq!(thinking.current(), "A"); // Still A
    }

    #[test]
    fn test_reset() {
        let mut thinking = ThinkingMessages::with_messages(vec!["A", "B", "C"]);
        thinking.next();
        thinking.next();
        assert_eq!(thinking.current(), "C");

        thinking.reset();
        assert_eq!(thinking.current(), "A");
    }

    #[test]
    fn test_empty_messages_fallback() {
        let thinking = ThinkingMessages::with_messages(vec![]);
        assert_eq!(thinking.current(), "Thinking...");
        assert_eq!(thinking.count(), 0);
    }

    #[test]
    fn test_tick_empty_messages() {
        let mut thinking = ThinkingMessages::with_messages(vec![]);
        assert!(!thinking.tick());
        assert_eq!(thinking.current(), "Thinking...");
    }

    #[test]
    fn test_rotation_interval_customization() {
        let thinking = ThinkingMessages::new().with_rotation_interval(Duration::from_secs(5));

        // Verify interval is set (can't directly test private field, but we can test behavior)
        assert_eq!(thinking.current(), "Pondering...");
    }
}

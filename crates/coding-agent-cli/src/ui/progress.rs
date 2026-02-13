//! Progress bar component for showing operation progress

use indicatif::{ProgressBar as IndicatifBar, ProgressStyle};

/// Progress bar for showing completion percentage
pub struct ProgressBar {
    bar: IndicatifBar,
}

impl ProgressBar {
    /// Create a new progress bar with the given total
    pub fn new(total: u64) -> Self {
        let bar = IndicatifBar::new(total);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.green/dim}] {pos}/{len} ({percent}%)")
                .unwrap()
                .progress_chars("██░"),
        );

        Self { bar }
    }

    /// Create a progress bar styled for context usage
    pub fn context_bar(total: u64) -> Self {
        let bar = IndicatifBar::new(total);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("Context: [{bar:30}] {percent}% | {pos}/{len}")
                .unwrap()
                .progress_chars("█▓░"),
        );

        Self { bar }
    }

    /// Set the current progress
    pub fn set_position(&self, pos: u64) {
        self.bar.set_position(pos);
    }

    /// Increment the progress by the given amount
    pub fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }

    /// Get the current position
    pub fn position(&self) -> u64 {
        self.bar.position()
    }

    /// Get the total length
    pub fn length(&self) -> Option<u64> {
        self.bar.length()
    }

    /// Set a message to display alongside the progress bar
    pub fn set_message(&self, message: &str) {
        self.bar.set_message(message.to_string());
    }

    /// Finish the progress bar
    pub fn finish(&self) {
        self.bar.finish();
    }

    /// Finish with a message
    pub fn finish_with_message(&self, message: &str) {
        self.bar.finish_with_message(message.to_string());
    }

    /// Finish and clear the progress bar from the terminal
    pub fn finish_and_clear(&self) {
        self.bar.finish_and_clear();
    }

    /// Check if the progress bar is finished
    pub fn is_finished(&self) -> bool {
        self.bar.is_finished()
    }

    /// Get the percentage complete (0-100)
    pub fn percent(&self) -> u64 {
        let pos = self.bar.position();
        let len = self.bar.length().unwrap_or(100);
        if len == 0 {
            return 0;
        }
        (pos * 100) / len
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        if !self.bar.is_finished() {
            self.bar.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_0_to_100() {
        let bar = ProgressBar::new(100);

        // Start at 0%
        assert_eq!(bar.position(), 0);
        assert_eq!(bar.percent(), 0);

        // Set to 50%
        bar.set_position(50);
        assert_eq!(bar.position(), 50);
        assert_eq!(bar.percent(), 50);

        // Set to 100%
        bar.set_position(100);
        assert_eq!(bar.position(), 100);
        assert_eq!(bar.percent(), 100);
    }

    #[test]
    fn test_progress_bar_increment() {
        let bar = ProgressBar::new(100);

        bar.inc(10);
        assert_eq!(bar.position(), 10);

        bar.inc(20);
        assert_eq!(bar.position(), 30);
    }

    #[test]
    fn test_progress_bar_length() {
        let bar = ProgressBar::new(200);
        assert_eq!(bar.length(), Some(200));
    }

    #[test]
    fn test_progress_bar_finish() {
        let bar = ProgressBar::new(100);
        assert!(!bar.is_finished());

        bar.finish();
        assert!(bar.is_finished());
    }

    #[test]
    fn test_context_bar() {
        let bar = ProgressBar::context_bar(200000);
        assert_eq!(bar.length(), Some(200000));

        bar.set_position(50000);
        assert_eq!(bar.percent(), 25);
    }

    #[test]
    fn test_progress_bar_zero_length() {
        let bar = ProgressBar::new(0);
        assert_eq!(bar.percent(), 0);
    }
}

//! Long wait detection for operations that take more than a threshold time
//!
//! This module provides a simple timer to detect when operations are taking
//! longer than expected, which can be used to trigger additional UI feedback
//! like fun facts or progress indicators.

use std::time::{Duration, Instant};

/// Default threshold for considering a wait "long" (10 seconds)
pub const DEFAULT_LONG_WAIT_THRESHOLD: Duration = Duration::from_secs(10);

/// A timer that detects when operations take longer than a threshold
#[derive(Debug, Clone)]
pub struct LongWaitDetector {
    /// The start time of the operation
    start_time: Option<Instant>,
    /// The threshold duration for considering a wait "long"
    threshold: Duration,
    /// Whether we've already triggered the long wait callback
    triggered: bool,
}

impl LongWaitDetector {
    /// Create a new long wait detector with the default threshold (10 seconds)
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_LONG_WAIT_THRESHOLD)
    }

    /// Create a new long wait detector with a custom threshold
    pub fn with_threshold(threshold: Duration) -> Self {
        Self {
            start_time: None,
            threshold,
            triggered: false,
        }
    }

    /// Start the timer
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.triggered = false;
    }

    /// Stop the timer and reset
    pub fn stop(&mut self) {
        self.start_time = None;
        self.triggered = false;
    }

    /// Check if the operation has been running longer than the threshold
    ///
    /// Returns true the first time the threshold is exceeded, then false
    /// on subsequent calls until reset. This prevents triggering multiple
    /// times for the same operation.
    pub fn check(&mut self) -> bool {
        if self.triggered {
            return false;
        }

        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            if elapsed >= self.threshold {
                self.triggered = true;
                return true;
            }
        }

        false
    }

    /// Get the elapsed time since start
    ///
    /// Returns None if the timer hasn't been started
    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Check if the timer is currently running
    pub fn is_running(&self) -> bool {
        self.start_time.is_some()
    }

    /// Check if the long wait threshold has been exceeded
    pub fn has_exceeded_threshold(&self) -> bool {
        self.elapsed()
            .map(|e| e >= self.threshold)
            .unwrap_or(false)
    }

    /// Get the threshold duration
    pub fn threshold(&self) -> Duration {
        self.threshold
    }

    /// Set a new threshold duration
    pub fn set_threshold(&mut self, threshold: Duration) {
        self.threshold = threshold;
    }

    /// Reset the triggered flag without stopping the timer
    ///
    /// This allows the long wait detection to trigger again for the same
    /// operation, which can be useful for periodic updates.
    pub fn reset_trigger(&mut self) {
        self.triggered = false;
    }
}

impl Default for LongWaitDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_detector() {
        let detector = LongWaitDetector::new();
        assert!(!detector.is_running());
        assert_eq!(detector.threshold(), DEFAULT_LONG_WAIT_THRESHOLD);
        assert!(!detector.has_exceeded_threshold());
        assert_eq!(detector.elapsed(), None);
    }

    #[test]
    fn test_with_threshold() {
        let custom_threshold = Duration::from_secs(5);
        let detector = LongWaitDetector::with_threshold(custom_threshold);
        assert_eq!(detector.threshold(), custom_threshold);
    }

    #[test]
    fn test_start_and_stop() {
        let mut detector = LongWaitDetector::new();

        assert!(!detector.is_running());

        detector.start();
        assert!(detector.is_running());
        assert!(detector.elapsed().is_some());

        detector.stop();
        assert!(!detector.is_running());
        assert_eq!(detector.elapsed(), None);
    }

    #[test]
    fn test_elapsed_time() {
        let mut detector = LongWaitDetector::new();

        detector.start();
        thread::sleep(Duration::from_millis(100));

        let elapsed = detector.elapsed().expect("Should have elapsed time");
        assert!(
            elapsed >= Duration::from_millis(100),
            "Elapsed time should be at least 100ms"
        );
        assert!(
            elapsed < Duration::from_millis(200),
            "Elapsed time should be less than 200ms"
        );
    }

    #[test]
    fn test_check_before_threshold() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_secs(1));

        detector.start();
        thread::sleep(Duration::from_millis(100));

        assert!(!detector.check(), "Should not trigger before threshold");
        assert!(!detector.has_exceeded_threshold());
    }

    #[test]
    fn test_check_after_threshold() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_millis(50));

        detector.start();
        thread::sleep(Duration::from_millis(100));

        assert!(detector.check(), "Should trigger after threshold");
        assert!(detector.has_exceeded_threshold());
    }

    #[test]
    fn test_check_triggers_only_once() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_millis(50));

        detector.start();
        thread::sleep(Duration::from_millis(100));

        assert!(detector.check(), "First check should trigger");
        assert!(!detector.check(), "Second check should not trigger");
        assert!(!detector.check(), "Third check should not trigger");
    }

    #[test]
    fn test_reset_trigger() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_millis(50));

        detector.start();
        thread::sleep(Duration::from_millis(100));

        assert!(detector.check(), "First check should trigger");
        assert!(!detector.check(), "Second check should not trigger");

        detector.reset_trigger();
        assert!(detector.check(), "Check should trigger again after reset");
    }

    #[test]
    fn test_stop_resets_triggered_flag() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_millis(50));

        detector.start();
        thread::sleep(Duration::from_millis(100));

        assert!(detector.check(), "Should trigger");

        detector.stop();
        detector.start();
        thread::sleep(Duration::from_millis(100));

        assert!(
            detector.check(),
            "Should trigger again after stop and restart"
        );
    }

    #[test]
    fn test_set_threshold() {
        let mut detector = LongWaitDetector::new();
        assert_eq!(detector.threshold(), DEFAULT_LONG_WAIT_THRESHOLD);

        let new_threshold = Duration::from_secs(5);
        detector.set_threshold(new_threshold);
        assert_eq!(detector.threshold(), new_threshold);
    }

    #[test]
    fn test_check_without_start() {
        let mut detector = LongWaitDetector::new();
        assert!(!detector.check(), "Should not trigger without starting");
    }

    #[test]
    fn test_has_exceeded_threshold_false_initially() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_millis(50));
        detector.start();

        assert!(
            !detector.has_exceeded_threshold(),
            "Should not have exceeded threshold immediately"
        );
    }

    #[test]
    fn test_has_exceeded_threshold_true_after_wait() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_millis(50));
        detector.start();
        thread::sleep(Duration::from_millis(100));

        assert!(
            detector.has_exceeded_threshold(),
            "Should have exceeded threshold"
        );
    }

    #[test]
    fn test_default() {
        let detector = LongWaitDetector::default();
        assert_eq!(detector.threshold(), DEFAULT_LONG_WAIT_THRESHOLD);
        assert!(!detector.is_running());
    }

    #[test]
    fn test_multiple_start_stops() {
        let mut detector = LongWaitDetector::with_threshold(Duration::from_millis(50));

        // First operation
        detector.start();
        thread::sleep(Duration::from_millis(100));
        assert!(detector.check());
        detector.stop();

        // Second operation
        detector.start();
        thread::sleep(Duration::from_millis(100));
        assert!(
            detector.check(),
            "Should trigger again for new operation"
        );
        detector.stop();

        // Third operation - short one
        detector.start();
        thread::sleep(Duration::from_millis(10));
        assert!(
            !detector.check(),
            "Should not trigger for short operation"
        );
        detector.stop();
    }

    #[test]
    fn test_elapsed_tracks_full_duration() {
        let mut detector = LongWaitDetector::new();

        detector.start();
        thread::sleep(Duration::from_millis(100));

        let elapsed1 = detector.elapsed().expect("Should have elapsed time");

        thread::sleep(Duration::from_millis(100));

        let elapsed2 = detector.elapsed().expect("Should have elapsed time");

        assert!(
            elapsed2 > elapsed1,
            "Elapsed time should increase with continued running"
        );
        assert!(
            elapsed2 >= Duration::from_millis(200),
            "Total elapsed should be at least 200ms"
        );
    }

    #[test]
    fn test_threshold_boundary() {
        let threshold = Duration::from_millis(100);
        let mut detector = LongWaitDetector::with_threshold(threshold);

        detector.start();

        // Just before threshold - use a safer margin
        thread::sleep(Duration::from_millis(50));
        assert!(!detector.check(), "Should not trigger well before threshold");

        // Wait to definitely exceed threshold
        thread::sleep(Duration::from_millis(60));
        assert!(detector.check(), "Should trigger after threshold");
    }
}

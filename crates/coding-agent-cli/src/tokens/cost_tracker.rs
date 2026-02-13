//! Cost tracking for session token usage.
//!
//! Tracks input and output tokens separately to provide accurate
//! cost calculation and detailed breakdowns.

use super::pricing::ModelPricing;
use std::time::Instant;

/// Tracks token usage and costs for a session.
#[derive(Debug, Clone)]
pub struct CostTracker {
    /// Number of input tokens used.
    input_tokens: u64,
    /// Number of output tokens used.
    output_tokens: u64,
    /// Number of messages in the session.
    message_count: u32,
    /// Session start time.
    start_time: Option<Instant>,
    /// Current model pricing.
    pricing: ModelPricing,
}

impl CostTracker {
    /// Create a new cost tracker with the given model pricing.
    pub fn new(pricing: ModelPricing) -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            message_count: 0,
            start_time: Some(Instant::now()),
            pricing,
        }
    }

    /// Create a cost tracker with the default model (Claude 3 Opus).
    pub fn with_default_model() -> Self {
        Self::new(ModelPricing::default_pricing())
    }

    /// Add input tokens (user messages, context).
    pub fn add_input_tokens(&mut self, tokens: u64) {
        self.input_tokens = self.input_tokens.saturating_add(tokens);
    }

    /// Add output tokens (assistant responses).
    pub fn add_output_tokens(&mut self, tokens: u64) {
        self.output_tokens = self.output_tokens.saturating_add(tokens);
    }

    /// Increment the message count.
    pub fn add_message(&mut self) {
        self.message_count = self.message_count.saturating_add(1);
    }

    /// Get the number of input tokens.
    pub fn input_tokens(&self) -> u64 {
        self.input_tokens
    }

    /// Get the number of output tokens.
    pub fn output_tokens(&self) -> u64 {
        self.output_tokens
    }

    /// Get the total number of tokens.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }

    /// Get the number of messages.
    pub fn message_count(&self) -> u32 {
        self.message_count
    }

    /// Get the model name.
    pub fn model_name(&self) -> &'static str {
        self.pricing.name
    }

    /// Get the context window size.
    pub fn context_window(&self) -> usize {
        self.pricing.context_window
    }

    /// Calculate cost for input tokens.
    pub fn input_cost(&self) -> f64 {
        self.pricing.calculate_input_cost(self.input_tokens as usize)
    }

    /// Calculate cost for output tokens.
    pub fn output_cost(&self) -> f64 {
        self.pricing.calculate_output_cost(self.output_tokens as usize)
    }

    /// Calculate total cost.
    pub fn total_cost(&self) -> f64 {
        self.input_cost() + self.output_cost()
    }

    /// Get context usage as a percentage.
    pub fn context_percent(&self) -> f64 {
        self.pricing.context_usage_percent(self.total_tokens() as usize)
    }

    /// Get the session duration as a human-readable string.
    pub fn duration_string(&self) -> String {
        match self.start_time {
            Some(start) => {
                let elapsed = start.elapsed();
                let total_secs = elapsed.as_secs();
                let hours = total_secs / 3600;
                let minutes = (total_secs % 3600) / 60;

                if hours > 0 {
                    format!("{}h {}m", hours, minutes)
                } else if minutes > 0 {
                    format!("{}m", minutes)
                } else {
                    "< 1m".to_string()
                }
            }
            None => "unknown".to_string(),
        }
    }

    /// Update the model pricing.
    pub fn set_pricing(&mut self, pricing: ModelPricing) {
        self.pricing = pricing;
    }

    /// Reset all counters.
    pub fn reset(&mut self) {
        self.input_tokens = 0;
        self.output_tokens = 0;
        self.message_count = 0;
        self.start_time = Some(Instant::now());
    }

    /// Format token count with commas (e.g., "45,230").
    pub fn format_tokens(tokens: u64) -> String {
        let s = tokens.to_string();
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(*c);
        }
        result
    }

    /// Format cost as currency (e.g., "$0.675").
    pub fn format_cost(cost: f64) -> String {
        if cost < 0.01 {
            format!("${:.4}", cost)
        } else if cost < 1.0 {
            format!("${:.3}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }

    /// Generate a detailed cost breakdown string.
    pub fn render_breakdown(&self) -> String {
        let separator = "──────────────────────────────────────────────";

        let mut output = String::new();
        output.push_str("Session Cost Breakdown\n");
        output.push_str(separator);
        output.push_str("\n\n");

        output.push_str(&format!("Model: {}\n\n", self.model_name()));

        output.push_str(&format!(
            "Input tokens:   {:>10}   ({})\n",
            Self::format_tokens(self.input_tokens),
            Self::format_cost(self.input_cost())
        ));
        output.push_str(&format!(
            "Output tokens:  {:>10}   ({})\n",
            Self::format_tokens(self.output_tokens),
            Self::format_cost(self.output_cost())
        ));
        output.push_str(separator);
        output.push_str("\n");
        output.push_str(&format!(
            "Total:          {:>10}   ({})\n\n",
            Self::format_tokens(self.total_tokens()),
            Self::format_cost(self.total_cost())
        ));

        output.push_str(&format!(
            "Context used:   {} / {} ({:.0}%)\n\n",
            Self::format_tokens(self.total_tokens()),
            Self::format_tokens(self.context_window() as u64),
            self.context_percent()
        ));

        output.push_str(&format!(
            "This session: {} messages over {}",
            self.message_count(),
            self.duration_string()
        ));

        output
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::with_default_model()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_tracker_new() {
        let tracker = CostTracker::with_default_model();
        assert_eq!(tracker.input_tokens(), 0);
        assert_eq!(tracker.output_tokens(), 0);
        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.message_count(), 0);
    }

    #[test]
    fn test_cost_tracker_add_tokens() {
        let mut tracker = CostTracker::with_default_model();
        tracker.add_input_tokens(1000);
        tracker.add_output_tokens(500);

        assert_eq!(tracker.input_tokens(), 1000);
        assert_eq!(tracker.output_tokens(), 500);
        assert_eq!(tracker.total_tokens(), 1500);
    }

    #[test]
    fn test_cost_tracker_add_message() {
        let mut tracker = CostTracker::with_default_model();
        tracker.add_message();
        tracker.add_message();

        assert_eq!(tracker.message_count(), 2);
    }

    #[test]
    fn test_cost_calculation_opus() {
        let tracker = CostTracker::new(ModelPricing::CLAUDE_3_OPUS);
        let mut tracker = tracker;
        tracker.add_input_tokens(1_000_000); // 1M tokens
        tracker.add_output_tokens(1_000_000); // 1M tokens

        // Opus: $15/M input, $75/M output
        assert!((tracker.input_cost() - 15.0).abs() < 0.001);
        assert!((tracker.output_cost() - 75.0).abs() < 0.001);
        assert!((tracker.total_cost() - 90.0).abs() < 0.001);
    }

    #[test]
    fn test_cost_calculation_sonnet() {
        let mut tracker = CostTracker::new(ModelPricing::CLAUDE_3_SONNET);
        tracker.add_input_tokens(1_000_000); // 1M tokens
        tracker.add_output_tokens(1_000_000); // 1M tokens

        // Sonnet: $3/M input, $15/M output
        assert!((tracker.input_cost() - 3.0).abs() < 0.001);
        assert!((tracker.output_cost() - 15.0).abs() < 0.001);
        assert!((tracker.total_cost() - 18.0).abs() < 0.001);
    }

    #[test]
    fn test_context_percent() {
        let mut tracker = CostTracker::new(ModelPricing::CLAUDE_3_OPUS);
        tracker.add_input_tokens(100_000);
        tracker.add_output_tokens(100_000);

        // 200k out of 200k = 100%
        assert!((tracker.context_percent() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(CostTracker::format_tokens(0), "0");
        assert_eq!(CostTracker::format_tokens(999), "999");
        assert_eq!(CostTracker::format_tokens(1000), "1,000");
        assert_eq!(CostTracker::format_tokens(45230), "45,230");
        assert_eq!(CostTracker::format_tokens(1000000), "1,000,000");
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(CostTracker::format_cost(0.0001), "$0.0001");
        assert_eq!(CostTracker::format_cost(0.005), "$0.0050");
        assert_eq!(CostTracker::format_cost(0.675), "$0.675");
        assert_eq!(CostTracker::format_cost(1.61), "$1.61");
        assert_eq!(CostTracker::format_cost(15.0), "$15.00");
    }

    #[test]
    fn test_reset() {
        let mut tracker = CostTracker::with_default_model();
        tracker.add_input_tokens(1000);
        tracker.add_output_tokens(500);
        tracker.add_message();

        tracker.reset();

        assert_eq!(tracker.input_tokens(), 0);
        assert_eq!(tracker.output_tokens(), 0);
        assert_eq!(tracker.message_count(), 0);
    }

    #[test]
    fn test_render_breakdown_contains_expected_parts() {
        let mut tracker = CostTracker::new(ModelPricing::CLAUDE_3_OPUS);
        tracker.add_input_tokens(45230);
        tracker.add_output_tokens(12450);
        tracker.add_message();
        tracker.add_message();

        let breakdown = tracker.render_breakdown();

        assert!(breakdown.contains("Session Cost Breakdown"));
        assert!(breakdown.contains("Model: claude-3-opus"));
        assert!(breakdown.contains("Input tokens:"));
        assert!(breakdown.contains("Output tokens:"));
        assert!(breakdown.contains("Total:"));
        assert!(breakdown.contains("Context used:"));
        assert!(breakdown.contains("2 messages"));
    }

    #[test]
    fn test_model_name() {
        let tracker = CostTracker::new(ModelPricing::CLAUDE_3_SONNET);
        assert_eq!(tracker.model_name(), "claude-3-sonnet");
    }

    #[test]
    fn test_set_pricing() {
        let mut tracker = CostTracker::new(ModelPricing::CLAUDE_3_OPUS);
        assert_eq!(tracker.model_name(), "claude-3-opus");

        tracker.set_pricing(ModelPricing::CLAUDE_3_HAIKU);
        assert_eq!(tracker.model_name(), "claude-3-haiku");
    }

    #[test]
    fn test_saturating_add() {
        let mut tracker = CostTracker::with_default_model();
        tracker.add_input_tokens(u64::MAX - 10);
        tracker.add_input_tokens(100);

        // Should saturate at u64::MAX, not overflow
        assert_eq!(tracker.input_tokens(), u64::MAX);
    }
}

//! Model pricing and cost calculation.
//!
//! Provides pricing information for various AI models and cost calculation
//! based on token usage.

use thiserror::Error;

/// Errors that can occur during pricing operations.
#[derive(Debug, Error)]
pub enum PricingError {
    /// Unknown model name.
    #[error("unknown model: {0}")]
    UnknownModel(String),
}

/// Pricing information for a model.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    /// Model name.
    pub name: &'static str,
    /// Cost per 1M input tokens in USD.
    pub input_cost_per_million: f64,
    /// Cost per 1M output tokens in USD.
    pub output_cost_per_million: f64,
    /// Maximum context window size in tokens.
    pub context_window: usize,
}

impl ModelPricing {
    /// Claude 3 Opus pricing.
    pub const CLAUDE_3_OPUS: Self = Self {
        name: "claude-3-opus",
        input_cost_per_million: 15.0,
        output_cost_per_million: 75.0,
        context_window: 200_000,
    };

    /// Claude 3 Sonnet pricing.
    pub const CLAUDE_3_SONNET: Self = Self {
        name: "claude-3-sonnet",
        input_cost_per_million: 3.0,
        output_cost_per_million: 15.0,
        context_window: 200_000,
    };

    /// Claude 3 Haiku pricing.
    pub const CLAUDE_3_HAIKU: Self = Self {
        name: "claude-3-haiku",
        input_cost_per_million: 0.25,
        output_cost_per_million: 1.25,
        context_window: 200_000,
    };

    /// Claude 3.5 Sonnet pricing.
    pub const CLAUDE_3_5_SONNET: Self = Self {
        name: "claude-3-5-sonnet",
        input_cost_per_million: 3.0,
        output_cost_per_million: 15.0,
        context_window: 200_000,
    };

    /// Get pricing for a model by name.
    pub fn from_name(name: &str) -> Result<Self, PricingError> {
        let normalized = name.to_lowercase();
        match normalized.as_str() {
            "claude-3-opus" | "claude-3-opus-20240229" => Ok(Self::CLAUDE_3_OPUS),
            "claude-3-sonnet" | "claude-3-sonnet-20240229" => Ok(Self::CLAUDE_3_SONNET),
            "claude-3-haiku" | "claude-3-haiku-20240307" => Ok(Self::CLAUDE_3_HAIKU),
            "claude-3-5-sonnet" | "claude-3-5-sonnet-20240620" | "claude-3.5-sonnet" => {
                Ok(Self::CLAUDE_3_5_SONNET)
            }
            _ => Err(PricingError::UnknownModel(name.to_string())),
        }
    }

    /// Get the default pricing (used when model is unknown).
    /// Uses the highest pricing to be conservative.
    pub fn default_pricing() -> Self {
        Self::CLAUDE_3_OPUS
    }

    /// Calculate cost for input tokens.
    pub fn calculate_input_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.input_cost_per_million
    }

    /// Calculate cost for output tokens.
    pub fn calculate_output_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1_000_000.0) * self.output_cost_per_million
    }

    /// Calculate total cost for input and output tokens.
    pub fn calculate_total_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        self.calculate_input_cost(input_tokens) + self.calculate_output_cost(output_tokens)
    }

    /// Get context usage as a percentage.
    pub fn context_usage_percent(&self, tokens: usize) -> f64 {
        (tokens as f64 / self.context_window as f64) * 100.0
    }
}

/// List of all available models.
pub fn available_models() -> Vec<&'static str> {
    vec![
        "claude-3-opus",
        "claude-3-sonnet",
        "claude-3-haiku",
        "claude-3-5-sonnet",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_name_opus() {
        let pricing = ModelPricing::from_name("claude-3-opus").unwrap();
        assert_eq!(pricing.name, "claude-3-opus");
        assert_eq!(pricing.context_window, 200_000);
    }

    #[test]
    fn test_from_name_case_insensitive() {
        let pricing = ModelPricing::from_name("Claude-3-Opus").unwrap();
        assert_eq!(pricing.name, "claude-3-opus");
    }

    #[test]
    fn test_from_name_unknown() {
        let result = ModelPricing::from_name("unknown-model");
        assert!(result.is_err());
        match result {
            Err(PricingError::UnknownModel(name)) => assert_eq!(name, "unknown-model"),
            _ => panic!("expected UnknownModel error"),
        }
    }

    #[test]
    fn test_calculate_input_cost() {
        let pricing = ModelPricing::CLAUDE_3_OPUS;
        // 1M tokens at $15/M = $15
        let cost = pricing.calculate_input_cost(1_000_000);
        assert!((cost - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_output_cost() {
        let pricing = ModelPricing::CLAUDE_3_OPUS;
        // 1M tokens at $75/M = $75
        let cost = pricing.calculate_output_cost(1_000_000);
        assert!((cost - 75.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_total_cost() {
        let pricing = ModelPricing::CLAUDE_3_OPUS;
        // 100k input + 50k output
        let cost = pricing.calculate_total_cost(100_000, 50_000);
        let expected = (100_000.0 / 1_000_000.0) * 15.0 + (50_000.0 / 1_000_000.0) * 75.0;
        assert!((cost - expected).abs() < 0.001);
    }

    #[test]
    fn test_context_usage_percent() {
        let pricing = ModelPricing::CLAUDE_3_OPUS;
        // 100k out of 200k = 50%
        let percent = pricing.context_usage_percent(100_000);
        assert!((percent - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_cost_calculation_sonnet() {
        let pricing = ModelPricing::CLAUDE_3_SONNET;
        // Sonnet is cheaper
        let cost = pricing.calculate_input_cost(1_000_000);
        assert!((cost - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_cost_calculation_haiku() {
        let pricing = ModelPricing::CLAUDE_3_HAIKU;
        // Haiku is cheapest
        let cost = pricing.calculate_input_cost(1_000_000);
        assert!((cost - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_available_models() {
        let models = available_models();
        assert!(models.contains(&"claude-3-opus"));
        assert!(models.contains(&"claude-3-sonnet"));
        assert!(models.contains(&"claude-3-haiku"));
        assert!(models.contains(&"claude-3-5-sonnet"));
    }

    #[test]
    fn test_default_pricing_uses_highest() {
        let default = ModelPricing::default_pricing();
        // Should use Opus (highest pricing) to be conservative
        assert_eq!(default.name, "claude-3-opus");
    }
}

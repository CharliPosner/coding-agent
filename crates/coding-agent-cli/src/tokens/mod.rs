//! Token counting and context window management.
//!
//! This module provides token counting using tiktoken-rs to track
//! context window usage and calculate costs.

mod cost_tracker;
mod counter;
mod pricing;

pub use cost_tracker::CostTracker;
pub use counter::{TokenCount, TokenCounter, TokenCounterError};
pub use pricing::ModelPricing;

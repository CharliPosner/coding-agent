//! Agent management for self-healing and autonomous tasks.
//!
//! This module provides infrastructure for spawning and managing autonomous agents
//! that can handle complex tasks like error recovery and code fixes.

mod fix_agent;
pub mod manager;
pub mod status;

pub use fix_agent::{
    categorize_deviation, should_auto_fix, DeviationCategory, DeviationRule, FixAgent,
    FixAgentConfig, FixAttempt, FixResult, FixStatus,
};
pub use manager::AgentManager;
pub use status::AgentId;

//! Agent management for self-healing and autonomous tasks.
//!
//! This module provides infrastructure for spawning and managing autonomous agents
//! that can handle complex tasks like error recovery and code fixes.

mod fix_agent;

pub use fix_agent::{FixAgent, FixAgentConfig, FixAttempt, FixResult, FixStatus};

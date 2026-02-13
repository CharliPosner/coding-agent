//! Tool execution framework with error categorization and recovery support.
//!
//! This module provides the infrastructure for executing tools with:
//! - Error categorization (code, permission, network, resource errors)
//! - Execution tracking with spinners and status updates
//! - Retry logic with exponential backoff for transient errors
//! - Hooks for self-healing recovery (fix-agent spawning)

mod executor;

pub use executor::{
    ErrorCategory, ToolError, ToolExecutionResult, ToolExecutor, ToolExecutorConfig,
};

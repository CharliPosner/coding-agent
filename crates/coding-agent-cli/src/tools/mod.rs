//! Tool execution framework with error categorization and recovery support.
//!
//! This module provides the infrastructure for executing tools with:
//! - Error categorization (code, permission, network, resource errors)
//! - Execution tracking with spinners and status updates
//! - Retry logic with exponential backoff for transient errors
//! - Hooks for self-healing recovery (fix-agent spawning)
//! - Diagnostic analysis for parsing compiler errors
//! - Auto-fix application for code errors
//! - Regression test generation to prevent fix reversions

mod auto_fix;
mod definitions;
mod diagnostics;
mod executor;
mod regression_tests;

pub use auto_fix::FixApplicationResult;
pub use definitions::{create_tool_definitions, execute_tool, tool_definitions_to_api};
pub use diagnostics::{extract_fix_info, parse_compiler_output, Diagnostic, FixInfo, FixType};
pub use executor::{ErrorCategory, ToolError, ToolExecutionResult, ToolExecutor, ToolExecutorConfig};
pub use regression_tests::{generate_regression_test, RegressionTest, RegressionTestConfig};

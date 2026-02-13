//! Tool execution framework with error categorization and recovery support.
//!
//! This module provides the infrastructure for executing tools with:
//! - Error categorization (code, permission, network, resource errors)
//! - Execution tracking with spinners and status updates
//! - Retry logic with exponential backoff for transient errors
//! - Hooks for self-healing recovery (fix-agent spawning)
//! - Diagnostic analysis for parsing compiler errors
//! - Auto-fix application for code errors

mod auto_fix;
mod diagnostics;
mod executor;

pub use auto_fix::{apply_fix, AutoFixConfig, FixApplicationResult};
pub use diagnostics::{
    extract_fix_info, parse_compiler_output, CompilerType, Diagnostic, DiagnosticLocation,
    DiagnosticReport, DiagnosticSeverity, DiagnosticSuggestion, FixInfo, FixType,
};
pub use executor::{
    ErrorCategory, ToolError, ToolExecutionResult, ToolExecutor, ToolExecutorConfig,
};

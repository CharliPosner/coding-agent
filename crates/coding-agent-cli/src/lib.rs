//! Coding Agent CLI library
//!
//! This library provides the core functionality for the coding-agent CLI,
//! including the REPL, commands, UI components, and integrations.

pub mod agents;
pub mod cli;
pub mod config;
pub mod integrations;
pub mod permissions;
pub mod tokens;
pub mod tools;
pub mod ui;

pub use agents::{FixAgent, FixAgentConfig, FixAttempt, FixResult, FixStatus};
pub use cli::{InputHandler, Repl, ReplConfig, Terminal};
pub use config::{Config, PersistenceConfig};
pub use integrations::{Session, SessionManager, SpecStoryError};
pub use permissions::{
    OperationType, PermissionChecker, PermissionDecision, SessionPermissions, TrustedPaths,
    TrustedPathsError,
};
pub use tokens::{ModelPricing, TokenCount, TokenCounter, TokenCounterError};
pub use tools::{
    create_tool_definitions, execute_tool, tool_definitions_to_api, ErrorCategory, ToolError,
    ToolExecutionResult, ToolExecutor, ToolExecutorConfig,
};
pub use ui::ContextBar;

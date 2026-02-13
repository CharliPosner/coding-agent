//! Coding Agent CLI library
//!
//! This library provides the core functionality for the coding-agent CLI,
//! including the REPL, commands, UI components, and integrations.

pub mod cli;
pub mod config;
pub mod integrations;
pub mod permissions;
pub mod tokens;
pub mod tools;
pub mod ui;

pub use cli::{InputHandler, Repl, ReplConfig, Terminal};
pub use config::{Config, PersistenceConfig};
pub use integrations::{Session, SessionManager, SpecStoryError};
pub use permissions::{
    OperationType, PermissionChecker, PermissionDecision, SessionPermissions, TrustedPaths,
    TrustedPathsError,
};
pub use tokens::{ModelPricing, TokenCount, TokenCounter, TokenCounterError};
pub use tools::{ErrorCategory, ToolError, ToolExecutionResult, ToolExecutor, ToolExecutorConfig};
pub use ui::ContextBar;

//! Coding Agent CLI library
//!
//! This library provides the core functionality for the coding-agent CLI,
//! including the REPL, commands, UI components, and integrations.

pub mod cli;
pub mod config;
pub mod integrations;
pub mod tokens;
pub mod ui;

pub use cli::{InputHandler, Repl, ReplConfig, Terminal};
pub use config::{Config, PersistenceConfig};
pub use integrations::{Session, SessionManager, SpecStoryError};
pub use tokens::{ModelPricing, TokenCount, TokenCounter, TokenCounterError};
pub use ui::ContextBar;

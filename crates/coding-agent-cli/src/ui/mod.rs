//! UI components for the coding-agent CLI
//!
//! This module provides styled output, spinners, progress bars, and themes.

pub mod components;
pub mod context_bar;
pub mod output;
pub mod progress;
pub mod spinner;
pub mod syntax;
pub mod theme;
pub mod tool_spinner;

pub use components::MessageBox;
pub use context_bar::ContextBar;
pub use output::StyledOutput;
pub use progress::ProgressBar;
pub use spinner::Spinner;
pub use syntax::SyntaxHighlighter;
pub use theme::{Color, Theme};
pub use tool_spinner::{ToolExecutionSpinner, ToolStatus};

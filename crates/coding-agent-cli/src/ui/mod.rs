//! UI components for the coding-agent CLI
//!
//! This module provides styled output, spinners, progress bars, and themes.

pub mod commit_preview;
pub mod components;
pub mod context_bar;
pub mod file_picker;
pub mod output;
pub mod progress;
pub mod spinner;
pub mod syntax;
pub mod theme;
pub mod tool_result;
pub mod tool_spinner;

pub use commit_preview::{edit_commit_message, CommitPreview, CommitPreviewResult};
pub use components::MessageBox;
pub use context_bar::ContextBar;
pub use file_picker::{FileEntry, FilePicker, FilePickerResult};
pub use output::StyledOutput;
pub use progress::ProgressBar;
pub use spinner::Spinner;
pub use syntax::SyntaxHighlighter;
pub use theme::{Color, Theme};
pub use tool_result::{ToolResultConfig, ToolResultFormatter};
pub use tool_spinner::{ToolExecutionSpinner, ToolStatus};

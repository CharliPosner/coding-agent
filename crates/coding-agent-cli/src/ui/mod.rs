//! UI components for the coding-agent CLI
//!
//! This module provides styled output, spinners, progress bars, and themes.

pub mod commit_preview;
pub mod components;
pub mod context_bar;
pub mod file_picker;
pub mod fun_facts;
pub mod long_wait;
pub mod markdown;
pub mod output;
pub mod progress;
pub mod spinner;
pub mod status_bar;
pub mod syntax;
pub mod theme;
pub mod thinking;
pub mod tool_result;
pub mod tool_spinner;

pub use commit_preview::{edit_commit_message, CommitPreview, CommitPreviewResult};
pub use context_bar::ContextBar;
pub use file_picker::{FileEntry, FilePicker, FilePickerResult};
pub use fun_facts::{FunFact, FunFactCache, FunFactClient};
pub use long_wait::LongWaitDetector;
pub use markdown::MarkdownRenderer;
pub use status_bar::StatusBar;
pub use theme::{Color, Theme};
pub use thinking::ThinkingMessages;
pub use tool_result::{FormattedResult, ToolResultFormatter};
pub use tool_spinner::ToolExecutionSpinner;

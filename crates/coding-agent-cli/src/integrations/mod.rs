//! Integrations with external systems
//!
//! This module provides integrations with various external systems like
//! SpecStory for conversation persistence, Git for version control, etc.

pub mod git;
pub mod obsidian;
pub mod specstory;

pub use git::{FileGrouper, GitRepo};
pub use obsidian::{NoteType, ObsidianError, ObsidianVault};
pub use specstory::{Session, SessionInfo, SessionManager, SpecStoryError};

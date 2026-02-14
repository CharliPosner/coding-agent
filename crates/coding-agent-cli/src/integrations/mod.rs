//! Integrations with external systems
//!
//! This module provides integrations with various external systems like
//! SpecStory for conversation persistence, Git for version control, etc.

pub mod git;
pub mod obsidian;
pub mod specstory;

pub use git::{
    suggest_commit_splits, FileGroup, FileGrouper, FileStatus, FileStatusKind, GitError, GitRepo,
    GroupReason, RepoStatus,
};
pub use obsidian::{Note, NoteMetadata, NoteType, ObsidianError, ObsidianVault, SearchResult};
pub use specstory::{MessageRole, Session, SessionInfo, SessionManager, SpecStoryError};

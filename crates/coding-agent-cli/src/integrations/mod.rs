//! Integrations with external systems
//!
//! This module provides integrations with various external systems like
//! SpecStory for conversation persistence, Git for version control, etc.

pub mod git;
pub mod specstory;

pub use git::{FileGroup, FileGrouper, FileStatus, FileStatusKind, GitError, GitRepo, GroupReason, RepoStatus, suggest_commit_splits};
pub use specstory::{MessageRole, Session, SessionInfo, SessionManager, SpecStoryError};

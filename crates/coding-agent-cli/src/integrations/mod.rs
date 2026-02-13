//! Integrations with external systems
//!
//! This module provides integrations with various external systems like
//! SpecStory for conversation persistence, Git for version control, etc.

pub mod specstory;

pub use specstory::{MessageRole, Session, SessionInfo, SessionManager, SpecStoryError};

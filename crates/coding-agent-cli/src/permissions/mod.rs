//! Permission system for the coding-agent CLI
//!
//! This module handles permission checking for file operations,
//! including trusted path management and path matching.

mod trusted;

pub use trusted::{TrustedPaths, TrustedPathsError};

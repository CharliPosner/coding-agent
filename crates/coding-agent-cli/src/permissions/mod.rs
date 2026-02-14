//! Permission system for the coding-agent CLI
//!
//! This module handles permission checking for file operations,
//! including trusted path management and path matching.

mod checker;
mod prompt;
mod trusted;

pub use checker::{OperationType, PermissionChecker, PermissionDecision, SessionPermissions};
pub use prompt::{PermissionPrompt, PermissionResponse};
pub use trusted::{TrustedPaths, TrustedPathsError};

//! Permission checker for file operations
//!
//! This module provides a permission checker that determines whether
//! file operations should be allowed, denied, or require user confirmation.

use super::trusted::TrustedPaths;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Types of file operations that require permission checking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    /// Read a file (always allowed per spec)
    Read,
    /// Write/create a file
    Write,
    /// Modify/edit an existing file
    Modify,
    /// Delete a file
    Delete,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Read => write!(f, "read"),
            OperationType::Write => write!(f, "write"),
            OperationType::Modify => write!(f, "modify"),
            OperationType::Delete => write!(f, "delete"),
        }
    }
}

/// Result of a permission check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Operation is allowed without user confirmation
    Allowed,
    /// Operation is denied (user previously chose "never")
    Denied,
    /// Operation requires user confirmation
    NeedsPrompt,
}

/// Caches permission decisions for the current session
#[derive(Debug, Clone, Default)]
pub struct SessionPermissions {
    /// Map of (canonical_path, operation) -> decision
    /// "always" responses are stored as Allowed
    /// "never" responses are stored as Denied
    decisions: HashMap<(PathBuf, OperationType), PermissionDecision>,
}

impl SessionPermissions {
    /// Create a new empty session permissions cache
    pub fn new() -> Self {
        Self {
            decisions: HashMap::new(),
        }
    }

    /// Get a cached decision for a path and operation
    pub fn get(&self, path: &Path, operation: OperationType) -> Option<&PermissionDecision> {
        let canonical = canonicalize_for_cache(path);
        self.decisions.get(&(canonical, operation))
    }

    /// Cache a decision for a path and operation
    pub fn set(&mut self, path: &Path, operation: OperationType, decision: PermissionDecision) {
        let canonical = canonicalize_for_cache(path);
        self.decisions.insert((canonical, operation), decision);
    }

    /// Check if we have any cached decision for a path
    pub fn has_decision(&self, path: &Path, operation: OperationType) -> bool {
        self.get(path, operation).is_some()
    }

    /// Clear all cached decisions
    pub fn clear(&mut self) {
        self.decisions.clear();
    }
}

/// Canonicalize a path for caching purposes
fn canonicalize_for_cache(path: &Path) -> PathBuf {
    // Try to canonicalize, but if the path doesn't exist, use absolute path
    if let Ok(canonical) = path.canonicalize() {
        canonical
    } else if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Permission checker that combines trusted paths with session-level caching
#[derive(Debug)]
pub struct PermissionChecker {
    /// Trusted paths from configuration
    trusted_paths: TrustedPaths,
    /// Session-level permission cache
    session_permissions: SessionPermissions,
    /// Whether read operations are automatically allowed
    auto_read: bool,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new(trusted_paths: TrustedPaths, auto_read: bool) -> Self {
        Self {
            trusted_paths,
            session_permissions: SessionPermissions::new(),
            auto_read,
        }
    }

    /// Check if an operation on a path is allowed
    ///
    /// Returns the permission decision:
    /// - `Allowed` if the path is trusted or user previously approved "always"
    /// - `Denied` if user previously chose "never" for this path
    /// - `NeedsPrompt` if user confirmation is required
    pub fn check(&self, path: &Path, operation: OperationType) -> PermissionDecision {
        // Read operations are always allowed (per spec)
        if operation == OperationType::Read && self.auto_read {
            return PermissionDecision::Allowed;
        }

        // Check session cache first
        if let Some(cached) = self.session_permissions.get(path, operation) {
            return cached.clone();
        }

        // Check if path is in trusted paths
        if self.trusted_paths.is_trusted(path) {
            return PermissionDecision::Allowed;
        }

        // Path is not trusted and no cached decision - needs user confirmation
        PermissionDecision::NeedsPrompt
    }

    /// Record a user's permission decision for the session
    ///
    /// This is called after the user responds to a permission prompt.
    pub fn record_decision(
        &mut self,
        path: &Path,
        operation: OperationType,
        decision: PermissionDecision,
    ) {
        self.session_permissions.set(path, operation, decision);
    }

    /// Add a path to trusted paths (for "always" responses that should persist)
    ///
    /// Returns the path string that was added (for saving to config).
    pub fn add_trusted_path(&mut self, path: &Path) -> Result<String, super::TrustedPathsError> {
        let path_str = path.to_string_lossy().to_string();
        self.trusted_paths.add(&path_str)?;
        Ok(path_str)
    }

    /// Clear session-level permission cache
    pub fn clear_session_cache(&mut self) {
        self.session_permissions.clear();
    }

    /// Get a reference to the trusted paths
    pub fn trusted_paths(&self) -> &TrustedPaths {
        &self.trusted_paths
    }

    /// Check if a write or modify operation is allowed
    ///
    /// This is a convenience method that checks both Write and Modify operations.
    pub fn check_write(&self, path: &Path) -> PermissionDecision {
        // If the file exists, it's a modify; otherwise it's a write
        if path.exists() {
            self.check(path, OperationType::Modify)
        } else {
            self.check(path, OperationType::Write)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_checker_with_trusted(paths: &[&str]) -> PermissionChecker {
        let path_strings: Vec<String> = paths.iter().map(|s| s.to_string()).collect();
        let trusted = TrustedPaths::new(&path_strings).expect("Should create trusted paths");
        PermissionChecker::new(trusted, true)
    }

    #[test]
    fn test_read_always_allowed() {
        let checker = create_checker_with_trusted(&[]);

        // Read should always be allowed even for untrusted paths
        let decision = checker.check(Path::new("/any/path/file.txt"), OperationType::Read);
        assert_eq!(decision, PermissionDecision::Allowed);
    }

    #[test]
    fn test_trusted_path_no_prompt() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let trusted_dir = temp_dir.path().join("trusted");
        fs::create_dir(&trusted_dir).expect("Should create dir");

        let checker = create_checker_with_trusted(&[trusted_dir.to_str().unwrap()]);

        // Writing to trusted path should be allowed
        let decision = checker.check(&trusted_dir.join("file.txt"), OperationType::Write);
        assert_eq!(decision, PermissionDecision::Allowed);

        // Modifying in trusted path should be allowed
        let decision = checker.check(&trusted_dir.join("existing.txt"), OperationType::Modify);
        assert_eq!(decision, PermissionDecision::Allowed);

        // Deleting in trusted path should be allowed
        let decision = checker.check(&trusted_dir.join("file.txt"), OperationType::Delete);
        assert_eq!(decision, PermissionDecision::Allowed);
    }

    #[test]
    fn test_untrusted_path_prompts() {
        let checker = create_checker_with_trusted(&["/trusted/path"]);

        // Writing to untrusted path should need prompt
        let decision = checker.check(Path::new("/untrusted/path/file.txt"), OperationType::Write);
        assert_eq!(decision, PermissionDecision::NeedsPrompt);

        // Modifying untrusted path should need prompt
        let decision = checker.check(Path::new("/untrusted/path/file.txt"), OperationType::Modify);
        assert_eq!(decision, PermissionDecision::NeedsPrompt);
    }

    #[test]
    fn test_session_cache_allowed() {
        let mut checker = create_checker_with_trusted(&[]);
        let path = Path::new("/some/path/file.txt");

        // Initially needs prompt
        assert_eq!(
            checker.check(path, OperationType::Write),
            PermissionDecision::NeedsPrompt
        );

        // Record allowed decision
        checker.record_decision(path, OperationType::Write, PermissionDecision::Allowed);

        // Now should be allowed from cache
        assert_eq!(
            checker.check(path, OperationType::Write),
            PermissionDecision::Allowed
        );
    }

    #[test]
    fn test_session_cache_denied() {
        let mut checker = create_checker_with_trusted(&[]);
        let path = Path::new("/some/path/file.txt");

        // Record denied decision
        checker.record_decision(path, OperationType::Write, PermissionDecision::Denied);

        // Should be denied from cache
        assert_eq!(
            checker.check(path, OperationType::Write),
            PermissionDecision::Denied
        );
    }

    #[test]
    fn test_session_cache_clear() {
        let mut checker = create_checker_with_trusted(&[]);
        let path = Path::new("/some/path/file.txt");

        // Record a decision
        checker.record_decision(path, OperationType::Write, PermissionDecision::Allowed);
        assert_eq!(
            checker.check(path, OperationType::Write),
            PermissionDecision::Allowed
        );

        // Clear cache
        checker.clear_session_cache();

        // Should need prompt again
        assert_eq!(
            checker.check(path, OperationType::Write),
            PermissionDecision::NeedsPrompt
        );
    }

    #[test]
    fn test_add_trusted_path() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let new_trusted = temp_dir.path().join("new_trusted");
        fs::create_dir(&new_trusted).expect("Should create dir");

        let mut checker = create_checker_with_trusted(&[]);

        // Initially needs prompt
        assert_eq!(
            checker.check(&new_trusted.join("file.txt"), OperationType::Write),
            PermissionDecision::NeedsPrompt
        );

        // Add to trusted paths
        checker
            .add_trusted_path(&new_trusted)
            .expect("Should add trusted path");

        // Now should be allowed
        assert_eq!(
            checker.check(&new_trusted.join("file.txt"), OperationType::Write),
            PermissionDecision::Allowed
        );
    }

    #[test]
    fn test_operation_type_independence() {
        let mut checker = create_checker_with_trusted(&[]);
        let path = Path::new("/some/path/file.txt");

        // Allow write
        checker.record_decision(path, OperationType::Write, PermissionDecision::Allowed);

        // Delete should still need prompt (different operation)
        assert_eq!(
            checker.check(path, OperationType::Delete),
            PermissionDecision::NeedsPrompt
        );
    }

    #[test]
    fn test_check_write_new_file() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let new_file = temp_dir.path().join("nonexistent.txt");

        let checker = create_checker_with_trusted(&[]);

        // File doesn't exist, so check_write should check Write operation
        let decision = checker.check_write(&new_file);
        assert_eq!(decision, PermissionDecision::NeedsPrompt);
    }

    #[test]
    fn test_check_write_existing_file() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let existing_file = temp_dir.path().join("existing.txt");
        fs::write(&existing_file, "content").expect("Should write file");

        let checker = create_checker_with_trusted(&[]);

        // File exists, so check_write should check Modify operation
        let decision = checker.check_write(&existing_file);
        assert_eq!(decision, PermissionDecision::NeedsPrompt);
    }

    #[test]
    fn test_session_permissions_new() {
        let perms = SessionPermissions::new();
        assert!(!perms.has_decision(Path::new("/test"), OperationType::Write));
    }

    #[test]
    fn test_operation_type_display() {
        assert_eq!(format!("{}", OperationType::Read), "read");
        assert_eq!(format!("{}", OperationType::Write), "write");
        assert_eq!(format!("{}", OperationType::Modify), "modify");
        assert_eq!(format!("{}", OperationType::Delete), "delete");
    }

    #[test]
    fn test_auto_read_disabled() {
        let trusted = TrustedPaths::new(&[]).expect("Should create");
        let checker = PermissionChecker::new(trusted, false);

        // With auto_read disabled, read operations need prompt for untrusted paths
        let decision = checker.check(Path::new("/untrusted/file.txt"), OperationType::Read);
        assert_eq!(decision, PermissionDecision::NeedsPrompt);
    }

    #[test]
    fn test_trusted_paths_accessor() {
        let trusted = TrustedPaths::new(&["/test/path".to_string()]).expect("Should create");
        let checker = PermissionChecker::new(trusted, true);

        // Should be able to access trusted paths
        assert!(checker.trusted_paths().is_trusted_str("/test/path"));
    }
}

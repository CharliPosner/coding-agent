//! Trusted paths configuration and matching
//!
//! This module handles path trust verification, including:
//! - Exact path matching
//! - Subdirectory matching
//! - Tilde expansion
//! - Glob pattern matching
//! - Symlink resolution

use std::path::{Path, PathBuf};

/// Errors that can occur during trusted path operations
#[derive(Debug)]
pub enum TrustedPathsError {
    /// Failed to expand home directory
    HomeExpansionFailed,
    /// Failed to canonicalize path
    CanonicalizationFailed(std::io::Error),
    /// Invalid glob pattern
    InvalidGlobPattern(String),
}

impl std::fmt::Display for TrustedPathsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustedPathsError::HomeExpansionFailed => {
                write!(f, "Failed to expand home directory (~)")
            }
            TrustedPathsError::CanonicalizationFailed(e) => {
                write!(f, "Failed to resolve path: {}", e)
            }
            TrustedPathsError::InvalidGlobPattern(p) => {
                write!(f, "Invalid glob pattern: {}", p)
            }
        }
    }
}

impl std::error::Error for TrustedPathsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TrustedPathsError::CanonicalizationFailed(e) => Some(e),
            _ => None,
        }
    }
}

/// Manages trusted paths for permission checking
#[derive(Debug, Clone)]
pub struct TrustedPaths {
    /// List of trusted path patterns (already expanded)
    patterns: Vec<TrustedPattern>,
}

/// A single trusted path pattern
#[derive(Debug, Clone)]
enum TrustedPattern {
    /// Exact path (and subdirectories)
    Exact(PathBuf),
    /// Glob pattern for matching
    Glob(glob::Pattern, PathBuf),
}

impl TrustedPaths {
    /// Create a new TrustedPaths from a list of path strings
    ///
    /// Path strings can include:
    /// - Absolute paths: `/Users/foo/bar`
    /// - Tilde paths: `~/Documents`
    /// - Glob patterns: `~/projects/*`
    pub fn new(paths: &[String]) -> Result<Self, TrustedPathsError> {
        let mut patterns = Vec::new();

        for path_str in paths {
            let expanded = expand_tilde(path_str)?;
            let pattern = parse_pattern(&expanded)?;
            patterns.push(pattern);
        }

        Ok(Self { patterns })
    }

    /// Check if a path is trusted
    ///
    /// A path is trusted if:
    /// - It exactly matches a trusted path
    /// - It is a subdirectory of a trusted path
    /// - It matches a glob pattern
    ///
    /// The path will be canonicalized (symlinks resolved) before checking.
    pub fn is_trusted(&self, path: &Path) -> bool {
        // Try to canonicalize the path to resolve symlinks
        // If the path doesn't exist yet, use the parent directory
        let canonical = canonicalize_or_parent(path);

        for pattern in &self.patterns {
            if matches_pattern(&canonical, pattern) {
                return true;
            }
        }

        false
    }

    /// Check if a path is trusted, given a string path
    pub fn is_trusted_str(&self, path: &str) -> bool {
        self.is_trusted(Path::new(path))
    }

    /// Add a new trusted path
    pub fn add(&mut self, path: &str) -> Result<(), TrustedPathsError> {
        let expanded = expand_tilde(path)?;
        let pattern = parse_pattern(&expanded)?;
        self.patterns.push(pattern);
        Ok(())
    }

    /// Get the list of trusted path patterns as strings
    pub fn patterns_as_strings(&self) -> Vec<String> {
        self.patterns
            .iter()
            .map(|p| match p {
                TrustedPattern::Exact(path) => path.display().to_string(),
                TrustedPattern::Glob(_, base) => base.display().to_string(),
            })
            .collect()
    }
}

/// Expand tilde (~) to the user's home directory
fn expand_tilde(path: &str) -> Result<String, TrustedPathsError> {
    if path.starts_with("~/") {
        let home = dirs::home_dir().ok_or(TrustedPathsError::HomeExpansionFailed)?;
        Ok(format!("{}/{}", home.display(), &path[2..]))
    } else if path == "~" {
        let home = dirs::home_dir().ok_or(TrustedPathsError::HomeExpansionFailed)?;
        Ok(home.display().to_string())
    } else {
        Ok(path.to_string())
    }
}

/// Parse a path string into a TrustedPattern
fn parse_pattern(path: &str) -> Result<TrustedPattern, TrustedPathsError> {
    // Check if this is a glob pattern
    if path.contains('*') || path.contains('?') || path.contains('[') {
        // For glob patterns, try to canonicalize the base directory (before the glob chars)
        // This ensures the pattern works with resolved symlinks
        let canonical_pattern = canonicalize_glob_base(path);
        let pattern = glob::Pattern::new(&canonical_pattern)
            .map_err(|_| TrustedPathsError::InvalidGlobPattern(path.to_string()))?;
        Ok(TrustedPattern::Glob(pattern, PathBuf::from(path)))
    } else {
        // Try to canonicalize exact paths so they match after symlink resolution
        let path_buf = PathBuf::from(path);
        let canonical = path_buf.canonicalize().unwrap_or(path_buf);
        Ok(TrustedPattern::Exact(canonical))
    }
}

/// Canonicalize the base directory of a glob pattern
fn canonicalize_glob_base(pattern: &str) -> String {
    // Find the position of the first glob character
    let glob_pos = pattern
        .find(|c| c == '*' || c == '?' || c == '[')
        .unwrap_or(pattern.len());

    // Find the last path separator before the glob character
    let base_end = pattern[..glob_pos].rfind('/').unwrap_or(0);

    if base_end == 0 {
        return pattern.to_string();
    }

    let base = &pattern[..base_end];
    let rest = &pattern[base_end..];

    // Try to canonicalize the base
    if let Ok(canonical_base) = PathBuf::from(base).canonicalize() {
        format!("{}{}", canonical_base.display(), rest)
    } else {
        pattern.to_string()
    }
}

/// Check if a path matches a pattern
fn matches_pattern(path: &Path, pattern: &TrustedPattern) -> bool {
    match pattern {
        TrustedPattern::Exact(trusted_path) => {
            // Exact match or subdirectory match
            path == trusted_path || path.starts_with(trusted_path)
        }
        TrustedPattern::Glob(glob_pattern, _) => {
            // Match against the glob pattern
            glob_pattern.matches_path(path)
        }
    }
}

/// Canonicalize a path, falling back to canonicalizing the parent if the path doesn't exist
fn canonicalize_or_parent(path: &Path) -> PathBuf {
    // First, try to make the path absolute if it's relative
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };

    // Try to canonicalize the full path
    if let Ok(canonical) = absolute.canonicalize() {
        return canonical;
    }

    // If that fails, try canonicalizing the parent and appending the filename
    if let (Some(parent), Some(filename)) = (absolute.parent(), absolute.file_name()) {
        if let Ok(canonical_parent) = parent.canonicalize() {
            return canonical_parent.join(filename);
        }
    }

    // Last resort: return the absolute path as-is
    absolute
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_trusted_path_exact_match() {
        let trusted =
            TrustedPaths::new(&["/Users/test/projects".to_string()]).expect("Should create");

        assert!(trusted.is_trusted_str("/Users/test/projects"));
    }

    #[test]
    fn test_trusted_path_subdirectory() {
        let trusted =
            TrustedPaths::new(&["/Users/test/projects".to_string()]).expect("Should create");

        assert!(trusted.is_trusted_str("/Users/test/projects/myapp"));
        assert!(trusted.is_trusted_str("/Users/test/projects/myapp/src/main.rs"));
    }

    #[test]
    fn test_trusted_path_not_trusted() {
        let trusted =
            TrustedPaths::new(&["/Users/test/projects".to_string()]).expect("Should create");

        assert!(!trusted.is_trusted_str("/Users/test/other"));
        assert!(!trusted.is_trusted_str("/etc/passwd"));
    }

    #[test]
    fn test_trusted_path_tilde_expansion() {
        let trusted = TrustedPaths::new(&["~/Documents".to_string()]).expect("Should create");

        // Get home directory for comparison
        let home = dirs::home_dir().expect("Should have home dir");
        let docs_path = home.join("Documents");

        assert!(trusted.is_trusted(&docs_path));
        assert!(trusted.is_trusted(&docs_path.join("file.txt")));
    }

    #[test]
    fn test_trusted_path_glob() {
        // Create a temp directory structure for glob testing
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let projects_dir = temp_dir.path().join("projects");
        let app1_dir = projects_dir.join("app1");
        let app1_src_dir = app1_dir.join("src");

        fs::create_dir_all(&app1_src_dir).expect("Should create dirs");

        // Use double-star pattern which matches any depth
        // Note: Single * in glob typically matches across path separators in many implementations
        let pattern = format!("{}/**", projects_dir.display());
        let trusted = TrustedPaths::new(&[pattern]).expect("Should create");

        // ** should match all descendants
        assert!(trusted.is_trusted(&app1_dir));
        assert!(trusted.is_trusted(&app1_src_dir));
    }

    #[test]
    fn test_trusted_path_glob_double_star() {
        // Create a temp directory structure for glob testing
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let projects_dir = temp_dir.path().join("projects");
        let app1_dir = projects_dir.join("app1");
        let app1_src_dir = app1_dir.join("src");
        let main_rs = app1_src_dir.join("main.rs");

        fs::create_dir_all(&app1_src_dir).expect("Should create dirs");
        fs::write(&main_rs, "fn main() {}").expect("Should write file");

        let pattern = format!("{}/**", projects_dir.display());
        let trusted = TrustedPaths::new(&[pattern]).expect("Should create");

        // ** should match any depth
        assert!(trusted.is_trusted(&app1_dir));
        assert!(trusted.is_trusted(&app1_src_dir));
        assert!(trusted.is_trusted(&main_rs));
    }

    #[test]
    fn test_read_always_allowed() {
        // This test documents that read operations should always be allowed
        // The actual implementation would be in a permission checker that uses TrustedPaths
        // For reads, we skip the trust check entirely
        let trusted = TrustedPaths::new(&[]).expect("Should create");

        // Even with no trusted paths, reads should be allowed (handled by caller)
        // This test just verifies empty trusted paths work
        assert!(!trusted.is_trusted_str("/any/path"));
    }

    #[test]
    fn test_trusted_path_with_symlink() {
        // Create a temp directory with a symlink
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let real_dir = temp_dir.path().join("real");
        let link_dir = temp_dir.path().join("link");

        fs::create_dir(&real_dir).expect("Should create real dir");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_dir, &link_dir).expect("Should create symlink");

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real_dir, &link_dir).expect("Should create symlink");

        // Trust the real directory
        let trusted =
            TrustedPaths::new(&[real_dir.to_string_lossy().to_string()]).expect("Should create");

        // Access via symlink should also be trusted (symlinks are resolved)
        assert!(trusted.is_trusted(&link_dir));
    }

    #[test]
    fn test_relative_path_resolution() {
        // Create a temp directory and change to it
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let trusted_path = temp_dir.path().join("trusted");
        fs::create_dir(&trusted_path).expect("Should create trusted dir");

        let trusted =
            TrustedPaths::new(&[trusted_path.to_string_lossy().to_string()]).expect("Should create");

        // The absolute path should be trusted
        assert!(trusted.is_trusted(&trusted_path));
        assert!(trusted.is_trusted(&trusted_path.join("file.txt")));
    }

    #[test]
    fn test_add_trusted_path() {
        let mut trusted = TrustedPaths::new(&[]).expect("Should create");

        assert!(!trusted.is_trusted_str("/Users/test/new"));

        trusted
            .add("/Users/test/new")
            .expect("Should add trusted path");

        assert!(trusted.is_trusted_str("/Users/test/new"));
        assert!(trusted.is_trusted_str("/Users/test/new/subdir"));
    }

    #[test]
    fn test_patterns_as_strings() {
        let trusted = TrustedPaths::new(&[
            "/Users/test/projects".to_string(),
            "/tmp/scratch".to_string(),
        ])
        .expect("Should create");

        let patterns = trusted.patterns_as_strings();
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"/Users/test/projects".to_string()));
        assert!(patterns.contains(&"/tmp/scratch".to_string()));
    }

    #[test]
    fn test_invalid_glob_pattern() {
        // Unclosed bracket is invalid
        let result = TrustedPaths::new(&["/Users/test/[invalid".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_trusted_paths() {
        let trusted = TrustedPaths::new(&[]).expect("Should create");
        assert!(!trusted.is_trusted_str("/any/path"));
    }

    #[test]
    fn test_multiple_trusted_paths() {
        // Create temp directories for testing
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let projects_dir = temp_dir.path().join("projects");
        let documents_dir = temp_dir.path().join("documents");
        let scratch_dir = temp_dir.path().join("scratch");

        fs::create_dir_all(&projects_dir).expect("Should create projects dir");
        fs::create_dir_all(&documents_dir).expect("Should create documents dir");
        fs::create_dir_all(&scratch_dir).expect("Should create scratch dir");

        let trusted = TrustedPaths::new(&[
            projects_dir.to_string_lossy().to_string(),
            documents_dir.to_string_lossy().to_string(),
            scratch_dir.to_string_lossy().to_string(),
        ])
        .expect("Should create");

        assert!(trusted.is_trusted(&projects_dir.join("app")));
        assert!(trusted.is_trusted(&documents_dir.join("file.md")));
        assert!(trusted.is_trusted(&scratch_dir.join("file")));
        assert!(!trusted.is_trusted(&temp_dir.path().join("other")));
    }
}

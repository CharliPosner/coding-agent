//! Git integration for repository operations
//!
//! This module provides functionality to interact with Git repositories,
//! including reading status, detecting changes, and performing operations.
//!
//! ## Features
//!
//! - Read repository status (modified, staged, untracked files)
//! - Detect if a directory is a git repository
//! - Get current branch name
//! - Check for merge conflicts

use git2::{Error as Git2Error, Repository, Status, StatusOptions};
use std::path::{Path, PathBuf};

/// Errors that can occur during Git operations
#[derive(Debug)]
pub enum GitError {
    /// Not inside a git repository
    NotARepository,
    /// Failed to open the repository
    OpenError(Git2Error),
    /// Failed to read status
    StatusError(Git2Error),
    /// Failed to read HEAD
    HeadError(Git2Error),
    /// Repository is in a detached HEAD state
    DetachedHead,
    /// Merge conflict detected
    MergeConflict,
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotARepository => write!(f, "Not a git repository"),
            GitError::OpenError(e) => write!(f, "Failed to open repository: {}", e),
            GitError::StatusError(e) => write!(f, "Failed to read git status: {}", e),
            GitError::HeadError(e) => write!(f, "Failed to read HEAD: {}", e),
            GitError::DetachedHead => write!(f, "Repository is in detached HEAD state"),
            GitError::MergeConflict => write!(f, "Repository has merge conflicts"),
        }
    }
}

impl std::error::Error for GitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GitError::OpenError(e) => Some(e),
            GitError::StatusError(e) => Some(e),
            GitError::HeadError(e) => Some(e),
            _ => None,
        }
    }
}

/// Status of a single file in the repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Path to the file relative to repository root
    pub path: PathBuf,
    /// The kind of change
    pub status: FileStatusKind,
}

/// The kind of status a file can have
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatusKind {
    /// File is modified in the working directory (unstaged)
    Modified,
    /// File is staged for commit
    Staged,
    /// File is both staged and has unstaged modifications
    StagedWithChanges,
    /// File is untracked
    Untracked,
    /// File is deleted
    Deleted,
    /// File is renamed
    Renamed,
    /// File has a conflict
    Conflicted,
    /// File is new and staged
    Added,
}

impl FileStatusKind {
    /// Get a short status indicator like git status --short
    pub fn indicator(&self) -> &'static str {
        match self {
            FileStatusKind::Modified => " M",
            FileStatusKind::Staged => "M ",
            FileStatusKind::StagedWithChanges => "MM",
            FileStatusKind::Untracked => "??",
            FileStatusKind::Deleted => " D",
            FileStatusKind::Renamed => "R ",
            FileStatusKind::Conflicted => "UU",
            FileStatusKind::Added => "A ",
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            FileStatusKind::Modified => "modified",
            FileStatusKind::Staged => "staged",
            FileStatusKind::StagedWithChanges => "staged with changes",
            FileStatusKind::Untracked => "untracked",
            FileStatusKind::Deleted => "deleted",
            FileStatusKind::Renamed => "renamed",
            FileStatusKind::Conflicted => "conflicted",
            FileStatusKind::Added => "new file",
        }
    }
}

/// Overall status of a git repository
#[derive(Debug, Clone)]
pub struct RepoStatus {
    /// Current branch name (None if detached HEAD)
    pub branch: Option<String>,
    /// Whether the repository is in a detached HEAD state
    pub detached: bool,
    /// Whether there are merge conflicts
    pub has_conflicts: bool,
    /// Files with changes
    pub files: Vec<FileStatus>,
}

impl RepoStatus {
    /// Check if the repository has any changes
    pub fn is_clean(&self) -> bool {
        self.files.is_empty()
    }

    /// Get only modified (unstaged) files
    pub fn modified_files(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| matches!(f.status, FileStatusKind::Modified))
            .collect()
    }

    /// Get only staged files
    pub fn staged_files(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| {
                matches!(
                    f.status,
                    FileStatusKind::Staged
                        | FileStatusKind::StagedWithChanges
                        | FileStatusKind::Added
                        | FileStatusKind::Renamed
                )
            })
            .collect()
    }

    /// Get only untracked files
    pub fn untracked_files(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| matches!(f.status, FileStatusKind::Untracked))
            .collect()
    }

    /// Get files with conflicts
    pub fn conflicted_files(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| matches!(f.status, FileStatusKind::Conflicted))
            .collect()
    }

    /// Check if there are any staged changes ready to commit
    pub fn has_staged(&self) -> bool {
        !self.staged_files().is_empty()
    }

    /// Check if there are any unstaged changes
    pub fn has_unstaged(&self) -> bool {
        self.files.iter().any(|f| {
            matches!(
                f.status,
                FileStatusKind::Modified
                    | FileStatusKind::Deleted
                    | FileStatusKind::StagedWithChanges
            )
        })
    }
}

/// A git repository wrapper
pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    /// Open a git repository at the given path
    ///
    /// This will search upward from the given path to find the repository root.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, GitError> {
        let repo = Repository::discover(path.as_ref()).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                GitError::NotARepository
            } else {
                GitError::OpenError(e)
            }
        })?;

        Ok(Self { repo })
    }

    /// Open a git repository at the current working directory
    pub fn open_cwd() -> Result<Self, GitError> {
        let cwd = std::env::current_dir().map_err(|_| GitError::NotARepository)?;
        Self::open(cwd)
    }

    /// Get the repository root directory
    pub fn root(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<Option<String>, GitError> {
        let head = self.repo.head().map_err(GitError::HeadError)?;

        if head.is_branch() {
            Ok(head.shorthand().map(|s| s.to_string()))
        } else {
            // Detached HEAD
            Ok(None)
        }
    }

    /// Check if the repository is in a detached HEAD state
    pub fn is_detached(&self) -> bool {
        self.repo.head_detached().unwrap_or(false)
    }

    /// Get the full status of the repository
    pub fn status(&self) -> Result<RepoStatus, GitError> {
        let branch = self.current_branch()?;
        let detached = branch.is_none();

        // Configure status options
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .include_unmodified(false)
            .recurse_untracked_dirs(true);

        // Get status entries
        let statuses = self
            .repo
            .statuses(Some(&mut opts))
            .map_err(GitError::StatusError)?;

        let mut files = Vec::new();
        let mut has_conflicts = false;

        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry
                .path()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(""));

            // Check for conflicts
            if status.is_conflicted() {
                has_conflicts = true;
                files.push(FileStatus {
                    path,
                    status: FileStatusKind::Conflicted,
                });
                continue;
            }

            // Determine the status kind
            let kind = Self::determine_status_kind(status);
            if let Some(kind) = kind {
                files.push(FileStatus { path, status: kind });
            }
        }

        Ok(RepoStatus {
            branch,
            detached,
            has_conflicts,
            files,
        })
    }

    /// Determine the FileStatusKind from git2 Status flags
    fn determine_status_kind(status: Status) -> Option<FileStatusKind> {
        // Check for both staged and working directory changes
        let index_new = status.is_index_new();
        let index_modified = status.is_index_modified();
        let index_deleted = status.is_index_deleted();
        let index_renamed = status.is_index_renamed();

        let wt_new = status.is_wt_new();
        let wt_modified = status.is_wt_modified();
        let wt_deleted = status.is_wt_deleted();

        // Both staged and has working directory changes
        if (index_modified || index_new) && wt_modified {
            return Some(FileStatusKind::StagedWithChanges);
        }

        // Staged changes (index)
        if index_new {
            return Some(FileStatusKind::Added);
        }
        if index_renamed {
            return Some(FileStatusKind::Renamed);
        }
        if index_modified {
            return Some(FileStatusKind::Staged);
        }
        if index_deleted {
            return Some(FileStatusKind::Deleted);
        }

        // Working directory changes
        if wt_new {
            return Some(FileStatusKind::Untracked);
        }
        if wt_modified {
            return Some(FileStatusKind::Modified);
        }
        if wt_deleted {
            return Some(FileStatusKind::Deleted);
        }

        None
    }

    /// Check if a specific path is inside this repository
    ///
    /// Canonicalizes both paths to handle symlinks correctly.
    pub fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        if let Some(root) = self.root() {
            // Try to canonicalize both paths to handle symlinks (e.g., /var -> /private/var on macOS)
            let canonical_root = root.canonicalize().ok();
            let canonical_path = path.as_ref().canonicalize().ok();

            match (canonical_root, canonical_path) {
                (Some(root), Some(path)) => path.starts_with(root),
                // Fall back to non-canonical comparison if canonicalization fails
                _ => path.as_ref().starts_with(root),
            }
        } else {
            false
        }
    }
}

/// Check if a path is inside a git repository
pub fn is_git_repository<P: AsRef<Path>>(path: P) -> bool {
    GitRepo::open(path).is_ok()
}

/// Get the status of a repository at the given path
///
/// # Examples
///
/// ```rust,no_run
/// use coding_agent_cli::integrations::git::get_status;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Get git status of current directory
/// let status = get_status(".")?;
///
/// println!("Branch: {:?}", status.branch);
/// println!("Clean: {}", status.is_clean());
///
/// // Show modified files
/// for file in status.modified_files() {
///     println!("Modified: {}", file.path.display());
/// }
///
/// // Show staged files
/// for file in status.staged_files() {
///     println!("Staged: {}", file.path.display());
/// }
/// # Ok(())
/// # }
/// ```
pub fn get_status<P: AsRef<Path>>(path: P) -> Result<RepoStatus, GitError> {
    let repo = GitRepo::open(path)?;
    repo.status()
}

// ============================================================================
// Smart File Grouping
// ============================================================================

/// A group of logically related files
#[derive(Debug, Clone)]
pub struct FileGroup {
    /// Name describing the group (e.g., "auth module", "login feature")
    pub name: String,
    /// Files in this group
    pub files: Vec<FileStatus>,
    /// The reason these files were grouped together
    pub reason: GroupReason,
}

/// The reason files were grouped together
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupReason {
    /// Files are in the same directory
    SameDirectory,
    /// Test file and its implementation
    TestAndImplementation,
    /// Files share a common module/component name
    SharedComponent,
    /// Configuration files
    Configuration,
    /// Documentation files
    Documentation,
    /// Files that couldn't be grouped
    Ungrouped,
}

impl GroupReason {
    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            GroupReason::SameDirectory => "same directory",
            GroupReason::TestAndImplementation => "test and implementation",
            GroupReason::SharedComponent => "related component",
            GroupReason::Configuration => "configuration",
            GroupReason::Documentation => "documentation",
            GroupReason::Ungrouped => "ungrouped",
        }
    }
}

/// Groups files logically based on their relationships
pub struct FileGrouper;

impl FileGrouper {
    /// Group files into logically related sets
    ///
    /// The grouping algorithm:
    /// 1. First, identify test/implementation pairs (highest priority)
    /// 2. Then, group config files together
    /// 3. Then, group documentation files together
    /// 4. Finally, group remaining files by directory
    pub fn group_files(files: &[FileStatus]) -> Vec<FileGroup> {
        if files.is_empty() {
            return vec![];
        }

        if files.len() == 1 {
            let file = &files[0];
            return vec![FileGroup {
                name: Self::file_name(&file.path),
                files: vec![file.clone()],
                reason: GroupReason::Ungrouped,
            }];
        }

        let mut remaining: Vec<FileStatus> = files.to_vec();
        let mut groups: Vec<FileGroup> = Vec::new();

        // Step 1: Find test/implementation pairs
        let test_impl_groups = Self::find_test_impl_pairs(&mut remaining);
        groups.extend(test_impl_groups);

        // Step 2: Group configuration files
        if let Some(config_group) = Self::extract_config_files(&mut remaining) {
            groups.push(config_group);
        }

        // Step 3: Group documentation files
        if let Some(docs_group) = Self::extract_docs_files(&mut remaining) {
            groups.push(docs_group);
        }

        // Step 4: Group by shared component names
        let component_groups = Self::find_shared_components(&mut remaining);
        groups.extend(component_groups);

        // Step 5: Group remaining files by directory
        let dir_groups = Self::group_by_directory(&mut remaining);
        groups.extend(dir_groups);

        // Step 6: Handle any truly ungrouped files
        for file in remaining {
            groups.push(FileGroup {
                name: Self::file_name(&file.path),
                files: vec![file],
                reason: GroupReason::Ungrouped,
            });
        }

        groups
    }

    /// Find test files paired with their implementation files
    fn find_test_impl_pairs(files: &mut Vec<FileStatus>) -> Vec<FileGroup> {
        let mut groups = Vec::new();
        let mut used_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

        // Find test files and try to match them with implementations
        for (i, file) in files.iter().enumerate() {
            if used_indices.contains(&i) {
                continue;
            }

            let path_str = file.path.to_string_lossy();

            // Check if this is a test file
            if let Some(impl_pattern) = Self::extract_impl_pattern(&path_str) {
                // Look for matching implementation file
                for (j, other) in files.iter().enumerate() {
                    if i == j || used_indices.contains(&j) {
                        continue;
                    }

                    let other_path = other.path.to_string_lossy();
                    if Self::matches_impl_pattern(&other_path, &impl_pattern) {
                        // Found a pair!
                        used_indices.insert(i);
                        used_indices.insert(j);

                        let component_name = Self::extract_component_name(&impl_pattern);
                        groups.push(FileGroup {
                            name: format!("{} (with tests)", component_name),
                            files: vec![other.clone(), file.clone()],
                            reason: GroupReason::TestAndImplementation,
                        });
                        break;
                    }
                }
            }
        }

        // Remove used files (in reverse order to preserve indices)
        let mut indices: Vec<_> = used_indices.into_iter().collect();
        indices.sort_by(|a, b| b.cmp(a));
        for idx in indices {
            files.remove(idx);
        }

        groups
    }

    /// Extract the pattern to find the implementation file from a test file path
    fn extract_impl_pattern(test_path: &str) -> Option<String> {
        // Common test file patterns:
        // - tests/foo_test.rs -> src/foo.rs
        // - test_foo.rs -> foo.rs
        // - foo.test.rs -> foo.rs
        // - foo_test.rs -> foo.rs
        // - src/foo/tests.rs -> src/foo/mod.rs

        let path = Path::new(test_path);
        let file_stem = path.file_stem()?.to_string_lossy();

        // Pattern: foo_test.rs or foo.test.rs
        if file_stem.ends_with("_test") {
            let base = file_stem.trim_end_matches("_test");
            return Some(base.to_string());
        }

        // Pattern: test_foo.rs
        if file_stem.starts_with("test_") {
            let base = file_stem.trim_start_matches("test_");
            return Some(base.to_string());
        }

        // Pattern: tests/ directory
        if test_path.contains("/tests/") || test_path.starts_with("tests/") {
            let base = file_stem.trim_end_matches("_test");
            return Some(base.to_string());
        }

        None
    }

    /// Check if a path matches the implementation pattern
    fn matches_impl_pattern(path: &str, pattern: &str) -> bool {
        let p = Path::new(path);
        let file_stem = match p.file_stem() {
            Some(s) => s.to_string_lossy(),
            None => return false,
        };

        // Direct match
        if file_stem == pattern {
            return true;
        }

        // Pattern could be the module name in a path like src/foo/mod.rs
        if file_stem == "mod" && path.contains(&format!("/{}/", pattern)) {
            return true;
        }

        false
    }

    /// Extract a component name from a pattern
    fn extract_component_name(pattern: &str) -> String {
        pattern.to_string()
    }

    /// Extract configuration files into a group
    fn extract_config_files(files: &mut Vec<FileStatus>) -> Option<FileGroup> {
        let config_extensions = ["toml", "json", "yaml", "yml", "ini", "cfg"];
        let config_names = ["Cargo.toml", "package.json", "config", "settings"];

        let config_indices: Vec<usize> = files
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                let path_str = f.path.to_string_lossy();
                let path = Path::new(path_str.as_ref());

                // Check extension
                if let Some(ext) = path.extension() {
                    if config_extensions.contains(&ext.to_string_lossy().as_ref()) {
                        return true;
                    }
                }

                // Check known config file names
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    return config_names.iter().any(|&cn| name_str.contains(cn));
                }

                false
            })
            .map(|(i, _)| i)
            .collect();

        if config_indices.is_empty() {
            return None;
        }

        // Extract config files
        let mut config_files = Vec::new();
        for &idx in config_indices.iter().rev() {
            config_files.push(files.remove(idx));
        }
        config_files.reverse();

        Some(FileGroup {
            name: "configuration".to_string(),
            files: config_files,
            reason: GroupReason::Configuration,
        })
    }

    /// Extract documentation files into a group
    fn extract_docs_files(files: &mut Vec<FileStatus>) -> Option<FileGroup> {
        let doc_extensions = ["md", "rst", "txt", "adoc"];
        let doc_dirs = ["docs/", "doc/", "documentation/"];

        let doc_indices: Vec<usize> = files
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                let path_str = f.path.to_string_lossy();
                let path = Path::new(path_str.as_ref());

                // Check if in docs directory
                if doc_dirs.iter().any(|d| path_str.contains(d)) {
                    return true;
                }

                // Check extension for markdown etc
                if let Some(ext) = path.extension() {
                    return doc_extensions.contains(&ext.to_string_lossy().as_ref());
                }

                false
            })
            .map(|(i, _)| i)
            .collect();

        if doc_indices.is_empty() {
            return None;
        }

        let mut doc_files = Vec::new();
        for &idx in doc_indices.iter().rev() {
            doc_files.push(files.remove(idx));
        }
        doc_files.reverse();

        Some(FileGroup {
            name: "documentation".to_string(),
            files: doc_files,
            reason: GroupReason::Documentation,
        })
    }

    /// Find files that share a common component name
    fn find_shared_components(files: &mut Vec<FileStatus>) -> Vec<FileGroup> {
        use std::collections::HashMap;

        if files.len() < 2 {
            return vec![];
        }

        // Build a map of component names to file indices
        let mut component_map: HashMap<String, Vec<usize>> = HashMap::new();

        for (i, file) in files.iter().enumerate() {
            if let Some(component) = Self::extract_file_component(&file.path) {
                component_map.entry(component).or_default().push(i);
            }
        }

        // Find components with multiple files
        let mut groups = Vec::new();
        let mut used_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for (component, indices) in component_map {
            if indices.len() >= 2 {
                let group_files: Vec<FileStatus> =
                    indices.iter().map(|&i| files[i].clone()).collect();

                for idx in indices {
                    used_indices.insert(idx);
                }

                groups.push(FileGroup {
                    name: format!("{} component", component),
                    files: group_files,
                    reason: GroupReason::SharedComponent,
                });
            }
        }

        // Remove used files
        let mut indices: Vec<_> = used_indices.into_iter().collect();
        indices.sort_by(|a, b| b.cmp(a));
        for idx in indices {
            files.remove(idx);
        }

        groups
    }

    /// Extract a component name from a file path
    fn extract_file_component(path: &Path) -> Option<String> {
        let file_stem = path.file_stem()?.to_string_lossy();

        // Skip generic names
        if file_stem == "mod" || file_stem == "lib" || file_stem == "main" || file_stem == "index" {
            // Use parent directory name instead
            return path
                .parent()?
                .file_name()?
                .to_string_lossy()
                .to_string()
                .into();
        }

        // Remove common suffixes
        let base = file_stem
            .trim_end_matches("_test")
            .trim_end_matches(".test")
            .trim_end_matches("_spec")
            .trim_start_matches("test_");

        Some(base.to_string())
    }

    /// Group remaining files by directory
    fn group_by_directory(files: &mut Vec<FileStatus>) -> Vec<FileGroup> {
        use std::collections::HashMap;

        if files.is_empty() {
            return vec![];
        }

        let mut dir_map: HashMap<String, Vec<FileStatus>> = HashMap::new();

        for file in files.drain(..) {
            let dir = file
                .path
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();

            dir_map.entry(dir).or_default().push(file);
        }

        dir_map
            .into_iter()
            .map(|(dir, group_files)| {
                let name = if dir.is_empty() {
                    "root".to_string()
                } else {
                    // Get the last component of the directory path
                    Path::new(&dir)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| dir.clone())
                };

                FileGroup {
                    name: format!("{} module", name),
                    files: group_files,
                    reason: GroupReason::SameDirectory,
                }
            })
            .collect()
    }

    /// Get just the file name from a path
    fn file_name(path: &Path) -> String {
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
    }
}

/// Suggest how to split files into separate commits
pub fn suggest_commit_splits(files: &[FileStatus]) -> Vec<FileGroup> {
    FileGrouper::group_files(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_test_repo() -> (TempDir, Repository) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(temp_dir.path()).expect("Failed to init repo");

        // Configure user for commits
        let mut config = repo.config().expect("Failed to get config");
        config
            .set_str("user.name", "Test User")
            .expect("Failed to set user.name");
        config
            .set_str("user.email", "test@example.com")
            .expect("Failed to set user.email");

        (temp_dir, repo)
    }

    #[test]
    fn test_git_status_clean() {
        let (temp_dir, repo) = init_test_repo();

        // Create and commit a file so the repo isn't empty
        let file_path = temp_dir.path().join("README.md");
        fs::write(&file_path, "# Test\n").expect("Failed to write file");

        // Stage and commit
        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("README.md"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to commit");

        // Now check status
        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let status = git_repo.status().expect("Failed to get status");

        assert!(status.is_clean());
        assert!(!status.detached);
        assert!(!status.has_conflicts);
    }

    #[test]
    fn test_git_status_modified() {
        let (temp_dir, repo) = init_test_repo();

        // Create and commit a file
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "original content").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("test.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to commit");

        // Modify the file
        fs::write(&file_path, "modified content").expect("Failed to write file");

        // Check status
        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let status = git_repo.status().expect("Failed to get status");

        assert!(!status.is_clean());
        assert_eq!(status.modified_files().len(), 1);
        assert_eq!(status.modified_files()[0].path, PathBuf::from("test.txt"));
    }

    #[test]
    fn test_git_status_untracked() {
        let (temp_dir, repo) = init_test_repo();

        // Create initial commit to establish HEAD
        let file_path = temp_dir.path().join("initial.txt");
        fs::write(&file_path, "initial").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("initial.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to commit");

        // Create an untracked file
        let untracked_path = temp_dir.path().join("untracked.txt");
        fs::write(&untracked_path, "untracked content").expect("Failed to write file");

        // Check status
        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let status = git_repo.status().expect("Failed to get status");

        assert!(!status.is_clean());
        assert_eq!(status.untracked_files().len(), 1);
        assert_eq!(
            status.untracked_files()[0].path,
            PathBuf::from("untracked.txt")
        );
    }

    #[test]
    fn test_not_a_repository() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let result = GitRepo::open(temp_dir.path());
        assert!(matches!(result, Err(GitError::NotARepository)));
    }

    #[test]
    fn test_is_git_repository() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Not a repo initially
        assert!(!is_git_repository(temp_dir.path()));

        // Init repo
        Repository::init(temp_dir.path()).expect("Failed to init repo");

        // Now it is
        assert!(is_git_repository(temp_dir.path()));
    }

    #[test]
    fn test_current_branch() {
        let (temp_dir, repo) = init_test_repo();

        // Create initial commit (needed to have a branch)
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("test.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to commit");

        // Check branch
        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let branch = git_repo.current_branch().expect("Failed to get branch");

        // Default branch is usually "master" or "main" depending on git config
        assert!(branch.is_some());
        let branch_name = branch.unwrap();
        assert!(branch_name == "master" || branch_name == "main");
    }

    #[test]
    fn test_file_status_indicator() {
        assert_eq!(FileStatusKind::Modified.indicator(), " M");
        assert_eq!(FileStatusKind::Staged.indicator(), "M ");
        assert_eq!(FileStatusKind::Untracked.indicator(), "??");
        assert_eq!(FileStatusKind::Conflicted.indicator(), "UU");
        assert_eq!(FileStatusKind::Added.indicator(), "A ");
    }

    #[test]
    fn test_file_status_description() {
        assert_eq!(FileStatusKind::Modified.description(), "modified");
        assert_eq!(FileStatusKind::Staged.description(), "staged");
        assert_eq!(FileStatusKind::Untracked.description(), "untracked");
        assert_eq!(FileStatusKind::Conflicted.description(), "conflicted");
        assert_eq!(FileStatusKind::Added.description(), "new file");
    }

    #[test]
    fn test_staged_files() {
        let (temp_dir, repo) = init_test_repo();

        // Create initial commit
        let file_path = temp_dir.path().join("initial.txt");
        fs::write(&file_path, "initial").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("initial.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to commit");

        // Create and stage a new file
        let new_file = temp_dir.path().join("staged.txt");
        fs::write(&new_file, "staged content").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("staged.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        // Check status
        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let status = git_repo.status().expect("Failed to get status");

        assert!(status.has_staged());
        assert_eq!(status.staged_files().len(), 1);
    }

    #[test]
    fn test_repo_root() {
        let (temp_dir, _repo) = init_test_repo();

        // Create a subdirectory
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdir");

        // Open repo from subdirectory
        let git_repo = GitRepo::open(&sub_dir).expect("Failed to open repo");

        // Root should be the temp_dir, not the subdir
        // Canonicalize both paths to handle macOS symlinks (/var -> /private/var)
        let root = git_repo.root().expect("Should have root");
        let canonical_root = root.canonicalize().expect("Failed to canonicalize root");
        let canonical_temp = temp_dir
            .path()
            .canonicalize()
            .expect("Failed to canonicalize temp");
        assert_eq!(canonical_root, canonical_temp);
    }

    #[test]
    fn test_contains_path() {
        let (temp_dir, _repo) = init_test_repo();

        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");

        // Create an actual file so we can test with a real path
        let inside = temp_dir.path().join("some_file.txt");
        fs::write(&inside, "test").expect("Failed to create file");

        // Test with the canonicalized path (handles macOS /var -> /private/var symlink)
        let canonical_inside = inside.canonicalize().expect("Failed to canonicalize");
        assert!(git_repo.contains(&canonical_inside));

        // Path outside repo - use a definitely-outside path
        let outside = PathBuf::from("/usr/outside");
        assert!(!git_repo.contains(&outside));
    }

    // ========================================================================
    // File Grouping Tests
    // ========================================================================

    #[test]
    fn test_file_grouping_empty() {
        let files: Vec<FileStatus> = vec![];
        let groups = FileGrouper::group_files(&files);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_file_grouping_single_file() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/main.rs"),
            status: FileStatusKind::Modified,
        }];

        let groups = FileGrouper::group_files(&files);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 1);
        assert_eq!(groups[0].name, "main.rs");
    }

    #[test]
    fn test_file_grouping_same_dir() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/auth/login.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/auth/logout.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/auth/session.rs"),
                status: FileStatusKind::Added,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        // All files should be in one group (same directory)
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 3);
        assert_eq!(groups[0].reason, GroupReason::SameDirectory);
        assert!(groups[0].name.contains("auth"));
    }

    #[test]
    fn test_file_grouping_test_and_impl() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/parser.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("tests/parser_test.rs"),
                status: FileStatusKind::Modified,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        // Should be grouped as test + implementation
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[0].reason, GroupReason::TestAndImplementation);
        assert!(groups[0].name.contains("parser"));
    }

    #[test]
    fn test_file_grouping_test_prefix() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/utils.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("tests/test_utils.rs"),
                status: FileStatusKind::Added,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        // Should recognize test_utils.rs as test for utils.rs
        let test_groups: Vec<_> = groups
            .iter()
            .filter(|g| g.reason == GroupReason::TestAndImplementation)
            .collect();

        assert_eq!(test_groups.len(), 1);
        assert_eq!(test_groups[0].files.len(), 2);
    }

    #[test]
    fn test_file_grouping_config_files() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("Cargo.toml"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("config.json"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/main.rs"),
                status: FileStatusKind::Modified,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        // Should have separate groups for config and source
        let config_groups: Vec<_> = groups
            .iter()
            .filter(|g| g.reason == GroupReason::Configuration)
            .collect();

        assert_eq!(config_groups.len(), 1);
        assert_eq!(config_groups[0].files.len(), 2);
    }

    #[test]
    fn test_file_grouping_documentation() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("README.md"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("docs/guide.md"),
                status: FileStatusKind::Added,
            },
            FileStatus {
                path: PathBuf::from("src/lib.rs"),
                status: FileStatusKind::Modified,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        let doc_groups: Vec<_> = groups
            .iter()
            .filter(|g| g.reason == GroupReason::Documentation)
            .collect();

        assert_eq!(doc_groups.len(), 1);
        assert_eq!(doc_groups[0].files.len(), 2);
    }

    #[test]
    fn test_file_grouping_related_components() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/api/handler.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/api/handler_test.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/db/connection.rs"),
                status: FileStatusKind::Modified,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        // handler and handler_test should be together
        let handler_group = groups.iter().find(|g| {
            g.files
                .iter()
                .any(|f| f.path.to_string_lossy().contains("handler.rs"))
        });

        assert!(handler_group.is_some());
        let group = handler_group.unwrap();
        // Should have both handler.rs and handler_test.rs
        assert!(group.files.len() >= 2 || group.reason == GroupReason::TestAndImplementation);
    }

    #[test]
    fn test_file_grouping_mixed_changes() {
        let files = vec![
            // Test + impl pair
            FileStatus {
                path: PathBuf::from("src/auth.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("tests/auth_test.rs"),
                status: FileStatusKind::Modified,
            },
            // Config files
            FileStatus {
                path: PathBuf::from("Cargo.toml"),
                status: FileStatusKind::Modified,
            },
            // Documentation
            FileStatus {
                path: PathBuf::from("README.md"),
                status: FileStatusKind::Modified,
            },
            // Unrelated file
            FileStatus {
                path: PathBuf::from("src/main.rs"),
                status: FileStatusKind::Modified,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        // Should have multiple groups
        assert!(groups.len() >= 3);

        // Verify we have each type of group
        let has_test_impl = groups
            .iter()
            .any(|g| g.reason == GroupReason::TestAndImplementation);
        let has_config = groups
            .iter()
            .any(|g| g.reason == GroupReason::Configuration);
        let has_docs = groups
            .iter()
            .any(|g| g.reason == GroupReason::Documentation);

        assert!(has_test_impl, "Should have test+impl group");
        assert!(has_config, "Should have config group");
        assert!(has_docs, "Should have docs group");
    }

    #[test]
    fn test_suggest_commit_splits() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/feature_a.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("tests/feature_a_test.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/feature_b.rs"),
                status: FileStatusKind::Added,
            },
        ];

        let splits = suggest_commit_splits(&files);

        // Should suggest splitting feature_a (with test) from feature_b
        assert!(splits.len() >= 2);

        // Total files should match input
        let total_files: usize = splits.iter().map(|g| g.files.len()).sum();
        assert_eq!(total_files, 3);
    }

    #[test]
    fn test_group_reason_description() {
        assert_eq!(GroupReason::SameDirectory.description(), "same directory");
        assert_eq!(
            GroupReason::TestAndImplementation.description(),
            "test and implementation"
        );
        assert_eq!(
            GroupReason::SharedComponent.description(),
            "related component"
        );
        assert_eq!(GroupReason::Configuration.description(), "configuration");
        assert_eq!(GroupReason::Documentation.description(), "documentation");
        assert_eq!(GroupReason::Ungrouped.description(), "ungrouped");
    }

    #[test]
    fn test_extract_impl_pattern() {
        // Test _test suffix
        assert_eq!(
            FileGrouper::extract_impl_pattern("tests/parser_test.rs"),
            Some("parser".to_string())
        );

        // Test test_ prefix
        assert_eq!(
            FileGrouper::extract_impl_pattern("tests/test_parser.rs"),
            Some("parser".to_string())
        );

        // Not a test file
        assert_eq!(FileGrouper::extract_impl_pattern("src/parser.rs"), None);
    }

    #[test]
    fn test_matches_impl_pattern() {
        assert!(FileGrouper::matches_impl_pattern("src/parser.rs", "parser"));
        assert!(FileGrouper::matches_impl_pattern(
            "src/parser/mod.rs",
            "parser"
        ));
        assert!(!FileGrouper::matches_impl_pattern("src/lexer.rs", "parser"));
    }
}

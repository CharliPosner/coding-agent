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
                FileStatusKind::Modified | FileStatusKind::Deleted | FileStatusKind::StagedWithChanges
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
        let statuses = self.repo.statuses(Some(&mut opts)).map_err(GitError::StatusError)?;

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
pub fn get_status<P: AsRef<Path>>(path: P) -> Result<RepoStatus, GitError> {
    let repo = GitRepo::open(path)?;
    repo.status()
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
        assert_eq!(
            status.modified_files()[0].path,
            PathBuf::from("test.txt")
        );
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
        let canonical_temp = temp_dir.path().canonicalize().expect("Failed to canonicalize temp");
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
}

//! The /undo command - reverts last commit or file change
//!
//! This command can:
//! - Undo the last commit (soft reset to HEAD~1, keeping changes staged)
//! - Restore a modified file to its last committed state
//! - Unstage a staged file

use super::{Command, CommandContext, CommandResult};
use crate::integrations::git::{GitError, GitRepo};
use git2::{Repository, ResetType};
use std::path::Path;

pub struct UndoCommand;

impl Command for UndoCommand {
    fn name(&self) -> &'static str {
        "undo"
    }

    fn description(&self) -> &'static str {
        "Undo last commit or revert file changes"
    }

    fn usage(&self) -> &'static str {
        "/undo [--hard] [file...]"
    }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        // Parse arguments
        let options = parse_undo_args(args);

        // Open the repository
        let git_repo = match GitRepo::open_cwd() {
            Ok(r) => r,
            Err(GitError::NotARepository) => {
                return CommandResult::Error(
                    "Not in a git repository. Initialize one with `git init`.".to_string(),
                );
            }
            Err(e) => {
                return CommandResult::Error(format!("Failed to open repository: {}", e));
            }
        };

        let repo_root = match git_repo.root() {
            Some(r) => r,
            None => return CommandResult::Error("Could not determine repository root.".to_string()),
        };

        let repo = match Repository::open(repo_root) {
            Ok(r) => r,
            Err(e) => return CommandResult::Error(format!("Failed to open repository: {}", e)),
        };

        // If specific files are provided, revert those files
        if !options.files.is_empty() {
            return revert_files(&repo, &options.files, options.hard);
        }

        // Otherwise, undo the last commit
        undo_last_commit(&repo, options.hard)
    }
}

/// Options for the undo command
#[derive(Debug, Default)]
struct UndoOptions {
    /// Hard reset (discard changes) vs soft reset (keep changes staged)
    hard: bool,
    /// Specific files to revert
    files: Vec<String>,
}

/// Parse command arguments
fn parse_undo_args(args: &[&str]) -> UndoOptions {
    let mut options = UndoOptions::default();
    let mut files = Vec::new();

    for arg in args {
        match *arg {
            "--hard" | "-h" => {
                options.hard = true;
            }
            _ => {
                // Treat as file path
                files.push(arg.to_string());
            }
        }
    }

    options.files = files;
    options
}

/// Undo the last commit
fn undo_last_commit(repo: &Repository, hard: bool) -> CommandResult {
    // Get the current HEAD commit
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            return CommandResult::Error(
                "No commits to undo. Repository has no HEAD.".to_string(),
            );
        }
    };

    let current_commit = match head.peel_to_commit() {
        Ok(c) => c,
        Err(e) => {
            return CommandResult::Error(format!("Failed to get current commit: {}", e));
        }
    };

    // Get the parent commit (HEAD~1)
    let parent_commit = match current_commit.parent(0) {
        Ok(p) => p,
        Err(_) => {
            return CommandResult::Error(
                "Cannot undo the initial commit (no parent commit).".to_string(),
            );
        }
    };

    // Get commit message before undoing for output
    let undone_message = current_commit
        .message()
        .unwrap_or("<no message>")
        .lines()
        .next()
        .unwrap_or("<no message>");
    let commit_id = &current_commit.id().to_string()[..7];

    // Reset to parent commit
    let reset_type = if hard {
        ResetType::Hard
    } else {
        ResetType::Soft
    };

    let parent_object = parent_commit.as_object();

    if let Err(e) = repo.reset(parent_object, reset_type, None) {
        return CommandResult::Error(format!("Failed to reset: {}", e));
    }

    let reset_mode = if hard { "hard" } else { "soft" };
    let changes_note = if hard {
        "Changes were discarded."
    } else {
        "Changes are still staged."
    };

    CommandResult::Output(format!(
        "✓ Undid last commit ({} reset) [{}]\n\n\"{}\"\n\n{}",
        reset_mode, commit_id, undone_message, changes_note
    ))
}

/// Revert changes to specific files
fn revert_files(repo: &Repository, files: &[String], discard: bool) -> CommandResult {
    let mut reverted_files = Vec::new();
    let mut errors = Vec::new();

    // Get HEAD commit
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            return CommandResult::Error(
                "No HEAD commit found. Cannot revert files in an empty repository.".to_string(),
            );
        }
    };

    let commit = match head.peel_to_commit() {
        Ok(c) => c,
        Err(e) => {
            return CommandResult::Error(format!("Failed to get HEAD commit: {}", e));
        }
    };

    let tree = match commit.tree() {
        Ok(t) => t,
        Err(e) => {
            return CommandResult::Error(format!("Failed to get commit tree: {}", e));
        }
    };

    for file_path in files {
        let path = Path::new(file_path);

        // First, check if the file is staged (needs to be unstaged first)
        let mut index = match repo.index() {
            Ok(i) => i,
            Err(e) => {
                errors.push(format!("{}: Failed to get index: {}", file_path, e));
                continue;
            }
        };

        // Try to get the file from the tree (HEAD version)
        let tree_entry = match tree.get_path(path) {
            Ok(e) => e,
            Err(_) => {
                errors.push(format!(
                    "{}: File not found in HEAD commit (may be a new file)",
                    file_path
                ));
                continue;
            }
        };

        let blob = match repo.find_blob(tree_entry.id()) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("{}: Failed to read file content: {}", file_path, e));
                continue;
            }
        };

        if discard {
            // Write the HEAD version to the working directory
            let workdir = match repo.workdir() {
                Some(w) => w,
                None => {
                    errors.push(format!("{}: No working directory", file_path));
                    continue;
                }
            };

            let full_path = workdir.join(path);
            if let Some(parent) = full_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    errors.push(format!("{}: Failed to create directory: {}", file_path, e));
                    continue;
                }
            }

            if let Err(e) = std::fs::write(&full_path, blob.content()) {
                errors.push(format!("{}: Failed to write file: {}", file_path, e));
                continue;
            }

            // Also update the index to match HEAD
            if let Err(e) = index.add_path(path) {
                errors.push(format!("{}: Failed to update index: {}", file_path, e));
                continue;
            }

            reverted_files.push(format!("{} (reverted to HEAD)", file_path));
        } else {
            // Just unstage the file (reset index entry to HEAD)
            // Remove from index and re-add from HEAD
            let _ = index.remove_path(path); // Ignore error if not in index

            // Read the blob and add it to the index
            let oid = tree_entry.id();
            let mode = tree_entry.filemode();

            if let Err(e) = index.add_frombuffer(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: mode as u32,
                uid: 0,
                gid: 0,
                file_size: blob.size() as u32,
                id: oid,
                flags: 0,
                flags_extended: 0,
                path: path.as_os_str().as_encoded_bytes().to_vec(),
            }, blob.content()) {
                errors.push(format!("{}: Failed to reset index entry: {}", file_path, e));
                continue;
            }

            reverted_files.push(format!("{} (unstaged)", file_path));
        }

        if let Err(e) = index.write() {
            errors.push(format!("Failed to write index: {}", e));
        }
    }

    // Build result
    let mut output = String::new();

    if !reverted_files.is_empty() {
        let action = if discard { "Reverted" } else { "Unstaged" };
        output.push_str(&format!("✓ {} {} file(s):\n", action, reverted_files.len()));
        for file in reverted_files {
            output.push_str(&format!("  {}\n", file));
        }
    }

    if !errors.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("Errors:\n");
        for error in errors {
            output.push_str(&format!("  ✗ {}\n", error));
        }
    }

    if output.is_empty() {
        output = "No files were reverted.".to_string();
    }

    CommandResult::Output(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::CostTracker;
    use std::fs;
    use std::path::PathBuf;
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

    fn create_commit(temp_dir: &TempDir, repo: &Repository, filename: &str, content: &str, message: &str) {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new(filename))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");

        // Get parent commit if it exists
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<_> = parent.iter().collect();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .expect("Failed to commit");
    }

    #[test]
    fn test_undo_command_name() {
        let cmd = UndoCommand;
        assert_eq!(cmd.name(), "undo");
    }

    #[test]
    fn test_undo_command_description() {
        let cmd = UndoCommand;
        assert!(!cmd.description().is_empty());
        assert!(cmd.description().to_lowercase().contains("undo") || cmd.description().to_lowercase().contains("revert"));
    }

    #[test]
    fn test_undo_command_usage() {
        let cmd = UndoCommand;
        assert!(cmd.usage().contains("/undo"));
    }

    #[test]
    fn test_parse_undo_args_empty() {
        let options = parse_undo_args(&[]);
        assert!(!options.hard);
        assert!(options.files.is_empty());
    }

    #[test]
    fn test_parse_undo_args_hard() {
        let options = parse_undo_args(&["--hard"]);
        assert!(options.hard);
        assert!(options.files.is_empty());

        // Short form
        let options = parse_undo_args(&["-h"]);
        assert!(options.hard);
    }

    #[test]
    fn test_parse_undo_args_files() {
        let options = parse_undo_args(&["src/main.rs", "src/lib.rs"]);
        assert!(!options.hard);
        assert_eq!(options.files.len(), 2);
        assert_eq!(options.files[0], "src/main.rs");
        assert_eq!(options.files[1], "src/lib.rs");
    }

    #[test]
    fn test_parse_undo_args_combined() {
        let options = parse_undo_args(&["--hard", "src/main.rs"]);
        assert!(options.hard);
        assert_eq!(options.files.len(), 1);
        assert_eq!(options.files[0], "src/main.rs");
    }

    #[test]
    fn test_undo_last_commit_soft() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create two commits
            create_commit(&temp_dir, &repo, "file1.txt", "content1", "First commit");
            create_commit(&temp_dir, &repo, "file2.txt", "content2", "Second commit");

            // Change to temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            // Count commits before undo
            let mut revwalk = repo.revwalk().map_err(|e| format!("Failed to create revwalk: {}", e))?;
            revwalk.push_head().map_err(|e| format!("Failed to push head: {}", e))?;
            let commits_before = revwalk.count();

            let cmd = UndoCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore directory
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Verify result
            match result {
                CommandResult::Output(output) => {
                    if !output.contains("Undid last commit") {
                        return Err(format!("Expected 'Undid last commit', got: {}", output));
                    }
                    if !output.contains("soft") {
                        return Err(format!("Expected 'soft' reset, got: {}", output));
                    }
                    if !output.contains("Second commit") {
                        return Err(format!("Expected 'Second commit' in message, got: {}", output));
                    }
                }
                CommandResult::Error(e) => return Err(format!("Got error: {}", e)),
                other => return Err(format!("Expected Output result, got: {:?}", other)),
            }

            // Verify commit was undone
            let mut revwalk = repo.revwalk().map_err(|e| format!("Failed to create revwalk: {}", e))?;
            revwalk.push_head().map_err(|e| format!("Failed to push head: {}", e))?;
            let commits_after = revwalk.count();

            if commits_after != commits_before - 1 {
                return Err(format!("Expected {} commits, got {}", commits_before - 1, commits_after));
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_undo_last_commit_hard() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create two commits
            create_commit(&temp_dir, &repo, "file1.txt", "content1", "First commit");
            create_commit(&temp_dir, &repo, "file2.txt", "content2", "Second commit");

            // Change to temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = UndoCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            let result = cmd.execute(&["--hard"], &mut ctx);

            // Restore directory
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Verify result
            match result {
                CommandResult::Output(output) => {
                    if !output.contains("hard") {
                        return Err(format!("Expected 'hard' reset, got: {}", output));
                    }
                    if !output.contains("discarded") {
                        return Err(format!("Expected 'discarded' in message, got: {}", output));
                    }
                }
                _ => return Err("Expected Output result".to_string()),
            }

            // Verify file2.txt no longer exists (hard reset removed it)
            let file2_path = temp_dir.path().join("file2.txt");
            if file2_path.exists() {
                return Err("Expected file2.txt to be removed by hard reset".to_string());
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_undo_no_commits() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, _repo) = init_test_repo();

            // No commits yet

            // Change to temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = UndoCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore directory
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Should error
            match result {
                CommandResult::Error(msg) => {
                    if !msg.contains("No commits") && !msg.contains("no HEAD") {
                        return Err(format!("Expected 'no commits' error, got: {}", msg));
                    }
                }
                _ => return Err("Expected Error result for repo with no commits".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_undo_initial_commit() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create only one commit
            create_commit(&temp_dir, &repo, "file1.txt", "content1", "Initial commit");

            // Change to temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = UndoCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore directory
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Should error (can't undo initial commit)
            match result {
                CommandResult::Error(msg) => {
                    if !msg.contains("initial commit") && !msg.contains("no parent") {
                        return Err(format!("Expected 'initial commit' error, got: {}", msg));
                    }
                }
                _ => return Err("Expected Error result for undoing initial commit".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_undo_not_in_repo() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;

            // Change to temp directory (not a git repo)
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = UndoCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore directory
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Should error
            match result {
                CommandResult::Error(msg) => {
                    if !msg.contains("git repository") {
                        return Err(format!("Expected 'git repository' error, got: {}", msg));
                    }
                }
                _ => return Err("Expected Error result for non-repo".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_undo_revert_file() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create initial commit
            create_commit(&temp_dir, &repo, "test.txt", "original content", "Initial commit");

            // Modify the file
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "modified content").map_err(|e| format!("Failed to write: {}", e))?;

            // Change to temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = UndoCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            // Revert the file with --hard
            let result = cmd.execute(&["--hard", "test.txt"], &mut ctx);

            // Restore directory
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Verify result
            match result {
                CommandResult::Output(output) => {
                    if !output.contains("Reverted") || !output.contains("test.txt") {
                        return Err(format!("Expected 'Reverted test.txt', got: {}", output));
                    }
                }
                _ => return Err("Expected Output result".to_string()),
            }

            // Verify file content was restored
            let content = fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            if content != "original content" {
                return Err(format!("Expected 'original content', got: {}", content));
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }
}

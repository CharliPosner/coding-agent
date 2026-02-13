//! The /commit command - commit changes with purpose-focused messages
//!
//! This command analyzes git changes and commits them with a message
//! focused on the purpose/intent of the changes rather than just describing
//! what was changed.

use super::{Command, CommandContext, CommandResult};
use crate::integrations::git::{FileStatus, FileStatusKind, GitError, GitRepo, RepoStatus};
use crate::ui::{FileEntry, FilePicker, FilePickerResult};
use git2::{Repository, Signature};

pub struct CommitCommand;

impl Command for CommitCommand {
    fn name(&self) -> &'static str {
        "commit"
    }

    fn description(&self) -> &'static str {
        "Commit changes with a purpose-focused message"
    }

    fn usage(&self) -> &'static str {
        "/commit [--pick] [--all] [message...]"
    }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        // Parse arguments
        let options = match parse_commit_args(args) {
            Ok(opts) => opts,
            Err(e) => return CommandResult::Error(e),
        };

        // Open the repository
        let repo = match GitRepo::open_cwd() {
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

        // Get repository status
        let status = match repo.status() {
            Ok(s) => s,
            Err(e) => {
                return CommandResult::Error(format!("Failed to get repository status: {}", e));
            }
        };

        // Check for merge conflicts
        if status.has_conflicts {
            return CommandResult::Error(
                "Repository has merge conflicts. Resolve them before committing.".to_string(),
            );
        }

        // Check for detached HEAD
        if status.detached {
            return CommandResult::Error(
                "Repository is in detached HEAD state. Create or checkout a branch first."
                    .to_string(),
            );
        }

        // Check if there are any changes to commit
        if status.is_clean() {
            return CommandResult::Output("Nothing to commit. Working tree is clean.".to_string());
        }

        // Execute the commit based on options
        if options.pick {
            return execute_pick_commit(&repo, &status, options.message.as_deref());
        }

        // Auto-commit mode: stage and commit changes
        execute_auto_commit(&repo, &status, options.stage_all, options.message.as_deref())
    }
}

/// Options parsed from commit command arguments
#[derive(Debug, Default)]
struct CommitOptions {
    /// Use interactive file picker
    pick: bool,
    /// Stage all modified files before committing
    stage_all: bool,
    /// Custom commit message (if provided)
    message: Option<String>,
}

/// Parse command arguments into CommitOptions
fn parse_commit_args(args: &[&str]) -> Result<CommitOptions, String> {
    let mut options = CommitOptions::default();
    let mut message_parts: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = args[i];
        match arg {
            "--pick" | "-p" => options.pick = true,
            "--all" | "-a" => options.stage_all = true,
            "-m" => {
                // Next arg is the message
                i += 1;
                if i < args.len() {
                    message_parts.push(args[i]);
                } else {
                    return Err("Missing message after -m flag".to_string());
                }
            }
            _ => {
                // Collect remaining args as message
                message_parts.push(arg);
            }
        }
        i += 1;
    }

    if !message_parts.is_empty() {
        options.message = Some(message_parts.join(" "));
    }

    Ok(options)
}

/// Execute an automatic commit
fn execute_auto_commit(
    git_repo: &GitRepo,
    status: &RepoStatus,
    stage_all: bool,
    custom_message: Option<&str>,
) -> CommandResult {
    let repo_root = match git_repo.root() {
        Some(r) => r,
        None => return CommandResult::Error("Could not determine repository root.".to_string()),
    };

    // Open the raw git2 repository for operations
    let repo = match Repository::open(repo_root) {
        Ok(r) => r,
        Err(e) => return CommandResult::Error(format!("Failed to open repository: {}", e)),
    };

    // Determine which files to commit
    let files_to_stage: Vec<_> = if stage_all {
        // Stage all modified and untracked files
        status
            .files
            .iter()
            .filter(|f| {
                matches!(
                    f.status,
                    FileStatusKind::Modified
                        | FileStatusKind::Untracked
                        | FileStatusKind::Deleted
                        | FileStatusKind::Staged
                        | FileStatusKind::StagedWithChanges
                        | FileStatusKind::Added
                )
            })
            .collect()
    } else {
        // Only commit already staged files plus modified files
        status
            .files
            .iter()
            .filter(|f| {
                matches!(
                    f.status,
                    FileStatusKind::Modified
                        | FileStatusKind::Staged
                        | FileStatusKind::StagedWithChanges
                        | FileStatusKind::Added
                        | FileStatusKind::Deleted
                )
            })
            .collect()
    };

    if files_to_stage.is_empty() {
        return CommandResult::Output("No changes to commit.".to_string());
    }

    // Stage files
    let mut index = match repo.index() {
        Ok(i) => i,
        Err(e) => return CommandResult::Error(format!("Failed to get index: {}", e)),
    };

    for file in &files_to_stage {
        let path = &file.path;
        match file.status {
            FileStatusKind::Deleted => {
                if let Err(e) = index.remove_path(path) {
                    return CommandResult::Error(format!("Failed to stage deletion of {:?}: {}", path, e));
                }
            }
            FileStatusKind::Untracked | FileStatusKind::Modified | FileStatusKind::StagedWithChanges => {
                if let Err(e) = index.add_path(path) {
                    return CommandResult::Error(format!("Failed to stage {:?}: {}", path, e));
                }
            }
            // Already staged files don't need to be re-added
            FileStatusKind::Staged | FileStatusKind::Added => {}
            _ => {}
        }
    }

    if let Err(e) = index.write() {
        return CommandResult::Error(format!("Failed to write index: {}", e));
    }

    // Generate commit message
    let message = match custom_message {
        Some(msg) => msg.to_string(),
        None => generate_commit_message(&files_to_stage, status),
    };

    // Create the commit
    let tree_id = match index.write_tree() {
        Ok(id) => id,
        Err(e) => return CommandResult::Error(format!("Failed to write tree: {}", e)),
    };

    let tree = match repo.find_tree(tree_id) {
        Ok(t) => t,
        Err(e) => return CommandResult::Error(format!("Failed to find tree: {}", e)),
    };

    let signature = match repo.signature() {
        Ok(s) => s,
        Err(_) => {
            // Try to create a default signature if git config doesn't have user info
            match Signature::now("coding-agent", "coding-agent@local") {
                Ok(s) => s,
                Err(e) => return CommandResult::Error(format!("Failed to create signature: {}. Configure git with 'git config user.name' and 'git config user.email'.", e)),
            }
        }
    };

    // Get parent commit (if any)
    let parent = match repo.head() {
        Ok(head) => {
            match head.peel_to_commit() {
                Ok(commit) => Some(commit),
                Err(_) => None,
            }
        }
        Err(_) => None, // Initial commit
    };

    let parents: Vec<_> = parent.iter().collect();

    match repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &message,
        &tree,
        &parents,
    ) {
        Ok(oid) => {
            let short_id = &oid.to_string()[..7];
            let file_count = files_to_stage.len();
            let file_word = if file_count == 1 { "file" } else { "files" };

            // Build output
            let mut output = String::new();
            output.push_str(&format!("✓ Committed {} {} [{}]\n\n", file_count, file_word, short_id));
            output.push_str(&format!("{}\n", message));

            // List committed files
            output.push_str("\nFiles committed:\n");
            for file in files_to_stage {
                output.push_str(&format!("  {} {}\n", file.status.indicator(), file.path.display()));
            }

            CommandResult::Output(output)
        }
        Err(e) => CommandResult::Error(format!("Failed to create commit: {}", e)),
    }
}

/// Execute an interactive pick commit
fn execute_pick_commit(
    git_repo: &GitRepo,
    status: &RepoStatus,
    custom_message: Option<&str>,
) -> CommandResult {
    // Build list of file entries for the picker
    let entries: Vec<FileEntry> = status
        .files
        .iter()
        .filter(|f| {
            matches!(
                f.status,
                FileStatusKind::Modified
                    | FileStatusKind::Untracked
                    | FileStatusKind::Deleted
                    | FileStatusKind::Staged
                    | FileStatusKind::StagedWithChanges
                    | FileStatusKind::Added
            )
        })
        .map(|f| {
            // Pre-select already staged files
            let selected = matches!(
                f.status,
                FileStatusKind::Staged | FileStatusKind::StagedWithChanges | FileStatusKind::Added
            );
            FileEntry {
                path: f.path.to_string_lossy().to_string(),
                status: f.status.indicator().to_string(),
                selected,
            }
        })
        .collect();

    if entries.is_empty() {
        return CommandResult::Output("No changes to commit.".to_string());
    }

    // Run the interactive picker
    let mut picker = FilePicker::new(entries);
    let selected_paths = match picker.run() {
        Ok(FilePickerResult::Selected(paths)) => paths,
        Ok(FilePickerResult::Cancelled) => {
            return CommandResult::Output("Commit cancelled.".to_string());
        }
        Err(e) => {
            return CommandResult::Error(format!("File picker error: {}", e));
        }
    };

    if selected_paths.is_empty() {
        return CommandResult::Output("No files selected. Commit cancelled.".to_string());
    }

    // Now commit the selected files
    let repo_root = match git_repo.root() {
        Some(r) => r,
        None => return CommandResult::Error("Could not determine repository root.".to_string()),
    };

    let repo = match Repository::open(repo_root) {
        Ok(r) => r,
        Err(e) => return CommandResult::Error(format!("Failed to open repository: {}", e)),
    };

    // Get the selected files from status
    let files_to_commit: Vec<&FileStatus> = status
        .files
        .iter()
        .filter(|f| selected_paths.contains(&f.path.to_string_lossy().to_string()))
        .collect();

    // Stage the selected files
    let mut index = match repo.index() {
        Ok(i) => i,
        Err(e) => return CommandResult::Error(format!("Failed to get index: {}", e)),
    };

    for file in &files_to_commit {
        let path = &file.path;
        match file.status {
            FileStatusKind::Deleted => {
                if let Err(e) = index.remove_path(path) {
                    return CommandResult::Error(format!(
                        "Failed to stage deletion of {:?}: {}",
                        path, e
                    ));
                }
            }
            FileStatusKind::Untracked
            | FileStatusKind::Modified
            | FileStatusKind::StagedWithChanges => {
                if let Err(e) = index.add_path(path) {
                    return CommandResult::Error(format!("Failed to stage {:?}: {}", path, e));
                }
            }
            // Already staged files don't need to be re-added
            FileStatusKind::Staged | FileStatusKind::Added => {}
            _ => {}
        }
    }

    if let Err(e) = index.write() {
        return CommandResult::Error(format!("Failed to write index: {}", e));
    }

    // Generate commit message
    let message = match custom_message {
        Some(msg) => msg.to_string(),
        None => generate_commit_message(&files_to_commit, status),
    };

    // Create the commit
    let tree_id = match index.write_tree() {
        Ok(id) => id,
        Err(e) => return CommandResult::Error(format!("Failed to write tree: {}", e)),
    };

    let tree = match repo.find_tree(tree_id) {
        Ok(t) => t,
        Err(e) => return CommandResult::Error(format!("Failed to find tree: {}", e)),
    };

    let signature = match repo.signature() {
        Ok(s) => s,
        Err(_) => match Signature::now("coding-agent", "coding-agent@local") {
            Ok(s) => s,
            Err(e) => {
                return CommandResult::Error(format!(
                    "Failed to create signature: {}. Configure git with 'git config user.name' and 'git config user.email'.",
                    e
                ))
            }
        },
    };

    let parent = match repo.head() {
        Ok(head) => match head.peel_to_commit() {
            Ok(commit) => Some(commit),
            Err(_) => None,
        },
        Err(_) => None,
    };

    let parents: Vec<_> = parent.iter().collect();

    match repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &message,
        &tree,
        &parents,
    ) {
        Ok(oid) => {
            let short_id = &oid.to_string()[..7];
            let file_count = files_to_commit.len();
            let file_word = if file_count == 1 { "file" } else { "files" };

            let mut output = String::new();
            output.push_str(&format!(
                "✓ Committed {} {} [{}]\n\n",
                file_count, file_word, short_id
            ));
            output.push_str(&format!("{}\n", message));

            output.push_str("\nFiles committed:\n");
            for file in files_to_commit {
                output.push_str(&format!(
                    "  {} {}\n",
                    file.status.indicator(),
                    file.path.display()
                ));
            }

            CommandResult::Output(output)
        }
        Err(e) => CommandResult::Error(format!("Failed to create commit: {}", e)),
    }
}

/// Generate a purpose-focused commit message based on the changes
fn generate_commit_message(
    files: &[&crate::integrations::git::FileStatus],
    _status: &RepoStatus,
) -> String {
    // Analyze the files to determine the nature of the change
    let mut has_tests = false;
    let mut has_src = false;
    let mut has_config = false;
    let mut has_docs = false;
    let mut directories: std::collections::HashSet<String> = std::collections::HashSet::new();

    for file in files {
        let path_str = file.path.to_string_lossy();

        // Track what types of files are being changed
        if path_str.contains("test") || path_str.contains("spec") {
            has_tests = true;
        }
        if path_str.contains("src/") || path_str.ends_with(".rs") {
            has_src = true;
        }
        if path_str.contains("config") || path_str.ends_with(".toml") || path_str.ends_with(".json") {
            has_config = true;
        }
        if path_str.ends_with(".md") || path_str.contains("docs/") {
            has_docs = true;
        }

        // Extract the directory
        if let Some(parent) = file.path.parent() {
            if let Some(dir_name) = parent.file_name() {
                directories.insert(dir_name.to_string_lossy().to_string());
            }
        }
    }

    // Count changes by type
    let new_files: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.status, FileStatusKind::Added | FileStatusKind::Untracked))
        .collect();
    let modified_files: Vec<_> = files
        .iter()
        .filter(|f| {
            matches!(
                f.status,
                FileStatusKind::Modified | FileStatusKind::Staged | FileStatusKind::StagedWithChanges
            )
        })
        .collect();
    let deleted_files: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.status, FileStatusKind::Deleted))
        .collect();

    // Generate title based on the primary action
    let title = if !new_files.is_empty() && modified_files.is_empty() && deleted_files.is_empty() {
        if has_tests {
            format!("Add tests for {}", summarize_scope(&directories, files))
        } else if has_docs {
            format!("Add documentation for {}", summarize_scope(&directories, files))
        } else {
            format!("Add {}", summarize_scope(&directories, files))
        }
    } else if !deleted_files.is_empty() && new_files.is_empty() && modified_files.is_empty() {
        format!("Remove {}", summarize_scope(&directories, files))
    } else if !modified_files.is_empty() {
        if has_tests && !has_src {
            format!("Update tests for {}", summarize_scope(&directories, files))
        } else if has_config && !has_src {
            format!("Update configuration for {}", summarize_scope(&directories, files))
        } else {
            format!("Update {}", summarize_scope(&directories, files))
        }
    } else {
        "Update codebase".to_string()
    };

    // Generate description
    let mut description = String::new();

    if !new_files.is_empty() {
        description.push_str(&format!(
            "Added {} new {}. ",
            new_files.len(),
            if new_files.len() == 1 { "file" } else { "files" }
        ));
    }
    if !modified_files.is_empty() {
        description.push_str(&format!(
            "Modified {} {}. ",
            modified_files.len(),
            if modified_files.len() == 1 { "file" } else { "files" }
        ));
    }
    if !deleted_files.is_empty() {
        description.push_str(&format!(
            "Removed {} {}.",
            deleted_files.len(),
            if deleted_files.len() == 1 { "file" } else { "files" }
        ));
    }

    format!("{}\n\n{}", title.trim(), description.trim())
}

/// Summarize the scope of changes based on directories and files
fn summarize_scope(
    directories: &std::collections::HashSet<String>,
    files: &[&crate::integrations::git::FileStatus],
) -> String {
    // If all files are in one directory, use that
    if directories.len() == 1 {
        if let Some(dir) = directories.iter().next() {
            if !dir.is_empty() && dir != "." {
                return format!("{} module", dir);
            }
        }
    }

    // If there's just one file, use its name
    if files.len() == 1 {
        if let Some(file_name) = files[0].path.file_name() {
            return file_name.to_string_lossy().to_string();
        }
    }

    // Otherwise, try to find a common pattern
    if directories.len() <= 3 {
        let dir_list: Vec<_> = directories.iter().take(3).cloned().collect();
        if !dir_list.is_empty() && dir_list.iter().all(|d| !d.is_empty()) {
            return dir_list.join(", ");
        }
    }

    // Fallback to generic description
    format!("{} files", files.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::integrations::git::FileStatus;
    use crate::tokens::CostTracker;
    use std::fs;
    use std::path::{Path, PathBuf};
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
    fn test_commit_command_name() {
        let cmd = CommitCommand;
        assert_eq!(cmd.name(), "commit");
    }

    #[test]
    fn test_commit_command_description() {
        let cmd = CommitCommand;
        assert!(!cmd.description().is_empty());
        assert!(cmd.description().contains("commit") || cmd.description().contains("Commit"));
    }

    #[test]
    fn test_commit_command_usage() {
        let cmd = CommitCommand;
        assert!(cmd.usage().contains("/commit"));
    }

    #[test]
    fn test_parse_commit_args_empty() {
        let result = parse_commit_args(&[]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(!options.pick);
        assert!(!options.stage_all);
        assert!(options.message.is_none());
    }

    #[test]
    fn test_parse_commit_args_pick() {
        let result = parse_commit_args(&["--pick"]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(options.pick);

        // Short form
        let result = parse_commit_args(&["-p"]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(options.pick);
    }

    #[test]
    fn test_parse_commit_args_all() {
        let result = parse_commit_args(&["--all"]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(options.stage_all);

        // Short form
        let result = parse_commit_args(&["-a"]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(options.stage_all);
    }

    #[test]
    fn test_parse_commit_args_message() {
        let result = parse_commit_args(&["-m", "test message"]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert_eq!(options.message, Some("test message".to_string()));
    }

    #[test]
    fn test_parse_commit_args_message_without_value() {
        let result = parse_commit_args(&["-m"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing message"));
    }

    #[test]
    fn test_parse_commit_args_combined() {
        let result = parse_commit_args(&["-a", "-m", "fix bug"]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(options.stage_all);
        assert_eq!(options.message, Some("fix bug".to_string()));
    }

    #[test]
    fn test_commit_message_format_basic() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/main.rs"),
            status: FileStatusKind::Modified,
        }];
        let file_refs: Vec<_> = files.iter().collect();
        let status = RepoStatus {
            branch: Some("main".to_string()),
            detached: false,
            has_conflicts: false,
            files: files.clone(),
        };

        let message = generate_commit_message(&file_refs, &status);

        // Should have a title and description
        assert!(message.contains("\n\n"));
        // Should mention the file count
        assert!(message.contains("1 file"));
    }

    #[test]
    fn test_commit_message_format_new_files() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/new_module.rs"),
            status: FileStatusKind::Added,
        }];
        let file_refs: Vec<_> = files.iter().collect();
        let status = RepoStatus {
            branch: Some("main".to_string()),
            detached: false,
            has_conflicts: false,
            files: files.clone(),
        };

        let message = generate_commit_message(&file_refs, &status);

        // Should indicate adding
        assert!(message.contains("Add"));
    }

    #[test]
    fn test_commit_message_format_tests() {
        let files = vec![FileStatus {
            path: PathBuf::from("tests/integration_test.rs"),
            status: FileStatusKind::Added,
        }];
        let file_refs: Vec<_> = files.iter().collect();
        let status = RepoStatus {
            branch: Some("main".to_string()),
            detached: false,
            has_conflicts: false,
            files: files.clone(),
        };

        let message = generate_commit_message(&file_refs, &status);

        // Should mention tests
        assert!(message.to_lowercase().contains("test"));
    }

    #[test]
    fn test_commit_creates_commit() {
        // Save original dir at the start - we'll restore it at the end
        let original_dir = std::env::current_dir().expect("Failed to get cwd");

        // Use a closure to ensure cleanup happens
        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create initial commit
            let file_path = temp_dir.path().join("initial.txt");
            fs::write(&file_path, "initial").map_err(|e| format!("Failed to write file: {}", e))?;

            let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
            index
                .add_path(Path::new("initial.txt"))
                .map_err(|e| format!("Failed to add file: {}", e))?;
            index.write().map_err(|e| format!("Failed to write index: {}", e))?;

            let tree_id = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
            let tree = repo.find_tree(tree_id).map_err(|e| format!("Failed to find tree: {}", e))?;
            let sig = repo.signature().map_err(|e| format!("Failed to get signature: {}", e))?;

            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .map_err(|e| format!("Failed to commit: {}", e))?;

            // Create a new file to commit
            let new_file = temp_dir.path().join("new_file.txt");
            fs::write(&new_file, "new content").map_err(|e| format!("Failed to write file: {}", e))?;

            // Change to the temp directory and run commit
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = CommitCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
            };

            // Run the commit with -a flag to stage the new file
            let result = cmd.execute(&["-a", "-m", "Add new file"], &mut ctx);

            // Restore original directory BEFORE checking results or dropping temp_dir
            std::env::set_current_dir(&original_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Check result
            match result {
                CommandResult::Output(output) => {
                    if !output.contains("Committed") {
                        return Err(format!("Expected 'Committed' in output: {}", output));
                    }
                    if !output.contains("new_file.txt") {
                        return Err(format!("Expected 'new_file.txt' in output: {}", output));
                    }
                }
                CommandResult::Error(e) => return Err(format!("Commit failed: {}", e)),
                _ => return Err("Unexpected result type".to_string()),
            }

            // Verify commit was created
            let head = repo.head().map_err(|e| format!("Failed to get HEAD: {}", e))?;
            let commit = head.peel_to_commit().map_err(|e| format!("Failed to get commit: {}", e))?;
            if commit.message().unwrap() != "Add new file" {
                return Err(format!("Wrong commit message: {:?}", commit.message()));
            }

            Ok(())
        })();

        // Always restore the original directory
        let _ = std::env::set_current_dir(&original_dir);

        // Now check if the test passed
        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_commit_no_changes() {
        let original_dir = std::env::current_dir().expect("Failed to get cwd");

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create initial commit
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "content").map_err(|e| format!("Failed to write file: {}", e))?;

            let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
            index
                .add_path(Path::new("test.txt"))
                .map_err(|e| format!("Failed to add file: {}", e))?;
            index.write().map_err(|e| format!("Failed to write index: {}", e))?;

            let tree_id = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
            let tree = repo.find_tree(tree_id).map_err(|e| format!("Failed to find tree: {}", e))?;
            let sig = repo.signature().map_err(|e| format!("Failed to get signature: {}", e))?;

            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .map_err(|e| format!("Failed to commit: {}", e))?;

            // Change to the temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = CommitCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore original directory BEFORE temp_dir is dropped
            std::env::set_current_dir(&original_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Should report nothing to commit
            match result {
                CommandResult::Output(output) => {
                    if !output.to_lowercase().contains("nothing to commit")
                        && !output.to_lowercase().contains("no changes")
                        && !output.to_lowercase().contains("clean") {
                        return Err(format!("Expected 'nothing to commit' message, got: {}", output));
                    }
                }
                _ => return Err("Expected Output result for clean working tree".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&original_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_commit_not_in_repo() {
        let original_dir = std::env::current_dir().expect("Failed to get cwd");

        let test_result: Result<(), String> = (|| {
            let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;

            // Change to the temp directory (not a git repo)
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = CommitCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore original directory BEFORE temp_dir is dropped
            std::env::set_current_dir(&original_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Should report not a repository
            match result {
                CommandResult::Error(msg) => {
                    if !msg.contains("git repository") {
                        return Err(format!("Expected git repo error: {}", msg));
                    }
                }
                _ => return Err("Expected Error result for non-repo".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&original_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_summarize_scope_single_file() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/main.rs"),
            status: FileStatusKind::Modified,
        }];
        let file_refs: Vec<_> = files.iter().collect();
        let mut directories = std::collections::HashSet::new();
        directories.insert("src".to_string());

        let scope = summarize_scope(&directories, &file_refs);
        assert!(scope.contains("main.rs") || scope.contains("src"));
    }

    #[test]
    fn test_summarize_scope_multiple_files() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/a.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/b.rs"),
                status: FileStatusKind::Modified,
            },
        ];
        let file_refs: Vec<_> = files.iter().collect();
        let mut directories = std::collections::HashSet::new();
        directories.insert("src".to_string());

        let scope = summarize_scope(&directories, &file_refs);
        // Should reference the module or file count
        assert!(scope.contains("src") || scope.contains("2"));
    }

    #[test]
    fn test_file_grouping_same_dir() {
        // Files in the same directory should be recognized as related
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/auth/login.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/auth/logout.rs"),
                status: FileStatusKind::Modified,
            },
        ];
        let file_refs: Vec<_> = files.iter().collect();
        let status = RepoStatus {
            branch: Some("main".to_string()),
            detached: false,
            has_conflicts: false,
            files: files.clone(),
        };

        let message = generate_commit_message(&file_refs, &status);
        // The message should reference auth in some way
        assert!(message.contains("auth") || message.contains("2 files"));
    }

    #[test]
    fn test_file_grouping_test_and_impl() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/module.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("tests/module_test.rs"),
                status: FileStatusKind::Modified,
            },
        ];
        let file_refs: Vec<_> = files.iter().collect();
        let status = RepoStatus {
            branch: Some("main".to_string()),
            detached: false,
            has_conflicts: false,
            files: files.clone(),
        };

        let message = generate_commit_message(&file_refs, &status);
        // Should recognize we have both source and test files
        assert!(message.contains("Update") || message.contains("Modified"));
    }

    #[test]
    fn test_pick_mode_file_entry_construction() {
        // Test that file entries are correctly constructed from git status
        let status = RepoStatus {
            branch: Some("main".to_string()),
            detached: false,
            has_conflicts: false,
            files: vec![
                FileStatus {
                    path: PathBuf::from("src/modified.rs"),
                    status: FileStatusKind::Modified,
                },
                FileStatus {
                    path: PathBuf::from("src/staged.rs"),
                    status: FileStatusKind::Staged,
                },
                FileStatus {
                    path: PathBuf::from("src/new.rs"),
                    status: FileStatusKind::Untracked,
                },
                FileStatus {
                    path: PathBuf::from("src/added.rs"),
                    status: FileStatusKind::Added,
                },
            ],
        };

        // Build entries the same way execute_pick_commit does
        let entries: Vec<FileEntry> = status
            .files
            .iter()
            .filter(|f| {
                matches!(
                    f.status,
                    FileStatusKind::Modified
                        | FileStatusKind::Untracked
                        | FileStatusKind::Deleted
                        | FileStatusKind::Staged
                        | FileStatusKind::StagedWithChanges
                        | FileStatusKind::Added
                )
            })
            .map(|f| {
                let selected = matches!(
                    f.status,
                    FileStatusKind::Staged | FileStatusKind::StagedWithChanges | FileStatusKind::Added
                );
                FileEntry {
                    path: f.path.to_string_lossy().to_string(),
                    status: f.status.indicator().to_string(),
                    selected,
                }
            })
            .collect();

        assert_eq!(entries.len(), 4);

        // Modified file should not be pre-selected
        assert!(!entries[0].selected);
        assert_eq!(entries[0].path, "src/modified.rs");

        // Staged file should be pre-selected
        assert!(entries[1].selected);
        assert_eq!(entries[1].path, "src/staged.rs");

        // Untracked file should not be pre-selected
        assert!(!entries[2].selected);
        assert_eq!(entries[2].path, "src/new.rs");

        // Added file should be pre-selected
        assert!(entries[3].selected);
        assert_eq!(entries[3].path, "src/added.rs");
    }

    #[test]
    fn test_pick_mode_empty_status() {
        // When there are no changes, execute_pick_commit should return early
        // This test uses GitRepo::open() with explicit path instead of relying on cwd

        let (temp_dir, repo) = init_test_repo();

        // Create initial commit
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(Path::new("test.txt")).expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to commit");

        // Open repo using explicit path (not cwd)
        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let status = git_repo.status().expect("Failed to get status");

        // Verify status is clean
        assert!(status.is_clean(), "Repo should be clean after commit");

        // With a clean repo, execute_pick_commit should return "No changes"
        let result = execute_pick_commit(&git_repo, &status, None);

        match result {
            CommandResult::Output(output) => {
                assert!(
                    output.contains("No changes"),
                    "Expected 'No changes' message, got: {}",
                    output
                );
            }
            other => panic!("Expected Output result for clean repo, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_commit_args_pick_with_message() {
        // Test parsing --pick with a custom message
        let result = parse_commit_args(&["--pick", "-m", "custom message"]);
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(options.pick);
        assert_eq!(options.message, Some("custom message".to_string()));
    }
}

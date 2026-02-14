//! The /diff command - shows pending/recent changes
//!
//! This command displays both staged and unstaged changes in the repository,
//! similar to running `git diff` and `git diff --cached` together.

use super::{Command, CommandContext, CommandResult};
use crate::integrations::git::{GitError, GitRepo};
use git2::{DiffOptions, Repository};

pub struct DiffCommand;

impl Command for DiffCommand {
    fn name(&self) -> &'static str {
        "diff"
    }

    fn description(&self) -> &'static str {
        "Show pending/recent changes (staged and unstaged)"
    }

    fn usage(&self) -> &'static str {
        "/diff [--staged] [--unstaged] [file...]"
    }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        // Parse arguments
        let options = parse_diff_args(args);

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
            None => {
                return CommandResult::Error("Could not determine repository root.".to_string())
            }
        };

        let repo = match Repository::open(repo_root) {
            Ok(r) => r,
            Err(e) => return CommandResult::Error(format!("Failed to open repository: {}", e)),
        };

        // Get status to determine if there are changes
        let status = match git_repo.status() {
            Ok(s) => s,
            Err(e) => return CommandResult::Error(format!("Failed to get status: {}", e)),
        };

        if status.is_clean() {
            return CommandResult::Output("No changes. Working tree is clean.".to_string());
        }

        // Build the diff output
        let mut output = String::new();

        // Show staged changes if requested (or if showing both)
        if options.show_staged {
            match get_staged_diff(&repo, &options.files) {
                Ok(diff) if !diff.is_empty() => {
                    output.push_str("Staged changes (to be committed):\n");
                    output.push_str("────────────────────────────────────────────────\n");
                    output.push_str(&diff);
                    output.push('\n');
                }
                Ok(_) => {
                    if options.staged_only {
                        output.push_str("No staged changes.\n");
                    }
                }
                Err(e) => {
                    output.push_str(&format!("Error getting staged diff: {}\n", e));
                }
            }
        }

        // Show unstaged changes if requested (or if showing both)
        if options.show_unstaged {
            match get_unstaged_diff(&repo, &options.files) {
                Ok(diff) if !diff.is_empty() => {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str("Unstaged changes (not staged for commit):\n");
                    output.push_str("────────────────────────────────────────────────\n");
                    output.push_str(&diff);
                }
                Ok(_) => {
                    if options.unstaged_only {
                        output.push_str("No unstaged changes.\n");
                    }
                }
                Err(e) => {
                    output.push_str(&format!("Error getting unstaged diff: {}\n", e));
                }
            }
        }

        // Show untracked files if present and showing all
        if !options.staged_only && !options.unstaged_only {
            let untracked = status.untracked_files();
            if !untracked.is_empty() {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str("Untracked files:\n");
                output.push_str("────────────────────────────────────────────────\n");
                for file in untracked {
                    output.push_str(&format!("  ?? {}\n", file.path.display()));
                }
            }
        }

        if output.is_empty() {
            output = "No changes to display.".to_string();
        }

        CommandResult::Output(output)
    }
}

/// Options for the diff command
#[derive(Debug, Default)]
struct DiffOptions2 {
    /// Show only staged changes
    staged_only: bool,
    /// Show only unstaged changes
    unstaged_only: bool,
    /// Show staged changes (true if not filtering to unstaged only)
    show_staged: bool,
    /// Show unstaged changes (true if not filtering to staged only)
    show_unstaged: bool,
    /// Specific files to diff
    files: Vec<String>,
}

/// Parse command arguments
fn parse_diff_args(args: &[&str]) -> DiffOptions2 {
    let mut options = DiffOptions2::default();
    let mut files = Vec::new();

    for arg in args {
        match *arg {
            "--staged" | "--cached" | "-s" => {
                options.staged_only = true;
            }
            "--unstaged" | "-u" => {
                options.unstaged_only = true;
            }
            _ => {
                // Treat as file path
                files.push(arg.to_string());
            }
        }
    }

    options.files = files;

    // Determine what to show
    if options.staged_only && !options.unstaged_only {
        options.show_staged = true;
        options.show_unstaged = false;
    } else if options.unstaged_only && !options.staged_only {
        options.show_staged = false;
        options.show_unstaged = true;
    } else {
        // Show both by default
        options.show_staged = true;
        options.show_unstaged = true;
    }

    options
}

/// Get the diff of staged changes (changes between HEAD and index)
fn get_staged_diff(repo: &Repository, files: &[String]) -> Result<String, String> {
    let head = match repo.head() {
        Ok(h) => match h.peel_to_tree() {
            Ok(t) => Some(t),
            Err(_) => None,
        },
        Err(_) => None, // No HEAD yet (initial commit)
    };

    let mut opts = DiffOptions::new();
    opts.include_untracked(false);

    // Add file filters if specified
    for file in files {
        opts.pathspec(file);
    }

    let diff = repo
        .diff_tree_to_index(head.as_ref(), None, Some(&mut opts))
        .map_err(|e| format!("Failed to get staged diff: {}", e))?;

    format_diff(&diff)
}

/// Get the diff of unstaged changes (changes between index and working directory)
fn get_unstaged_diff(repo: &Repository, files: &[String]) -> Result<String, String> {
    let mut opts = DiffOptions::new();
    opts.include_untracked(false);

    // Add file filters if specified
    for file in files {
        opts.pathspec(file);
    }

    let diff = repo
        .diff_index_to_workdir(None, Some(&mut opts))
        .map_err(|e| format!("Failed to get unstaged diff: {}", e))?;

    format_diff(&diff)
}

/// Format a git2 Diff into a readable string
fn format_diff(diff: &git2::Diff) -> Result<String, String> {
    let mut output = String::new();
    let stats = diff
        .stats()
        .map_err(|e| format!("Failed to get diff stats: {}", e))?;

    // If no changes, return empty
    if stats.files_changed() == 0 {
        return Ok(String::new());
    }

    // Format each delta (file change)
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        // Get file path
        if let Some(new_file) = delta.new_file().path() {
            // Only print file header once per file
            if line.origin() == 'F' {
                // File header
                let old_path = delta
                    .old_file()
                    .path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "/dev/null".to_string());
                let new_path = new_file.display().to_string();

                if old_path != new_path {
                    output.push_str(&format!("diff {} -> {}\n", old_path, new_path));
                } else {
                    output.push_str(&format!("diff {}\n", new_path));
                }
            }
        }

        // Format the line based on its origin
        let prefix = match line.origin() {
            '+' => "+",
            '-' => "-",
            ' ' => " ",
            'H' => "", // Hunk header
            'F' => "", // File header (handled above)
            'B' => "", // Binary
            _ => "",
        };

        if !prefix.is_empty() || line.origin() == 'H' {
            let content = std::str::from_utf8(line.content()).unwrap_or("<binary>");
            if line.origin() == 'H' {
                output.push_str(&format!("{}", content));
            } else {
                output.push_str(&format!("{}{}", prefix, content));
            }
        }

        true
    })
    .map_err(|e| format!("Failed to format diff: {}", e))?;

    // Add summary
    output.push_str(&format!(
        "\n{} file(s) changed, {} insertion(s), {} deletion(s)\n",
        stats.files_changed(),
        stats.insertions(),
        stats.deletions()
    ));

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::CostTracker;
    use std::fs;
    use std::path::Path;
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

    fn create_initial_commit(temp_dir: &TempDir, repo: &Repository) {
        let file_path = temp_dir.path().join("README.md");
        fs::write(&file_path, "# Test\n").expect("Failed to write file");

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
    }

    #[test]
    fn test_diff_command_name() {
        let cmd = DiffCommand;
        assert_eq!(cmd.name(), "diff");
    }

    #[test]
    fn test_diff_command_description() {
        let cmd = DiffCommand;
        assert!(!cmd.description().is_empty());
        assert!(cmd.description().to_lowercase().contains("change"));
    }

    #[test]
    fn test_diff_command_usage() {
        let cmd = DiffCommand;
        assert!(cmd.usage().contains("/diff"));
    }

    #[test]
    fn test_parse_diff_args_empty() {
        let options = parse_diff_args(&[]);
        assert!(!options.staged_only);
        assert!(!options.unstaged_only);
        assert!(options.show_staged);
        assert!(options.show_unstaged);
        assert!(options.files.is_empty());
    }

    #[test]
    fn test_parse_diff_args_staged() {
        let options = parse_diff_args(&["--staged"]);
        assert!(options.staged_only);
        assert!(options.show_staged);
        assert!(!options.show_unstaged);

        // Test short form
        let options = parse_diff_args(&["-s"]);
        assert!(options.staged_only);

        // Test --cached alias
        let options = parse_diff_args(&["--cached"]);
        assert!(options.staged_only);
    }

    #[test]
    fn test_parse_diff_args_unstaged() {
        let options = parse_diff_args(&["--unstaged"]);
        assert!(options.unstaged_only);
        assert!(!options.show_staged);
        assert!(options.show_unstaged);

        // Test short form
        let options = parse_diff_args(&["-u"]);
        assert!(options.unstaged_only);
    }

    #[test]
    fn test_parse_diff_args_files() {
        let options = parse_diff_args(&["src/main.rs", "src/lib.rs"]);
        assert_eq!(options.files.len(), 2);
        assert_eq!(options.files[0], "src/main.rs");
        assert_eq!(options.files[1], "src/lib.rs");
    }

    #[test]
    fn test_parse_diff_args_combined() {
        let options = parse_diff_args(&["--staged", "src/main.rs"]);
        assert!(options.staged_only);
        assert_eq!(options.files.len(), 1);
        assert_eq!(options.files[0], "src/main.rs");
    }

    #[test]
    fn test_diff_shows_changes() {
        let (temp_dir, repo) = init_test_repo();
        create_initial_commit(&temp_dir, &repo);

        // Modify the file (unstaged change)
        let file_path = temp_dir.path().join("README.md");
        fs::write(&file_path, "# Test\n\nNew content here.\n").expect("Failed to write");

        // Test getting unstaged diff directly
        let diff = get_unstaged_diff(&repo, &[]).expect("Failed to get unstaged diff");

        // Should contain the new content
        assert!(
            diff.contains("New content"),
            "Expected diff to contain 'New content', got: {}",
            diff
        );
        assert!(
            diff.contains("+"),
            "Expected diff to contain additions, got: {}",
            diff
        );
    }

    #[test]
    fn test_diff_shows_staged_changes() {
        let (temp_dir, repo) = init_test_repo();
        create_initial_commit(&temp_dir, &repo);

        // Modify and stage the file
        let file_path = temp_dir.path().join("README.md");
        fs::write(&file_path, "# Test\n\nStaged content.\n").expect("Failed to write");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("README.md"))
            .expect("Failed to stage");
        index.write().expect("Failed to write index");

        // Test getting staged diff directly
        let diff = get_staged_diff(&repo, &[]).expect("Failed to get staged diff");

        // Should contain the staged content
        assert!(
            diff.contains("Staged content"),
            "Expected diff to contain 'Staged content', got: {}",
            diff
        );
        assert!(
            diff.contains("+"),
            "Expected diff to contain additions, got: {}",
            diff
        );
    }

    #[test]
    fn test_diff_clean_repo() {
        let (temp_dir, repo) = init_test_repo();
        create_initial_commit(&temp_dir, &repo);

        // Test getting diffs on a clean repo
        let staged_diff = get_staged_diff(&repo, &[]).expect("Failed to get staged diff");
        let unstaged_diff = get_unstaged_diff(&repo, &[]).expect("Failed to get unstaged diff");

        // Both should be empty for a clean repo
        assert!(
            staged_diff.is_empty(),
            "Expected empty staged diff for clean repo, got: {}",
            staged_diff
        );
        assert!(
            unstaged_diff.is_empty(),
            "Expected empty unstaged diff for clean repo, got: {}",
            unstaged_diff
        );
    }

    #[test]
    fn test_diff_not_in_repo() {
        // Use manifest directory as a stable restore point (won't be deleted by other tests)
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let temp_dir =
                TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;

            // Change to temp directory (not a git repo)
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = DiffCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore to manifest directory (guaranteed to exist)
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

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

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_diff_shows_untracked_files() {
        // Use manifest directory as a stable restore point (won't be deleted by other tests)
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();
            create_initial_commit(&temp_dir, &repo);

            // Create an untracked file
            let new_file = temp_dir.path().join("new_file.txt");
            fs::write(&new_file, "new content").map_err(|e| format!("Failed to write: {}", e))?;

            // Change to temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = DiffCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore to manifest directory (guaranteed to exist)
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            match result {
                CommandResult::Output(output) => {
                    // Should show untracked files
                    if !output.contains("Untracked files") || !output.contains("new_file.txt") {
                        return Err(format!(
                            "Expected untracked files in output, got: {}",
                            output
                        ));
                    }
                }
                CommandResult::Error(e) => return Err(format!("Diff failed: {}", e)),
                _ => return Err("Unexpected result type".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_diff_staged_only_no_untracked() {
        // Use manifest directory as a stable restore point (won't be deleted by other tests)
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();
            create_initial_commit(&temp_dir, &repo);

            // Create an untracked file
            let new_file = temp_dir.path().join("new_file.txt");
            fs::write(&new_file, "new content").map_err(|e| format!("Failed to write: {}", e))?;

            // Change to temp directory
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = DiffCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
            };

            // Request only staged changes
            let result = cmd.execute(&["--staged"], &mut ctx);

            // Restore to manifest directory (guaranteed to exist)
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            match result {
                CommandResult::Output(output) => {
                    // Should NOT show untracked files when --staged is specified
                    if output.contains("Untracked") {
                        return Err(format!(
                            "Expected no untracked files with --staged, got: {}",
                            output
                        ));
                    }
                }
                CommandResult::Error(e) => return Err(format!("Diff failed: {}", e)),
                _ => return Err("Unexpected result type".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }
}

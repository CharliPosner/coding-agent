//! The /commit command - commit changes with purpose-focused messages
//!
//! This command analyzes git changes and commits them with a message
//! focused on the purpose/intent of the changes rather than just describing
//! what was changed.

use super::{Command, CommandContext, CommandResult};
use crate::integrations::git::{
    FileGrouper, FileStatus, FileStatusKind, GitError, GitRepo, RepoStatus,
};
use crate::ui::{
    edit_commit_message, CommitPreview, CommitPreviewResult, FileEntry, FilePicker,
    FilePickerResult,
};
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

        // Auto-commit mode: analyze grouping and commit
        execute_auto_commit_with_grouping(
            &repo,
            &status,
            options.stage_all,
            options.message.as_deref(),
        )
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

/// Execute an automatic commit with smart file grouping
fn execute_auto_commit_with_grouping(
    git_repo: &GitRepo,
    status: &RepoStatus,
    stage_all: bool,
    custom_message: Option<&str>,
) -> CommandResult {
    // Determine which files to consider
    let files_to_consider: Vec<FileStatus> = if stage_all {
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
            .cloned()
            .collect()
    } else {
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
            .cloned()
            .collect()
    };

    if files_to_consider.is_empty() {
        return CommandResult::Output("No changes to commit.".to_string());
    }

    // Group files by logical relationships
    let groups = FileGrouper::group_files(&files_to_consider);

    // If there's only one group, commit everything together
    if groups.len() == 1 {
        return execute_auto_commit(
            git_repo,
            status,
            stage_all,
            custom_message,
        );
    }

    // Multiple groups found - suggest splitting the commit
    let mut output = String::new();
    output.push_str("Found multiple logical groups in your changes:\n\n");

    for (i, group) in groups.iter().enumerate() {
        output.push_str(&format!(
            "  {}. {} ({}) - {} {}\n",
            i + 1,
            group.name,
            group.reason.description(),
            group.files.len(),
            if group.files.len() == 1 {
                "file"
            } else {
                "files"
            }
        ));

        // Show the files in this group (first 3)
        for (j, file) in group.files.iter().take(3).enumerate() {
            output.push_str(&format!(
                "     {} {}\n",
                file.status.indicator(),
                file.path.display()
            ));
            if j == 2 && group.files.len() > 3 {
                output.push_str(&format!("     ... and {} more\n", group.files.len() - 3));
            }
        }
        output.push_str("\n");
    }

    output.push_str(&format!(
        "Consider committing these groups separately for better history.\n\
         \n\
         To commit a specific group:\n\
         - Use /commit --pick to select files interactively\n\
         - Or commit all together by re-running /commit (agent will proceed with all files)\n\
         \n\
         Would you like to:\n\
         [1] Commit all changes together\n\
         [2] Use interactive picker to select files\n\
         [q] Cancel\n"
    ));

    CommandResult::Output(output)
}

/// Execute an automatic commit (original implementation without grouping)
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

    // Generate commit message (or use custom)
    let initial_message = match custom_message {
        Some(msg) => msg.to_string(),
        None => generate_commit_message(&files_to_stage, status),
    };

    // Show preview and get final message (skip preview if custom message provided)
    let final_message = if custom_message.is_some() {
        initial_message
    } else {
        // Build file list for preview
        let file_paths: Vec<String> = files_to_stage
            .iter()
            .map(|f| format!("{} {}", f.status.indicator(), f.path.display()))
            .collect();

        let preview = CommitPreview::new(initial_message.clone(), file_paths);

        match run_commit_preview_loop(preview) {
            Ok(Some(msg)) => msg,
            Ok(None) => return CommandResult::Output("Commit cancelled.".to_string()),
            Err(e) => return CommandResult::Error(format!("Preview error: {}", e)),
        }
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
                Err(e) => {
                    return CommandResult::Error(format!(
                        "Failed to create signature: {}. Configure git with 'git config user.name' and 'git config user.email'.",
                        e
                    ))
                }
            }
        }
    };

    // Get parent commit (if any)
    let parent = match repo.head() {
        Ok(head) => match head.peel_to_commit() {
            Ok(commit) => Some(commit),
            Err(_) => None,
        },
        Err(_) => None, // Initial commit
    };

    let parents: Vec<_> = parent.iter().collect();

    match repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &final_message,
        &tree,
        &parents,
    ) {
        Ok(oid) => {
            let short_id = &oid.to_string()[..7];
            let file_count = files_to_stage.len();
            let file_word = if file_count == 1 { "file" } else { "files" };

            // Build output
            let mut output = String::new();
            output.push_str(&format!(
                "✓ Committed {} {} [{}]\n\n",
                file_count, file_word, short_id
            ));
            output.push_str(&format!("{}\n", final_message));

            // List committed files
            output.push_str("\nFiles committed:\n");
            for file in files_to_stage {
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

/// Run the commit preview loop, allowing the user to edit the message
fn run_commit_preview_loop(mut preview: CommitPreview) -> std::io::Result<Option<String>> {
    loop {
        match preview.run()? {
            CommitPreviewResult::Confirmed(msg) => return Ok(Some(msg)),
            CommitPreviewResult::Cancelled => return Ok(None),
            CommitPreviewResult::Edit => {
                // Open editor
                match edit_commit_message(preview.message())? {
                    Some(edited_msg) => {
                        preview.set_message(edited_msg);
                        // Loop back to show the preview again
                    }
                    None => {
                        // User cancelled the edit (empty message)
                        return Ok(None);
                    }
                }
            }
        }
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

    // Generate commit message (or use custom)
    let initial_message = match custom_message {
        Some(msg) => msg.to_string(),
        None => generate_commit_message(&files_to_commit, status),
    };

    // Show preview and get final message (skip preview if custom message provided)
    let final_message = if custom_message.is_some() {
        initial_message
    } else {
        // Build file list for preview
        let file_paths: Vec<String> = files_to_commit
            .iter()
            .map(|f| format!("{} {}", f.status.indicator(), f.path.display()))
            .collect();

        let preview = CommitPreview::new(initial_message.clone(), file_paths);

        match run_commit_preview_loop(preview) {
            Ok(Some(msg)) => msg,
            Ok(None) => return CommandResult::Output("Commit cancelled.".to_string()),
            Err(e) => return CommandResult::Error(format!("Preview error: {}", e)),
        }
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
        &final_message,
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
            output.push_str(&format!("{}\n", final_message));

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

/// Analysis of changes for generating commit messages
#[derive(Debug, Default)]
struct ChangeAnalysis {
    /// Whether the changes include test files
    has_tests: bool,
    /// Whether the changes include source code files
    has_src: bool,
    /// Whether the changes include configuration files
    has_config: bool,
    /// Whether the changes include documentation files
    has_docs: bool,
    /// Whether the changes include CLI-related files
    has_cli: bool,
    /// Whether the changes include integration files
    has_integrations: bool,
    /// Whether the changes include UI-related files
    has_ui: bool,
    /// Primary directory of changes
    primary_dir: Option<String>,
    /// Component/module name if identifiable
    component_name: Option<String>,
    /// New files
    new_files: Vec<String>,
    /// Modified files
    modified_files: Vec<String>,
    /// Deleted files
    deleted_files: Vec<String>,
}

impl ChangeAnalysis {
    fn analyze(files: &[&crate::integrations::git::FileStatus]) -> Self {
        let mut analysis = Self::default();
        let mut directories: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut component_candidates: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for file in files {
            let path_str = file.path.to_string_lossy().to_string();

            // Categorize by file type
            if path_str.contains("test") || path_str.contains("spec") {
                analysis.has_tests = true;
            }
            if path_str.contains("src/") || path_str.ends_with(".rs") {
                analysis.has_src = true;
            }
            if path_str.contains("config")
                || path_str.ends_with(".toml")
                || path_str.ends_with(".json")
            {
                analysis.has_config = true;
            }
            if path_str.ends_with(".md") || path_str.contains("docs/") {
                analysis.has_docs = true;
            }
            if path_str.contains("cli/") || path_str.contains("/commands/") {
                analysis.has_cli = true;
            }
            if path_str.contains("integrations/") {
                analysis.has_integrations = true;
            }
            if path_str.contains("ui/") {
                analysis.has_ui = true;
            }

            // Track directories
            if let Some(parent) = file.path.parent() {
                let dir = parent.to_string_lossy().to_string();
                *directories.entry(dir).or_insert(0) += 1;
            }

            // Extract component name from file
            if let Some(stem) = file.path.file_stem() {
                let name = stem
                    .to_string_lossy()
                    .trim_end_matches("_test")
                    .trim_start_matches("test_")
                    .to_string();
                if name != "mod" && name != "lib" && name != "main" {
                    *component_candidates.entry(name).or_insert(0) += 1;
                }
            }

            // Categorize by change type
            match file.status {
                FileStatusKind::Added | FileStatusKind::Untracked => {
                    analysis.new_files.push(path_str);
                }
                FileStatusKind::Modified
                | FileStatusKind::Staged
                | FileStatusKind::StagedWithChanges => {
                    analysis.modified_files.push(path_str);
                }
                FileStatusKind::Deleted => {
                    analysis.deleted_files.push(path_str);
                }
                _ => {}
            }
        }

        // Find primary directory (most files)
        analysis.primary_dir =
            directories
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(dir, _)| {
                    std::path::Path::new(&dir)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or(dir)
                });

        // Find most common component name
        analysis.component_name = component_candidates
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name);

        analysis
    }

    /// Determine the primary action verb
    fn action_verb(&self) -> &'static str {
        if !self.new_files.is_empty()
            && self.modified_files.is_empty()
            && self.deleted_files.is_empty()
        {
            "Add"
        } else if !self.deleted_files.is_empty()
            && self.new_files.is_empty()
            && self.modified_files.is_empty()
        {
            "Remove"
        } else if !self.deleted_files.is_empty() && !self.new_files.is_empty() {
            "Refactor"
        } else {
            "Update"
        }
    }

    /// Get a human-readable subject for the changes
    fn subject(&self) -> String {
        // Try to construct a meaningful subject
        if let Some(ref component) = self.component_name {
            if self.has_tests && !self.has_src {
                return format!("tests for {}", component);
            }
            if self.has_cli {
                return format!("{} command", component);
            }
            if self.has_integrations {
                return format!("{} integration", component);
            }
            if self.has_ui {
                return format!("{} UI component", component);
            }
            return format!("{} functionality", component);
        }

        if let Some(ref dir) = self.primary_dir {
            if dir == "commands" {
                return "CLI commands".to_string();
            }
            if dir == "integrations" {
                return "external integrations".to_string();
            }
            if dir == "ui" {
                return "user interface".to_string();
            }
            if dir == "tests" {
                return "test suite".to_string();
            }
            return format!("{} module", dir);
        }

        if self.has_docs {
            return "documentation".to_string();
        }
        if self.has_config {
            return "configuration".to_string();
        }

        "codebase".to_string()
    }

    /// Generate the purpose sentence (why this change matters)
    fn purpose_sentence(&self) -> String {
        let verb = self.action_verb();
        let subject = self.subject();

        match verb {
            "Add" => {
                if self.has_tests {
                    format!("This improves code reliability by adding test coverage.")
                } else if self.has_docs {
                    format!("This improves developer experience with better documentation.")
                } else if self.has_cli {
                    format!("This extends the CLI with new capabilities for users.")
                } else if self.has_integrations {
                    format!("This enables new workflows through external system integration.")
                } else if self.has_ui {
                    format!("This enhances the user interface with new visual elements.")
                } else {
                    format!("This introduces new functionality to the {}.", subject)
                }
            }
            "Remove" => {
                format!("This simplifies the codebase by removing unused code.")
            }
            "Refactor" => {
                format!("This improves code structure without changing behavior.")
            }
            _ => {
                if self.has_tests {
                    format!("This maintains test accuracy as the implementation evolves.")
                } else if self.has_config {
                    format!("This adjusts settings to better match requirements.")
                } else if self.has_docs {
                    format!("This keeps documentation in sync with the codebase.")
                } else {
                    format!("This improves the {} to better meet requirements.", subject)
                }
            }
        }
    }

    /// Generate the implementation sentence (what was done technically)
    fn implementation_sentence(&self) -> String {
        let mut parts = Vec::new();

        if !self.new_files.is_empty() {
            let count = self.new_files.len();
            parts.push(format!(
                "adds {} new {}",
                count,
                if count == 1 { "file" } else { "files" }
            ));
        }

        if !self.modified_files.is_empty() {
            let count = self.modified_files.len();
            parts.push(format!(
                "modifies {} existing {}",
                count,
                if count == 1 { "file" } else { "files" }
            ));
        }

        if !self.deleted_files.is_empty() {
            let count = self.deleted_files.len();
            parts.push(format!(
                "removes {} obsolete {}",
                count,
                if count == 1 { "file" } else { "files" }
            ));
        }

        if parts.is_empty() {
            return "Updates the implementation as needed.".to_string();
        }

        // Combine parts with proper grammar
        let combined = match parts.len() {
            1 => parts[0].clone(),
            2 => format!("{} and {}", parts[0], parts[1]),
            _ => {
                let last = parts.pop().unwrap();
                format!("{}, and {}", parts.join(", "), last)
            }
        };

        format!("The change {}.", combined.trim_start_matches(' '))
    }
}

/// Generate a purpose-focused commit message based on the changes
///
/// The message follows a 3-sentence format:
/// 1. Title: Short summary of what changed (imperative mood)
/// 2. Purpose: Why this change matters
/// 3. Implementation: Technical summary of the changes
fn generate_commit_message(
    files: &[&crate::integrations::git::FileStatus],
    _status: &RepoStatus,
) -> String {
    let analysis = ChangeAnalysis::analyze(files);

    // Generate title (short, imperative mood)
    let verb = analysis.action_verb();
    let subject = analysis.subject();
    let title = format!("{} {}", verb, subject);

    // Generate the two body sentences
    let purpose = analysis.purpose_sentence();
    let implementation = analysis.implementation_sentence();

    // Combine into the 3-sentence format
    format!("{}\n\n{}\n{}", title, purpose, implementation)
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

        // Should have a title and body separated by blank line
        assert!(
            message.contains("\n\n"),
            "Message should have blank line: {}",
            message
        );
        // Should mention the file modification
        assert!(
            message.contains("modifies") || message.contains("file"),
            "Message should mention file changes: {}",
            message
        );
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
        // Use manifest directory as a stable restore point (won't be deleted by other tests)
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Use a closure to ensure cleanup happens
        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create initial commit
            let file_path = temp_dir.path().join("initial.txt");
            fs::write(&file_path, "initial").map_err(|e| format!("Failed to write file: {}", e))?;

            let mut index = repo
                .index()
                .map_err(|e| format!("Failed to get index: {}", e))?;
            index
                .add_path(Path::new("initial.txt"))
                .map_err(|e| format!("Failed to add file: {}", e))?;
            index
                .write()
                .map_err(|e| format!("Failed to write index: {}", e))?;

            let tree_id = index
                .write_tree()
                .map_err(|e| format!("Failed to write tree: {}", e))?;
            let tree = repo
                .find_tree(tree_id)
                .map_err(|e| format!("Failed to find tree: {}", e))?;
            let sig = repo
                .signature()
                .map_err(|e| format!("Failed to get signature: {}", e))?;

            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .map_err(|e| format!("Failed to commit: {}", e))?;

            // Create a new file to commit
            let new_file = temp_dir.path().join("new_file.txt");
            fs::write(&new_file, "new content")
                .map_err(|e| format!("Failed to write file: {}", e))?;

            // Change to the temp directory and run commit
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = CommitCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            // Run the commit with -a flag to stage the new file
            let result = cmd.execute(&["-a", "-m", "Add new file"], &mut ctx);

            // Restore to manifest directory (guaranteed to exist)
            std::env::set_current_dir(&manifest_dir)
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
            let head = repo
                .head()
                .map_err(|e| format!("Failed to get HEAD: {}", e))?;
            let commit = head
                .peel_to_commit()
                .map_err(|e| format!("Failed to get commit: {}", e))?;
            if commit.message().unwrap() != "Add new file" {
                return Err(format!("Wrong commit message: {:?}", commit.message()));
            }

            Ok(())
        })();

        // Always restore to manifest directory
        let _ = std::env::set_current_dir(&manifest_dir);

        // Now check if the test passed
        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_commit_no_changes() {
        // Use manifest directory as a stable restore point (won't be deleted by other tests)
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let (temp_dir, repo) = init_test_repo();

            // Create initial commit
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "content").map_err(|e| format!("Failed to write file: {}", e))?;

            let mut index = repo
                .index()
                .map_err(|e| format!("Failed to get index: {}", e))?;
            index
                .add_path(Path::new("test.txt"))
                .map_err(|e| format!("Failed to add file: {}", e))?;
            index
                .write()
                .map_err(|e| format!("Failed to write index: {}", e))?;

            let tree_id = index
                .write_tree()
                .map_err(|e| format!("Failed to write tree: {}", e))?;
            let tree = repo
                .find_tree(tree_id)
                .map_err(|e| format!("Failed to find tree: {}", e))?;
            let sig = repo
                .signature()
                .map_err(|e| format!("Failed to get signature: {}", e))?;

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
                agent_manager: None,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore to manifest directory (guaranteed to exist)
            std::env::set_current_dir(&manifest_dir)
                .map_err(|e| format!("Failed to restore dir: {}", e))?;

            // Should report nothing to commit
            match result {
                CommandResult::Output(output) => {
                    if !output.to_lowercase().contains("nothing to commit")
                        && !output.to_lowercase().contains("no changes")
                        && !output.to_lowercase().contains("clean")
                    {
                        return Err(format!(
                            "Expected 'nothing to commit' message, got: {}",
                            output
                        ));
                    }
                }
                _ => return Err("Expected Output result for clean working tree".to_string()),
            }

            Ok(())
        })();

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_commit_not_in_repo() {
        // Use manifest directory as a stable restore point (won't be deleted by other tests)
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let test_result: Result<(), String> = (|| {
            let temp_dir =
                TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;

            // Change to the temp directory (not a git repo)
            std::env::set_current_dir(temp_dir.path())
                .map_err(|e| format!("Failed to change dir: {}", e))?;

            let cmd = CommitCommand;
            let registry = CommandRegistry::with_defaults();
            let cost_tracker = CostTracker::with_default_model();
            let mut ctx = CommandContext {
                registry,
                cost_tracker,
                agent_manager: None,
            };

            let result = cmd.execute(&[], &mut ctx);

            // Restore to manifest directory (guaranteed to exist)
            std::env::set_current_dir(&manifest_dir)
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

        let _ = std::env::set_current_dir(&manifest_dir);

        if let Err(e) = test_result {
            panic!("{}", e);
        }
    }

    // ========================================================================
    // 3-sentence Purpose-Focused Commit Message Tests
    // ========================================================================

    #[test]
    fn test_commit_message_has_three_parts() {
        // Verify the message has a title, blank line, and two body sentences
        let files = vec![FileStatus {
            path: PathBuf::from("src/auth.rs"),
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
        let parts: Vec<&str> = message.split("\n\n").collect();

        // Should have title and body separated by blank line
        assert_eq!(
            parts.len(),
            2,
            "Message should have title and body separated by blank line"
        );

        // Title should not be empty
        let title = parts[0];
        assert!(!title.is_empty(), "Title should not be empty");

        // Body should have two sentences (each ends with period)
        let body = parts[1];
        let sentences: Vec<&str> = body.split(".\n").filter(|s| !s.is_empty()).collect();
        assert!(sentences.len() >= 1, "Body should have sentences");
    }

    #[test]
    fn test_commit_message_title_starts_with_verb() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/new_file.rs"),
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
        let title = message.lines().next().unwrap();

        // Title should start with an action verb
        let starts_with_verb = title.starts_with("Add")
            || title.starts_with("Update")
            || title.starts_with("Remove")
            || title.starts_with("Refactor");
        assert!(
            starts_with_verb,
            "Title should start with an action verb: {}",
            title
        );
    }

    #[test]
    fn test_commit_message_purpose_sentence() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/cli/commands/new_cmd.rs"),
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

        // Should contain purpose-oriented language (why it matters)
        let has_purpose = message.contains("This ")
            || message.contains("improves")
            || message.contains("enables")
            || message.contains("introduces");
        assert!(has_purpose, "Message should explain purpose: {}", message);
    }

    #[test]
    fn test_commit_message_implementation_sentence() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/feature.rs"),
                status: FileStatusKind::Added,
            },
            FileStatus {
                path: PathBuf::from("src/old.rs"),
                status: FileStatusKind::Deleted,
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

        // Should mention what was added/modified/removed
        let has_implementation = message.contains("adds")
            || message.contains("modifies")
            || message.contains("removes")
            || message.contains("file");
        assert!(
            has_implementation,
            "Message should describe implementation: {}",
            message
        );
    }

    #[test]
    fn test_commit_message_cli_recognition() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/cli/commands/status.rs"),
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

        // Should recognize CLI-related changes
        let recognizes_cli =
            message.to_lowercase().contains("command") || message.to_lowercase().contains("cli");
        assert!(recognizes_cli, "Should recognize CLI changes: {}", message);
    }

    #[test]
    fn test_commit_message_test_recognition() {
        let files = vec![FileStatus {
            path: PathBuf::from("tests/auth_test.rs"),
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

        // Should recognize test files
        let recognizes_tests = message.to_lowercase().contains("test");
        assert!(
            recognizes_tests,
            "Should recognize test changes: {}",
            message
        );
    }

    #[test]
    fn test_commit_message_delete_operation() {
        let files = vec![FileStatus {
            path: PathBuf::from("src/deprecated.rs"),
            status: FileStatusKind::Deleted,
        }];
        let file_refs: Vec<_> = files.iter().collect();
        let status = RepoStatus {
            branch: Some("main".to_string()),
            detached: false,
            has_conflicts: false,
            files: files.clone(),
        };

        let message = generate_commit_message(&file_refs, &status);

        // Should use Remove verb for deletions
        assert!(
            message.starts_with("Remove"),
            "Delete should use 'Remove' verb: {}",
            message
        );
    }

    #[test]
    fn test_commit_message_mixed_changes() {
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/new.rs"),
                status: FileStatusKind::Added,
            },
            FileStatus {
                path: PathBuf::from("src/existing.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("src/old.rs"),
                status: FileStatusKind::Deleted,
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

        // Should use Refactor for mixed add+delete
        assert!(
            message.starts_with("Refactor"),
            "Mixed changes should use 'Refactor': {}",
            message
        );

        // Implementation sentence should mention all types
        assert!(
            message.contains("adds") && message.contains("removes"),
            "Should mention both adds and removes: {}",
            message
        );
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
        // The message should be an Update since we're modifying files
        assert!(
            message.starts_with("Update"),
            "Should start with Update: {}",
            message
        );
        // Should mention the file count in implementation sentence
        assert!(
            message.contains("2 existing files") || message.contains("auth"),
            "Should reference changes: {}",
            message
        );
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
                    FileStatusKind::Staged
                        | FileStatusKind::StagedWithChanges
                        | FileStatusKind::Added
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
        index
            .add_path(Path::new("test.txt"))
            .expect("Failed to add file");
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

    // ========================================================================
    // File Grouping Integration Tests
    // ========================================================================

    #[test]
    fn test_auto_commit_suggests_groups() {
        // Test that auto-commit suggests splitting when multiple groups are found
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

        // Create changes in multiple logical groups
        // Group 1: Auth module
        let auth_impl = temp_dir.path().join("src");
        fs::create_dir_all(&auth_impl).expect("Failed to create src dir");
        fs::write(auth_impl.join("auth.rs"), "// auth implementation")
            .expect("Failed to write auth.rs");

        // Group 2: Test for auth
        let tests_dir = temp_dir.path().join("tests");
        fs::create_dir_all(&tests_dir).expect("Failed to create tests dir");
        fs::write(tests_dir.join("auth_test.rs"), "// auth tests")
            .expect("Failed to write auth_test.rs");

        // Group 3: Config file
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]")
            .expect("Failed to write Cargo.toml");

        // Open repo and get status
        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let status = git_repo.status().expect("Failed to get status");

        // Run auto-commit with grouping
        let result = execute_auto_commit_with_grouping(&git_repo, &status, true, None);

        match result {
            CommandResult::Output(output) => {
                // Should suggest splitting
                assert!(
                    output.contains("multiple logical groups"),
                    "Should suggest groups: {}",
                    output
                );
                assert!(
                    output.contains("configuration")
                        || output.contains("test and implementation")
                        || output.contains("Commit all"),
                    "Should show grouping info: {}",
                    output
                );
            }
            _ => panic!("Expected Output result suggesting groups"),
        }
    }

    #[test]
    fn test_auto_commit_single_group_proceeds() {
        // When all files are in one logical group, should proceed with commit
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

        // Create changes in the same directory (single logical group)
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).expect("Failed to create src dir");
        fs::write(src_dir.join("foo.rs"), "// foo").expect("Failed to write foo.rs");
        fs::write(src_dir.join("bar.rs"), "// bar").expect("Failed to write bar.rs");

        let git_repo = GitRepo::open(temp_dir.path()).expect("Failed to open repo");
        let status = git_repo.status().expect("Failed to get status");

        // With files in the same directory, FileGrouper will create 1 group
        // So execute_auto_commit_with_grouping should call execute_auto_commit
        // which requires user interaction (preview/edit), so we can't test the full flow here.
        // Instead, verify that grouping recognizes them as a single group
        let groups = FileGrouper::group_files(&status.files);

        assert_eq!(
            groups.len(),
            1,
            "Files in same directory should form one group"
        );
    }

    #[test]
    fn test_file_grouping_integration_with_commit() {
        // Verify that FileGrouper correctly identifies test+impl pairs
        // and that the commit command can use this information
        let files = vec![
            FileStatus {
                path: PathBuf::from("src/parser.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("tests/parser_test.rs"),
                status: FileStatusKind::Modified,
            },
            FileStatus {
                path: PathBuf::from("Cargo.toml"),
                status: FileStatusKind::Modified,
            },
        ];

        let groups = FileGrouper::group_files(&files);

        // Should have at least 2 groups: test+impl and config
        assert!(groups.len() >= 2, "Should have multiple groups");

        // Verify one group is test+impl
        let has_test_impl = groups
            .iter()
            .any(|g| g.reason == crate::integrations::git::GroupReason::TestAndImplementation);
        assert!(has_test_impl, "Should have test+impl group");

        // Verify one group is configuration
        let has_config = groups
            .iter()
            .any(|g| g.reason == crate::integrations::git::GroupReason::Configuration);
        assert!(has_config, "Should have config group");
    }
}

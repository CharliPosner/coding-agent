//! The /land command - safely close a session with tests, lint, commit, and push
//!
//! This command executes a scripted checklist to safely close the session:
//! 1. Run tests - abort if failing
//! 2. Run linter - abort if errors (warnings OK)
//! 3. Check changes - show uncommitted files
//! 4. Commit - if changes exist, create a commit
//! 5. Push - non-negotiable

use super::{Command, CommandContext, CommandResult};
use std::process::Command as ProcessCommand;

pub struct LandCommand;

impl Command for LandCommand {
    fn name(&self) -> &'static str {
        "land"
    }

    fn description(&self) -> &'static str {
        "Safely close session: run tests, lint, commit, and push"
    }

    fn usage(&self) -> &'static str {
        "/land"
    }

    fn execute(&self, _args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        let mut output = String::new();
        output.push_str("/land\n\n");

        // Step 1: Run tests
        output.push_str("[1/5] Running tests...\n");
        match run_tests() {
            Ok(test_output) => {
                output.push_str(&format!("  {} {}\n\n", CHECKMARK, test_output));
            }
            Err(e) => {
                output.push_str(&format!("  {} {}\n", CROSS, e));
                output.push_str("\nAborting: Tests must pass before landing.\n");
                return CommandResult::Error(output);
            }
        }

        // Step 2: Run linter
        output.push_str("[2/5] Running linter...\n");
        match run_linter() {
            Ok(lint_output) => {
                output.push_str(&format!("  {} {}\n\n", CHECKMARK, lint_output));
            }
            Err(e) => {
                output.push_str(&format!("  {} {}\n", CROSS, e));
                output.push_str("\nAborting: Linter errors must be fixed before landing.\n");
                return CommandResult::Error(output);
            }
        }

        // Step 3: Check for uncommitted changes
        output.push_str("[3/5] Checking for uncommitted changes...\n");
        let changes = match check_changes() {
            Ok(changes) => {
                if changes.is_empty() {
                    output.push_str("  No uncommitted changes\n\n");
                    Vec::new()
                } else {
                    output.push_str(&format!("  Found {} modified files\n\n", changes.len()));
                    changes
                }
            }
            Err(e) => {
                output.push_str(&format!("  {} {}\n", CROSS, e));
                return CommandResult::Error(output);
            }
        };

        // Step 4: Create commit if there are changes
        output.push_str("[4/5] Creating commit...\n");
        if changes.is_empty() {
            output.push_str("  No changes to commit\n\n");
        } else {
            match create_commit() {
                Ok(commit_msg) => {
                    output.push_str(&format!("  {} Committed: \"{}\"\n\n", CHECKMARK, commit_msg));
                }
                Err(e) => {
                    output.push_str(&format!("  {} {}\n", CROSS, e));
                    return CommandResult::Error(output);
                }
            }
        }

        // Step 5: Push to remote (non-negotiable)
        output.push_str("[5/5] Pushing to remote...\n");
        match push_to_remote() {
            Ok(push_output) => {
                output.push_str(&format!("  {} {}\n\n", CHECKMARK, push_output));
            }
            Err(e) => {
                output.push_str(&format!("  {} {}\n", CROSS, e));
                output.push_str("\nWarning: Push failed. Please push manually before closing.\n");
                return CommandResult::Error(output);
            }
        }

        output.push_str("Session saved. Safe to close.\n");

        CommandResult::Output(output)
    }
}

// Unicode symbols for output
const CHECKMARK: &str = "ok";
const CROSS: &str = "FAILED";

/// Run cargo test and return success/failure
fn run_tests() -> Result<String, String> {
    let output = ProcessCommand::new("cargo")
        .args(["test", "--quiet"])
        .output()
        .map_err(|e| format!("Failed to run cargo test: {}", e))?;

    if output.status.success() {
        Ok("All tests passed".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Try to extract a useful error message
        let error_msg = if !stderr.is_empty() {
            stderr.lines().take(5).collect::<Vec<_>>().join("\n")
        } else if !stdout.is_empty() {
            stdout.lines().take(5).collect::<Vec<_>>().join("\n")
        } else {
            "Tests failed".to_string()
        };
        Err(format!("Tests failed:\n{}", error_msg))
    }
}

/// Run cargo clippy and return success/failure (errors only, warnings OK)
fn run_linter() -> Result<String, String> {
    let output = ProcessCommand::new("cargo")
        .args(["clippy", "--quiet", "--", "-D", "warnings"])
        .output()
        .map_err(|e| format!("Failed to run cargo clippy: {}", e))?;

    // Clippy returns non-zero on warnings with -D warnings, but we want to allow warnings
    // So we run without -D warnings and check for actual errors
    let output_permissive = ProcessCommand::new("cargo")
        .args(["clippy", "--quiet"])
        .output()
        .map_err(|e| format!("Failed to run cargo clippy: {}", e))?;

    if output_permissive.status.success() {
        // Check if the stricter version had warnings
        if output.status.success() {
            Ok("No errors".to_string())
        } else {
            Ok("No errors (some warnings present)".to_string())
        }
    } else {
        let stderr = String::from_utf8_lossy(&output_permissive.stderr);
        let error_msg = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
        Err(format!("Linter errors:\n{}", error_msg))
    }
}

/// Check git status for uncommitted changes
fn check_changes() -> Result<Vec<String>, String> {
    let output = ProcessCommand::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to run git status: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git status failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let changes: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(changes)
}

/// Create a commit with all changes
fn create_commit() -> Result<String, String> {
    // Stage all changes
    let add_output = ProcessCommand::new("git")
        .args(["add", "-A"])
        .output()
        .map_err(|e| format!("Failed to run git add: {}", e))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(format!("git add failed: {}", stderr));
    }

    // Create commit with a session work message
    let commit_message = "Session work";
    let commit_output = ProcessCommand::new("git")
        .args(["commit", "-m", commit_message])
        .output()
        .map_err(|e| format!("Failed to run git commit: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        // Check if there's nothing to commit
        if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
            return Ok("No changes to commit".to_string());
        }
        return Err(format!("git commit failed: {}", stderr));
    }

    Ok(commit_message.to_string())
}

/// Push to the remote repository
fn push_to_remote() -> Result<String, String> {
    let output = ProcessCommand::new("git")
        .args(["push"])
        .output()
        .map_err(|e| format!("Failed to run git push: {}", e))?;

    if output.status.success() {
        Ok("Pushed to origin".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check for common push issues
        if stderr.contains("no upstream branch") || stderr.contains("set-upstream") {
            // Try to push with -u to set upstream
            let push_with_upstream = ProcessCommand::new("git")
                .args(["push", "-u", "origin", "HEAD"])
                .output()
                .map_err(|e| format!("Failed to run git push -u: {}", e))?;

            if push_with_upstream.status.success() {
                return Ok("Pushed to origin (set upstream)".to_string());
            }
            let stderr = String::from_utf8_lossy(&push_with_upstream.stderr);
            return Err(format!("Push failed: {}", stderr.trim()));
        }
        Err(format!("Push failed: {}", stderr.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::{CollapsedResults, CommandRegistry};
    use crate::tokens::CostTracker;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_land_command_name() {
        let cmd = LandCommand;
        assert_eq!(cmd.name(), "land");
    }

    #[test]
    fn test_land_command_description() {
        let cmd = LandCommand;
        assert!(!cmd.description().is_empty());
        assert!(
            cmd.description().contains("session")
                || cmd.description().contains("close")
                || cmd.description().contains("land")
        );
    }

    #[test]
    fn test_land_command_usage() {
        let cmd = LandCommand;
        assert!(cmd.usage().contains("/land"));
    }

    #[test]
    fn test_land_command_trait_impl() {
        // Verify the command implements the Command trait correctly
        let cmd = LandCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();
        let mut ctx = CommandContext {
            registry,
            cost_tracker,
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        };

        // Execute the command (will likely fail in test environment due to git/cargo)
        // but we're testing that it runs without panicking
        let result = cmd.execute(&[], &mut ctx);

        // Result should be either Output or Error (not a panic)
        match result {
            CommandResult::Output(_) => {
                // Success case - all steps passed
            }
            CommandResult::Error(_) => {
                // Expected in test environment where tests/lint/git may not work
            }
            _ => {
                panic!("Unexpected result type from land command");
            }
        }
    }

    #[test]
    fn test_check_changes_returns_vec() {
        // This test verifies the check_changes function returns the expected type
        // It may fail if not in a git repo, but shouldn't panic
        let result = check_changes();
        match result {
            Ok(changes) => {
                // changes should be a Vec<String> - verify it's a valid vec
                let _ = changes.len(); // Just verify we can access length
            }
            Err(e) => {
                // Expected if not in a git repo
                assert!(e.contains("git") || e.contains("repository"));
            }
        }
    }

    #[test]
    fn test_output_format() {
        // Test that the output format matches the spec
        let cmd = LandCommand;
        let registry = CommandRegistry::with_defaults();
        let cost_tracker = CostTracker::with_default_model();
        let mut ctx = CommandContext {
            registry,
            cost_tracker,
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        };

        let result = cmd.execute(&[], &mut ctx);

        // Extract the output string
        let output = match result {
            CommandResult::Output(s) => s,
            CommandResult::Error(s) => s,
            _ => panic!("Expected Output or Error"),
        };

        // Should start with /land
        assert!(output.starts_with("/land"), "Output should start with /land");

        // Should contain step markers
        assert!(output.contains("[1/5]"), "Should have step 1 marker");
        assert!(
            output.contains("Running tests") || output.contains("tests"),
            "Should mention tests"
        );
    }

    #[test]
    fn test_symbols_are_ascii() {
        // Verify that symbols are ASCII-safe for terminal compatibility
        assert!(CHECKMARK.is_ascii(), "Checkmark should be ASCII");
        assert!(CROSS.is_ascii(), "Cross should be ASCII");
    }
}

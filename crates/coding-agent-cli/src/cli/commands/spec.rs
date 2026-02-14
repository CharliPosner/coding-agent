//! The /spec command - create spec files and enter planning mode

use super::{Command, CommandContext, CommandResult};
use crate::cli::Mode;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SpecCommand;

impl Command for SpecCommand {
    fn name(&self) -> &'static str {
        "spec"
    }

    fn description(&self) -> &'static str {
        "Create or open a spec file and enter planning mode"
    }

    fn usage(&self) -> &'static str {
        "/spec <name>"
    }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::Error(
                "Usage: /spec <name>\nExample: /spec authentication".to_string(),
            );
        }

        let spec_name = args[0];

        // Validate spec name (should be alphanumeric with hyphens/underscores)
        if !is_valid_spec_name(spec_name) {
            return CommandResult::Error(
                format!(
                    "Invalid spec name: '{}'\nSpec names should contain only letters, numbers, hyphens, and underscores.",
                    spec_name
                )
            );
        }

        let spec_path = get_spec_path(spec_name);

        // Check if spec file already exists
        let file_exists = spec_path.exists();

        if file_exists {
            // Load existing spec
            match fs::read_to_string(&spec_path) {
                Ok(content) => {
                    let output = format!(
                        "Opened existing spec: {}\n\n\
                        Entering planning mode...\n\n\
                        Current content:\n\
                        {}\n\n\
                        Let's continue working on this specification. What would you like to discuss or add?",
                        spec_path.display(),
                        truncate_content(&content, 500)
                    );
                    CommandResult::ModeChange {
                        mode: Mode::planning(spec_path.display().to_string()),
                        output: Some(output),
                    }
                }
                Err(e) => CommandResult::Error(format!("Failed to read spec file: {}", e)),
            }
        } else {
            // Create new spec with template
            let template = create_spec_template(spec_name);

            // Ensure specs directory exists
            if let Some(parent) = spec_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return CommandResult::Error(format!(
                        "Failed to create specs directory: {}",
                        e
                    ));
                }
            }

            // Write template to file
            match fs::write(&spec_path, &template) {
                Ok(_) => {
                    let output = format!(
                        "Created new spec: {}\n\n\
                        Entering planning mode...\n\n\
                        I've created a template for your specification. Let's design the {} system together.\n\n\
                        To get started, tell me:\n\
                        1. What problem are you trying to solve?\n\
                        2. Who will use this feature?\n\
                        3. What are the key requirements?\n\n\
                        Or feel free to ask me questions about the best approach!",
                        spec_path.display(),
                        spec_name
                    );
                    CommandResult::ModeChange {
                        mode: Mode::planning(spec_path.display().to_string()),
                        output: Some(output),
                    }
                }
                Err(e) => CommandResult::Error(format!("Failed to create spec file: {}", e)),
            }
        }
    }
}

/// Validate that a spec name contains only safe characters
fn is_valid_spec_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Get the path for a spec file
fn get_spec_path(name: &str) -> PathBuf {
    Path::new("specs").join(format!("{}.md", name))
}

/// Create a template for a new spec file
fn create_spec_template(name: &str) -> String {
    let title = name.replace('-', " ").replace('_', " ");
    let title = title
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        r#"# {} Specification

> A brief description of what this feature/system does

---

## Overview

[High-level description of the feature/system]

## Goals

- [ ] Goal 1
- [ ] Goal 2
- [ ] Goal 3

## Requirements

### Functional Requirements

1. **Requirement 1**
   - Detail
   - Detail

2. **Requirement 2**
   - Detail
   - Detail

### Non-Functional Requirements

- **Performance**: [performance requirements]
- **Security**: [security considerations]
- **Usability**: [usability requirements]

## Design

### Architecture

[High-level architecture overview]

### Components

#### Component 1

[Description]

#### Component 2

[Description]

## Implementation Plan

1. Phase 1: [description]
2. Phase 2: [description]
3. Phase 3: [description]

## Testing Strategy

- Unit tests: [what to test]
- Integration tests: [what to test]
- Edge cases: [what to consider]

## Open Questions

- [ ] Question 1
- [ ] Question 2

## References

- [Link to related docs]
- [Link to similar implementations]

---

*Last updated: [date]*
"#,
        title
    )
}

/// Truncate content to a maximum length with "..." suffix
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let truncated = &content[..max_len];
        format!(
            "{}...\n\n[Content truncated - {} more characters]",
            truncated,
            content.len() - max_len
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::CostTracker;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    // Mutex to serialize tests that change directories
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn get_test_dir() -> PathBuf {
        // Use a unique test directory in the target folder with timestamp and thread ID
        let test_id = std::thread::current().id();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = env::temp_dir().join(format!(
            "coding-agent-spec-tests-{:?}-{}",
            test_id, timestamp
        ));
        test_dir
    }

    fn setup_test_env() -> (CommandContext, Option<PathBuf>) {
        let test_dir = get_test_dir();

        // Clean up any existing test data
        let _ = fs::remove_dir_all(&test_dir);

        // Create test directory
        if let Err(e) = fs::create_dir_all(&test_dir) {
            eprintln!("Failed to create test directory: {}", e);
        }

        // Try to change to test directory, but don't fail if we can't
        let original_dir = env::current_dir().ok();
        let _ = env::set_current_dir(&test_dir);

        let ctx = CommandContext {
            registry: CommandRegistry::with_defaults(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
            config: std::sync::Arc::new(crate::config::Config::default()),
        };

        (ctx, original_dir)
    }

    fn teardown_test_env(original_dir: Option<PathBuf>) {
        // Change back to original directory if we saved one
        if let Some(dir) = original_dir {
            let _ = env::set_current_dir(&dir);
        }

        // Clean up test directory
        let test_dir = get_test_dir();
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_spec_command_name() {
        let cmd = SpecCommand;
        assert_eq!(cmd.name(), "spec");
    }

    #[test]
    fn test_spec_command_description() {
        let cmd = SpecCommand;
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_spec_no_args() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let cmd = SpecCommand;
        let (mut ctx, original_dir) = setup_test_env();

        let result = cmd.execute(&[], &mut ctx);

        teardown_test_env(original_dir);

        match result {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage"));
            }
            _ => panic!("Expected error when no args provided"),
        }
    }

    #[test]
    fn test_spec_creates_file() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let cmd = SpecCommand;
        let (mut ctx, original_dir) = setup_test_env();

        let result = cmd.execute(&["test-spec"], &mut ctx);

        // Check that file was created
        let spec_path = Path::new("specs/test-spec.md");
        assert!(spec_path.exists(), "Spec file should be created");

        // Check that result indicates success and mode change
        match result {
            CommandResult::ModeChange { mode, output } => {
                assert!(mode.is_planning());
                assert!(output.is_some());
                let msg = output.unwrap();
                assert!(msg.contains("Created new spec") || msg.contains("created a template"));
                assert!(msg.contains("planning mode"));
            }
            CommandResult::Error(e) => panic!("Expected ModeChange result, got Error: {}", e),
            _ => panic!("Expected ModeChange result, got: {:?}", result),
        }

        teardown_test_env(original_dir);
    }

    #[test]
    fn test_spec_template_valid() {
        let template = create_spec_template("authentication");

        // Check template has required sections
        assert!(template.contains("# Authentication Specification"));
        assert!(template.contains("## Overview"));
        assert!(template.contains("## Goals"));
        assert!(template.contains("## Requirements"));
        assert!(template.contains("## Design"));
        assert!(template.contains("## Implementation Plan"));
        assert!(template.contains("## Testing Strategy"));
    }

    #[test]
    fn test_spec_existing_file() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let cmd = SpecCommand;
        let (mut ctx, original_dir) = setup_test_env();

        // Create a spec first
        let _ = cmd.execute(&["existing-spec"], &mut ctx);

        // Try to open it again
        let result = cmd.execute(&["existing-spec"], &mut ctx);

        match result {
            CommandResult::ModeChange { mode, output } => {
                assert!(mode.is_planning());
                assert!(output.is_some());
                let msg = output.unwrap();
                assert!(msg.contains("Opened existing spec"));
                assert!(msg.contains("planning mode"));
            }
            _ => panic!("Expected ModeChange result when opening existing spec"),
        }

        teardown_test_env(original_dir);
    }

    #[test]
    fn test_spec_invalid_name() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let cmd = SpecCommand;
        let (mut ctx, original_dir) = setup_test_env();

        // Try with invalid characters
        let result = cmd.execute(&["invalid/name"], &mut ctx);

        match result {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Invalid spec name"));
            }
            _ => panic!("Expected error for invalid spec name"),
        }

        teardown_test_env(original_dir);
    }

    #[test]
    fn test_is_valid_spec_name() {
        // Valid names
        assert!(is_valid_spec_name("auth"));
        assert!(is_valid_spec_name("authentication"));
        assert!(is_valid_spec_name("user-auth"));
        assert!(is_valid_spec_name("user_auth"));
        assert!(is_valid_spec_name("auth123"));

        // Invalid names
        assert!(!is_valid_spec_name(""));
        assert!(!is_valid_spec_name("auth/system"));
        assert!(!is_valid_spec_name("auth.system"));
        assert!(!is_valid_spec_name("auth system"));
        assert!(!is_valid_spec_name("../etc/passwd"));
    }

    #[test]
    fn test_truncate_content() {
        let short = "Hello";
        assert_eq!(truncate_content(short, 10), "Hello");

        let long = "A".repeat(100);
        let truncated = truncate_content(&long, 50);
        assert!(truncated.len() > 50); // Includes "..." and message
        assert!(truncated.contains("..."));
        assert!(truncated.contains("50 more characters"));
    }
}

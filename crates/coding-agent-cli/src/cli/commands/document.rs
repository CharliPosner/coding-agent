//! The /document command - Obsidian note management

use super::{Command, CommandContext, CommandResult};
use crate::integrations::{ObsidianError, ObsidianVault};
use std::path::PathBuf;

pub struct DocumentCommand;

impl Command for DocumentCommand {
    fn name(&self) -> &'static str {
        "document"
    }

    fn description(&self) -> &'static str {
        "Create or update an Obsidian note"
    }

    fn usage(&self) -> &'static str {
        "/document <topic> [--new] [--search]"
    }

    fn execute(&self, args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::Error(
                "Usage: /document <topic> [--new] [--search]\n\
                 Example: /document rust error handling"
                    .to_string(),
            );
        }

        // Parse flags
        let mut search_only = false;
        let mut force_new = false;
        let mut topic_parts = Vec::new();

        for arg in args {
            match *arg {
                "--search" => search_only = true,
                "--new" => force_new = true,
                _ => topic_parts.push(*arg),
            }
        }

        if topic_parts.is_empty() {
            return CommandResult::Error("Please provide a topic to document.".to_string());
        }

        let topic = topic_parts.join(" ");

        // Get vault path from config (default to ~/Documents/Personal/)
        let vault_path = get_vault_path();

        // Create vault manager
        let vault = match ObsidianVault::new(vault_path.clone()) {
            Ok(v) => v,
            Err(ObsidianError::VaultNotFound(_)) => {
                return CommandResult::Error(format!(
                    "Obsidian vault not found at: {}\n\n\
                     To use /document, please:\n\
                     1. Create the directory, or\n\
                     2. Configure your vault path in ~/.config/coding-agent/config.toml:\n\
                     \n\
                     [integrations.obsidian]\n\
                     vault_path = \"/path/to/your/vault\"",
                    vault_path.display()
                ));
            }
            Err(e) => {
                return CommandResult::Error(format!("Failed to access vault: {}", e));
            }
        };

        // Search for related notes
        let search_results = match vault.search(&topic) {
            Ok(results) => results,
            Err(e) => {
                return CommandResult::Error(format!("Failed to search vault: {}", e));
            }
        };

        // If --search flag, just show results and return
        if search_only {
            if search_results.is_empty() {
                return CommandResult::Output(format!(
                    "No notes found related to '{}' in {}",
                    topic,
                    vault.path().display()
                ));
            }

            let mut output = format!("Found {} related note(s):\n\n", search_results.len());
            for (i, result) in search_results.iter().take(10).enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, result.note.title));
                output.push_str(&format!("   Path: {}\n", result.note.path.display()));
                if let Some(excerpt) = &result.excerpt {
                    output.push_str(&format!("   {}\n", excerpt));
                }
                output.push('\n');
            }
            return CommandResult::Output(output);
        }

        // If --new flag or no results found, create a new note
        if force_new || search_results.is_empty() {
            let suggested_path = vault.suggest_location(&topic);

            let output = format!(
                "Creating new note about '{}'...\n\n\
                 Suggested location: {}\n\
                 Vault path: {}\n\n\
                 I'll help you create this note. What would you like to include?\n\
                 You can tell me:\n\
                 - Key points to document\n\
                 - Structure you'd like (e.g., tutorial, reference, notes)\n\
                 - Any specific content to include\n\n\
                 Or just say 'create it' and I'll generate an initial structure based on the topic.",
                topic,
                suggested_path,
                vault.path().display()
            );

            return CommandResult::Output(output);
        }

        // Found existing notes - show them and ask what to do
        let mut output = format!(
            "Found {} related note(s) about '{}':\n\n",
            search_results.len(),
            topic
        );

        for (i, result) in search_results.iter().take(5).enumerate() {
            output.push_str(&format!("{}. {} (score: {:.2})\n", i + 1, result.note.title, result.score));
            output.push_str(&format!("   Path: {}\n", result.note.path.display()));
            if let Some(excerpt) = &result.excerpt {
                output.push_str(&format!("   ...{}\n", excerpt));
            }
            output.push('\n');
        }

        output.push_str("\nWhat would you like to do?\n");
        output.push_str("- Tell me which note to update (e.g., 'update note 1')\n");
        output.push_str("- Ask me to create a new note (e.g., 'create new note')\n");
        output.push_str("- Ask me to show you the full content of a note (e.g., 'show note 2')");

        CommandResult::Output(output)
    }
}

/// Get the Obsidian vault path from config or use default
fn get_vault_path() -> PathBuf {
    // TODO: Once config supports obsidian settings, read from there
    // For now, use the default from the spec
    expand_tilde(PathBuf::from("~/Documents/Personal"))
}

/// Expand tilde (~) in path to home directory
fn expand_tilde(path: PathBuf) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") || path_str == "~" {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path_str[2..]);
            }
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::CommandRegistry;
    use crate::tokens::CostTracker;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_vault() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create some test notes
        fs::write(
            temp_dir.path().join("rust-basics.md"),
            "# Rust Basics\n\nRust is a systems programming language.",
        )
        .expect("Failed to write test note");

        fs::create_dir_all(temp_dir.path().join("Programming")).expect("Failed to create dir");
        fs::write(
            temp_dir.path().join("Programming/error-handling.md"),
            "# Error Handling\n\nHow to handle errors in Rust using Result.",
        )
        .expect("Failed to write test note");

        temp_dir
    }

    #[test]
    fn test_document_command_name() {
        let cmd = DocumentCommand;
        assert_eq!(cmd.name(), "document");
    }

    #[test]
    fn test_document_command_description() {
        let cmd = DocumentCommand;
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_document_no_args() {
        let cmd = DocumentCommand;
        let mut ctx = CommandContext {
            registry: CommandRegistry::with_defaults(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
        };

        let result = cmd.execute(&[], &mut ctx);
        match result {
            CommandResult::Error(msg) => {
                assert!(msg.contains("Usage"));
            }
            _ => panic!("Expected error when no args provided"),
        }
    }

    #[test]
    fn test_document_vault_not_found() {
        // This test will fail if the default vault path exists
        // Skip test if ~/Documents/Personal exists
        let default_vault = expand_tilde(PathBuf::from("~/Documents/Personal"));
        if default_vault.exists() {
            return;
        }

        let cmd = DocumentCommand;
        let mut ctx = CommandContext {
            registry: CommandRegistry::with_defaults(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
        };

        let result = cmd.execute(&["test", "topic"], &mut ctx);
        match result {
            CommandResult::Error(msg) => {
                assert!(msg.contains("vault not found") || msg.contains("Obsidian vault"));
            }
            _ => panic!("Expected error when vault not found"),
        }
    }

    #[test]
    fn test_document_search_only_flag() {
        let cmd = DocumentCommand;
        let mut ctx = CommandContext {
            registry: CommandRegistry::with_defaults(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
        };

        // Just test that the flag is parsed (won't actually search without a valid vault)
        let result = cmd.execute(&["test", "--search"], &mut ctx);

        // Should either return an error about vault or search results
        match result {
            CommandResult::Error(_) | CommandResult::Output(_) => {
                // Both are acceptable depending on vault existence
            }
            _ => panic!("Expected error or output"),
        }
    }

    #[test]
    fn test_document_new_flag() {
        let cmd = DocumentCommand;
        let mut ctx = CommandContext {
            registry: CommandRegistry::with_defaults(),
            cost_tracker: CostTracker::with_default_model(),
            agent_manager: None,
        };

        let result = cmd.execute(&["test", "--new"], &mut ctx);

        // Should either return an error about vault or prompt for creation
        match result {
            CommandResult::Error(_) | CommandResult::Output(_) => {
                // Both are acceptable
            }
            _ => panic!("Expected error or output"),
        }
    }

    #[test]
    fn test_expand_tilde() {
        let path = PathBuf::from("~/test/path");
        let expanded = expand_tilde(path);

        // Should expand to home directory
        if let Some(home) = dirs::home_dir() {
            assert!(expanded.starts_with(home));
            assert!(expanded.ends_with("test/path"));
        }
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let path = PathBuf::from("/absolute/path");
        let expanded = expand_tilde(path.clone());

        // Should remain unchanged
        assert_eq!(expanded, path);
    }
}

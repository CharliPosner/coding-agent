//! The /config command - open configuration file in editor

use super::{Command, CommandContext, CommandResult};
use crate::config::Config;
use std::env;
use std::process;

pub struct ConfigCommand;

impl Command for ConfigCommand {
    fn name(&self) -> &'static str {
        "config"
    }

    fn description(&self) -> &'static str {
        "Open config file in your editor"
    }

    fn execute(&self, _args: &[&str], _ctx: &mut CommandContext) -> CommandResult {
        // Get the config file path
        let config_path = match Config::default_path() {
            Ok(path) => path,
            Err(e) => {
                return CommandResult::Error(format!("Could not determine config path: {}", e))
            }
        };

        // Ensure config file exists
        if !config_path.exists() {
            // Create default config
            if let Err(e) = Config::default().save_to(&config_path) {
                return CommandResult::Error(format!("Failed to create config file: {}", e));
            }
        }

        // Determine editor to use
        let editor = env::var("EDITOR")
            .or_else(|_| env::var("VISUAL"))
            .unwrap_or_else(|_| {
                // Default editors based on platform
                if cfg!(target_os = "macos") {
                    "open -t".to_string()
                } else if cfg!(target_os = "windows") {
                    "notepad".to_string()
                } else {
                    "nano".to_string()
                }
            });

        // Open the config file in the editor
        let path_str = config_path.to_string_lossy();

        // Parse the editor command (may have flags like "code --wait")
        let parts: Vec<&str> = editor.split_whitespace().collect();
        if parts.is_empty() {
            return CommandResult::Error("No editor configured".to_string());
        }

        let (cmd, args) = parts.split_first().unwrap();

        let result = process::Command::new(cmd)
            .args(args)
            .arg(&*path_str)
            .spawn();

        match result {
            Ok(mut child) => {
                // Wait for editor to close (some editors need this)
                match child.wait() {
                    Ok(_) => CommandResult::Output(format!(
                        "Config file edited: {}\nRestart to apply changes.",
                        path_str
                    )),
                    Err(e) => {
                        // Some editors (like macOS 'open') return immediately
                        // This is fine, just report success
                        if e.kind() == std::io::ErrorKind::InvalidInput {
                            CommandResult::Output(format!("Opened config file: {}", path_str))
                        } else {
                            CommandResult::Output(format!("Opened config file: {}", path_str))
                        }
                    }
                }
            }
            Err(e) => CommandResult::Error(format!("Failed to open editor '{}': {}", cmd, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_command_name() {
        let cmd = ConfigCommand;
        assert_eq!(cmd.name(), "config");
    }

    #[test]
    fn test_config_command_description() {
        let cmd = ConfigCommand;
        assert!(!cmd.description().is_empty());
        assert!(cmd.description().to_lowercase().contains("config"));
    }
}

//! Unit tests for the bash tool functionality

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;

/// Input schema for the bash tool
#[derive(Debug, Deserialize, JsonSchema)]
struct BashInput {
    /// The bash command to execute
    command: String,
}

/// Execute a bash command and return its output
fn bash(input: Value) -> Result<String, String> {
    let bash_input: BashInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let output = Command::new("bash")
        .arg("-c")
        .arg(&bash_input.command)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        // Return stdout, or stderr if stdout is empty
        let result = if stdout.is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        Ok(result)
    } else {
        // Command failed - include both stdout and stderr in error
        let error_msg = format!(
            "Command failed with exit code: {}\nstdout: {}\nstderr: {}",
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim()
        );
        Err(error_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_echo_command() {
        // Test simple command "echo hello" - should return "hello"
        let input = json!({
            "command": "echo hello"
        });

        let result = bash(input);

        assert!(result.is_ok(), "Expected Ok result, got: {:?}", result);
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_command_with_exit_code_1() {
        // Test command with exit code 1 "exit 1" - should return error
        let input = json!({
            "command": "exit 1"
        });

        let result = bash(input);

        assert!(result.is_err(), "Expected Err result, got: {:?}", result);
        let error_msg = result.unwrap_err();
        assert!(
            error_msg.contains("Command failed with exit code: 1"),
            "Error message should contain exit code 1, got: {}",
            error_msg
        );
    }

    #[test]
    fn test_capturing_stderr() {
        // Test capturing stderr "ls nonexistent_file_xyz" - should capture error output
        let input = json!({
            "command": "ls nonexistent_file_xyz"
        });

        let result = bash(input);

        assert!(result.is_err(), "Expected Err result, got: {:?}", result);
        let error_msg = result.unwrap_err();
        // The error should contain the stderr output about the nonexistent file
        assert!(
            error_msg.contains("nonexistent_file_xyz"),
            "Error message should mention the nonexistent file, got: {}",
            error_msg
        );
        assert!(
            error_msg.contains("stderr:"),
            "Error message should include stderr section, got: {}",
            error_msg
        );
    }
}

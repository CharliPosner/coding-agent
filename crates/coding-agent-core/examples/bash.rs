//! Example: Agent with a bash tool
//!
//! This example demonstrates adding a bash tool to the agent that can execute
//! shell commands using `std::process::Command`.

use coding_agent_core::{generate_schema, Agent, ToolDefinition};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
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

fn main() {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Get API key from environment
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");

    // Check for verbose flag
    let verbose = std::env::args().any(|arg| arg == "--verbose" || arg == "-v");

    // Create the bash tool definition
    let bash_tool = ToolDefinition {
        name: "bash".to_string(),
        description:
            "Execute a bash command and return its output. Use this to run shell commands."
                .to_string(),
        input_schema: generate_schema::<BashInput>(),
        function: bash,
    };

    // Create and run the agent with the bash tool
    let agent = Agent::new(api_key, vec![bash_tool], verbose);

    if let Err(e) = agent.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

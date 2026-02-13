use coding_agent::{generate_schema, Agent, ToolDefinition};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::env;
use std::fs;

// ============================================================================
// ReadFile Tool - Read contents of a file
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileInput {
    /// The relative path of a file in the working directory.
    path: String,
}

fn read_file(input: Value) -> Result<String, String> {
    let input: ReadFileInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    eprintln!("Reading file: {}", input.path);

    let content =
        fs::read_to_string(&input.path).map_err(|e| format!("Failed to read file: {}", e))?;

    eprintln!(
        "Successfully read file {} ({} bytes)",
        input.path,
        content.len()
    );

    Ok(content)
}

// ============================================================================
// Main - Set up agent with read_file tool
// ============================================================================

fn main() {
    let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
    let verbose = env::args().any(|arg| arg == "--verbose");

    let tools = vec![ToolDefinition {
        name: "read_file".to_string(),
        description: "Read the contents of a given relative file path. Use this when you want to see what's inside a file. Do not use this with directory names.".to_string(),
        input_schema: generate_schema::<ReadFileInput>(),
        function: read_file,
    }];

    let agent = Agent::new(api_key, tools, verbose);
    if let Err(e) = agent.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

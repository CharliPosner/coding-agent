//! Code search example - search for code patterns using ripgrep (rg).
//!
//! Usage:
//!   cargo run --example code_search
//!   cargo run --example code_search -- --verbose
//!
//! Set ANTHROPIC_API_KEY environment variable before running.
//! Requires ripgrep (rg) to be installed on the system.

use coding_agent::{generate_schema, Agent, ToolDefinition};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::env;
use std::process::Command;

// ============================================================================
// CodeSearch Tool - Search for code patterns using ripgrep
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct CodeSearchInput {
    /// The search pattern or regex to look for
    pattern: String,
    /// Optional path to search in (file or directory)
    #[serde(default)]
    path: Option<String>,
    /// Optional file extension to limit search to (e.g., 'go', 'js', 'py')
    #[serde(default)]
    file_type: Option<String>,
    /// Whether the search should be case sensitive (default: false)
    #[serde(default)]
    case_sensitive: bool,
}

fn code_search(input: Value) -> Result<String, String> {
    let input: CodeSearchInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    if input.pattern.is_empty() {
        return Err("pattern is required".to_string());
    }

    eprintln!("Searching for pattern: {}", input.pattern);

    // Build ripgrep command
    let mut args = vec![
        "--line-number".to_string(),
        "--with-filename".to_string(),
        "--color=never".to_string(),
    ];

    // Add case sensitivity flag
    if !input.case_sensitive {
        args.push("--ignore-case".to_string());
    }

    // Add file type filter if specified
    if let Some(ref file_type) = input.file_type {
        args.push("--type".to_string());
        args.push(file_type.clone());
    }

    // Add pattern
    args.push(input.pattern.clone());

    // Add path if specified, otherwise search current directory
    args.push(input.path.unwrap_or_else(|| ".".to_string()));

    eprintln!("Executing ripgrep with args: {:?}", args);

    let output = Command::new("rg")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute ripgrep: {}", e))?;

    // ripgrep returns exit code 1 when no matches are found, which is not an error
    if !output.status.success() {
        if output.status.code() == Some(1) {
            eprintln!("No matches found for pattern: {}", input.pattern);
            return Ok("No matches found".to_string());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("search failed: {}", stderr));
    }

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let lines: Vec<&str> = result.lines().collect();

    eprintln!("Found {} matches for pattern: {}", lines.len(), input.pattern);

    // Limit output to 50 matches to prevent overwhelming responses
    if lines.len() > 50 {
        let truncated: String = lines[..50].join("\n");
        Ok(format!(
            "{}\n... (showing first 50 of {} matches)",
            truncated,
            lines.len()
        ))
    } else {
        Ok(result)
    }
}

// ============================================================================
// Main - Set up agent with code_search tool
// ============================================================================

fn main() {
    dotenvy::dotenv().ok();

    let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
    let verbose = env::args().any(|arg| arg == "--verbose" || arg == "-v");

    if verbose {
        eprintln!("[verbose] Verbose logging enabled");
    }

    let tools = vec![ToolDefinition {
        name: "code_search".to_string(),
        description: r#"Search for code patterns using ripgrep (rg).

Use this to find code patterns, function definitions, variable usage, or any text in the codebase.
You can search by pattern, file type, or directory."#
            .to_string(),
        input_schema: generate_schema::<CodeSearchInput>(),
        function: code_search,
    }];

    if verbose {
        eprintln!("[verbose] Agent initialized with code_search tool");
    }

    let agent = Agent::new(api_key, tools, verbose);
    if let Err(e) = agent.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

use coding_agent::{generate_schema, Agent, ToolDefinition};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use walkdir::WalkDir;

#[derive(Debug, Deserialize, JsonSchema)]
struct ListFilesInput {
    /// Optional relative path to list files from. Defaults to current directory if not provided.
    #[serde(default)]
    path: Option<String>,
}

fn list_files(input: Value) -> Result<String, String> {
    let input: ListFilesInput =
        serde_json::from_value(input).map_err(|e| format!("Invalid input: {}", e))?;

    let dir = input.path.unwrap_or_else(|| ".".to_string());

    let mut files: Vec<String> = Vec::new();

    for entry in WalkDir::new(&dir).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        name != ".git" && name != ".devenv"
    }) {
        let entry = entry.map_err(|e| format!("Error walking directory: {}", e))?;
        let path = entry.path();

        // Skip the root directory itself
        if path.to_string_lossy() == dir {
            continue;
        }

        // Get relative path from the starting directory
        let rel_path = path
            .strip_prefix(&dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        if entry.file_type().is_dir() {
            files.push(format!("{}/", rel_path));
        } else {
            files.push(rel_path);
        }
    }

    serde_json::to_string(&files).map_err(|e| format!("Failed to serialize result: {}", e))
}

fn main() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");

    let tools = vec![ToolDefinition {
        name: "list_files".to_string(),
        description: "List files and directories at a given path. If no path is provided, lists files in the current directory.".to_string(),
        input_schema: generate_schema::<ListFilesInput>(),
        function: list_files,
    }];

    let verbose = std::env::args().any(|arg| arg == "--verbose");
    let agent = Agent::new(api_key, tools, verbose);

    if let Err(e) = agent.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

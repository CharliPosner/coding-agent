//! Example: Agent with edit_file tool
//!
//! This example demonstrates how to add an edit_file tool to the agent.
//! The edit_file tool can:
//! - Replace text in existing files (old_str must appear exactly once)
//! - Create new files when old_str is empty and file doesn't exist
//! - Append to files when old_str is empty and file exists
//! - Create parent directories if needed

use coding_agent::{generate_schema, Agent, ToolDefinition};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

// ============================================================================
// Edit File Tool
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct EditFileInput {
    /// The path to the file
    path: String,
    /// Text to search for - must match exactly and must only have one match exactly
    old_str: String,
    /// Text to replace old_str with
    new_str: String,
}

fn edit_file(input: Value) -> Result<String, String> {
    let input: EditFileInput =
        serde_json::from_value(input).map_err(|e| format!("Invalid input: {}", e))?;

    // Validate: path must not be empty
    if input.path.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    // Validate: old_str and new_str must be different
    if input.old_str == input.new_str {
        return Err("old_str and new_str must be different".to_string());
    }

    let path = Path::new(&input.path);

    // Check if file exists
    if !path.exists() {
        // File doesn't exist - only allow creation if old_str is empty
        if input.old_str.is_empty() {
            return create_new_file(&input.path, &input.new_str);
        } else {
            return Err(format!("file '{}' does not exist", input.path));
        }
    }

    // File exists - read content
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let new_content = if input.old_str.is_empty() {
        // Append to file
        format!("{}{}", content, input.new_str)
    } else {
        // Replace old_str with new_str
        // First, validate that old_str appears exactly once
        let count = content.matches(&input.old_str).count();
        if count == 0 {
            return Err("old_str not found in file".to_string());
        }
        if count > 1 {
            return Err(format!(
                "old_str found {} times in file, must be unique",
                count
            ));
        }

        content.replacen(&input.old_str, &input.new_str, 1)
    };

    // Write the modified content
    fs::write(path, &new_content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok("OK".to_string())
}

fn create_new_file(file_path: &str, content: &str) -> Result<String, String> {
    let path = Path::new(file_path);

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    // Write the file
    fs::write(path, content).map_err(|e| format!("Failed to create file: {}", e))?;

    Ok(format!("Successfully created file {}", file_path))
}

fn edit_file_definition() -> ToolDefinition {
    ToolDefinition {
        name: "edit_file".to_string(),
        description: r#"Make edits to a text file.

Replaces 'old_str' with 'new_str' in the given file. 'old_str' and 'new_str' MUST be different from each other.

If the file specified with path doesn't exist, it will be created (only when old_str is empty).
"#
        .to_string(),
        input_schema: generate_schema::<EditFileInput>(),
        function: edit_file,
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    let verbose = std::env::args().any(|arg| arg == "--verbose" || arg == "-v");

    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");

    let tools = vec![edit_file_definition()];

    let agent = Agent::new(api_key, tools, verbose);

    if let Err(e) = agent.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_edit_file_replace() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "old_str": "World",
            "new_str": "Rust"
        });

        let result = edit_file(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "OK");

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, Rust!");
    }

    #[test]
    fn test_edit_file_old_str_not_found() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "old_str": "Goodbye",
            "new_str": "Hi"
        });

        let result = edit_file(input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "old_str not found in file");
    }

    #[test]
    fn test_edit_file_multiple_matches() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello Hello Hello").unwrap();

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "old_str": "Hello",
            "new_str": "Hi"
        });

        let result = edit_file(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("found 3 times"));
    }

    #[test]
    fn test_edit_file_create_new() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("new_file.txt");

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "old_str": "",
            "new_str": "New content"
        });

        let result = edit_file(input);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Successfully created"));

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "New content");
    }

    #[test]
    fn test_edit_file_create_with_directories() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("subdir/nested/new_file.txt");

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "old_str": "",
            "new_str": "Nested content"
        });

        let result = edit_file(input);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Nested content");
    }

    #[test]
    fn test_edit_file_append() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello").unwrap();

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "old_str": "",
            "new_str": ", World!"
        });

        let result = edit_file(input);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_edit_file_same_old_new() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello").unwrap();

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "old_str": "Hello",
            "new_str": "Hello"
        });

        let result = edit_file(input);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "old_str and new_str must be different"
        );
    }

    #[test]
    fn test_edit_file_empty_path() {
        let input = json!({
            "path": "",
            "old_str": "test",
            "new_str": "replaced"
        });

        let result = edit_file(input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "path cannot be empty");
    }

    #[test]
    fn test_edit_file_nonexistent_with_old_str() {
        let input = json!({
            "path": "/nonexistent/path/file.txt",
            "old_str": "something",
            "new_str": "replaced"
        });

        let result = edit_file(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }
}

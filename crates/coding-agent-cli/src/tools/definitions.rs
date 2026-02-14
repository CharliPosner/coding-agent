//! Tool definitions for the CLI.
//!
//! This module defines all the tools that Claude can use to interact with the codebase.
//! Each tool has a JSON schema for input validation and a function to execute the tool.

use crate::permissions::{PermissionChecker, PermissionDecision};
use coding_agent_core::{generate_schema, Tool, ToolDefinition};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

// ============================================================================
// ReadFile Tool
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileInput {
    /// The path of the file to read (relative to current directory or absolute).
    path: String,
}

fn read_file(input: Value) -> Result<String, String> {
    let input: ReadFileInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    if input.path.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    let content =
        fs::read_to_string(&input.path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Truncate very large files
    const MAX_SIZE: usize = 100_000;
    if content.len() > MAX_SIZE {
        let truncated = &content[..MAX_SIZE];
        let line_count = truncated.lines().count();
        Ok(format!(
            "{}\n\n... [Truncated: showing first {} characters, {} lines. File is {} bytes total]",
            truncated,
            MAX_SIZE,
            line_count,
            content.len()
        ))
    } else {
        Ok(content)
    }
}

// ============================================================================
// WriteFile Tool
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct WriteFileInput {
    /// The path of the file to write (relative to current directory or absolute).
    path: String,
    /// The content to write to the file.
    content: String,
}

fn write_file(input: Value) -> Result<String, String> {
    let input: WriteFileInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    if input.path.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    let path = Path::new(&input.path);

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    fs::write(path, &input.content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(format!(
        "Successfully wrote {} bytes to {}",
        input.content.len(),
        input.path
    ))
}

// ============================================================================
// EditFile Tool
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct EditFileInput {
    /// The path to the file.
    path: String,
    /// Text to search for - must match exactly and must only have one match exactly.
    old_str: String,
    /// Text to replace old_str with.
    new_str: String,
}

fn edit_file(input: Value) -> Result<String, String> {
    let input: EditFileInput =
        serde_json::from_value(input).map_err(|e| format!("Invalid input: {}", e))?;

    if input.path.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    if input.old_str == input.new_str {
        return Err("old_str and new_str must be different".to_string());
    }

    let path = Path::new(&input.path);

    // Check if file exists
    if !path.exists() {
        // File doesn't exist - only allow creation if old_str is empty
        if input.old_str.is_empty() {
            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }
            }
            fs::write(path, &input.new_str).map_err(|e| format!("Failed to create file: {}", e))?;
            return Ok(format!("Successfully created file {}", input.path));
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

    fs::write(path, &new_content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok("OK".to_string())
}

// ============================================================================
// ListFiles Tool
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct ListFilesInput {
    /// Optional path to list files from. Defaults to current directory if not provided.
    #[serde(default)]
    path: Option<String>,
}

fn list_files(input: Value) -> Result<String, String> {
    let input: ListFilesInput =
        serde_json::from_value(input).map_err(|e| format!("Invalid input: {}", e))?;

    let dir = input.path.unwrap_or_else(|| ".".to_string());

    let mut files: Vec<String> = Vec::new();

    for entry in WalkDir::new(&dir)
        .max_depth(3)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != ".devenv" && name != "target" && name != "node_modules"
        })
    {
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

        // Limit to 500 files to avoid overwhelming output
        if files.len() >= 500 {
            files.push("... (truncated, more files exist)".to_string());
            break;
        }
    }

    serde_json::to_string_pretty(&files).map_err(|e| format!("Failed to serialize result: {}", e))
}

// ============================================================================
// Bash Tool
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct BashInput {
    /// The bash command to execute.
    command: String,
}

fn bash(input: Value) -> Result<String, String> {
    let input: BashInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    if input.command.is_empty() {
        return Err("command cannot be empty".to_string());
    }

    // Check for dangerous commands
    let dangerous_patterns = ["rm -rf /", "rm -rf /*", "> /dev/sda", "mkfs", ":(){:|:&};:"];
    for pattern in &dangerous_patterns {
        if input.command.contains(pattern) {
            return Err(format!(
                "Refusing to execute potentially dangerous command containing '{}'",
                pattern
            ));
        }
    }

    let output = Command::new("bash")
        .arg("-c")
        .arg(&input.command)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        let result = if stdout.is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };

        // Truncate very long output
        const MAX_OUTPUT: usize = 50_000;
        if result.len() > MAX_OUTPUT {
            Ok(format!(
                "{}\n\n... [Truncated: showing first {} characters of {} total]",
                &result[..MAX_OUTPUT],
                MAX_OUTPUT,
                result.len()
            ))
        } else {
            Ok(result)
        }
    } else {
        Err(format!(
            "Command failed with exit code: {}\nstdout: {}\nstderr: {}",
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim()
        ))
    }
}

// ============================================================================
// CodeSearch Tool
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct CodeSearchInput {
    /// The search pattern or regex to look for.
    pattern: String,
    /// Optional path to search in (file or directory). Defaults to current directory.
    #[serde(default)]
    path: Option<String>,
    /// Optional file extension to limit search to (e.g., 'rs', 'js', 'py').
    #[serde(default)]
    file_type: Option<String>,
    /// Whether the search should be case sensitive (default: false).
    #[serde(default)]
    case_sensitive: bool,
}

fn code_search(input: Value) -> Result<String, String> {
    let input: CodeSearchInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    if input.pattern.is_empty() {
        return Err("pattern is required".to_string());
    }

    // Build ripgrep command
    let mut args = vec![
        "--line-number".to_string(),
        "--with-filename".to_string(),
        "--color=never".to_string(),
    ];

    if !input.case_sensitive {
        args.push("--ignore-case".to_string());
    }

    if let Some(ref file_type) = input.file_type {
        args.push("--type".to_string());
        args.push(file_type.clone());
    }

    args.push(input.pattern.clone());
    args.push(input.path.unwrap_or_else(|| ".".to_string()));

    let output = Command::new("rg")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute ripgrep: {}", e))?;

    // ripgrep returns exit code 1 when no matches are found
    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Ok("No matches found".to_string());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("search failed: {}", stderr));
    }

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let lines: Vec<&str> = result.lines().collect();

    // Limit output to 50 matches
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
// Tool Definitions
// ============================================================================

/// Create all tool definitions for the CLI.
pub fn create_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file. Use this when you want to see what's inside a file. Do not use this with directory names.".to_string(),
            input_schema: generate_schema::<ReadFileInput>(),
            function: read_file,
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Create a new file or overwrite an existing file with the given content. Use this when you need to create a new file from scratch.".to_string(),
            input_schema: generate_schema::<WriteFileInput>(),
            function: write_file,
        },
        ToolDefinition {
            name: "edit_file".to_string(),
            description: r#"Make edits to a text file. Replaces 'old_str' with 'new_str' in the given file. 'old_str' and 'new_str' MUST be different from each other. The old_str must appear exactly once in the file. If the file doesn't exist and old_str is empty, the file will be created with new_str as content."#.to_string(),
            input_schema: generate_schema::<EditFileInput>(),
            function: edit_file,
        },
        ToolDefinition {
            name: "list_files".to_string(),
            description: "List files and directories at a given path. If no path is provided, lists files in the current directory. Useful for exploring the codebase structure.".to_string(),
            input_schema: generate_schema::<ListFilesInput>(),
            function: list_files,
        },
        ToolDefinition {
            name: "bash".to_string(),
            description: "Execute a bash command and return its output. Use this to run shell commands like 'cargo build', 'npm install', 'git status', etc.".to_string(),
            input_schema: generate_schema::<BashInput>(),
            function: bash,
        },
        ToolDefinition {
            name: "code_search".to_string(),
            description: r#"Search for code patterns using ripgrep (rg). Use this to find code patterns, function definitions, variable usage, or any text in the codebase. You can filter by file type (e.g., 'rs', 'js', 'py')."#.to_string(),
            input_schema: generate_schema::<CodeSearchInput>(),
            function: code_search,
        },
    ]
}

/// Convert tool definitions to the format expected by the Claude API.
pub fn tool_definitions_to_api(definitions: &[ToolDefinition]) -> Vec<Tool> {
    definitions
        .iter()
        .map(|def| Tool {
            name: def.name.clone(),
            description: def.description.clone(),
            input_schema: def.input_schema.clone(),
        })
        .collect()
}

/// Execute a tool by name with the given input.
pub fn execute_tool(
    definitions: &[ToolDefinition],
    name: &str,
    input: Value,
) -> Result<String, String> {
    for def in definitions {
        if def.name == name {
            return (def.function)(input);
        }
    }
    Err(format!("Unknown tool: {}", name))
}

/// Execute a tool with permission checking for write operations.
///
/// This wraps `execute_tool` with permission checking for write/modify operations.
/// Read operations and bash commands are executed without permission checks.
///
/// Note: This function will return an error with category Permission if user confirmation
/// is needed. The caller should catch this and prompt the user, then retry the operation.
pub fn execute_tool_with_permissions(
    definitions: &[ToolDefinition],
    name: &str,
    input: Value,
    permission_checker: Option<&PermissionChecker>,
) -> Result<String, String> {
    // If no permission checker provided, execute without checks
    let Some(checker) = permission_checker else {
        return execute_tool(definitions, name, input);
    };

    // Check if this tool requires permission checking
    match name {
        "write_file" => {
            // Extract path from input
            let path_str = input
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "write_file requires 'path' parameter".to_string())?;
            let path = Path::new(path_str);

            // Check write permission
            match checker.check_write(path) {
                PermissionDecision::Allowed => execute_tool(definitions, name, input),
                PermissionDecision::Denied => {
                    Err(format!("Permission denied: Cannot write to {}", path_str))
                }
                PermissionDecision::NeedsPrompt => Err(format!(
                    "ErrorCategory::Permission|Writing to {} requires confirmation",
                    path_str
                )),
            }
        }
        "edit_file" => {
            // Extract path from input
            let path_str = input
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "edit_file requires 'path' parameter".to_string())?;
            let path = Path::new(path_str);

            // Check write permission (edit_file can create or modify)
            match checker.check_write(path) {
                PermissionDecision::Allowed => execute_tool(definitions, name, input),
                PermissionDecision::Denied => {
                    Err(format!("Permission denied: Cannot edit {}", path_str))
                }
                PermissionDecision::NeedsPrompt => Err(format!(
                    "ErrorCategory::Permission|Editing {} requires confirmation",
                    path_str
                )),
            }
        }
        // Other tools don't require permission checks
        // - read_file: reads are always allowed per spec
        // - list_files: only lists, doesn't modify
        // - bash: executing commands is a conscious decision
        // - code_search: only searches, doesn't modify
        _ => execute_tool(definitions, name, input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_tool_definitions_basic() {
        let definitions = create_tool_definitions();
        assert_eq!(definitions.len(), 6);

        let names: Vec<&str> = definitions.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"edit_file"));
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"code_search"));
    }

    #[test]
    fn test_tool_definitions_to_api_conversion() {
        let definitions = create_tool_definitions();
        let api_tools = tool_definitions_to_api(&definitions);

        assert_eq!(api_tools.len(), definitions.len());
        for (api, def) in api_tools.iter().zip(definitions.iter()) {
            assert_eq!(api.name, def.name);
            assert_eq!(api.description, def.description);
        }
    }

    #[test]
    fn test_read_file_basic_functionality() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let input = json!({ "path": file_path.to_str().unwrap() });
        let result = read_file(input);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_write_file_basic_functionality() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("output.txt");

        let input = json!({
            "path": file_path.to_str().unwrap(),
            "content": "Test content"
        });
        let result = write_file(input);
        
        assert!(result.is_ok());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Test content");
    }

    #[test]
    fn test_error_handling() {
        // Test file not found
        let result = read_file(json!({ "path": "/nonexistent/file.txt" }));
        assert!(result.is_err());

        // Test invalid input
        let result = read_file(json!({ "invalid": "input" }));
        assert!(result.is_err());
    }
}
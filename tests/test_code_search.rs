use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;

// ============================================================================
// CodeSearch Tool - Copied from examples/code_search.rs for testing
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

    let output = Command::new("rg")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute ripgrep: {}", e))?;

    // ripgrep returns exit code 1 when no matches are found, which is not an error
    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Ok("No matches found".to_string());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("search failed: {}", stderr));
    }

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let lines: Vec<&str> = result.lines().collect();

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
// Tests - Requires ripgrep (rg) to be installed
// ============================================================================

#[test]
fn test_find_fn_pattern_in_fixtures() {
    // Test finding pattern "fn " in fixtures/ directory - should return matching lines
    let input = json!({
        "pattern": "fn ",
        "path": "fixtures/"
    });

    let result = code_search(input);

    assert!(result.is_ok(), "Expected Ok result, got: {:?}", result);

    let content = result.unwrap();
    // Should find "fn main()" in main.rs and "fn add()" / "fn test_add()" in lib.rs
    assert!(
        content.contains("fn main"),
        "Expected to find 'fn main' in fixtures, got: {}",
        content
    );
    assert!(
        content.contains("fn add"),
        "Expected to find 'fn add' in fixtures, got: {}",
        content
    );
    // Verify line numbers are included
    assert!(
        content.contains(":"),
        "Expected output to contain ':' for filename:line format, got: {}",
        content
    );
}

#[test]
fn test_pattern_with_no_matches() {
    // Test pattern with no matches "zzzznotfound" - should return "No matches found"
    let input = json!({
        "pattern": "zzzznotfound",
        "path": "fixtures/"
    });

    let result = code_search(input);

    assert!(result.is_ok(), "Expected Ok result, got: {:?}", result);

    let content = result.unwrap();
    assert_eq!(
        content, "No matches found",
        "Expected 'No matches found' for non-existent pattern, got: {}",
        content
    );
}

#[test]
fn test_search_with_file_type_filter() {
    // Test with file_type filter - search for "function" with file_type "js" in fixtures/
    let input = json!({
        "pattern": "function",
        "path": "fixtures/",
        "file_type": "js"
    });

    let result = code_search(input);

    assert!(result.is_ok(), "Expected Ok result, got: {:?}", result);

    let content = result.unwrap();
    // Should find "function fizzbuzz" in fizzbuzz.js
    assert!(
        content.contains("function fizzbuzz"),
        "Expected to find 'function fizzbuzz' in js files, got: {}",
        content
    );
    assert!(
        content.contains("fizzbuzz.js"),
        "Expected results from fizzbuzz.js, got: {}",
        content
    );
    // Should NOT find anything from .rs files
    assert!(
        !content.contains(".rs"),
        "Expected no .rs files when filtering by js type, got: {}",
        content
    );
}

#[test]
fn test_empty_pattern_returns_error() {
    // Test that empty pattern returns an error
    let input = json!({
        "pattern": "",
        "path": "fixtures/"
    });

    let result = code_search(input);

    assert!(result.is_err(), "Expected Err result for empty pattern");

    let error_message = result.unwrap_err();
    assert_eq!(
        error_message, "pattern is required",
        "Expected 'pattern is required' error, got: {}",
        error_message
    );
}

#[test]
fn test_case_insensitive_search() {
    // Test case insensitive search (default behavior)
    let input = json!({
        "pattern": "FIZZBUZZ",
        "path": "fixtures/"
    });

    let result = code_search(input);

    assert!(result.is_ok(), "Expected Ok result, got: {:?}", result);

    let content = result.unwrap();
    // Should find matches even though case doesn't match
    assert!(
        content.contains("FizzBuzz") || content.contains("fizzbuzz"),
        "Expected case insensitive match for 'FIZZBUZZ', got: {}",
        content
    );
}

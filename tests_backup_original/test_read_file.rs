use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;

// ============================================================================
// ReadFile Tool - Copied from examples/read.rs for testing
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadFileInput {
    /// The relative path of a file in the working directory.
    path: String,
}

fn read_file(input: Value) -> Result<String, String> {
    let input: ReadFileInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let content =
        fs::read_to_string(&input.path).map_err(|e| format!("Failed to read file: {}", e))?;

    Ok(content)
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_read_existing_file_fizzbuzz() {
    let input = json!({
        "path": "fixtures/fizzbuzz.js"
    });

    let result = read_file(input);

    assert!(result.is_ok(), "Expected Ok result, got: {:?}", result);

    let content = result.unwrap();
    assert!(
        content.contains("function fizzbuzz"),
        "Expected content to contain 'function fizzbuzz'"
    );
    assert!(
        content.contains("FizzBuzz"),
        "Expected content to contain 'FizzBuzz'"
    );
}

#[test]
fn test_read_nonexistent_file() {
    let input = json!({
        "path": "fixtures/nonexistent_file.txt"
    });

    let result = read_file(input);

    assert!(result.is_err(), "Expected Err result for non-existent file");

    let error_message = result.unwrap_err();
    assert!(
        error_message.contains("Failed to read file"),
        "Expected error message to contain 'Failed to read file', got: {}",
        error_message
    );
}

#[test]
fn test_read_existing_file_riddle() {
    let input = json!({
        "path": "fixtures/riddle.txt"
    });

    let result = read_file(input);

    assert!(result.is_ok(), "Expected Ok result, got: {:?}", result);

    let content = result.unwrap();
    assert!(
        content.contains("What has keys but no locks"),
        "Expected content to contain riddle question"
    );
    assert!(
        content.contains("A keyboard!"),
        "Expected content to contain riddle answer"
    );
}

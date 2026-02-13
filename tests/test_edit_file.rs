//! Unit tests for the edit_file function
//!
//! Tests cover:
//! - Replacing text (old_str found once)
//! - Text not found error
//! - Multiple matches error (ambiguous)
//! - Creating new file when old_str is empty
//! - Creating file with parent directories

use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

// ============================================================================
// Edit File Implementation (copied from examples/edit.rs for testing)
// ============================================================================

#[derive(Debug, Deserialize)]
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

// ============================================================================
// Tests
// ============================================================================

/// Test 1: Replacing text - old="foo" new="bar" in file containing "foo" should work
#[test]
fn test_replacing_text_foo_to_bar() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "foo").unwrap();

    let input = json!({
        "path": file_path.to_str().unwrap(),
        "old_str": "foo",
        "new_str": "bar"
    });

    let result = edit_file(input);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    assert_eq!(result.unwrap(), "OK");

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "bar");
}

/// Test 2: Text not found - old="xyz" not in file should return error
#[test]
fn test_text_not_found_returns_error() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello world").unwrap();

    let input = json!({
        "path": file_path.to_str().unwrap(),
        "old_str": "xyz",
        "new_str": "replaced"
    });

    let result = edit_file(input);
    assert!(result.is_err(), "Expected Err, got {:?}", result);
    assert_eq!(result.unwrap_err(), "old_str not found in file");
}

/// Test 3: Multiple matches - old="the" appearing 5x should return error (ambiguous)
#[test]
fn test_multiple_matches_returns_error() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    // Create content with "the" appearing exactly 5 times
    fs::write(&file_path, "the the the the the").unwrap();

    let input = json!({
        "path": file_path.to_str().unwrap(),
        "old_str": "the",
        "new_str": "a"
    });

    let result = edit_file(input);
    assert!(result.is_err(), "Expected Err, got {:?}", result);
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("found 5 times"),
        "Expected error message to contain 'found 5 times', got: {}",
        err_msg
    );
    assert!(
        err_msg.contains("must be unique"),
        "Expected error message to contain 'must be unique', got: {}",
        err_msg
    );
}

/// Test 4: Creating new file - old="" new="content" with non-existent file should create it
#[test]
fn test_create_new_file_with_empty_old_str() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("new_file.txt");

    // Verify file does not exist before test
    assert!(!file_path.exists(), "File should not exist before test");

    let input = json!({
        "path": file_path.to_str().unwrap(),
        "old_str": "",
        "new_str": "content"
    });

    let result = edit_file(input);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    assert!(
        result.unwrap().contains("Successfully created"),
        "Expected success message for file creation"
    );

    // Verify file was created with correct content
    assert!(file_path.exists(), "File should exist after creation");
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "content");
}

/// Test 5: Creating file with parent directories - path "a/b/c.txt" should create parent dirs
#[test]
fn test_create_file_with_parent_directories() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("a/b/c.txt");
    let parent_dir = dir.path().join("a/b");

    // Verify parent directories and file do not exist before test
    assert!(
        !parent_dir.exists(),
        "Parent directory should not exist before test"
    );
    assert!(!file_path.exists(), "File should not exist before test");

    let input = json!({
        "path": file_path.to_str().unwrap(),
        "old_str": "",
        "new_str": "nested content"
    });

    let result = edit_file(input);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);

    // Verify parent directories were created
    assert!(
        parent_dir.exists(),
        "Parent directories should be created"
    );
    assert!(
        parent_dir.is_dir(),
        "Parent path should be a directory"
    );

    // Verify file was created with correct content
    assert!(file_path.exists(), "File should exist after creation");
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "nested content");
}

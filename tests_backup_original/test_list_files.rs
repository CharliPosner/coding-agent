use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_files_sample_project() {
        let input = json!({
            "path": "fixtures/sample_project"
        });

        let result = list_files(input).expect("list_files should succeed");
        let files: Vec<String> = serde_json::from_str(&result).expect("Should parse JSON");

        assert!(
            files.contains(&"main.rs".to_string()),
            "Should contain main.rs, got: {:?}",
            files
        );
        assert!(
            files.contains(&"lib.rs".to_string()),
            "Should contain lib.rs, got: {:?}",
            files
        );
    }

    #[test]
    fn test_list_files_nonexistent_directory() {
        let input = json!({
            "path": "this/directory/does/not/exist"
        });

        let result = list_files(input);
        assert!(
            result.is_err(),
            "Should return error for non-existent directory"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("Error walking directory"),
            "Error message should mention walking directory, got: {}",
            error
        );
    }

    #[test]
    fn test_list_files_skips_git_directory() {
        // Create a temporary directory with a .git folder inside
        let temp = tempdir().expect("Failed to create temp directory");
        let temp_path = temp.path();

        // Create .git directory
        let git_dir = temp_path.join(".git");
        fs::create_dir(&git_dir).expect("Failed to create .git directory");

        // Create a file inside .git
        fs::write(git_dir.join("config"), "git config content")
            .expect("Failed to write .git/config");

        // Create a regular file outside .git
        fs::write(temp_path.join("regular_file.txt"), "regular content")
            .expect("Failed to write regular_file.txt");

        // Create a subdirectory with a file
        let subdir = temp_path.join("subdir");
        fs::create_dir(&subdir).expect("Failed to create subdir");
        fs::write(subdir.join("nested.txt"), "nested content")
            .expect("Failed to write nested.txt");

        let input = json!({
            "path": temp_path.to_string_lossy()
        });

        let result = list_files(input).expect("list_files should succeed");
        let files: Vec<String> = serde_json::from_str(&result).expect("Should parse JSON");

        // Should contain regular files
        assert!(
            files.contains(&"regular_file.txt".to_string()),
            "Should contain regular_file.txt, got: {:?}",
            files
        );
        assert!(
            files.contains(&"subdir/".to_string()),
            "Should contain subdir/, got: {:?}",
            files
        );

        // Should NOT contain .git or anything inside it
        for file in &files {
            assert!(
                !file.contains(".git"),
                "Should not contain .git paths, but found: {}",
                file
            );
        }
    }
}

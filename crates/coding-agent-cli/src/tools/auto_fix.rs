//! Auto-fix application for self-healing error recovery.
//!
//! This module provides the actual fix application logic for various error types.
//! It works with the diagnostics module to parse errors and apply appropriate fixes.

use crate::tools::{FixInfo, FixType};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Result of applying a fix.
#[derive(Debug, Clone)]
pub struct FixApplicationResult {
    /// Whether the fix was applied successfully.
    pub success: bool,

    /// Files that were modified.
    pub modified_files: Vec<PathBuf>,

    /// Description of what was done.
    pub description: String,

    /// Error message if the fix failed.
    pub error: Option<String>,

    /// The original content of modified files (for rollback).
    pub original_content: HashMap<PathBuf, String>,
}

impl FixApplicationResult {
    /// Create a successful result.
    pub fn success(modified_files: Vec<PathBuf>, description: impl Into<String>) -> Self {
        Self {
            success: true,
            modified_files,
            description: description.into(),
            error: None,
            original_content: HashMap::new(),
        }
    }

    /// Create a successful result with rollback support.
    pub fn success_with_rollback(
        modified_files: Vec<PathBuf>,
        description: impl Into<String>,
        original_content: HashMap<PathBuf, String>,
    ) -> Self {
        Self {
            success: true,
            modified_files,
            description: description.into(),
            error: None,
            original_content,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            modified_files: vec![],
            description: String::new(),
            error: Some(error.into()),
            original_content: HashMap::new(),
        }
    }

    /// Rollback the applied fix by restoring original content.
    pub fn rollback(&self) -> Result<(), String> {
        for (path, content) in &self.original_content {
            fs::write(path, content).map_err(|e| format!("Failed to rollback {}: {}", path.display(), e))?;
        }
        Ok(())
    }
}

/// Configuration for auto-fix application.
#[derive(Debug, Clone)]
pub struct AutoFixConfig {
    /// Root directory for searching files.
    pub root_dir: PathBuf,

    /// Whether to create backup files before modifying.
    pub create_backups: bool,

    /// Whether to run in dry-run mode (no actual modifications).
    pub dry_run: bool,
}

impl Default for AutoFixConfig {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("."),
            create_backups: false,
            dry_run: false,
        }
    }
}

impl AutoFixConfig {
    /// Create a new config with the given root directory.
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
            ..Default::default()
        }
    }

    /// Enable dry-run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable backup creation.
    pub fn with_backups(mut self, create_backups: bool) -> Self {
        self.create_backups = create_backups;
        self
    }
}

/// Apply a fix based on the provided FixInfo.
///
/// This is the main entry point for applying auto-fixes. It dispatches to the
/// appropriate fix handler based on the fix type.
pub fn apply_fix(fix_info: &FixInfo, config: &AutoFixConfig) -> FixApplicationResult {
    match fix_info.fix_type {
        FixType::AddDependency => apply_add_dependency_fix(fix_info, config),
        FixType::AddImport => apply_add_import_fix(fix_info, config),
        FixType::FixType => apply_type_fix(fix_info, config),
        FixType::FixSyntax => apply_syntax_fix(fix_info, config),
    }
}

/// Apply a fix for a missing dependency.
///
/// This adds the dependency to Cargo.toml (or package.json for JS/TS projects).
fn apply_add_dependency_fix(fix_info: &FixInfo, config: &AutoFixConfig) -> FixApplicationResult {
    let dep_name = match &fix_info.target_item {
        Some(name) => name.clone(),
        None => return FixApplicationResult::failure("No dependency name specified"),
    };

    // Determine the manifest file
    let manifest_path = config.root_dir.join("Cargo.toml");
    if manifest_path.exists() {
        return apply_cargo_dependency(&manifest_path, &dep_name, config);
    }

    let package_json_path = config.root_dir.join("package.json");
    if package_json_path.exists() {
        return apply_npm_dependency(&package_json_path, &dep_name, config);
    }

    FixApplicationResult::failure("No Cargo.toml or package.json found in project root")
}

/// Add a dependency to Cargo.toml.
fn apply_cargo_dependency(
    manifest_path: &Path,
    dep_name: &str,
    config: &AutoFixConfig,
) -> FixApplicationResult {
    let original_content = match fs::read_to_string(manifest_path) {
        Ok(content) => content,
        Err(e) => return FixApplicationResult::failure(format!("Failed to read Cargo.toml: {}", e)),
    };

    // Parse the TOML
    let mut doc = match original_content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => doc,
        Err(e) => return FixApplicationResult::failure(format!("Failed to parse Cargo.toml: {}", e)),
    };

    // Check if dependencies section exists
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    // Check if dependency already exists
    if doc["dependencies"]
        .as_table()
        .map(|t| t.contains_key(dep_name))
        .unwrap_or(false)
    {
        return FixApplicationResult::failure(format!("Dependency '{}' already exists in Cargo.toml", dep_name));
    }

    // Normalize the crate name (replace underscores with hyphens for common crates)
    let crate_name = normalize_crate_name(dep_name);

    // Add the dependency with a default version
    let version = get_suggested_version(&crate_name);
    doc["dependencies"][&crate_name] = toml_edit::value(version.clone());

    // Get the new content
    let new_content = doc.to_string();

    if config.dry_run {
        return FixApplicationResult::success(
            vec![manifest_path.to_path_buf()],
            format!("Would add dependency: {} = \"{}\"", crate_name, version),
        );
    }

    // Create backup if configured
    if config.create_backups {
        let backup_path = manifest_path.with_extension("toml.bak");
        if let Err(e) = fs::write(&backup_path, &original_content) {
            return FixApplicationResult::failure(format!("Failed to create backup: {}", e));
        }
    }

    // Write the modified content
    if let Err(e) = fs::write(manifest_path, &new_content) {
        return FixApplicationResult::failure(format!("Failed to write Cargo.toml: {}", e));
    }

    let mut original_content_map = HashMap::new();
    original_content_map.insert(manifest_path.to_path_buf(), original_content);

    FixApplicationResult::success_with_rollback(
        vec![manifest_path.to_path_buf()],
        format!("Added dependency: {} = \"{}\"", crate_name, version),
        original_content_map,
    )
}

/// Add a dependency to package.json.
fn apply_npm_dependency(
    package_json_path: &Path,
    dep_name: &str,
    config: &AutoFixConfig,
) -> FixApplicationResult {
    let original_content = match fs::read_to_string(package_json_path) {
        Ok(content) => content,
        Err(e) => return FixApplicationResult::failure(format!("Failed to read package.json: {}", e)),
    };

    // Parse the JSON
    let mut package: serde_json::Value = match serde_json::from_str(&original_content) {
        Ok(v) => v,
        Err(e) => return FixApplicationResult::failure(format!("Failed to parse package.json: {}", e)),
    };

    // Ensure dependencies object exists
    if package.get("dependencies").is_none() {
        package["dependencies"] = serde_json::json!({});
    }

    // Check if dependency already exists
    if package["dependencies"].get(dep_name).is_some() {
        return FixApplicationResult::failure(format!(
            "Dependency '{}' already exists in package.json",
            dep_name
        ));
    }

    // Add the dependency with "latest" version (user should run npm install to get actual version)
    package["dependencies"][dep_name] = serde_json::json!("*");

    // Serialize back to JSON with pretty printing
    let new_content = match serde_json::to_string_pretty(&package) {
        Ok(s) => s,
        Err(e) => return FixApplicationResult::failure(format!("Failed to serialize package.json: {}", e)),
    };

    if config.dry_run {
        return FixApplicationResult::success(
            vec![package_json_path.to_path_buf()],
            format!("Would add dependency: {} to package.json", dep_name),
        );
    }

    // Create backup if configured
    if config.create_backups {
        let backup_path = package_json_path.with_extension("json.bak");
        if let Err(e) = fs::write(&backup_path, &original_content) {
            return FixApplicationResult::failure(format!("Failed to create backup: {}", e));
        }
    }

    // Write the modified content
    if let Err(e) = fs::write(package_json_path, &new_content) {
        return FixApplicationResult::failure(format!("Failed to write package.json: {}", e));
    }

    let mut original_content_map = HashMap::new();
    original_content_map.insert(package_json_path.to_path_buf(), original_content);

    FixApplicationResult::success_with_rollback(
        vec![package_json_path.to_path_buf()],
        format!("Added dependency: {} to package.json", dep_name),
        original_content_map,
    )
}

/// Apply a fix for a missing import.
///
/// This adds the appropriate use/import statement to the source file.
fn apply_add_import_fix(fix_info: &FixInfo, config: &AutoFixConfig) -> FixApplicationResult {
    let item_name = match &fix_info.target_item {
        Some(name) => name.clone(),
        None => return FixApplicationResult::failure("No item name specified for import"),
    };

    let target_file = match &fix_info.target_file {
        Some(path) => config.root_dir.join(path),
        None => return FixApplicationResult::failure("No target file specified for import fix"),
    };

    if !target_file.exists() {
        return FixApplicationResult::failure(format!("Target file does not exist: {}", target_file.display()));
    }

    // Determine the file type and apply appropriate import
    let extension = target_file.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "rs" => apply_rust_import(&target_file, &item_name, config),
        "ts" | "tsx" | "js" | "jsx" => apply_js_import(&target_file, &item_name, config),
        "go" => apply_go_import(&target_file, &item_name, config),
        _ => FixApplicationResult::failure(format!("Unsupported file type for import fix: {}", extension)),
    }
}

/// Add a use statement to a Rust file.
fn apply_rust_import(
    file_path: &Path,
    item_name: &str,
    config: &AutoFixConfig,
) -> FixApplicationResult {
    let original_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => return FixApplicationResult::failure(format!("Failed to read {}: {}", file_path.display(), e)),
    };

    // Try to find the right import path for common items
    let import_path = get_rust_import_path(item_name);

    // Check if import already exists
    if original_content.contains(&format!("use {}", import_path))
        || original_content.contains(&format!("use {};", import_path))
    {
        return FixApplicationResult::failure(format!("Import '{}' already exists", import_path));
    }

    // Find the best insertion point (after other use statements, or at the top)
    let new_content = insert_rust_import(&original_content, &import_path);

    if config.dry_run {
        return FixApplicationResult::success(
            vec![file_path.to_path_buf()],
            format!("Would add import: use {};", import_path),
        );
    }

    // Create backup if configured
    if config.create_backups {
        let backup_path = file_path.with_extension("rs.bak");
        if let Err(e) = fs::write(&backup_path, &original_content) {
            return FixApplicationResult::failure(format!("Failed to create backup: {}", e));
        }
    }

    // Write the modified content
    if let Err(e) = fs::write(file_path, &new_content) {
        return FixApplicationResult::failure(format!("Failed to write {}: {}", file_path.display(), e));
    }

    let mut original_content_map = HashMap::new();
    original_content_map.insert(file_path.to_path_buf(), original_content);

    FixApplicationResult::success_with_rollback(
        vec![file_path.to_path_buf()],
        format!("Added import: use {};", import_path),
        original_content_map,
    )
}

/// Insert a use statement into Rust source code at the appropriate location.
fn insert_rust_import(content: &str, import_path: &str) -> String {
    let use_statement = format!("use {};\n", import_path);

    // Find the last use statement and insert after it
    let lines: Vec<&str> = content.lines().collect();
    let mut last_use_line = None;
    let mut last_mod_line = None;
    let mut first_code_line = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") {
            last_use_line = Some(i);
        } else if trimmed.starts_with("mod ") {
            last_mod_line = Some(i);
        } else if !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("/*")
            && !trimmed.starts_with("*")
            && !trimmed.starts_with("#")
            && first_code_line.is_none()
        {
            first_code_line = Some(i);
        }
    }

    // Determine insertion point: after last use, or after last mod, or at the start
    let insert_after = last_use_line.or(last_mod_line);

    let mut result = String::new();

    if let Some(insert_line) = insert_after {
        for (i, line) in lines.iter().enumerate() {
            result.push_str(line);
            result.push('\n');
            if i == insert_line {
                result.push_str(&use_statement);
            }
        }
    } else {
        // Insert at the beginning (after any initial comments/attributes)
        let mut inserted = false;
        for line in lines.iter() {
            let trimmed = line.trim();
            if !inserted
                && !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("*")
                && !trimmed.starts_with("#")
            {
                result.push_str(&use_statement);
                inserted = true;
            }
            result.push_str(line);
            result.push('\n');
        }
        if !inserted {
            result.push_str(&use_statement);
        }
    }

    // Remove trailing extra newline if content didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Get the import path for a common Rust item.
fn get_rust_import_path(item_name: &str) -> String {
    // Map of common types to their import paths
    let common_imports: HashMap<&str, &str> = [
        ("HashMap", "std::collections::HashMap"),
        ("HashSet", "std::collections::HashSet"),
        ("BTreeMap", "std::collections::BTreeMap"),
        ("BTreeSet", "std::collections::BTreeSet"),
        ("VecDeque", "std::collections::VecDeque"),
        ("BinaryHeap", "std::collections::BinaryHeap"),
        ("LinkedList", "std::collections::LinkedList"),
        ("Rc", "std::rc::Rc"),
        ("Arc", "std::sync::Arc"),
        ("Mutex", "std::sync::Mutex"),
        ("RwLock", "std::sync::RwLock"),
        ("Cell", "std::cell::Cell"),
        ("RefCell", "std::cell::RefCell"),
        ("Pin", "std::pin::Pin"),
        ("PathBuf", "std::path::PathBuf"),
        ("Path", "std::path::Path"),
        ("File", "std::fs::File"),
        ("Read", "std::io::Read"),
        ("Write", "std::io::Write"),
        ("BufRead", "std::io::BufRead"),
        ("BufReader", "std::io::BufReader"),
        ("BufWriter", "std::io::BufWriter"),
        ("Cursor", "std::io::Cursor"),
        ("Duration", "std::time::Duration"),
        ("Instant", "std::time::Instant"),
        ("SystemTime", "std::time::SystemTime"),
        ("Error", "std::error::Error"),
        ("Display", "std::fmt::Display"),
        ("Debug", "std::fmt::Debug"),
        ("Formatter", "std::fmt::Formatter"),
        ("Result", "std::result::Result"),
        ("Option", "std::option::Option"),
        ("PhantomData", "std::marker::PhantomData"),
        ("NonNull", "std::ptr::NonNull"),
        ("Ordering", "std::cmp::Ordering"),
        ("Reverse", "std::cmp::Reverse"),
    ]
    .into_iter()
    .collect();

    common_imports
        .get(item_name)
        .map(|s| s.to_string())
        .unwrap_or_else(|| item_name.to_string())
}

/// Add an import statement to a JavaScript/TypeScript file.
fn apply_js_import(
    file_path: &Path,
    item_name: &str,
    config: &AutoFixConfig,
) -> FixApplicationResult {
    let original_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => return FixApplicationResult::failure(format!("Failed to read {}: {}", file_path.display(), e)),
    };

    // For JS/TS, we can't easily determine the source module without more context
    // Just add a TODO comment for now
    let import_statement = format!("// TODO: Add import for '{}'\n", item_name);

    // Check if a similar comment already exists
    if original_content.contains(&format!("import for '{}'", item_name)) {
        return FixApplicationResult::failure(format!("Import marker for '{}' already exists", item_name));
    }

    let new_content = format!("{}{}", import_statement, original_content);

    if config.dry_run {
        return FixApplicationResult::success(
            vec![file_path.to_path_buf()],
            format!("Would add import marker for: {}", item_name),
        );
    }

    // Create backup if configured
    if config.create_backups {
        let backup_ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("bak");
        let backup_path = file_path.with_extension(format!("{}.bak", backup_ext));
        if let Err(e) = fs::write(&backup_path, &original_content) {
            return FixApplicationResult::failure(format!("Failed to create backup: {}", e));
        }
    }

    // Write the modified content
    if let Err(e) = fs::write(file_path, &new_content) {
        return FixApplicationResult::failure(format!("Failed to write {}: {}", file_path.display(), e));
    }

    let mut original_content_map = HashMap::new();
    original_content_map.insert(file_path.to_path_buf(), original_content);

    FixApplicationResult::success_with_rollback(
        vec![file_path.to_path_buf()],
        format!("Added import marker for: {}", item_name),
        original_content_map,
    )
}

/// Add an import statement to a Go file.
fn apply_go_import(
    file_path: &Path,
    item_name: &str,
    config: &AutoFixConfig,
) -> FixApplicationResult {
    let original_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => return FixApplicationResult::failure(format!("Failed to read {}: {}", file_path.display(), e)),
    };

    // For Go, we can guess some common packages
    let import_path = get_go_import_path(item_name);

    // Check if import already exists
    if original_content.contains(&format!("\"{}\"", import_path)) {
        return FixApplicationResult::failure(format!("Import '{}' already exists", import_path));
    }

    let new_content = insert_go_import(&original_content, &import_path);

    if config.dry_run {
        return FixApplicationResult::success(
            vec![file_path.to_path_buf()],
            format!("Would add import: \"{}\"", import_path),
        );
    }

    // Create backup if configured
    if config.create_backups {
        let backup_path = file_path.with_extension("go.bak");
        if let Err(e) = fs::write(&backup_path, &original_content) {
            return FixApplicationResult::failure(format!("Failed to create backup: {}", e));
        }
    }

    // Write the modified content
    if let Err(e) = fs::write(file_path, &new_content) {
        return FixApplicationResult::failure(format!("Failed to write {}: {}", file_path.display(), e));
    }

    let mut original_content_map = HashMap::new();
    original_content_map.insert(file_path.to_path_buf(), original_content);

    FixApplicationResult::success_with_rollback(
        vec![file_path.to_path_buf()],
        format!("Added import: \"{}\"", import_path),
        original_content_map,
    )
}

/// Get the import path for a Go item.
fn get_go_import_path(item_name: &str) -> String {
    // Map of common Go types/packages
    let common_imports: HashMap<&str, &str> = [
        ("Context", "context"),
        ("Mutex", "sync"),
        ("RWMutex", "sync"),
        ("WaitGroup", "sync"),
        ("Time", "time"),
        ("Duration", "time"),
        ("Reader", "io"),
        ("Writer", "io"),
        ("File", "os"),
        ("Printf", "fmt"),
        ("Sprintf", "fmt"),
        ("Println", "fmt"),
        ("Error", "errors"),
        ("New", "errors"),
        ("Marshal", "encoding/json"),
        ("Unmarshal", "encoding/json"),
    ]
    .into_iter()
    .collect();

    common_imports
        .get(item_name)
        .map(|s| s.to_string())
        .unwrap_or_else(|| item_name.to_lowercase())
}

/// Insert an import into Go source code.
fn insert_go_import(content: &str, import_path: &str) -> String {
    let import_statement = format!("\t\"{}\"\n", import_path);

    // Find existing import block or create one
    if let Some(import_start) = content.find("import (") {
        if let Some(import_end) = content[import_start..].find(')') {
            // Insert into existing import block
            let insert_pos = import_start + import_end;
            let mut result = String::new();
            result.push_str(&content[..insert_pos]);
            result.push_str(&import_statement);
            result.push_str(&content[insert_pos..]);
            return result;
        }
    }

    // Check for single import statement
    if content.contains("import \"") {
        // Convert to import block - find the import line and expand it
        let lines: Vec<&str> = content.lines().collect();
        let mut result = String::new();
        let mut replaced = false;

        for line in lines {
            if !replaced && line.trim().starts_with("import \"") {
                // Extract existing import
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let existing = &line[start + 1..start + 1 + end];
                        result.push_str("import (\n");
                        result.push_str(&format!("\t\"{}\"\n", existing));
                        result.push_str(&import_statement);
                        result.push_str(")\n");
                        replaced = true;
                        continue;
                    }
                }
            }
            result.push_str(line);
            result.push('\n');
        }

        return result;
    }

    // No import found, add one after package declaration
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::new();
    let mut added = false;

    for line in lines {
        result.push_str(line);
        result.push('\n');
        if !added && line.trim().starts_with("package ") {
            result.push('\n');
            result.push_str("import (\n");
            result.push_str(&import_statement);
            result.push_str(")\n");
            added = true;
        }
    }

    result
}

/// Apply a type fix (placeholder - requires more context to implement).
fn apply_type_fix(_fix_info: &FixInfo, _config: &AutoFixConfig) -> FixApplicationResult {
    // Type fixes are complex and typically require more context about what
    // conversion or annotation is needed. This is a placeholder that returns
    // a failure indicating manual intervention is needed.
    FixApplicationResult::failure(
        "Type fixes require manual intervention. Please review the error message and fix the type mismatch.",
    )
}

/// Apply a syntax fix (placeholder - requires more context to implement).
fn apply_syntax_fix(_fix_info: &FixInfo, _config: &AutoFixConfig) -> FixApplicationResult {
    // Syntax fixes are complex and require precise knowledge of what's wrong.
    // This is a placeholder that returns a failure indicating manual intervention.
    FixApplicationResult::failure(
        "Syntax fixes require manual intervention. Please review the error message and fix the syntax error.",
    )
}

/// Normalize a crate name (underscores to hyphens for cargo).
fn normalize_crate_name(name: &str) -> String {
    // Common crates that use hyphens in cargo but underscores in code
    let underscore_to_hyphen = [
        "serde_json",
        "serde_derive",
        "serde_yaml",
        "tokio_util",
        "tower_http",
        "tracing_subscriber",
    ];

    if underscore_to_hyphen.contains(&name) {
        name.replace('_', "-")
    } else {
        name.to_string()
    }
}

/// Get a suggested version for a crate.
fn get_suggested_version(crate_name: &str) -> String {
    // Map of common crates to their typical latest major versions
    // These are conservative choices that should work for most projects
    let versions: HashMap<&str, &str> = [
        ("serde", "1"),
        ("serde-json", "1"),
        ("serde-derive", "1"),
        ("serde-yaml", "0.9"),
        ("tokio", "1"),
        ("tokio-util", "0.7"),
        ("reqwest", "0.12"),
        ("anyhow", "1"),
        ("thiserror", "2"),
        ("tracing", "0.1"),
        ("tracing-subscriber", "0.3"),
        ("clap", "4"),
        ("regex", "1"),
        ("chrono", "0.4"),
        ("uuid", "1"),
        ("rand", "0.8"),
        ("log", "0.4"),
        ("env_logger", "0.11"),
        ("toml", "0.8"),
        ("toml_edit", "0.22"),
        ("walkdir", "2"),
        ("glob", "0.3"),
        ("once_cell", "1"),
        ("lazy_static", "1"),
        ("parking_lot", "0.12"),
        ("crossbeam", "0.8"),
        ("rayon", "1"),
        ("itertools", "0.13"),
        ("bytes", "1"),
        ("futures", "0.3"),
        ("async-trait", "0.1"),
        ("pin-project", "1"),
    ]
    .into_iter()
    .collect();

    versions
        .get(crate_name)
        .map(|v| v.to_string())
        .unwrap_or_else(|| "*".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_cargo_toml(dir: &Path) -> PathBuf {
        let cargo_toml = dir.join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
"#,
        )
        .unwrap();
        cargo_toml
    }

    fn create_test_rust_file(dir: &Path) -> PathBuf {
        let rust_file = dir.join("src").join("main.rs");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            &rust_file,
            r#"use serde::Deserialize;

fn main() {
    let map: HashMap<String, i32> = HashMap::new();
    println!("{:?}", map);
}
"#,
        )
        .unwrap();
        rust_file
    }

    #[test]
    fn test_apply_cargo_dependency_success() {
        let temp_dir = TempDir::new().unwrap();
        create_test_cargo_toml(temp_dir.path());

        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("tokio".to_string()),
            suggested_change: "Add tokio dependency".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(result.success, "Fix should succeed: {:?}", result.error);
        assert_eq!(result.modified_files.len(), 1);

        // Verify the dependency was added
        let content = fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
        assert!(content.contains("tokio"), "Cargo.toml should contain tokio");
    }

    #[test]
    fn test_apply_cargo_dependency_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        create_test_cargo_toml(temp_dir.path());

        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("serde".to_string()),
            suggested_change: "Add serde dependency".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("already exists"));
    }

    #[test]
    fn test_apply_cargo_dependency_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = create_test_cargo_toml(temp_dir.path());
        let original = fs::read_to_string(&cargo_toml).unwrap();

        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("tokio".to_string()),
            suggested_change: "Add tokio dependency".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path()).with_dry_run(true);
        let result = apply_fix(&fix_info, &config);

        assert!(result.success);
        // File should not be modified
        let after = fs::read_to_string(&cargo_toml).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn test_apply_rust_import_success() {
        let temp_dir = TempDir::new().unwrap();
        create_test_rust_file(temp_dir.path());

        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: Some("src/main.rs".to_string()),
            target_item: Some("HashMap".to_string()),
            suggested_change: "Add HashMap import".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(result.success, "Fix should succeed: {:?}", result.error);

        // Verify the import was added
        let content = fs::read_to_string(temp_dir.path().join("src/main.rs")).unwrap();
        assert!(
            content.contains("use std::collections::HashMap;"),
            "File should contain HashMap import. Content:\n{}",
            content
        );
    }

    #[test]
    fn test_apply_rust_import_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("src").join("main.rs");
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(
            &rust_file,
            r#"use std::collections::HashMap;

fn main() {}
"#,
        )
        .unwrap();

        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: Some("src/main.rs".to_string()),
            target_item: Some("HashMap".to_string()),
            suggested_change: "Add HashMap import".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("already exists"));
    }

    #[test]
    fn test_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = create_test_cargo_toml(temp_dir.path());
        let original = fs::read_to_string(&cargo_toml).unwrap();

        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("tokio".to_string()),
            suggested_change: "Add tokio dependency".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(result.success);

        // Content should be changed
        let after_fix = fs::read_to_string(&cargo_toml).unwrap();
        assert!(after_fix.contains("tokio"));

        // Rollback
        result.rollback().unwrap();

        // Content should be restored
        let after_rollback = fs::read_to_string(&cargo_toml).unwrap();
        assert_eq!(original, after_rollback);
    }

    #[test]
    fn test_insert_rust_import_after_existing_use() {
        let content = r#"use std::fmt::Debug;
use serde::Serialize;

fn main() {}
"#;

        let result = insert_rust_import(content, "std::collections::HashMap");
        assert!(result.contains("use std::collections::HashMap;"));
        // Should be after the other use statements
        let hashmap_pos = result.find("use std::collections::HashMap").unwrap();
        let serialize_pos = result.find("use serde::Serialize").unwrap();
        assert!(hashmap_pos > serialize_pos);
    }

    #[test]
    fn test_insert_rust_import_no_existing_use() {
        let content = r#"fn main() {
    println!("hello");
}
"#;

        let result = insert_rust_import(content, "std::collections::HashMap");
        assert!(result.contains("use std::collections::HashMap;"));
        // Should be before fn main
        let hashmap_pos = result.find("use std::collections::HashMap").unwrap();
        let main_pos = result.find("fn main").unwrap();
        assert!(hashmap_pos < main_pos);
    }

    #[test]
    fn test_get_rust_import_path() {
        assert_eq!(get_rust_import_path("HashMap"), "std::collections::HashMap");
        assert_eq!(get_rust_import_path("Arc"), "std::sync::Arc");
        assert_eq!(get_rust_import_path("PathBuf"), "std::path::PathBuf");
        // Unknown items return themselves
        assert_eq!(get_rust_import_path("CustomType"), "CustomType");
    }

    #[test]
    fn test_normalize_crate_name() {
        assert_eq!(normalize_crate_name("serde_json"), "serde-json");
        assert_eq!(normalize_crate_name("tokio"), "tokio");
        assert_eq!(normalize_crate_name("my_crate"), "my_crate");
    }

    #[test]
    fn test_get_suggested_version() {
        assert_eq!(get_suggested_version("serde"), "1");
        assert_eq!(get_suggested_version("tokio"), "1");
        assert_eq!(get_suggested_version("unknown-crate"), "*");
    }

    #[test]
    fn test_type_fix_returns_failure() {
        let fix_info = FixInfo {
            fix_type: FixType::FixType,
            target_file: Some("src/main.rs".to_string()),
            target_item: None,
            suggested_change: "Fix type".to_string(),
        };

        let config = AutoFixConfig::default();
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("manual intervention"));
    }

    #[test]
    fn test_syntax_fix_returns_failure() {
        let fix_info = FixInfo {
            fix_type: FixType::FixSyntax,
            target_file: Some("src/main.rs".to_string()),
            target_item: None,
            suggested_change: "Fix syntax".to_string(),
        };

        let config = AutoFixConfig::default();
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("manual intervention"));
    }

    #[test]
    fn test_missing_dependency_name() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: None,
            suggested_change: "Add dependency".to_string(),
        };

        let config = AutoFixConfig::default();
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("No dependency name"));
    }

    #[test]
    fn test_no_manifest_file() {
        let temp_dir = TempDir::new().unwrap();

        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("tokio".to_string()),
            suggested_change: "Add dependency".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("No Cargo.toml"));
    }

    #[test]
    fn test_apply_import_missing_target_file() {
        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: None,
            target_item: Some("HashMap".to_string()),
            suggested_change: "Add import".to_string(),
        };

        let config = AutoFixConfig::default();
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("No target file"));
    }

    #[test]
    fn test_apply_import_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();

        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: Some("nonexistent.rs".to_string()),
            target_item: Some("HashMap".to_string()),
            suggested_change: "Add import".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("does not exist"));
    }

    #[test]
    fn test_fix_application_result_success() {
        let result = FixApplicationResult::success(
            vec![PathBuf::from("test.rs")],
            "Added import",
        );

        assert!(result.success);
        assert_eq!(result.modified_files.len(), 1);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_fix_application_result_failure() {
        let result = FixApplicationResult::failure("Something went wrong");

        assert!(!result.success);
        assert!(result.modified_files.is_empty());
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_config_builder_methods() {
        let config = AutoFixConfig::new("/tmp")
            .with_dry_run(true)
            .with_backups(true);

        assert!(config.dry_run);
        assert!(config.create_backups);
        assert_eq!(config.root_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_apply_package_json_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        fs::write(
            &package_json,
            r#"{
  "name": "test-project",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.18.0"
  }
}"#,
        )
        .unwrap();

        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("package.json".to_string()),
            target_item: Some("lodash".to_string()),
            suggested_change: "Add lodash dependency".to_string(),
        };

        let config = AutoFixConfig::new(temp_dir.path());
        let result = apply_fix(&fix_info, &config);

        assert!(result.success, "Fix should succeed: {:?}", result.error);

        // Verify the dependency was added
        let content = fs::read_to_string(&package_json).unwrap();
        assert!(content.contains("lodash"), "package.json should contain lodash");
    }

    #[test]
    fn test_insert_go_import() {
        let content = r#"package main

import (
	"fmt"
)

func main() {
	fmt.Println("hello")
}
"#;

        let result = insert_go_import(content, "sync");
        assert!(result.contains("\"sync\""));
    }

    #[test]
    fn test_get_go_import_path() {
        assert_eq!(get_go_import_path("Context"), "context");
        assert_eq!(get_go_import_path("Mutex"), "sync");
        assert_eq!(get_go_import_path("Marshal"), "encoding/json");
        assert_eq!(get_go_import_path("SomeCustomThing"), "somecustomthing");
    }
}

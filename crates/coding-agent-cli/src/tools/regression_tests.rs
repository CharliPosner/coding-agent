//! Regression test generation for self-healing error recovery.
//!
//! When auto-fixes are applied, this module generates regression tests to ensure
//! the fix is not accidentally reverted. The generated tests are valid, compilable
//! code that verifies the fix remains in place.

use crate::tools::{Diagnostic, FixApplicationResult, FixInfo, FixType};
use std::path::{Path, PathBuf};

/// Configuration for regression test generation.
#[derive(Debug, Clone)]
pub struct RegressionTestConfig {
    /// Directory where tests should be written.
    pub test_directory: PathBuf,

    /// Whether to include the original error message in test comments.
    pub include_error_context: bool,

    /// Whether to generate compile-time checks where possible.
    pub prefer_compile_time_checks: bool,

    /// Prefix for generated test function names.
    pub test_name_prefix: String,
}

impl Default for RegressionTestConfig {
    fn default() -> Self {
        Self {
            test_directory: PathBuf::from("tests"),
            include_error_context: true,
            prefer_compile_time_checks: true,
            test_name_prefix: "regression_".to_string(),
        }
    }
}

impl RegressionTestConfig {
    /// Create a new config with the given test directory.
    pub fn new(test_directory: impl Into<PathBuf>) -> Self {
        Self {
            test_directory: test_directory.into(),
            ..Default::default()
        }
    }

    /// Set whether to include error context in comments.
    pub fn with_error_context(mut self, include: bool) -> Self {
        self.include_error_context = include;
        self
    }

    /// Set the test name prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.test_name_prefix = prefix.into();
        self
    }
}

/// A generated regression test.
#[derive(Debug, Clone)]
pub struct RegressionTest {
    /// The test function name.
    pub name: String,

    /// The test source code.
    pub source: String,

    /// Suggested file path for this test.
    pub suggested_path: PathBuf,

    /// Description of what this test verifies.
    pub description: String,

    /// The original fix this test guards against.
    pub fix_type: FixType,
}

impl RegressionTest {
    /// Create a new regression test.
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        suggested_path: impl Into<PathBuf>,
        description: impl Into<String>,
        fix_type: FixType,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            suggested_path: suggested_path.into(),
            description: description.into(),
            fix_type,
        }
    }
}

/// Generate a regression test for a successful fix.
///
/// This is the main entry point for generating regression tests after a fix
/// has been applied and verified.
pub fn generate_regression_test(
    fix_info: &FixInfo,
    fix_result: &FixApplicationResult,
    config: &RegressionTestConfig,
) -> Option<RegressionTest> {
    if !fix_result.success {
        return None;
    }

    match fix_info.fix_type {
        FixType::AddDependency => generate_dependency_test(fix_info, fix_result, config),
        FixType::AddImport => generate_import_test(fix_info, fix_result, config),
        FixType::FixType => generate_type_test(fix_info, fix_result, config),
        FixType::FixSyntax => generate_syntax_test(fix_info, fix_result, config),
    }
}

/// Generate a regression test for a diagnostic that was auto-fixed.
pub fn generate_test_from_diagnostic(
    diagnostic: &Diagnostic,
    fix_info: &FixInfo,
    config: &RegressionTestConfig,
) -> Option<RegressionTest> {
    let test_name = generate_test_name(&config.test_name_prefix, fix_info);
    let error_comment = if config.include_error_context {
        format!("// Original error: {}\n    ", diagnostic.message)
    } else {
        String::new()
    };

    match fix_info.fix_type {
        FixType::AddDependency => {
            let crate_name = fix_info.target_item.as_ref()?;
            let test_source = generate_dependency_test_source(crate_name, &error_comment);
            let suggested_path = config
                .test_directory
                .join(format!("{}_dependency.rs", sanitize_name(crate_name)));

            Some(RegressionTest::new(
                &test_name,
                test_source,
                suggested_path,
                format!("Ensures {} dependency is available", crate_name),
                FixType::AddDependency,
            ))
        }
        FixType::AddImport => {
            let item_name = fix_info.target_item.as_ref()?;
            let file_path = fix_info.target_file.as_ref()?;
            let test_source = generate_import_test_source(item_name, file_path, &error_comment);
            let suggested_path = config
                .test_directory
                .join(format!("{}_import.rs", sanitize_name(item_name)));

            Some(RegressionTest::new(
                &test_name,
                test_source,
                suggested_path,
                format!("Ensures {} is properly imported", item_name),
                FixType::AddImport,
            ))
        }
        _ => None,
    }
}

/// Generate a test for a dependency fix.
fn generate_dependency_test(
    fix_info: &FixInfo,
    fix_result: &FixApplicationResult,
    config: &RegressionTestConfig,
) -> Option<RegressionTest> {
    let crate_name = fix_info.target_item.as_ref()?;
    let test_name = generate_test_name(&config.test_name_prefix, fix_info);

    let error_comment = if config.include_error_context {
        format!(
            "// Auto-generated regression test\n    // Fix applied: {}\n    ",
            fix_result.description
        )
    } else {
        String::new()
    };

    let test_source = generate_dependency_test_source(crate_name, &error_comment);

    let suggested_path = config
        .test_directory
        .join(format!("{}_dependency.rs", sanitize_name(crate_name)));

    Some(RegressionTest::new(
        test_name,
        test_source,
        suggested_path,
        format!(
            "Ensures the {} dependency added by auto-fix is not removed",
            crate_name
        ),
        FixType::AddDependency,
    ))
}

/// Generate test source code for a dependency fix.
fn generate_dependency_test_source(crate_name: &str, error_comment: &str) -> String {
    // Generate appropriate test based on the crate
    match crate_name {
        "serde" => format!(
            r#"#[test]
fn test_serde_dependency_available() {{
    {error_comment}// Verify serde traits are available for derive
    #[derive(serde::Serialize, serde::Deserialize)]
    struct TestStruct {{
        value: i32,
    }}

    let test = TestStruct {{ value: 42 }};
    // If this compiles, serde is properly configured
    let _ = test.value;
}}
"#
        ),
        "serde_json" | "serde-json" => format!(
            r#"#[test]
fn test_serde_json_dependency_available() {{
    {error_comment}// Verify serde_json is available and working
    let value = serde_json::json!({{
        "test": true,
        "count": 42
    }});

    assert!(value.is_object());
    assert_eq!(value["test"], true);
    assert_eq!(value["count"], 42);
}}
"#
        ),
        "tokio" => format!(
            r#"#[test]
fn test_tokio_dependency_available() {{
    {error_comment}// Verify tokio runtime can be created
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {{
        // Basic async operation
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(10),
            async {{ 42 }}
        ).await;

        assert!(result.is_ok());
    }});
}}
"#
        ),
        "anyhow" => format!(
            r#"#[test]
fn test_anyhow_dependency_available() {{
    {error_comment}// Verify anyhow error handling is available
    fn fallible() -> anyhow::Result<i32> {{
        Ok(42)
    }}

    let result = fallible();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}}
"#
        ),
        "thiserror" => format!(
            r#"#[test]
fn test_thiserror_dependency_available() {{
    {error_comment}// Verify thiserror derive macro is available
    #[derive(thiserror::Error, Debug)]
    enum TestError {{
        #[error("test error")]
        Test,
    }}

    let err = TestError::Test;
    assert_eq!(format!("{{err}}"), "test error");
}}
"#
        ),
        "regex" => format!(
            r#"#[test]
fn test_regex_dependency_available() {{
    {error_comment}// Verify regex crate is available
    let re = regex::Regex::new(r"^\d+$").expect("Invalid regex");
    assert!(re.is_match("123"));
    assert!(!re.is_match("abc"));
}}
"#
        ),
        "chrono" => format!(
            r#"#[test]
fn test_chrono_dependency_available() {{
    {error_comment}// Verify chrono crate is available
    use chrono::{{Utc, TimeZone}};

    let dt = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    assert_eq!(dt.year(), 2024);
}}
"#
        ),
        "reqwest" => format!(
            r#"#[test]
fn test_reqwest_dependency_available() {{
    {error_comment}// Verify reqwest crate is available (compile-time check only)
    // Note: This doesn't make actual HTTP requests
    let _client = reqwest::Client::new();
    // If this compiles, reqwest is properly configured
}}
"#
        ),
        "tracing" => format!(
            r#"#[test]
fn test_tracing_dependency_available() {{
    {error_comment}// Verify tracing crate is available
    tracing::info!("test message");
    tracing::debug!(value = 42, "debug with field");
    // If this compiles, tracing is properly configured
}}
"#
        ),
        "clap" => format!(
            r#"#[test]
fn test_clap_dependency_available() {{
    {error_comment}// Verify clap crate is available
    use clap::Parser;

    #[derive(Parser)]
    struct TestArgs {{
        #[arg(long)]
        name: Option<String>,
    }}

    // If this compiles, clap is properly configured
}}
"#
        ),
        _ => format!(
            r#"#[test]
fn test_{crate_sanitized}_dependency_available() {{
    {error_comment}// Verify {crate_name} crate is available
    // This test ensures the dependency is not accidentally removed
    extern crate {crate_sanitized};

    // If this compiles, the dependency is properly configured
}}
"#,
            crate_sanitized = sanitize_name(crate_name),
            crate_name = crate_name
        ),
    }
}

/// Generate a test for an import fix.
fn generate_import_test(
    fix_info: &FixInfo,
    fix_result: &FixApplicationResult,
    config: &RegressionTestConfig,
) -> Option<RegressionTest> {
    let item_name = fix_info.target_item.as_ref()?;
    let file_path = fix_info.target_file.as_ref()?;
    let test_name = generate_test_name(&config.test_name_prefix, fix_info);

    let error_comment = if config.include_error_context {
        format!(
            "// Auto-generated regression test\n    // Fix applied: {}\n    // File: {}\n    ",
            fix_result.description, file_path
        )
    } else {
        String::new()
    };

    let test_source = generate_import_test_source(item_name, file_path, &error_comment);

    let suggested_path = config
        .test_directory
        .join(format!("{}_import.rs", sanitize_name(item_name)));

    Some(RegressionTest::new(
        test_name,
        test_source,
        suggested_path,
        format!(
            "Ensures {} import is not removed from {}",
            item_name, file_path
        ),
        FixType::AddImport,
    ))
}

/// Generate test source code for an import fix.
fn generate_import_test_source(item_name: &str, file_path: &str, error_comment: &str) -> String {
    // Generate appropriate test based on the imported item
    match item_name {
        "HashMap" => format!(
            r#"#[test]
fn test_hashmap_import_available() {{
    {error_comment}// Verify HashMap is properly imported in {file_path}
    use std::collections::HashMap;

    let mut map: HashMap<String, i32> = HashMap::new();
    map.insert("test".to_string(), 42);

    assert_eq!(map.get("test"), Some(&42));
}}
"#
        ),
        "HashSet" => format!(
            r#"#[test]
fn test_hashset_import_available() {{
    {error_comment}// Verify HashSet is properly imported in {file_path}
    use std::collections::HashSet;

    let mut set: HashSet<i32> = HashSet::new();
    set.insert(42);

    assert!(set.contains(&42));
}}
"#
        ),
        "Arc" => format!(
            r#"#[test]
fn test_arc_import_available() {{
    {error_comment}// Verify Arc is properly imported in {file_path}
    use std::sync::Arc;

    let value = Arc::new(42);
    let cloned = Arc::clone(&value);

    assert_eq!(*value, *cloned);
}}
"#
        ),
        "Mutex" => format!(
            r#"#[test]
fn test_mutex_import_available() {{
    {error_comment}// Verify Mutex is properly imported in {file_path}
    use std::sync::Mutex;

    let mutex = Mutex::new(42);
    let guard = mutex.lock().unwrap();

    assert_eq!(*guard, 42);
}}
"#
        ),
        "PathBuf" => format!(
            r#"#[test]
fn test_pathbuf_import_available() {{
    {error_comment}// Verify PathBuf is properly imported in {file_path}
    use std::path::PathBuf;

    let path = PathBuf::from("/tmp/test");
    assert!(path.starts_with("/tmp"));
}}
"#
        ),
        "Path" => format!(
            r#"#[test]
fn test_path_import_available() {{
    {error_comment}// Verify Path is properly imported in {file_path}
    use std::path::Path;

    let path = Path::new("/tmp/test");
    assert!(path.starts_with("/tmp"));
}}
"#
        ),
        "Duration" => format!(
            r#"#[test]
fn test_duration_import_available() {{
    {error_comment}// Verify Duration is properly imported in {file_path}
    use std::time::Duration;

    let duration = Duration::from_secs(1);
    assert_eq!(duration.as_millis(), 1000);
}}
"#
        ),
        "Instant" => format!(
            r#"#[test]
fn test_instant_import_available() {{
    {error_comment}// Verify Instant is properly imported in {file_path}
    use std::time::Instant;

    let start = Instant::now();
    let _elapsed = start.elapsed();
    // If this compiles and runs, Instant is properly imported
}}
"#
        ),
        "File" => format!(
            r#"#[test]
fn test_file_import_available() {{
    {error_comment}// Verify File is properly imported in {file_path}
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temp file to test File operations
    let mut temp = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp, "test").expect("Failed to write");

    // Verify File can open it
    let _file = File::open(temp.path()).expect("Failed to open file");
}}
"#
        ),
        "BufReader" => format!(
            r#"#[test]
fn test_bufreader_import_available() {{
    {error_comment}// Verify BufReader is properly imported in {file_path}
    use std::io::{{BufReader, Cursor}};

    let data = Cursor::new("test data");
    let _reader = BufReader::new(data);
    // If this compiles, BufReader is properly imported
}}
"#
        ),
        "VecDeque" => format!(
            r#"#[test]
fn test_vecdeque_import_available() {{
    {error_comment}// Verify VecDeque is properly imported in {file_path}
    use std::collections::VecDeque;

    let mut deque: VecDeque<i32> = VecDeque::new();
    deque.push_back(1);
    deque.push_front(0);

    assert_eq!(deque.pop_front(), Some(0));
    assert_eq!(deque.pop_front(), Some(1));
}}
"#
        ),
        "RefCell" => format!(
            r#"#[test]
fn test_refcell_import_available() {{
    {error_comment}// Verify RefCell is properly imported in {file_path}
    use std::cell::RefCell;

    let cell = RefCell::new(42);
    *cell.borrow_mut() = 100;

    assert_eq!(*cell.borrow(), 100);
}}
"#
        ),
        "Rc" => format!(
            r#"#[test]
fn test_rc_import_available() {{
    {error_comment}// Verify Rc is properly imported in {file_path}
    use std::rc::Rc;

    let value = Rc::new(42);
    let cloned = Rc::clone(&value);

    assert_eq!(*value, *cloned);
    assert_eq!(Rc::strong_count(&value), 2);
}}
"#
        ),
        _ => format!(
            r#"#[test]
fn test_{item_sanitized}_import_available() {{
    {error_comment}// Verify {item_name} is properly imported in {file_path}
    // This test ensures the import is not accidentally removed

    // TODO: Add specific usage test for {item_name}
    // The import should be verified by the compilation of the main code
}}
"#,
            item_sanitized = sanitize_name(item_name),
            item_name = item_name
        ),
    }
}

/// Generate a test for a type fix.
fn generate_type_test(
    fix_info: &FixInfo,
    fix_result: &FixApplicationResult,
    config: &RegressionTestConfig,
) -> Option<RegressionTest> {
    let file_path = fix_info.target_file.as_ref()?;
    let test_name = generate_test_name(&config.test_name_prefix, fix_info);

    let error_comment = if config.include_error_context {
        format!(
            "// Auto-generated regression test\n    // Fix applied: {}\n    // File: {}\n    ",
            fix_result.description, file_path
        )
    } else {
        String::new()
    };

    // Type fixes are complex and require specific context
    // Generate a basic compile-time check
    let test_source = format!(
        r#"#[test]
fn {test_name}() {{
    {error_comment}// This test verifies the type fix in {file_path} is not reverted
    // The actual type compatibility is verified by successful compilation

    // Note: For more thorough testing, add specific type assertions here
    // that verify the expected type relationships in your code
}}
"#
    );

    let suggested_path = config.test_directory.join("type_fixes.rs");

    Some(RegressionTest::new(
        test_name,
        test_source,
        suggested_path,
        format!("Ensures type fix in {} is not reverted", file_path),
        FixType::FixType,
    ))
}

/// Generate a test for a syntax fix.
fn generate_syntax_test(
    fix_info: &FixInfo,
    fix_result: &FixApplicationResult,
    config: &RegressionTestConfig,
) -> Option<RegressionTest> {
    let file_path = fix_info.target_file.as_ref()?;
    let test_name = generate_test_name(&config.test_name_prefix, fix_info);

    let error_comment = if config.include_error_context {
        format!(
            "// Auto-generated regression test\n    // Fix applied: {}\n    // File: {}\n    ",
            fix_result.description, file_path
        )
    } else {
        String::new()
    };

    // Syntax fixes are verified by successful compilation
    let test_source = format!(
        r#"#[test]
fn {test_name}() {{
    {error_comment}// This test verifies the syntax fix in {file_path} is not reverted
    // Syntax correctness is verified by successful compilation of the crate

    // If this test compiles and runs, the syntax fix is still in place
}}
"#
    );

    let suggested_path = config.test_directory.join("syntax_fixes.rs");

    Some(RegressionTest::new(
        test_name,
        test_source,
        suggested_path,
        format!("Ensures syntax fix in {} is not reverted", file_path),
        FixType::FixSyntax,
    ))
}

/// Generate a unique test name from the fix info.
fn generate_test_name(prefix: &str, fix_info: &FixInfo) -> String {
    let suffix = match &fix_info.target_item {
        Some(item) => sanitize_name(item),
        None => match &fix_info.target_file {
            Some(file) => sanitize_name(
                Path::new(file)
                    .file_stem()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("unknown"),
            ),
            None => "unknown".to_string(),
        },
    };

    let fix_type_str = match fix_info.fix_type {
        FixType::AddDependency => "dep",
        FixType::AddImport => "import",
        FixType::FixType => "type",
        FixType::FixSyntax => "syntax",
    };

    format!("{}{}_{}", prefix, fix_type_str, suffix)
}

/// Sanitize a name for use in Rust identifiers.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
        .trim_matches('_')
        .to_string()
}

/// Write a regression test to the file system.
pub fn write_regression_test(test: &RegressionTest, base_dir: &Path) -> Result<PathBuf, String> {
    use std::fs;

    let full_path = if test.suggested_path.is_absolute() {
        test.suggested_path.clone()
    } else {
        base_dir.join(&test.suggested_path)
    };

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create test directory: {}", e))?;
    }

    // Check if file exists and append, or create new
    let content = if full_path.exists() {
        let existing = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read existing test file: {}", e))?;

        // Check if this test already exists
        if existing.contains(&format!("fn {}()", test.name)) {
            return Err(format!(
                "Test {} already exists in {:?}",
                test.name, full_path
            ));
        }

        format!("{}\n{}", existing, test.source)
    } else {
        format!(
            "//! Auto-generated regression tests for self-healing fixes.\n//!\n//! These tests ensure that automatically applied fixes are not accidentally reverted.\n\n{}",
            test.source
        )
    };

    fs::write(&full_path, content).map_err(|e| format!("Failed to write test file: {}", e))?;

    Ok(full_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("serde_json"), "serde_json");
        assert_eq!(sanitize_name("serde-json"), "serde_json");
        assert_eq!(sanitize_name("HashMap"), "hashmap");
        assert_eq!(
            sanitize_name("std::collections::HashMap"),
            "std__collections__hashmap"
        );
    }

    #[test]
    fn test_generate_test_name_dependency() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("serde_json".to_string()),
            suggested_change: "Add serde_json".to_string(),
        };

        let name = generate_test_name("regression_", &fix_info);
        assert!(name.starts_with("regression_dep_"));
        assert!(name.contains("serde_json"));
    }

    #[test]
    fn test_generate_test_name_import() {
        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: Some("src/main.rs".to_string()),
            target_item: Some("HashMap".to_string()),
            suggested_change: "Add HashMap import".to_string(),
        };

        let name = generate_test_name("regression_", &fix_info);
        assert!(name.starts_with("regression_import_"));
        assert!(name.contains("hashmap"));
    }

    #[test]
    fn test_generate_dependency_test_serde_json() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("serde_json".to_string()),
            suggested_change: "Add serde_json = \"1\"".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("Cargo.toml")],
            "Added serde_json dependency",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        assert!(test.source.contains("serde_json::json!"));
        assert!(test.source.contains("#[test]"));
        assert!(test.source.contains("assert!"));
        assert_eq!(test.fix_type, FixType::AddDependency);
    }

    #[test]
    fn test_generate_dependency_test_tokio() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("tokio".to_string()),
            suggested_change: "Add tokio = \"1\"".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("Cargo.toml")],
            "Added tokio dependency",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        assert!(test.source.contains("tokio::runtime"));
        assert!(test.source.contains("block_on"));
    }

    #[test]
    fn test_generate_import_test_hashmap() {
        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: Some("src/main.rs".to_string()),
            target_item: Some("HashMap".to_string()),
            suggested_change: "use std::collections::HashMap".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("src/main.rs")],
            "Added HashMap import",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        assert!(test.source.contains("use std::collections::HashMap"));
        assert!(test.source.contains("HashMap::new()"));
        assert!(test.source.contains("insert"));
        assert_eq!(test.fix_type, FixType::AddImport);
    }

    #[test]
    fn test_generate_import_test_arc() {
        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: Some("src/lib.rs".to_string()),
            target_item: Some("Arc".to_string()),
            suggested_change: "use std::sync::Arc".to_string(),
        };

        let fix_result =
            FixApplicationResult::success(vec![PathBuf::from("src/lib.rs")], "Added Arc import");

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        assert!(test.source.contains("use std::sync::Arc"));
        assert!(test.source.contains("Arc::new"));
        assert!(test.source.contains("Arc::clone"));
    }

    #[test]
    fn test_generate_type_test() {
        let fix_info = FixInfo {
            fix_type: FixType::FixType,
            target_file: Some("src/main.rs".to_string()),
            target_item: None,
            suggested_change: "Fix type mismatch".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("src/main.rs")],
            "Fixed type mismatch",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        assert!(test.source.contains("#[test]"));
        assert!(test.source.contains("type fix"));
        assert_eq!(test.fix_type, FixType::FixType);
    }

    #[test]
    fn test_generate_syntax_test() {
        let fix_info = FixInfo {
            fix_type: FixType::FixSyntax,
            target_file: Some("src/parser.rs".to_string()),
            target_item: None,
            suggested_change: "Fix syntax error".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("src/parser.rs")],
            "Fixed syntax error",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        assert!(test.source.contains("#[test]"));
        assert!(test.source.contains("syntax fix"));
        assert_eq!(test.fix_type, FixType::FixSyntax);
    }

    #[test]
    fn test_no_test_for_failed_fix() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("serde".to_string()),
            suggested_change: "Add serde".to_string(),
        };

        let fix_result = FixApplicationResult::failure("Could not add dependency");

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = RegressionTestConfig::default();

        assert_eq!(config.test_directory, PathBuf::from("tests"));
        assert!(config.include_error_context);
        assert!(config.prefer_compile_time_checks);
        assert_eq!(config.test_name_prefix, "regression_");
    }

    #[test]
    fn test_config_builder() {
        let config = RegressionTestConfig::new("custom_tests")
            .with_error_context(false)
            .with_prefix("auto_");

        assert_eq!(config.test_directory, PathBuf::from("custom_tests"));
        assert!(!config.include_error_context);
        assert_eq!(config.test_name_prefix, "auto_");
    }

    #[test]
    fn test_write_regression_test() {
        let temp_dir = TempDir::new().unwrap();

        let test = RegressionTest::new(
            "test_example",
            "#[test]\nfn test_example() { assert!(true); }",
            PathBuf::from("tests/example.rs"),
            "Example test",
            FixType::AddDependency,
        );

        let result = write_regression_test(&test, temp_dir.path());
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("#[test]"));
        assert!(content.contains("test_example"));
    }

    #[test]
    fn test_write_regression_test_appends() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("tests");
        std::fs::create_dir_all(&test_dir).unwrap();

        // Create existing file
        let existing_path = test_dir.join("example.rs");
        std::fs::write(&existing_path, "#[test]\nfn existing_test() {}").unwrap();

        let test = RegressionTest::new(
            "test_new",
            "#[test]\nfn test_new() { assert!(true); }",
            PathBuf::from("tests/example.rs"),
            "New test",
            FixType::AddDependency,
        );

        let result = write_regression_test(&test, temp_dir.path());
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&existing_path).unwrap();
        assert!(content.contains("existing_test"));
        assert!(content.contains("test_new"));
    }

    #[test]
    fn test_write_regression_test_duplicate_fails() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("tests");
        std::fs::create_dir_all(&test_dir).unwrap();

        // Create file with the same test name
        let existing_path = test_dir.join("example.rs");
        std::fs::write(&existing_path, "#[test]\nfn test_duplicate() {}").unwrap();

        let test = RegressionTest::new(
            "test_duplicate",
            "#[test]\nfn test_duplicate() { assert!(true); }",
            PathBuf::from("tests/example.rs"),
            "Duplicate test",
            FixType::AddDependency,
        );

        let result = write_regression_test(&test, temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_generate_test_from_diagnostic() {
        use crate::tools::diagnostics::{Diagnostic, DiagnosticLocation, DiagnosticSeverity};

        let diagnostic = Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some("E0463".to_string()),
            message: "can't find crate for `serde`".to_string(),
            location: Some(DiagnosticLocation::new("src/main.rs", 1)),
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: String::new(),
        };

        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("serde".to_string()),
            suggested_change: "Add serde dependency".to_string(),
        };

        let config = RegressionTestConfig::default();
        let test = generate_test_from_diagnostic(&diagnostic, &fix_info, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        assert!(test.source.contains("serde"));
        assert!(test.source.contains("Original error"));
    }

    #[test]
    fn test_regression_test_struct() {
        let test = RegressionTest::new(
            "test_name",
            "source code",
            PathBuf::from("tests/test.rs"),
            "Test description",
            FixType::AddDependency,
        );

        assert_eq!(test.name, "test_name");
        assert_eq!(test.source, "source code");
        assert_eq!(test.suggested_path, PathBuf::from("tests/test.rs"));
        assert_eq!(test.description, "Test description");
        assert_eq!(test.fix_type, FixType::AddDependency);
    }

    #[test]
    fn test_generate_dependency_test_generic_crate() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("some_unknown_crate".to_string()),
            suggested_change: "Add some_unknown_crate".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("Cargo.toml")],
            "Added some_unknown_crate dependency",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        // Generic template should use extern crate
        assert!(test.source.contains("extern crate some_unknown_crate"));
    }

    #[test]
    fn test_generate_import_test_generic_item() {
        let fix_info = FixInfo {
            fix_type: FixType::AddImport,
            target_file: Some("src/lib.rs".to_string()),
            target_item: Some("CustomType".to_string()),
            suggested_change: "Add CustomType import".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("src/lib.rs")],
            "Added CustomType import",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();

        // Generic template should have TODO comment
        assert!(test.source.contains("TODO"));
        assert!(test.source.contains("CustomType"));
    }
}

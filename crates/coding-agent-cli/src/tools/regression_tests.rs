//! Regression test generation for self-healing error recovery.
//!
//! Streamlined version focused on core functionality.

use crate::tools::{Diagnostic, FixApplicationResult, FixInfo, FixType};
use std::path::{Path, PathBuf};

/// Configuration for regression test generation.
#[derive(Debug, Clone)]
pub struct RegressionTestConfig {
    pub test_directory: PathBuf,
    pub test_name_prefix: String,
}

impl Default for RegressionTestConfig {
    fn default() -> Self {
        Self {
            test_directory: PathBuf::from("tests"),
            test_name_prefix: "regression_".to_string(),
        }
    }
}

/// A generated regression test.
#[derive(Debug, Clone)]
pub struct RegressionTest {
    pub name: String,
    pub source: String,
    pub suggested_path: PathBuf,
    pub description: String,
    pub fix_type: FixType,
}

/// Generate a regression test for a successful fix.
pub fn generate_regression_test(
    fix_info: &FixInfo,
    fix_result: &FixApplicationResult,
    config: &RegressionTestConfig,
) -> Option<RegressionTest> {
    if !fix_result.success {
        return None;
    }

    let test_name = generate_test_name(&config.test_name_prefix, fix_info);
    let file_ref = fix_info.target_file.as_deref().unwrap_or("unknown");

    let test_source = match fix_info.fix_type {
        FixType::AddDependency => {
            let crate_name = fix_info.target_item.as_deref().unwrap_or("unknown");
            format!(
                r#"#[test]
fn {test_name}() {{
    // Ensures {crate_name} dependency is not accidentally removed
    extern crate {crate_sanitized};
    // If this compiles, the dependency is properly configured
}}
"#,
                crate_sanitized = sanitize_name(crate_name)
            )
        }
        FixType::AddImport => {
            let item_name = fix_info.target_item.as_deref().unwrap_or("Unknown");
            format!(
                r#"#[test]
fn {test_name}() {{
    // Ensures {item_name} import is not removed from {file_ref}
    // TODO: Add specific usage test for {item_name}
}}
"#
            )
        }
        FixType::FixType | FixType::FixSyntax => {
            format!(
                r#"#[test]
fn {test_name}() {{
    // Verifies fix in {file_ref} is not reverted
    // Correctness verified by successful compilation
}}
"#
            )
        }
    };

    let suggested_path = config
        .test_directory
        .join(format!("{}_fixes.rs", fix_info.fix_type.to_string().to_lowercase()));

    Some(RegressionTest {
        name: test_name,
        source: test_source,
        suggested_path,
        description: format!("Regression test for {}", fix_result.description),
        fix_type: fix_info.fix_type.clone(),
    })
}

fn generate_test_name(prefix: &str, fix_info: &FixInfo) -> String {
    let suffix = fix_info
        .target_item
        .as_ref()
        .map(|s| sanitize_name(s))
        .unwrap_or_else(|| "fix".to_string());
    format!("{}{}", prefix, suffix)
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("serde-json"), "serde_json");
        assert_eq!(sanitize_name("HashMap"), "hashmap");
    }

    #[test]
    fn test_generate_test_name() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("serde".to_string()),
            suggested_change: "Add serde".to_string(),
        };

        let name = generate_test_name("regression_", &fix_info);
        assert!(name.contains("serde"));
    }

    #[test]
    fn test_generate_dependency_test() {
        let fix_info = FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: Some("serde".to_string()),
            suggested_change: "Add serde".to_string(),
        };

        let fix_result = FixApplicationResult::success(
            vec![PathBuf::from("Cargo.toml")],
            "Added serde dependency",
        );

        let config = RegressionTestConfig::default();
        let test = generate_regression_test(&fix_info, &fix_result, &config);

        assert!(test.is_some());
        let test = test.unwrap();
        assert!(test.source.contains("extern crate serde"));
        assert_eq!(test.fix_type, FixType::AddDependency);
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
}
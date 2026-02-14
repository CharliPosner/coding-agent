//! Comprehensive end-to-end tests for permission prompts and config updates
//!
//! Tests the full workflow:
//! 1. Write to untrusted path triggers permission prompt
//! 2. "Always" response adds path to trusted_paths in config
//! 3. Config is persisted to disk
//! 4. Subsequent writes to same path don't prompt (cached)
//! 5. Config reloaded from disk retains trusted path

use coding_agent_cli::config::Config;
use coding_agent_cli::permissions::{
    OperationType, PermissionChecker, PermissionDecision, PermissionResponse, TrustedPaths,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test config path
fn create_test_config_path(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().join("config.toml")
}

#[test]
fn test_end_to_end_permission_prompt_workflow() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);
    let test_file = temp_dir.path().join("data").join("untrusted.txt");

    // Step 1: Create initial config with no trusted paths
    let mut config = Config::default();
    config.save_to(&config_path).expect("Should save config");

    // Step 2: Create permission checker from config
    let trusted = TrustedPaths::new(&config.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, config.permissions.auto_read);

    // Step 3: Check permission for write (should need prompt)
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt,
        "First write to untrusted path should need prompt"
    );

    // Step 4: Simulate user responding "Always"
    // In real code, this would come from PermissionPrompt::prompt()
    // For testing, we simulate the decision
    checker.record_decision(&test_file, OperationType::Write, PermissionDecision::Allowed);

    // Step 5: Add parent directory to trusted paths in config
    let parent_dir = test_file.parent().expect("Should have parent");
    config
        .add_trusted_path_to(parent_dir.to_str().unwrap(), &config_path)
        .expect("Should add trusted path");

    // Step 6: Verify config was updated in memory
    assert!(
        config
            .permissions
            .trusted_paths
            .contains(&parent_dir.to_str().unwrap().to_string()),
        "Trusted path should be added to config"
    );

    // Step 7: Verify config was persisted to disk
    assert!(config_path.exists(), "Config file should exist");
    let config_contents = fs::read_to_string(&config_path).expect("Should read config file");
    assert!(
        config_contents.contains(&parent_dir.to_str().unwrap()),
        "Config file should contain trusted path"
    );

    // Step 8: Reload config from disk to verify persistence
    let reloaded_config = Config::load_from(&config_path).expect("Should load config");
    assert!(
        reloaded_config
            .permissions
            .trusted_paths
            .contains(&parent_dir.to_str().unwrap().to_string()),
        "Reloaded config should contain trusted path"
    );

    // Step 9: Create new checker from reloaded config
    let trusted_reloaded = TrustedPaths::new(&reloaded_config.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker_reloaded =
        PermissionChecker::new(trusted_reloaded, reloaded_config.permissions.auto_read);

    // Step 10: Verify subsequent write doesn't need prompt
    assert_eq!(
        checker_reloaded.check(&test_file, OperationType::Write),
        PermissionDecision::Allowed,
        "Subsequent write to trusted path should be allowed"
    );
}

#[test]
fn test_permission_prompt_yes_does_not_update_config() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);
    let test_file = temp_dir.path().join("temp").join("file.txt");

    // Create config
    let config = Config::default();
    config.save_to(&config_path).expect("Should save config");

    let initial_trusted_count = config.permissions.trusted_paths.len();

    // Create checker
    let trusted = TrustedPaths::new(&config.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, config.permissions.auto_read);

    // Check needs prompt
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt
    );

    // Simulate "Yes" (allow once, not "Always")
    checker.record_decision(&test_file, OperationType::Write, PermissionDecision::Allowed);

    // Check is now allowed in session
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::Allowed
    );

    // Verify config NOT updated (because we didn't use "Always")
    assert_eq!(
        config.permissions.trusted_paths.len(),
        initial_trusted_count,
        "Config should not be updated for 'Yes' response"
    );

    // Reload config - should still not have the path
    let reloaded = Config::load_from(&config_path).expect("Should load config");
    assert_eq!(
        reloaded.permissions.trusted_paths.len(),
        initial_trusted_count,
        "Reloaded config should not have new path"
    );

    // New session (new checker) should require prompt again
    let trusted_new = TrustedPaths::new(&reloaded.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker_new = PermissionChecker::new(trusted_new, reloaded.permissions.auto_read);

    assert_eq!(
        checker_new.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt,
        "New session should require prompt again"
    );
}

#[test]
fn test_permission_prompt_never_blocks_permanently_in_session() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);
    let test_file = temp_dir.path().join("blocked").join("file.txt");

    // Create config
    let config = Config::default();
    config.save_to(&config_path).expect("Should save config");

    // Create checker
    let trusted = TrustedPaths::new(&config.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let mut checker = PermissionChecker::new(trusted, config.permissions.auto_read);

    // First check needs prompt
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt
    );

    // Simulate "Never" (deny for session)
    checker.record_decision(&test_file, OperationType::Write, PermissionDecision::Denied);

    // Subsequent checks in same session should be denied
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::Denied,
        "Should be denied after 'Never' response"
    );

    // Verify config NOT updated (Never doesn't persist)
    let reloaded = Config::load_from(&config_path).expect("Should load config");
    assert_eq!(
        reloaded.permissions.trusted_paths.len(),
        0,
        "'Never' should not add to config"
    );

    // New session should require prompt again (Never only lasts for session)
    let trusted_new = TrustedPaths::new(&reloaded.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker_new = PermissionChecker::new(trusted_new, reloaded.permissions.auto_read);

    assert_eq!(
        checker_new.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt,
        "New session should prompt again (Never doesn't persist)"
    );
}

#[test]
fn test_multiple_always_responses_accumulate() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);

    let file1 = temp_dir.path().join("dir1").join("file1.txt");
    let file2 = temp_dir.path().join("dir2").join("file2.txt");
    let file3 = temp_dir.path().join("dir3").join("file3.txt");

    // Create config
    let mut config = Config::default();
    config.save_to(&config_path).expect("Should save config");

    // Add three different directories as trusted
    config
        .add_trusted_path_to(
            file1.parent().unwrap().to_str().unwrap(),
            &config_path,
        )
        .expect("Should add path 1");

    config
        .add_trusted_path_to(
            file2.parent().unwrap().to_str().unwrap(),
            &config_path,
        )
        .expect("Should add path 2");

    config
        .add_trusted_path_to(
            file3.parent().unwrap().to_str().unwrap(),
            &config_path,
        )
        .expect("Should add path 3");

    // Verify all three are in config
    assert_eq!(
        config.permissions.trusted_paths.len(),
        3,
        "Should have three trusted paths"
    );

    // Reload and verify persistence
    let reloaded = Config::load_from(&config_path).expect("Should load config");
    assert_eq!(
        reloaded.permissions.trusted_paths.len(),
        3,
        "Reloaded config should have three trusted paths"
    );

    // Verify all paths work
    let trusted = TrustedPaths::new(&reloaded.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker = PermissionChecker::new(trusted, reloaded.permissions.auto_read);

    assert_eq!(
        checker.check(&file1, OperationType::Write),
        PermissionDecision::Allowed
    );
    assert_eq!(
        checker.check(&file2, OperationType::Write),
        PermissionDecision::Allowed
    );
    assert_eq!(
        checker.check(&file3, OperationType::Write),
        PermissionDecision::Allowed
    );
}

#[test]
fn test_subdirectories_of_trusted_paths_are_trusted() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);

    let parent_dir = temp_dir.path().join("trusted");
    let nested_file = parent_dir.join("subdir").join("nested.txt");

    // Create config and add parent directory as trusted
    let mut config = Config::default();
    config
        .add_trusted_path_to(parent_dir.to_str().unwrap(), &config_path)
        .expect("Should add trusted path");

    // Create checker
    let trusted = TrustedPaths::new(&config.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker = PermissionChecker::new(trusted, config.permissions.auto_read);

    // Nested file should be allowed (subdirectory of trusted path)
    assert_eq!(
        checker.check(&nested_file, OperationType::Write),
        PermissionDecision::Allowed,
        "Subdirectories of trusted paths should be trusted"
    );

    // File directly in trusted dir should also be allowed
    let direct_file = parent_dir.join("direct.txt");
    assert_eq!(
        checker.check(&direct_file, OperationType::Write),
        PermissionDecision::Allowed
    );
}

#[test]
fn test_sibling_directories_not_automatically_trusted() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);

    let trusted_dir = temp_dir.path().join("trusted");
    let sibling_dir = temp_dir.path().join("sibling");
    let sibling_file = sibling_dir.join("file.txt");

    // Add only one directory as trusted
    let mut config = Config::default();
    config
        .add_trusted_path_to(trusted_dir.to_str().unwrap(), &config_path)
        .expect("Should add trusted path");

    // Create checker
    let trusted = TrustedPaths::new(&config.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker = PermissionChecker::new(trusted, config.permissions.auto_read);

    // Sibling directory should NOT be trusted
    assert_eq!(
        checker.check(&sibling_file, OperationType::Write),
        PermissionDecision::NeedsPrompt,
        "Sibling directories should not be automatically trusted"
    );
}

#[test]
fn test_config_update_survives_multiple_reload_cycles() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);
    let test_path = "/Users/test/important";

    // Cycle 1: Create and save
    let mut config = Config::default();
    config
        .add_trusted_path_to(test_path, &config_path)
        .expect("Should add path");

    // Cycle 2: Reload and verify
    let config2 = Config::load_from(&config_path).expect("Should load");
    assert!(config2.permissions.trusted_paths.contains(&test_path.to_string()));

    // Cycle 3: Reload again and verify
    let config3 = Config::load_from(&config_path).expect("Should load");
    assert!(config3.permissions.trusted_paths.contains(&test_path.to_string()));

    // Cycle 4: Reload, add another path, save
    let mut config4 = Config::load_from(&config_path).expect("Should load");
    config4
        .add_trusted_path_to("/Users/test/another", &config_path)
        .expect("Should add second path");

    // Cycle 5: Final reload and verify both paths
    let config5 = Config::load_from(&config_path).expect("Should load");
    assert_eq!(
        config5.permissions.trusted_paths.len(),
        2,
        "Should have both paths"
    );
    assert!(config5.permissions.trusted_paths.contains(&test_path.to_string()));
    assert!(config5
        .permissions
        .trusted_paths
        .contains(&"/Users/test/another".to_string()));
}

#[test]
fn test_permission_response_enum_all_variants() {
    // Ensure all permission response variants are usable
    let responses = vec![
        PermissionResponse::Yes,
        PermissionResponse::No,
        PermissionResponse::Always,
        PermissionResponse::Never,
    ];

    // Verify they're all distinct
    assert_eq!(responses.len(), 4);
    for (i, r1) in responses.iter().enumerate() {
        for (j, r2) in responses.iter().enumerate() {
            if i == j {
                assert_eq!(r1, r2);
            } else {
                assert_ne!(r1, r2);
            }
        }
    }
}

#[test]
fn test_operation_type_specificity_with_config() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);
    let test_file = temp_dir.path().join("data").join("file.txt");

    // Create config with trusted path
    let mut config = Config::default();
    let parent_dir = test_file.parent().unwrap();
    config
        .add_trusted_path_to(parent_dir.to_str().unwrap(), &config_path)
        .expect("Should add trusted path");

    // Create checker
    let trusted = TrustedPaths::new(&config.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker = PermissionChecker::new(trusted, config.permissions.auto_read);

    // All operation types should be allowed for trusted paths
    assert_eq!(
        checker.check(&test_file, OperationType::Read),
        PermissionDecision::Allowed
    );
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::Allowed
    );
    assert_eq!(
        checker.check(&test_file, OperationType::Modify),
        PermissionDecision::Allowed
    );
    assert_eq!(
        checker.check(&test_file, OperationType::Delete),
        PermissionDecision::Allowed
    );
}

#[test]
fn test_read_operations_auto_allowed_when_auto_read_true() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let test_file = temp_dir.path().join("untrusted").join("file.txt");

    // Create config with auto_read = true (default)
    let config = Config::default();
    assert!(config.permissions.auto_read, "auto_read should be true by default");

    // Create checker with no trusted paths
    let trusted = TrustedPaths::new(&[]).expect("Should create trusted paths");
    let checker = PermissionChecker::new(trusted, config.permissions.auto_read);

    // Read should be auto-allowed
    assert_eq!(
        checker.check(&test_file, OperationType::Read),
        PermissionDecision::Allowed,
        "Read operations should be auto-allowed when auto_read is true"
    );

    // Write should still need prompt
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt,
        "Write operations should still need prompt"
    );
}

#[test]
fn test_config_file_format_is_valid_toml() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);

    // Create config with some data
    let mut config = Config::default();
    config
        .add_trusted_path_to("/Users/test/path1", &config_path)
        .expect("Should add path");
    config
        .add_trusted_path_to("/Users/test/path2", &config_path)
        .expect("Should add path");

    // Read the file as string
    let contents = fs::read_to_string(&config_path).expect("Should read file");

    // Verify it's valid TOML
    let parsed: toml::Value = toml::from_str(&contents).expect("Should be valid TOML");

    // Verify structure
    assert!(parsed.get("permissions").is_some());
    assert!(parsed["permissions"].get("trusted_paths").is_some());

    // Verify array contains our paths
    let paths = parsed["permissions"]["trusted_paths"]
        .as_array()
        .expect("Should be array");
    assert_eq!(paths.len(), 2);
}

#[test]
fn test_empty_trusted_paths_list_handled_correctly() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = create_test_config_path(&temp_dir);
    let test_file = temp_dir.path().join("file.txt");

    // Create config with empty trusted paths
    let config = Config::default();
    assert!(
        config.permissions.trusted_paths.is_empty(),
        "Default should have no trusted paths"
    );
    config.save_to(&config_path).expect("Should save");

    // Reload
    let loaded = Config::load_from(&config_path).expect("Should load");

    // Create checker
    let trusted = TrustedPaths::new(&loaded.permissions.trusted_paths)
        .expect("Should create trusted paths");
    let checker = PermissionChecker::new(trusted, loaded.permissions.auto_read);

    // Should need prompt for write
    assert_eq!(
        checker.check(&test_file, OperationType::Write),
        PermissionDecision::NeedsPrompt
    );
}

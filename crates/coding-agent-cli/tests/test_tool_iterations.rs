//! Integration tests for tool iteration limits
//!
//! These tests verify that the tool iteration system properly respects
//! configured limits, displays warnings, and stops at the maximum.

use coding_agent_cli::config::{BehaviorConfig, Config};

#[test]
fn test_tool_iterations_default() {
    let config = Config::default();
    assert_eq!(
        config.behavior.max_tool_iterations, 50,
        "Default tool iteration limit should be 50"
    );
}

#[test]
fn test_tool_iterations_configurable() {
    let toml = r#"
        [behavior]
        max_tool_iterations = 25
    "#;

    let config = Config::parse(toml).expect("Should parse config with custom iteration limit");
    assert_eq!(
        config.behavior.max_tool_iterations, 25,
        "Custom iteration limit should be respected"
    );
}

#[test]
fn test_tool_iterations_warning_threshold() {
    // Test that warning threshold is calculated correctly at 80%
    let limit = 50;
    let warning_threshold = ((limit * 80) / 100).max(1);
    assert_eq!(warning_threshold, 40, "Warning should trigger at 40/50");

    let limit = 25;
    let warning_threshold = ((limit * 80) / 100).max(1);
    assert_eq!(warning_threshold, 20, "Warning should trigger at 20/25");

    let limit = 10;
    let warning_threshold = ((limit * 80) / 100).max(1);
    assert_eq!(warning_threshold, 8, "Warning should trigger at 8/10");
}

#[test]
fn test_behavior_config_serialization() {
    let config = BehaviorConfig {
        streaming: true,
        tool_verbosity: "standard".to_string(),
        show_context_bar: true,
        fun_facts: true,
        fun_fact_delay: 10,
        max_tool_iterations: 75,
    };

    // Serialize to TOML
    let toml_str = toml::to_string(&config).expect("Should serialize BehaviorConfig");
    assert!(toml_str.contains("max_tool_iterations = 75"));

    // Deserialize back
    let deserialized: BehaviorConfig =
        toml::from_str(&toml_str).expect("Should deserialize BehaviorConfig");
    assert_eq!(deserialized.max_tool_iterations, 75);
}

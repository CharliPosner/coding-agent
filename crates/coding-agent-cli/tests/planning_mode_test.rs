//! Integration tests for planning mode functionality

use coding_agent_cli::cli::{Mode, Repl, ReplConfig};

#[test]
fn test_planning_mode_creation() {
    let mode = Mode::planning("test.md".to_string());
    assert!(mode.is_planning());
    assert_eq!(mode.spec_file(), Some("test.md"));
}

#[test]
fn test_planning_mode_indicator() {
    let normal = Mode::Normal;
    assert_eq!(normal.indicator(), None);

    let planning = Mode::planning("auth.md".to_string());
    let indicator = planning.indicator();
    assert!(indicator.is_some());
    let text = indicator.unwrap();
    assert!(text.contains("Planning"));
    assert!(text.contains("auth.md"));
}

#[test]
fn test_planning_mode_system_prompt() {
    let normal = Mode::Normal;
    let normal_prompt = normal.system_prompt();
    assert!(normal_prompt.contains("coding assistant"));
    assert!(!normal_prompt.contains("specification"));

    let planning = Mode::planning("test.md".to_string());
    let planning_prompt = planning.system_prompt();
    assert!(planning_prompt.contains("specification planning"));
    assert!(planning_prompt.contains("test.md"));
    assert!(planning_prompt.contains("collaborative"));
    assert!(planning_prompt.contains("requirements"));
}

#[test]
fn test_repl_default_mode() {
    let repl = Repl::new(ReplConfig::default());
    assert_eq!(*repl.mode(), Mode::Normal);
    assert!(!repl.mode().is_planning());
}

#[test]
fn test_repl_mode_change() {
    let mut repl = Repl::new(ReplConfig::default());
    assert_eq!(*repl.mode(), Mode::Normal);

    // Change to planning mode
    let planning_mode = Mode::planning("auth.md".to_string());
    repl.set_mode(planning_mode);

    assert!(repl.mode().is_planning());
    assert_eq!(repl.mode().spec_file(), Some("auth.md"));
}

#[test]
fn test_repl_reset_context_resets_mode() {
    let mut repl = Repl::new(ReplConfig::default());

    // Change to planning mode
    repl.set_mode(Mode::planning("test.md".to_string()));
    assert!(repl.mode().is_planning());

    // Reset context should reset mode to normal
    repl.reset_context();
    assert_eq!(*repl.mode(), Mode::Normal);
    assert!(!repl.mode().is_planning());
}

#[test]
fn test_normal_mode_prompt_content() {
    let normal = Mode::Normal;
    let prompt = normal.system_prompt();

    // Should mention being a coding assistant
    assert!(prompt.contains("coding assistant"));

    // Should mention tools
    assert!(prompt.contains("tools"));

    // Should not mention planning or specifications
    assert!(!prompt.contains("specification"));
    assert!(!prompt.contains("planning"));
}

#[test]
fn test_planning_mode_prompt_content() {
    let planning = Mode::planning("feature.md".to_string());
    let prompt = planning.system_prompt();

    // Should mention specification and planning
    assert!(prompt.contains("specification"));
    assert!(prompt.contains("planning"));

    // Should include the spec file name
    assert!(prompt.contains("feature.md"));

    // Should mention collaboration
    assert!(prompt.contains("collaborative"));

    // Should mention key activities
    assert!(prompt.contains("requirements"));
    assert!(prompt.contains("design"));

    // Should not claim to be a general coding assistant
    assert!(!prompt.contains("coding assistant"));
}

#[test]
fn test_mode_equality() {
    let normal1 = Mode::Normal;
    let normal2 = Mode::Normal;
    assert_eq!(normal1, normal2);

    let planning1 = Mode::planning("test.md".to_string());
    let planning2 = Mode::planning("test.md".to_string());
    assert_eq!(planning1, planning2);

    let planning3 = Mode::planning("other.md".to_string());
    assert_ne!(planning1, planning3);
    assert_ne!(normal1, planning1);
}

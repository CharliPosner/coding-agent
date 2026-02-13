//! CLI modes - normal conversation vs planning mode
//!
//! Modes affect how the AI agent behaves and what prompts it receives.

/// The current mode of the CLI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Normal conversation mode - general purpose assistant
    Normal,
    /// Planning mode - collaborative spec writing and design
    Planning { spec_file: String },
}

impl Mode {
    /// Create a new planning mode with the given spec file
    pub fn planning(spec_file: String) -> Self {
        Mode::Planning { spec_file }
    }

    /// Check if currently in planning mode
    pub fn is_planning(&self) -> bool {
        matches!(self, Mode::Planning { .. })
    }

    /// Get the spec file if in planning mode
    pub fn spec_file(&self) -> Option<&str> {
        match self {
            Mode::Planning { spec_file } => Some(spec_file),
            Mode::Normal => None,
        }
    }

    /// Get the system prompt for this mode
    pub fn system_prompt(&self) -> String {
        match self {
            Mode::Normal => {
                r#"You are a helpful AI coding assistant integrated into a command-line interface.

Your role is to:
- Help users understand, write, and debug code
- Answer technical questions
- Execute tools when needed to read files, run commands, etc.
- Be concise and clear in your responses
- Ask clarifying questions when requirements are unclear

You have access to various tools for file operations, shell commands, and code search.
Use them when appropriate to help answer questions or complete tasks."#.to_string()
            }
            Mode::Planning { spec_file } => {
                format!(
                    r#"You are a collaborative specification planning assistant.

The user is currently working on: {}

Your role in planning mode is to:
- Help refine and clarify requirements
- Suggest design approaches and trade-offs
- Ask probing questions to uncover hidden requirements
- Propose implementation strategies
- Identify potential issues and edge cases
- Structure the specification document clearly

Guidelines:
- Be collaborative, not prescriptive - guide the user to their own decisions
- Ask one or two focused questions at a time, don't overwhelm
- When proposing options, explain trade-offs clearly
- Focus on the "why" and "what" before diving into "how"
- Update the spec file regularly as decisions are made
- Help organize thoughts into a clear structure

Remember: You're helping the user think through their design, not designing it for them."#,
                    spec_file
                )
            }
        }
    }

    /// Get a visual indicator for the current mode
    pub fn indicator(&self) -> Option<String> {
        match self {
            Mode::Normal => None,
            Mode::Planning { spec_file } => {
                Some(format!("📋 Planning: {}", spec_file))
            }
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_default() {
        let mode = Mode::default();
        assert_eq!(mode, Mode::Normal);
    }

    #[test]
    fn test_mode_is_planning() {
        let normal = Mode::Normal;
        assert!(!normal.is_planning());

        let planning = Mode::planning("auth.md".to_string());
        assert!(planning.is_planning());
    }

    #[test]
    fn test_mode_spec_file() {
        let normal = Mode::Normal;
        assert_eq!(normal.spec_file(), None);

        let planning = Mode::planning("auth.md".to_string());
        assert_eq!(planning.spec_file(), Some("auth.md"));
    }

    #[test]
    fn test_mode_system_prompt() {
        let normal = Mode::Normal;
        let normal_prompt = normal.system_prompt();
        assert!(normal_prompt.contains("coding assistant"));
        assert!(!normal_prompt.contains("planning mode"));

        let planning = Mode::planning("auth.md".to_string());
        let planning_prompt = planning.system_prompt();
        assert!(planning_prompt.contains("specification planning"));
        assert!(planning_prompt.contains("auth.md"));
        assert!(planning_prompt.contains("collaborative"));
    }

    #[test]
    fn test_mode_indicator() {
        let normal = Mode::Normal;
        assert_eq!(normal.indicator(), None);

        let planning = Mode::planning("auth.md".to_string());
        let indicator = planning.indicator();
        assert!(indicator.is_some());
        let indicator_text = indicator.unwrap();
        assert!(indicator_text.contains("Planning"));
        assert!(indicator_text.contains("auth.md"));
    }
}

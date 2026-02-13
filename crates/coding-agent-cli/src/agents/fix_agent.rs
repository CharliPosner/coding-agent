//! Fix-agent for self-healing error recovery.
//!
//! The fix-agent is spawned when a code error occurs to automatically diagnose
//! and fix the issue. It can handle errors like missing dependencies, missing
//! imports, type errors, and syntax errors.

use crate::tools::{ErrorCategory, ToolError, ToolExecutionResult};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Counter for generating unique agent IDs.
static AGENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Configuration for the fix-agent.
#[derive(Debug, Clone)]
pub struct FixAgentConfig {
    /// Maximum number of fix attempts before giving up.
    pub max_attempts: u32,

    /// Whether to generate regression tests after successful fixes.
    pub generate_tests: bool,

    /// Timeout for each fix attempt in milliseconds.
    pub attempt_timeout_ms: u64,

    /// Whether to allow fixes that modify multiple files.
    pub allow_multi_file_fixes: bool,
}

impl Default for FixAgentConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            generate_tests: true,
            attempt_timeout_ms: 30000, // 30 seconds
            allow_multi_file_fixes: true,
        }
    }
}

/// Status of a fix agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixStatus {
    /// Agent is waiting to be started.
    Pending,

    /// Agent is currently analyzing the error.
    Analyzing,

    /// Agent is applying a fix.
    Applying,

    /// Agent is verifying the fix worked.
    Verifying,

    /// Agent successfully fixed the error.
    Success,

    /// Agent failed to fix the error after all attempts.
    Failed,

    /// Agent was cancelled.
    Cancelled,
}

impl std::fmt::Display for FixStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixStatus::Pending => write!(f, "Pending"),
            FixStatus::Analyzing => write!(f, "Analyzing"),
            FixStatus::Applying => write!(f, "Applying"),
            FixStatus::Verifying => write!(f, "Verifying"),
            FixStatus::Success => write!(f, "Success"),
            FixStatus::Failed => write!(f, "Failed"),
            FixStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// A single fix attempt with its result.
#[derive(Debug, Clone)]
pub struct FixAttempt {
    /// Attempt number (1-indexed).
    pub attempt_number: u32,

    /// Description of the fix that was attempted.
    pub description: String,

    /// Files that were modified.
    pub modified_files: Vec<String>,

    /// Whether the fix was successful.
    pub success: bool,

    /// Error message if the fix failed.
    pub error_message: Option<String>,

    /// Duration of this attempt.
    pub duration: Duration,
}

/// Result of a fix-agent operation.
#[derive(Debug, Clone)]
pub struct FixResult {
    /// The agent ID that produced this result.
    pub agent_id: u64,

    /// Final status of the fix operation.
    pub status: FixStatus,

    /// All fix attempts made.
    pub attempts: Vec<FixAttempt>,

    /// The original error that triggered the fix-agent.
    pub original_error: String,

    /// Generated regression test (if any).
    pub generated_test: Option<String>,

    /// Total duration of the fix operation.
    pub total_duration: Duration,
}

impl FixResult {
    /// Check if the fix was successful.
    pub fn is_success(&self) -> bool {
        self.status == FixStatus::Success
    }

    /// Get the number of attempts made.
    pub fn attempt_count(&self) -> usize {
        self.attempts.len()
    }

    /// Get the last attempt (if any).
    pub fn last_attempt(&self) -> Option<&FixAttempt> {
        self.attempts.last()
    }

    /// Get all modified files across all attempts.
    pub fn all_modified_files(&self) -> Vec<&str> {
        self.attempts
            .iter()
            .flat_map(|a| a.modified_files.iter())
            .map(|s| s.as_str())
            .collect()
    }
}

/// A fix-agent that attempts to automatically repair code errors.
///
/// The fix-agent is spawned when tool execution fails with a code-related error.
/// It analyzes the error, attempts to diagnose the issue, and applies fixes.
pub struct FixAgent {
    /// Unique identifier for this agent.
    id: u64,

    /// Configuration for this agent.
    config: FixAgentConfig,

    /// Current status of the agent.
    status: FixStatus,

    /// The error being fixed.
    error: ToolError,

    /// The tool execution result that triggered this agent.
    execution_result: ToolExecutionResult,

    /// All fix attempts made.
    attempts: Vec<FixAttempt>,

    /// When the agent was created.
    created_at: Instant,

    /// Generated regression test (if any).
    generated_test: Option<String>,

    /// Callback for status updates.
    status_callback: Option<Arc<dyn Fn(FixStatus) + Send + Sync>>,
}

impl FixAgent {
    /// Create a new fix-agent for the given tool execution result.
    ///
    /// Returns `None` if the error is not auto-fixable (i.e., not a code error).
    pub fn spawn(result: ToolExecutionResult, config: FixAgentConfig) -> Option<Self> {
        // Only spawn for auto-fixable errors
        let error = result.error()?.clone();
        if !error.is_auto_fixable() {
            return None;
        }

        let id = AGENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        Some(Self {
            id,
            config,
            status: FixStatus::Pending,
            error,
            execution_result: result,
            attempts: Vec::new(),
            created_at: Instant::now(),
            generated_test: None,
            status_callback: None,
        })
    }

    /// Create a fix-agent with default configuration.
    pub fn spawn_with_defaults(result: ToolExecutionResult) -> Option<Self> {
        Self::spawn(result, FixAgentConfig::default())
    }

    /// Set a callback to be notified of status changes.
    pub fn on_status_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(FixStatus) + Send + Sync + 'static,
    {
        self.status_callback = Some(Arc::new(callback));
        self
    }

    /// Get the agent's unique ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the current status.
    pub fn status(&self) -> &FixStatus {
        &self.status
    }

    /// Get the error being fixed.
    pub fn error(&self) -> &ToolError {
        &self.error
    }

    /// Get the original execution result.
    pub fn execution_result(&self) -> &ToolExecutionResult {
        &self.execution_result
    }

    /// Get the number of attempts made so far.
    pub fn attempt_count(&self) -> usize {
        self.attempts.len()
    }

    /// Check if the agent has more attempts remaining.
    pub fn has_attempts_remaining(&self) -> bool {
        (self.attempts.len() as u32) < self.config.max_attempts
    }

    /// Get the fix diagnosis based on the error category.
    ///
    /// Returns a tuple of (fix_type, description, suggested_action).
    pub fn diagnose(&self) -> (&'static str, &'static str, &'static str) {
        match &self.error.category {
            ErrorCategory::Code { error_type } => match error_type.as_str() {
                "missing_dependency" => (
                    "missing_dependency",
                    "A required dependency is not declared",
                    "Add the dependency to Cargo.toml or package.json",
                ),
                "missing_import" => (
                    "missing_import",
                    "A required module or item is not imported",
                    "Add the missing import statement",
                ),
                "type_error" => (
                    "type_error",
                    "Type mismatch in the code",
                    "Adjust types or add conversions",
                ),
                "syntax_error" => (
                    "syntax_error",
                    "Syntax error in the code",
                    "Fix the syntax issue",
                ),
                _ => (
                    "unknown_code_error",
                    "An unknown code error occurred",
                    "Investigate and fix the issue",
                ),
            },
            _ => (
                "not_code_error",
                "This error is not a code error",
                "Cannot auto-fix this type of error",
            ),
        }
    }

    /// Update the agent's status and notify callback if set.
    fn set_status(&mut self, status: FixStatus) {
        self.status = status.clone();
        if let Some(callback) = &self.status_callback {
            callback(status);
        }
    }

    /// Attempt to fix the error.
    ///
    /// This is a synchronous operation that will:
    /// 1. Analyze the error
    /// 2. Determine the appropriate fix
    /// 3. Apply the fix
    /// 4. Verify it worked
    ///
    /// The `apply_fix` callback is called with the fix description and should
    /// return Ok(modified_files) on success or Err(message) on failure.
    ///
    /// The `verify_fix` callback is called after the fix is applied and should
    /// return Ok(()) if the fix worked or Err(message) if it didn't.
    pub fn attempt_fix<F, V>(&mut self, mut apply_fix: F, mut verify_fix: V) -> FixResult
    where
        F: FnMut(&str, &ErrorCategory) -> Result<Vec<String>, String>,
        V: FnMut() -> Result<(), String>,
    {
        let start = Instant::now();

        // Start analyzing
        self.set_status(FixStatus::Analyzing);
        let (fix_type, description, _suggested_action) = self.diagnose();

        while self.has_attempts_remaining() {
            let attempt_start = Instant::now();
            let attempt_number = (self.attempts.len() + 1) as u32;

            // Apply the fix
            self.set_status(FixStatus::Applying);
            let apply_result = apply_fix(fix_type, &self.error.category);

            match apply_result {
                Ok(modified_files) => {
                    // Verify the fix
                    self.set_status(FixStatus::Verifying);
                    let verify_result = verify_fix();

                    match verify_result {
                        Ok(()) => {
                            // Success!
                            let attempt = FixAttempt {
                                attempt_number,
                                description: description.to_string(),
                                modified_files,
                                success: true,
                                error_message: None,
                                duration: attempt_start.elapsed(),
                            };
                            self.attempts.push(attempt);
                            self.set_status(FixStatus::Success);

                            // Generate regression test if configured
                            if self.config.generate_tests {
                                self.generated_test = Some(self.generate_regression_test());
                            }

                            return self.build_result(start.elapsed());
                        }
                        Err(verify_error) => {
                            // Fix didn't work
                            let attempt = FixAttempt {
                                attempt_number,
                                description: description.to_string(),
                                modified_files,
                                success: false,
                                error_message: Some(format!("Verification failed: {}", verify_error)),
                                duration: attempt_start.elapsed(),
                            };
                            self.attempts.push(attempt);
                            // Continue to next attempt
                        }
                    }
                }
                Err(apply_error) => {
                    let attempt = FixAttempt {
                        attempt_number,
                        description: description.to_string(),
                        modified_files: vec![],
                        success: false,
                        error_message: Some(format!("Failed to apply fix: {}", apply_error)),
                        duration: attempt_start.elapsed(),
                    };
                    self.attempts.push(attempt);
                    // Continue to next attempt
                }
            }
        }

        // All attempts exhausted
        self.set_status(FixStatus::Failed);
        self.build_result(start.elapsed())
    }

    /// Cancel the fix operation.
    pub fn cancel(&mut self) -> FixResult {
        self.set_status(FixStatus::Cancelled);
        self.build_result(self.created_at.elapsed())
    }

    /// Generate a regression test for the fix.
    fn generate_regression_test(&self) -> String {
        let (fix_type, _description, _action) = self.diagnose();
        let error_msg = &self.error.message;

        match fix_type {
            "missing_dependency" => format!(
                r#"#[test]
fn test_dependency_available() {{
    // Regression test: ensures the dependency fix for:
    // {}
    // is not reverted
    compile_error!("Replace this with actual dependency usage test");
}}"#,
                error_msg
            ),
            "missing_import" => format!(
                r#"#[test]
fn test_import_available() {{
    // Regression test: ensures the import fix for:
    // {}
    // is not reverted
    compile_error!("Replace this with actual import usage test");
}}"#,
                error_msg
            ),
            "type_error" => format!(
                r#"#[test]
fn test_type_compatibility() {{
    // Regression test: ensures the type fix for:
    // {}
    // is not reverted
    compile_error!("Replace this with actual type check test");
}}"#,
                error_msg
            ),
            "syntax_error" => format!(
                r#"#[test]
fn test_syntax_valid() {{
    // Regression test: ensures the syntax fix for:
    // {}
    // is not reverted
    // (This test verifies compilation succeeds)
}}"#,
                error_msg
            ),
            _ => format!(
                r#"#[test]
fn test_fix_not_reverted() {{
    // Regression test for: {}
    compile_error!("Replace this with actual verification test");
}}"#,
                error_msg
            ),
        }
    }

    /// Build the final result.
    fn build_result(&self, total_duration: Duration) -> FixResult {
        FixResult {
            agent_id: self.id,
            status: self.status.clone(),
            attempts: self.attempts.clone(),
            original_error: self.error.message.clone(),
            generated_test: self.generated_test.clone(),
            total_duration,
        }
    }
}

impl std::fmt::Debug for FixAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixAgent")
            .field("id", &self.id)
            .field("status", &self.status)
            .field("error", &self.error.message)
            .field("attempts", &self.attempts.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolError, ToolExecutionResult};
    use std::time::Duration;

    fn make_code_error_result(error_msg: &str) -> ToolExecutionResult {
        ToolExecutionResult {
            tool_name: "test_tool".to_string(),
            call_id: "call_1".to_string(),
            result: Err(ToolError::new(error_msg)),
            duration: Duration::from_millis(100),
            retries: 0,
        }
    }

    fn make_permission_error_result() -> ToolExecutionResult {
        ToolExecutionResult {
            tool_name: "test_tool".to_string(),
            call_id: "call_1".to_string(),
            result: Err(ToolError::new("Permission denied: '/etc/shadow'")),
            duration: Duration::from_millis(100),
            retries: 0,
        }
    }

    #[test]
    fn test_spawn_for_code_error() {
        let result = make_code_error_result("cannot find crate `serde_json`");
        let agent = FixAgent::spawn_with_defaults(result);

        assert!(agent.is_some());
        let agent = agent.unwrap();
        assert!(agent.id() > 0);
        assert_eq!(agent.status(), &FixStatus::Pending);
        assert!(agent.has_attempts_remaining());
    }

    #[test]
    fn test_no_spawn_for_permission_error() {
        let result = make_permission_error_result();
        let agent = FixAgent::spawn_with_defaults(result);

        assert!(agent.is_none());
    }

    #[test]
    fn test_no_spawn_for_success_result() {
        let result = ToolExecutionResult {
            tool_name: "test".to_string(),
            call_id: "1".to_string(),
            result: Ok("success".to_string()),
            duration: Duration::from_millis(100),
            retries: 0,
        };
        let agent = FixAgent::spawn_with_defaults(result);

        assert!(agent.is_none());
    }

    #[test]
    fn test_diagnose_missing_dependency() {
        let result = make_code_error_result("cannot find crate `serde_json`");
        let agent = FixAgent::spawn_with_defaults(result).unwrap();

        let (fix_type, description, _) = agent.diagnose();
        assert_eq!(fix_type, "missing_dependency");
        assert!(description.contains("dependency"));
    }

    #[test]
    fn test_diagnose_missing_import() {
        let result = make_code_error_result("cannot find value `HashMap` in this scope");
        let agent = FixAgent::spawn_with_defaults(result).unwrap();

        let (fix_type, _, _) = agent.diagnose();
        assert_eq!(fix_type, "missing_import");
    }

    #[test]
    fn test_diagnose_type_error() {
        let result = make_code_error_result("mismatched types: expected `&str`, found `String`");
        let agent = FixAgent::spawn_with_defaults(result).unwrap();

        let (fix_type, _, _) = agent.diagnose();
        assert_eq!(fix_type, "type_error");
    }

    #[test]
    fn test_diagnose_syntax_error() {
        let result = make_code_error_result("syntax error: expected `;`");
        let agent = FixAgent::spawn_with_defaults(result).unwrap();

        let (fix_type, _, _) = agent.diagnose();
        assert_eq!(fix_type, "syntax_error");
    }

    #[test]
    fn test_attempt_fix_success_first_try() {
        let result = make_code_error_result("cannot find crate `serde`");
        let mut agent = FixAgent::spawn_with_defaults(result).unwrap();

        let fix_result = agent.attempt_fix(
            |_fix_type, _category| Ok(vec!["Cargo.toml".to_string()]),
            || Ok(()),
        );

        assert!(fix_result.is_success());
        assert_eq!(fix_result.attempt_count(), 1);
        assert_eq!(fix_result.status, FixStatus::Success);
        assert!(fix_result.generated_test.is_some());
    }

    #[test]
    fn test_attempt_fix_success_after_retry() {
        use std::cell::Cell;
        use std::rc::Rc;

        let result = make_code_error_result("cannot find crate `serde`");
        let mut agent = FixAgent::spawn_with_defaults(result).unwrap();

        let call_count = Rc::new(Cell::new(0u32));
        let call_count_apply = call_count.clone();
        let call_count_verify = call_count.clone();

        let fix_result = agent.attempt_fix(
            move |_fix_type, _category| {
                call_count_apply.set(call_count_apply.get() + 1);
                Ok(vec!["Cargo.toml".to_string()])
            },
            move || {
                // Fail first two times, succeed on third
                if call_count_verify.get() < 3 {
                    Err("still broken".to_string())
                } else {
                    Ok(())
                }
            },
        );

        assert!(fix_result.is_success());
        assert_eq!(fix_result.attempt_count(), 3);
    }

    #[test]
    fn test_attempt_fix_all_attempts_fail() {
        let result = make_code_error_result("cannot find crate `serde`");
        let mut agent = FixAgent::spawn_with_defaults(result).unwrap();

        let fix_result = agent.attempt_fix(
            |_fix_type, _category| Ok(vec!["Cargo.toml".to_string()]),
            || Err("verification failed".to_string()),
        );

        assert!(!fix_result.is_success());
        assert_eq!(fix_result.status, FixStatus::Failed);
        assert_eq!(fix_result.attempt_count(), 3); // max_attempts default is 3
    }

    #[test]
    fn test_attempt_fix_apply_fails() {
        let result = make_code_error_result("cannot find crate `serde`");
        let mut agent = FixAgent::spawn_with_defaults(result).unwrap();

        let fix_result = agent.attempt_fix(
            |_fix_type, _category| Err("cannot modify file".to_string()),
            || Ok(()),
        );

        assert!(!fix_result.is_success());
        assert_eq!(fix_result.status, FixStatus::Failed);

        // All attempts should have failed to apply
        for attempt in &fix_result.attempts {
            assert!(!attempt.success);
            assert!(attempt.error_message.is_some());
            assert!(attempt.modified_files.is_empty());
        }
    }

    #[test]
    fn test_cancel() {
        let result = make_code_error_result("cannot find crate `serde`");
        let mut agent = FixAgent::spawn_with_defaults(result).unwrap();

        let fix_result = agent.cancel();

        assert_eq!(fix_result.status, FixStatus::Cancelled);
        assert_eq!(fix_result.attempt_count(), 0);
    }

    #[test]
    fn test_custom_config_max_attempts() {
        let result = make_code_error_result("cannot find crate `serde`");
        let config = FixAgentConfig {
            max_attempts: 5,
            ..Default::default()
        };
        let mut agent = FixAgent::spawn(result, config).unwrap();

        let fix_result = agent.attempt_fix(
            |_fix_type, _category| Ok(vec!["file.rs".to_string()]),
            || Err("still failing".to_string()),
        );

        assert_eq!(fix_result.attempt_count(), 5);
    }

    #[test]
    fn test_no_test_generation_when_disabled() {
        let result = make_code_error_result("cannot find crate `serde`");
        let config = FixAgentConfig {
            generate_tests: false,
            ..Default::default()
        };
        let mut agent = FixAgent::spawn(result, config).unwrap();

        let fix_result = agent.attempt_fix(
            |_fix_type, _category| Ok(vec!["Cargo.toml".to_string()]),
            || Ok(()),
        );

        assert!(fix_result.is_success());
        assert!(fix_result.generated_test.is_none());
    }

    #[test]
    fn test_status_callback() {
        use std::sync::Mutex;

        let result = make_code_error_result("cannot find crate `serde`");
        let statuses: Arc<Mutex<Vec<FixStatus>>> = Arc::new(Mutex::new(Vec::new()));
        let statuses_clone = statuses.clone();

        let mut agent = FixAgent::spawn_with_defaults(result)
            .unwrap()
            .on_status_change(move |status| {
                statuses_clone.lock().unwrap().push(status);
            });

        let _ = agent.attempt_fix(
            |_fix_type, _category| Ok(vec!["file.rs".to_string()]),
            || Ok(()),
        );

        let captured = statuses.lock().unwrap();
        assert!(captured.contains(&FixStatus::Analyzing));
        assert!(captured.contains(&FixStatus::Applying));
        assert!(captured.contains(&FixStatus::Verifying));
        assert!(captured.contains(&FixStatus::Success));
    }

    #[test]
    fn test_fix_result_all_modified_files() {
        use std::cell::Cell;
        use std::rc::Rc;

        let result = make_code_error_result("cannot find crate `serde`");
        let mut agent = FixAgent::spawn_with_defaults(result).unwrap();

        let call_count = Rc::new(Cell::new(0u32));
        let call_count_apply = call_count.clone();
        let call_count_verify = call_count.clone();

        let fix_result = agent.attempt_fix(
            move |_fix_type, _category| {
                let count = call_count_apply.get() + 1;
                call_count_apply.set(count);
                Ok(vec![format!("file{}.rs", count)])
            },
            move || {
                // Fail first, succeed second
                if call_count_verify.get() < 2 {
                    Err("retry".to_string())
                } else {
                    Ok(())
                }
            },
        );

        let files = fix_result.all_modified_files();
        assert!(files.len() >= 1);
    }

    #[test]
    fn test_fix_status_display() {
        assert_eq!(format!("{}", FixStatus::Pending), "Pending");
        assert_eq!(format!("{}", FixStatus::Analyzing), "Analyzing");
        assert_eq!(format!("{}", FixStatus::Applying), "Applying");
        assert_eq!(format!("{}", FixStatus::Verifying), "Verifying");
        assert_eq!(format!("{}", FixStatus::Success), "Success");
        assert_eq!(format!("{}", FixStatus::Failed), "Failed");
        assert_eq!(format!("{}", FixStatus::Cancelled), "Cancelled");
    }

    #[test]
    fn test_unique_agent_ids() {
        let result1 = make_code_error_result("cannot find crate `serde`");
        let result2 = make_code_error_result("cannot find crate `tokio`");

        let agent1 = FixAgent::spawn_with_defaults(result1).unwrap();
        let agent2 = FixAgent::spawn_with_defaults(result2).unwrap();

        assert_ne!(agent1.id(), agent2.id());
    }

    #[test]
    fn test_generated_test_for_missing_dependency() {
        let result = make_code_error_result("cannot find crate `serde`");
        let mut agent = FixAgent::spawn_with_defaults(result).unwrap();

        let fix_result = agent.attempt_fix(
            |_fix_type, _category| Ok(vec!["Cargo.toml".to_string()]),
            || Ok(()),
        );

        let test = fix_result.generated_test.unwrap();
        assert!(test.contains("#[test]"));
        assert!(test.contains("dependency"));
    }

    #[test]
    fn test_fix_agent_debug_impl() {
        let result = make_code_error_result("cannot find crate `serde`");
        let agent = FixAgent::spawn_with_defaults(result).unwrap();

        let debug_output = format!("{:?}", agent);
        assert!(debug_output.contains("FixAgent"));
        assert!(debug_output.contains("Pending"));
    }

    #[test]
    fn test_default_config_values() {
        let config = FixAgentConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert!(config.generate_tests);
        assert_eq!(config.attempt_timeout_ms, 30000);
        assert!(config.allow_multi_file_fixes);
    }
}

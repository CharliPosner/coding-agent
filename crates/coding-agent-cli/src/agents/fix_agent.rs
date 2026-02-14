//! Fix-agent for self-healing error recovery.
//!
//! The fix-agent is spawned when a code error occurs to automatically diagnose
//! and fix the issue. It can handle errors like missing dependencies, missing
//! imports, type errors, and syntax errors.
//!
//! ## Deviation Rules
//!
//! The fix-agent implements explicit autonomy boundaries that determine when
//! to auto-fix errors vs when to ask the user for permission:
//!
//! **Auto-fix (proceed without asking):**
//! - `DeviationCategory::AgentCode` - Code errors the agent introduced
//! - `DeviationCategory::Dependency` - Broken dependencies (missing crate in Cargo.toml)
//! - `DeviationCategory::TestLint` - Test/lint failures from agent's changes
//!
//! **Must ask (blocks execution until user confirms):**
//! - `DeviationCategory::Architecture` - New modules, schema changes
//! - `DeviationCategory::NewDependency` - Adding dependencies not mentioned in task
//! - `DeviationCategory::FileDeletion` - Deleting files

use crate::tools::{
    ErrorCategory, FixApplicationResult, FixInfo, FixType, RegressionTest, RegressionTestConfig,
    ToolError, ToolExecutionResult,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Categories of deviations that determine agent autonomy boundaries.
///
/// These categories classify the type of change or error to determine whether
/// the fix-agent should proceed automatically or ask the user for permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviationCategory {
    /// Code errors the agent introduced (compiler errors, type mismatches).
    /// These are typically safe to auto-fix since the agent caused them.
    AgentCode,

    /// Broken dependencies (missing crate in Cargo.toml that code needs).
    /// Safe to auto-fix by adding the required dependency.
    Dependency,

    /// Test/lint failures from agent's changes.
    /// Safe to auto-fix since these validate the agent's own work.
    TestLint,

    /// Architectural changes (new modules, schema changes, significant refactors).
    /// Requires user approval as these affect project structure.
    Architecture,

    /// Adding dependencies not mentioned in task.
    /// Requires user approval to avoid scope creep.
    NewDependency,

    /// Deleting files.
    /// Requires user approval due to potential data loss.
    FileDeletion,
}

/// Rules that determine whether the agent should auto-fix or ask the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviationRule {
    /// Auto-fix: proceed without asking the user.
    /// Used for errors that the agent introduced and can safely fix.
    AutoFix,

    /// Must ask: blocks execution until user confirms.
    /// Used for changes that affect project structure or scope.
    MustAsk,
}

impl DeviationCategory {
    /// Get the deviation rule for this category.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use coding_agent_cli::agents::DeviationCategory;
    /// use coding_agent_cli::agents::DeviationRule;
    ///
    /// // Auto-fix categories
    /// assert_eq!(DeviationCategory::AgentCode.deviation_rule(), DeviationRule::AutoFix);
    /// assert_eq!(DeviationCategory::Dependency.deviation_rule(), DeviationRule::AutoFix);
    /// assert_eq!(DeviationCategory::TestLint.deviation_rule(), DeviationRule::AutoFix);
    ///
    /// // Must-ask categories
    /// assert_eq!(DeviationCategory::Architecture.deviation_rule(), DeviationRule::MustAsk);
    /// assert_eq!(DeviationCategory::NewDependency.deviation_rule(), DeviationRule::MustAsk);
    /// assert_eq!(DeviationCategory::FileDeletion.deviation_rule(), DeviationRule::MustAsk);
    /// ```
    pub fn deviation_rule(&self) -> DeviationRule {
        match self {
            // Auto-fix rules (1-3): Safe to proceed without user approval
            Self::AgentCode => DeviationRule::AutoFix,
            Self::Dependency => DeviationRule::AutoFix,
            Self::TestLint => DeviationRule::AutoFix,
            // Must-ask rules (4-6): Require user approval
            Self::Architecture => DeviationRule::MustAsk,
            Self::NewDependency => DeviationRule::MustAsk,
            Self::FileDeletion => DeviationRule::MustAsk,
        }
    }

    /// Check if this category allows auto-fixing.
    pub fn allows_auto_fix(&self) -> bool {
        self.deviation_rule() == DeviationRule::AutoFix
    }
}

/// Categorize an error message into a deviation category.
///
/// Uses heuristics to determine what type of error or change is being proposed,
/// which then determines whether the fix-agent should auto-fix or ask the user.
///
/// # Examples
///
/// ```rust
/// use coding_agent_cli::agents::{categorize_deviation, DeviationCategory};
///
/// // Code errors
/// assert_eq!(
///     categorize_deviation("cannot find value `foo` in this scope"),
///     DeviationCategory::AgentCode
/// );
///
/// // Dependency errors
/// assert_eq!(
///     categorize_deviation("cannot find crate `serde`"),
///     DeviationCategory::Dependency
/// );
///
/// // Test/lint failures
/// assert_eq!(
///     categorize_deviation("test failed: expected 5, got 3"),
///     DeviationCategory::TestLint
/// );
/// ```
pub fn categorize_deviation(error_message: &str) -> DeviationCategory {
    let lower = error_message.to_lowercase();

    // Check for dependency-related errors (missing crate/package)
    if (lower.contains("cannot find") || lower.contains("not found") || lower.contains("unresolved"))
        && (lower.contains("crate") || lower.contains("package") || lower.contains("module"))
    {
        return DeviationCategory::Dependency;
    }

    // Check for test or lint failures
    if lower.contains("test failed")
        || lower.contains("test failure")
        || lower.contains("assertion failed")
        || lower.contains("clippy")
        || lower.contains("lint")
        || lower.contains("warning:")
            && (lower.contains("unused") || lower.contains("dead_code"))
    {
        return DeviationCategory::TestLint;
    }

    // Check for file deletion (must ask)
    if lower.contains("delete") && lower.contains("file")
        || lower.contains("remove") && lower.contains("file")
        || lower.contains("rm ") && !lower.contains("rm -rf /") // safety check
    {
        return DeviationCategory::FileDeletion;
    }

    // Check for architectural changes (must ask)
    if lower.contains("new module")
        || lower.contains("create module")
        || lower.contains("schema change")
        || lower.contains("refactor")
        || lower.contains("restructure")
        || lower.contains("reorganize")
    {
        return DeviationCategory::Architecture;
    }

    // Check for adding new dependencies not in task (must ask)
    if lower.contains("add dependency")
        || lower.contains("add crate")
        || lower.contains("install package")
        || lower.contains("npm install")
        || lower.contains("cargo add")
    {
        return DeviationCategory::NewDependency;
    }

    // Default to agent code errors (safe to auto-fix)
    // This covers: type errors, syntax errors, missing imports, etc.
    DeviationCategory::AgentCode
}

/// Determine if a fix should be attempted based on deviation rules.
///
/// Returns `true` if the error can be auto-fixed, `false` if user approval is needed.
///
/// # Arguments
///
/// * `error_message` - The error message to categorize
///
/// # Examples
///
/// ```rust
/// use coding_agent_cli::agents::should_auto_fix;
///
/// // Auto-fixable errors
/// assert!(should_auto_fix("cannot find value `foo` in this scope"));
/// assert!(should_auto_fix("cannot find crate `serde`"));
///
/// // Requires user approval
/// assert!(!should_auto_fix("need to delete file old_config.rs"));
/// assert!(!should_auto_fix("requires new module for feature"));
/// ```
pub fn should_auto_fix(error_message: &str) -> bool {
    categorize_deviation(error_message).allows_auto_fix()
}

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

    /// Configuration for regression test generation.
    pub regression_test_config: RegressionTestConfig,
}

impl Default for FixAgentConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            generate_tests: true,
            attempt_timeout_ms: 30000, // 30 seconds
            allow_multi_file_fixes: true,
            regression_test_config: RegressionTestConfig::default(),
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
    pub generated_test: Option<RegressionTest>,

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
    generated_test: Option<RegressionTest>,

    /// Callback for status updates.
    status_callback: Option<Arc<dyn Fn(FixStatus) + Send + Sync>>,

    /// Fix info extracted from the error (for test generation).
    fix_info: Option<FixInfo>,

    /// Last successful fix result (for test generation).
    last_fix_result: Option<FixApplicationResult>,
}

impl FixAgent {
    /// Create a new fix-agent for the given tool execution result.
    ///
    /// Returns `None` if the error is not auto-fixable (i.e., not a code error).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::agents::{FixAgent, FixAgentConfig};
    /// use coding_agent_cli::tools::{ToolExecutor, ToolError, ToolExecutionResult};
    /// use std::time::Duration;
    ///
    /// // Simulate a tool execution that failed with a code error
    /// let result = ToolExecutionResult {
    ///     tool_name: "cargo_build".to_string(),
    ///     call_id: "call_1".to_string(),
    ///     result: Err(ToolError::new("cannot find crate `serde_json`")),
    ///     duration: Duration::from_millis(100),
    ///     retries: 0,
    /// };
    ///
    /// // Try to spawn a fix-agent
    /// if let Some(mut agent) = FixAgent::spawn_with_defaults(result) {
    ///     println!("Fix-agent spawned with ID: {}", agent.id());
    ///
    ///     // Attempt to fix the error
    ///     let fix_result = agent.attempt_fix(
    ///         |fix_type, _category| {
    ///             // Apply fix: add dependency to Cargo.toml
    ///             Ok(vec!["Cargo.toml".to_string()])
    ///         },
    ///         || {
    ///             // Verify fix: rebuild the project
    ///             Ok(())
    ///         }
    ///     );
    ///
    ///     if fix_result.is_success() {
    ///         println!("Successfully fixed after {} attempts", fix_result.attempt_count());
    ///     }
    /// }
    /// ```
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
            fix_info: None,
            last_fix_result: None,
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

    /// Get the deviation category for this agent's error.
    ///
    /// This categorizes the error to determine whether the agent should
    /// auto-fix or ask the user for permission.
    pub fn deviation_category(&self) -> DeviationCategory {
        categorize_deviation(&self.error.message)
    }

    /// Get the deviation rule for this agent's error.
    ///
    /// Returns `DeviationRule::AutoFix` if the agent can proceed without asking,
    /// or `DeviationRule::MustAsk` if user approval is required.
    pub fn deviation_rule(&self) -> DeviationRule {
        self.deviation_category().deviation_rule()
    }

    /// Check if this agent should attempt to fix automatically based on deviation rules.
    ///
    /// Returns `true` if:
    /// - The error is technically auto-fixable (code error)
    /// - The deviation category allows auto-fixing
    ///
    /// Returns `false` if user approval is required.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::agents::FixAgent;
    /// use coding_agent_cli::tools::{ToolError, ToolExecutionResult};
    /// use std::time::Duration;
    ///
    /// // Code error - should auto-fix
    /// let result = ToolExecutionResult {
    ///     tool_name: "build".to_string(),
    ///     call_id: "1".to_string(),
    ///     result: Err(ToolError::new("cannot find value `foo` in this scope")),
    ///     duration: Duration::from_millis(100),
    ///     retries: 0,
    /// };
    /// if let Some(agent) = FixAgent::spawn_with_defaults(result) {
    ///     assert!(agent.should_attempt_fix());
    /// }
    /// ```
    pub fn should_attempt_fix(&self) -> bool {
        // Must be a technically fixable error AND have an auto-fix deviation rule
        self.error.is_auto_fixable() && self.deviation_category().allows_auto_fix()
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
    /// return Ok((modified_files, fix_result)) on success or Err(message) on failure.
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

        // Build fix info for test generation
        self.fix_info = Some(self.build_fix_info(fix_type));

        while self.has_attempts_remaining() {
            let attempt_start = Instant::now();
            let attempt_number = (self.attempts.len() + 1) as u32;

            // Apply the fix
            self.set_status(FixStatus::Applying);
            let apply_result = apply_fix(fix_type, &self.error.category);

            match apply_result {
                Ok(modified_files) => {
                    // Store fix result for test generation
                    self.last_fix_result = Some(FixApplicationResult::success(
                        modified_files.iter().map(PathBuf::from).collect(),
                        description,
                    ));

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
                                self.generated_test = self.generate_regression_test();
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
                                error_message: Some(format!(
                                    "Verification failed: {}",
                                    verify_error
                                )),
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

    /// Build FixInfo from the diagnosed fix type.
    fn build_fix_info(&self, fix_type: &str) -> FixInfo {
        let ft = match fix_type {
            "missing_dependency" => FixType::AddDependency,
            "missing_import" => FixType::AddImport,
            "type_error" => FixType::FixType,
            "syntax_error" => FixType::FixSyntax,
            _ => FixType::FixSyntax, // Default fallback
        };

        let (target_file, target_item) = match &self.error.category {
            ErrorCategory::Code { error_type } => match error_type.as_str() {
                "missing_dependency" => (
                    Some("Cargo.toml".to_string()),
                    extract_crate_name_from_error(&self.error.message),
                ),
                "missing_import" => (
                    extract_file_from_error(&self.error.message),
                    extract_item_name_from_error(&self.error.message),
                ),
                _ => (extract_file_from_error(&self.error.message), None),
            },
            _ => (None, None),
        };

        FixInfo {
            fix_type: ft,
            target_file,
            target_item,
            suggested_change: self.error.suggested_fix.clone().unwrap_or_default(),
        }
    }

    /// Cancel the fix operation.
    pub fn cancel(&mut self) -> FixResult {
        self.set_status(FixStatus::Cancelled);
        self.build_result(self.created_at.elapsed())
    }

    /// Generate a regression test for the fix.
    fn generate_regression_test(&self) -> Option<RegressionTest> {
        let fix_info = self.fix_info.as_ref()?;
        let fix_result = self.last_fix_result.as_ref()?;

        crate::tools::generate_regression_test(
            fix_info,
            fix_result,
            &self.config.regression_test_config,
        )
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

/// Extract a crate name from an error message.
fn extract_crate_name_from_error(message: &str) -> Option<String> {
    // Look for backtick-quoted names: `foo`
    if let Some(start) = message.find('`') {
        if let Some(end) = message[start + 1..].find('`') {
            let name = &message[start + 1..start + 1 + end];
            if !name.is_empty() && !name.contains(' ') {
                return Some(name.to_string());
            }
        }
    }

    // Look for single-quoted names: 'foo'
    if let Some(start) = message.find('\'') {
        if let Some(end) = message[start + 1..].find('\'') {
            let name = &message[start + 1..start + 1 + end];
            if !name.is_empty() && !name.contains(' ') {
                return Some(name.to_string());
            }
        }
    }

    None
}

/// Extract an item name from an error message.
fn extract_item_name_from_error(message: &str) -> Option<String> {
    extract_crate_name_from_error(message) // Same logic for now
}

/// Extract a file path from an error message.
fn extract_file_from_error(message: &str) -> Option<String> {
    // Look for patterns like "src/main.rs:10:5" or "in src/lib.rs"
    for word in message.split_whitespace() {
        let cleaned =
            word.trim_matches(|c| c == ':' || c == ',' || c == '.' || c == ')' || c == '(');
        if cleaned.ends_with(".rs") || cleaned.ends_with(".go") || cleaned.ends_with(".ts") {
            return Some(cleaned.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolError;
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
        assert!(test.source.contains("#[test]"));
        assert!(test.source.contains("serde"));
        assert_eq!(test.fix_type, FixType::AddDependency);
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

    // ==================== Deviation Rules Tests ====================

    mod deviation_rules {
        use super::*;

        // --- DeviationCategory::deviation_rule() tests ---

        #[test]
        fn test_agent_code_category_allows_auto_fix() {
            assert_eq!(
                DeviationCategory::AgentCode.deviation_rule(),
                DeviationRule::AutoFix
            );
            assert!(DeviationCategory::AgentCode.allows_auto_fix());
        }

        #[test]
        fn test_dependency_category_allows_auto_fix() {
            assert_eq!(
                DeviationCategory::Dependency.deviation_rule(),
                DeviationRule::AutoFix
            );
            assert!(DeviationCategory::Dependency.allows_auto_fix());
        }

        #[test]
        fn test_test_lint_category_allows_auto_fix() {
            assert_eq!(
                DeviationCategory::TestLint.deviation_rule(),
                DeviationRule::AutoFix
            );
            assert!(DeviationCategory::TestLint.allows_auto_fix());
        }

        #[test]
        fn test_architecture_category_requires_ask() {
            assert_eq!(
                DeviationCategory::Architecture.deviation_rule(),
                DeviationRule::MustAsk
            );
            assert!(!DeviationCategory::Architecture.allows_auto_fix());
        }

        #[test]
        fn test_new_dependency_category_requires_ask() {
            assert_eq!(
                DeviationCategory::NewDependency.deviation_rule(),
                DeviationRule::MustAsk
            );
            assert!(!DeviationCategory::NewDependency.allows_auto_fix());
        }

        #[test]
        fn test_file_deletion_category_requires_ask() {
            assert_eq!(
                DeviationCategory::FileDeletion.deviation_rule(),
                DeviationRule::MustAsk
            );
            assert!(!DeviationCategory::FileDeletion.allows_auto_fix());
        }

        // --- categorize_deviation() tests ---

        #[test]
        fn test_categorize_missing_crate_as_dependency() {
            assert_eq!(
                categorize_deviation("cannot find crate `serde`"),
                DeviationCategory::Dependency
            );
            assert_eq!(
                categorize_deviation("error: unresolved import crate `tokio`"),
                DeviationCategory::Dependency
            );
            assert_eq!(
                categorize_deviation("package `foo` not found in registry"),
                DeviationCategory::Dependency
            );
        }

        #[test]
        fn test_categorize_missing_module_as_dependency() {
            assert_eq!(
                categorize_deviation("cannot find module `utils` in this crate"),
                DeviationCategory::Dependency
            );
        }

        #[test]
        fn test_categorize_test_failures_as_test_lint() {
            assert_eq!(
                categorize_deviation("test failed: assertion `left == right` failed"),
                DeviationCategory::TestLint
            );
            assert_eq!(
                categorize_deviation("test failure in tests::my_test"),
                DeviationCategory::TestLint
            );
            assert_eq!(
                categorize_deviation("assertion failed: expected 5, got 3"),
                DeviationCategory::TestLint
            );
        }

        #[test]
        fn test_categorize_clippy_as_test_lint() {
            assert_eq!(
                categorize_deviation("clippy warning: unused variable `x`"),
                DeviationCategory::TestLint
            );
            assert_eq!(
                categorize_deviation("error from clippy: needless_return"),
                DeviationCategory::TestLint
            );
        }

        #[test]
        fn test_categorize_lint_warnings_as_test_lint() {
            assert_eq!(
                categorize_deviation("lint error: missing documentation"),
                DeviationCategory::TestLint
            );
            assert_eq!(
                categorize_deviation("warning: unused variable `foo`"),
                DeviationCategory::TestLint
            );
            assert_eq!(
                categorize_deviation("warning: dead_code on function `bar`"),
                DeviationCategory::TestLint
            );
        }

        #[test]
        fn test_categorize_file_deletion_as_file_deletion() {
            assert_eq!(
                categorize_deviation("need to delete file old_config.rs"),
                DeviationCategory::FileDeletion
            );
            assert_eq!(
                categorize_deviation("remove file deprecated_module.rs"),
                DeviationCategory::FileDeletion
            );
            assert_eq!(
                categorize_deviation("running rm unused_file.txt"),
                DeviationCategory::FileDeletion
            );
        }

        #[test]
        fn test_categorize_architecture_changes() {
            assert_eq!(
                categorize_deviation("requires new module for authentication"),
                DeviationCategory::Architecture
            );
            assert_eq!(
                categorize_deviation("need to create module for utilities"),
                DeviationCategory::Architecture
            );
            assert_eq!(
                categorize_deviation("schema change required for user table"),
                DeviationCategory::Architecture
            );
            assert_eq!(
                categorize_deviation("need to refactor the auth system"),
                DeviationCategory::Architecture
            );
            assert_eq!(
                categorize_deviation("should restructure the codebase"),
                DeviationCategory::Architecture
            );
        }

        #[test]
        fn test_categorize_new_dependency_additions() {
            assert_eq!(
                categorize_deviation("need to add dependency for JSON parsing"),
                DeviationCategory::NewDependency
            );
            assert_eq!(
                categorize_deviation("should add crate serde for serialization"),
                DeviationCategory::NewDependency
            );
            assert_eq!(
                categorize_deviation("running cargo add tokio"),
                DeviationCategory::NewDependency
            );
            assert_eq!(
                categorize_deviation("npm install express"),
                DeviationCategory::NewDependency
            );
        }

        #[test]
        fn test_categorize_general_code_errors_as_agent_code() {
            // Type errors
            assert_eq!(
                categorize_deviation("mismatched types: expected `&str`, found `String`"),
                DeviationCategory::AgentCode
            );
            // Syntax errors
            assert_eq!(
                categorize_deviation("expected `;` after expression"),
                DeviationCategory::AgentCode
            );
            // Undefined variable
            assert_eq!(
                categorize_deviation("use of undeclared identifier `foo`"),
                DeviationCategory::AgentCode
            );
            // Generic compile error
            assert_eq!(
                categorize_deviation("error[E0599]: no method named `bar`"),
                DeviationCategory::AgentCode
            );
        }

        // --- should_auto_fix() tests ---

        #[test]
        fn test_should_auto_fix_for_agent_code_errors() {
            assert!(should_auto_fix("cannot find value `x` in this scope"));
            assert!(should_auto_fix("mismatched types"));
            assert!(should_auto_fix("expected `;`"));
        }

        #[test]
        fn test_should_auto_fix_for_dependency_errors() {
            assert!(should_auto_fix("cannot find crate `serde`"));
            assert!(should_auto_fix("unresolved module `utils`"));
        }

        #[test]
        fn test_should_auto_fix_for_test_lint_errors() {
            assert!(should_auto_fix("test failed: assertion error"));
            assert!(should_auto_fix("clippy warning: unused import"));
        }

        #[test]
        fn test_should_not_auto_fix_for_architecture_changes() {
            assert!(!should_auto_fix("need to create module for auth"));
            assert!(!should_auto_fix("requires schema change"));
            assert!(!should_auto_fix("should refactor the code"));
        }

        #[test]
        fn test_should_not_auto_fix_for_new_dependencies() {
            assert!(!should_auto_fix("need to add dependency for parsing"));
            assert!(!should_auto_fix("cargo add tokio"));
            assert!(!should_auto_fix("npm install lodash"));
        }

        #[test]
        fn test_should_not_auto_fix_for_file_deletion() {
            assert!(!should_auto_fix("delete file old_code.rs"));
            assert!(!should_auto_fix("remove file unused.txt"));
        }

        // --- FixAgent deviation methods tests ---

        #[test]
        fn test_fix_agent_deviation_category_for_code_error() {
            let result = make_code_error_result("cannot find value `foo` in this scope");
            let agent = FixAgent::spawn_with_defaults(result).unwrap();

            // Note: The error message is categorized by the executor as a code error,
            // but the deviation categorization looks at the message content
            let category = agent.deviation_category();
            assert!(category.allows_auto_fix());
        }

        #[test]
        fn test_fix_agent_deviation_category_for_missing_crate() {
            let result = make_code_error_result("cannot find crate `serde`");
            let agent = FixAgent::spawn_with_defaults(result).unwrap();

            assert_eq!(agent.deviation_category(), DeviationCategory::Dependency);
            assert_eq!(agent.deviation_rule(), DeviationRule::AutoFix);
            assert!(agent.should_attempt_fix());
        }

        #[test]
        fn test_fix_agent_deviation_category_for_clippy_lint() {
            // Use a message that is both a "code error" for ToolError AND "TestLint" for deviation
            // The message must match ToolError's "cannot find ... in this scope" pattern
            // AND contain "clippy" for the deviation categorizer
            let result =
                make_code_error_result("clippy: cannot find value `unused_var` in this scope");
            let agent = FixAgent::spawn_with_defaults(result).unwrap();

            assert_eq!(agent.deviation_category(), DeviationCategory::TestLint);
            assert_eq!(agent.deviation_rule(), DeviationRule::AutoFix);
            assert!(agent.should_attempt_fix());
        }

        #[test]
        fn test_fix_agent_should_attempt_fix_respects_deviation_rules() {
            // Auto-fixable: code error with auto-fix deviation rule
            let result = make_code_error_result("cannot find value `x` in this scope");
            let agent = FixAgent::spawn_with_defaults(result).unwrap();
            assert!(agent.should_attempt_fix());

            // Another auto-fixable case: missing dependency (both Code error and Dependency deviation)
            let result2 = make_code_error_result("cannot find crate `missing_crate`");
            let agent2 = FixAgent::spawn_with_defaults(result2).unwrap();
            assert!(agent2.should_attempt_fix());
        }

        // --- DeviationCategory and DeviationRule derive trait tests ---

        #[test]
        fn test_deviation_category_debug() {
            assert_eq!(format!("{:?}", DeviationCategory::AgentCode), "AgentCode");
            assert_eq!(format!("{:?}", DeviationCategory::Dependency), "Dependency");
            assert_eq!(format!("{:?}", DeviationCategory::TestLint), "TestLint");
            assert_eq!(
                format!("{:?}", DeviationCategory::Architecture),
                "Architecture"
            );
            assert_eq!(
                format!("{:?}", DeviationCategory::NewDependency),
                "NewDependency"
            );
            assert_eq!(
                format!("{:?}", DeviationCategory::FileDeletion),
                "FileDeletion"
            );
        }

        #[test]
        fn test_deviation_rule_debug() {
            assert_eq!(format!("{:?}", DeviationRule::AutoFix), "AutoFix");
            assert_eq!(format!("{:?}", DeviationRule::MustAsk), "MustAsk");
        }

        #[test]
        fn test_deviation_category_clone() {
            let category = DeviationCategory::AgentCode;
            let cloned = category.clone();
            assert_eq!(category, cloned);
        }

        #[test]
        fn test_deviation_rule_clone() {
            let rule = DeviationRule::AutoFix;
            let cloned = rule.clone();
            assert_eq!(rule, cloned);
        }

        #[test]
        fn test_deviation_category_eq() {
            assert_eq!(DeviationCategory::AgentCode, DeviationCategory::AgentCode);
            assert_ne!(DeviationCategory::AgentCode, DeviationCategory::Dependency);
        }

        #[test]
        fn test_deviation_rule_eq() {
            assert_eq!(DeviationRule::AutoFix, DeviationRule::AutoFix);
            assert_ne!(DeviationRule::AutoFix, DeviationRule::MustAsk);
        }
    }
}

//! Tool executor framework with error categorization.
//!
//! Provides a structured way to execute tools and categorize their errors
//! for potential recovery or retry.

use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Categories of errors that can occur during tool execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Code-related errors (compilation, syntax, missing dependencies)
    /// These may be fixable by a fix-agent.
    Code {
        /// The type of code error (e.g., "missing_dependency", "type_error", "syntax_error")
        error_type: String,
    },

    /// Permission errors (file access denied, operation not permitted)
    /// May require user intervention or permission grants.
    Permission {
        /// The path or resource that was denied
        resource: String,
    },

    /// Network errors (connection refused, timeout, DNS failure)
    /// Usually transient and can be retried.
    Network {
        /// Whether the error is likely transient and retriable
        is_transient: bool,
    },

    /// Resource errors (disk full, out of memory, file not found)
    /// May require alternative approaches or cleanup.
    Resource {
        /// The type of resource issue (e.g., "disk_full", "not_found", "out_of_memory")
        resource_type: String,
    },

    /// Unknown or uncategorized errors
    Unknown,
}

/// A structured error from tool execution.
#[derive(Debug, Clone)]
pub struct ToolError {
    /// The error message
    pub message: String,

    /// The categorized error type
    pub category: ErrorCategory,

    /// The raw error output (for diagnostics)
    pub raw_output: Option<String>,

    /// Whether this error is retriable
    pub retriable: bool,

    /// Suggested fix, if any
    pub suggested_fix: Option<String>,
}

impl ToolError {
    /// Create a new tool error with automatic categorization.
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let (category, retriable, suggested_fix) = categorize_error(&message);

        Self {
            message,
            category,
            raw_output: None,
            retriable,
            suggested_fix,
        }
    }

    /// Create a tool error with explicit category.
    pub fn with_category(message: impl Into<String>, category: ErrorCategory) -> Self {
        let message = message.into();
        let retriable = matches!(category, ErrorCategory::Network { is_transient: true });

        Self {
            message,
            category,
            raw_output: None,
            retriable,
            suggested_fix: None,
        }
    }

    /// Add raw output to the error.
    pub fn with_raw_output(mut self, output: impl Into<String>) -> Self {
        self.raw_output = Some(output.into());
        self
    }

    /// Add a suggested fix to the error.
    pub fn with_suggested_fix(mut self, fix: impl Into<String>) -> Self {
        self.suggested_fix = Some(fix.into());
        self
    }

    /// Check if this error can potentially be auto-fixed.
    pub fn is_auto_fixable(&self) -> bool {
        matches!(self.category, ErrorCategory::Code { .. })
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// The tool name that was executed
    pub tool_name: String,

    /// The call ID for this execution
    pub call_id: String,

    /// The result (success output or error)
    pub result: Result<String, ToolError>,

    /// Duration of the execution
    pub duration: Duration,

    /// Number of retries attempted
    pub retries: u32,
}

impl ToolExecutionResult {
    /// Check if execution succeeded.
    pub fn is_success(&self) -> bool {
        self.result.is_ok()
    }

    /// Check if the error is auto-fixable.
    pub fn is_auto_fixable(&self) -> bool {
        self.result
            .as_ref()
            .err()
            .map(|e| e.is_auto_fixable())
            .unwrap_or(false)
    }

    /// Get the error if this result failed.
    pub fn error(&self) -> Option<&ToolError> {
        self.result.as_ref().err()
    }
}

/// Configuration for the tool executor.
#[derive(Debug, Clone)]
pub struct ToolExecutorConfig {
    /// Maximum number of retries for transient errors
    pub max_retries: u32,

    /// Base delay for exponential backoff (in milliseconds)
    pub base_retry_delay_ms: u64,

    /// Maximum delay between retries (in milliseconds)
    pub max_retry_delay_ms: u64,

    /// Whether to attempt auto-fixes for code errors
    pub auto_fix_enabled: bool,

    /// Maximum time to wait for a single tool execution (in milliseconds)
    pub execution_timeout_ms: u64,
}

impl Default for ToolExecutorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_retry_delay_ms: 1000,
            max_retry_delay_ms: 10000,
            auto_fix_enabled: true,
            execution_timeout_ms: 300000, // 5 minutes
        }
    }
}

/// A function that executes a tool and returns its result.
pub type ToolFunction = fn(Value) -> Result<String, String>;

/// The tool executor manages tool execution with error handling and recovery.
pub struct ToolExecutor {
    /// Configuration for the executor
    config: ToolExecutorConfig,

    /// Registered tool functions
    tools: HashMap<String, ToolFunction>,
}

impl ToolExecutor {
    /// Create a new tool executor with the given configuration.
    pub fn new(config: ToolExecutorConfig) -> Self {
        Self {
            config,
            tools: HashMap::new(),
        }
    }

    /// Create a new tool executor with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ToolExecutorConfig::default())
    }

    /// Register a tool function.
    pub fn register_tool(&mut self, name: impl Into<String>, func: ToolFunction) {
        self.tools.insert(name.into(), func);
    }

    /// Check if a tool is registered.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the list of registered tool names.
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Execute a tool with the given input.
    ///
    /// This will:
    /// 1. Look up the tool by name
    /// 2. Execute it with the provided input
    /// 3. Categorize any errors
    /// 4. Retry transient errors with exponential backoff
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::tools::ToolExecutor;
    /// use serde_json::{json, Value};
    ///
    /// let mut executor = ToolExecutor::with_defaults();
    ///
    /// // Register a tool
    /// fn read_file(input: Value) -> Result<String, String> {
    ///     let path = input["path"].as_str().ok_or("Missing path")?;
    ///     std::fs::read_to_string(path).map_err(|e| e.to_string())
    /// }
    /// executor.register_tool("read_file", read_file);
    ///
    /// // Execute with automatic retry on transient errors
    /// let result = executor.execute(
    ///     "call_123",
    ///     "read_file",
    ///     json!({"path": "/tmp/example.txt"})
    /// );
    ///
    /// if result.is_success() {
    ///     println!("File contents: {}", result.result.unwrap());
    /// } else if result.is_auto_fixable() {
    ///     println!("Error can be auto-fixed by fix-agent");
    /// }
    /// ```
    pub fn execute(
        &self,
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        input: Value,
    ) -> ToolExecutionResult {
        let call_id = call_id.into();
        let tool_name = tool_name.into();
        let start = Instant::now();

        // Look up the tool
        let tool_func = match self.tools.get(&tool_name) {
            Some(func) => func,
            None => {
                return ToolExecutionResult {
                    tool_name: tool_name.clone(),
                    call_id,
                    result: Err(ToolError::with_category(
                        format!("Unknown tool: {}", tool_name),
                        ErrorCategory::Resource {
                            resource_type: "tool_not_found".to_string(),
                        },
                    )),
                    duration: start.elapsed(),
                    retries: 0,
                };
            }
        };

        // Execute with retry logic
        let mut retries = 0;
        loop {
            let result = tool_func(input.clone());

            match result {
                Ok(output) => {
                    return ToolExecutionResult {
                        tool_name: tool_name.clone(),
                        call_id,
                        result: Ok(output),
                        duration: start.elapsed(),
                        retries,
                    };
                }
                Err(error_msg) => {
                    let error = ToolError::new(&error_msg).with_raw_output(&error_msg);

                    // Check if we should retry
                    if error.retriable && retries < self.config.max_retries {
                        retries += 1;
                        let delay = self.calculate_retry_delay(retries);
                        std::thread::sleep(delay);
                        continue;
                    }

                    return ToolExecutionResult {
                        tool_name: tool_name.clone(),
                        call_id,
                        result: Err(error),
                        duration: start.elapsed(),
                        retries,
                    };
                }
            }
        }
    }

    /// Calculate retry delay with exponential backoff.
    fn calculate_retry_delay(&self, retry_count: u32) -> Duration {
        let delay_ms = self.config.base_retry_delay_ms * 2u64.pow(retry_count - 1);
        let capped_delay = delay_ms.min(self.config.max_retry_delay_ms);
        Duration::from_millis(capped_delay)
    }

    /// Get the executor configuration.
    pub fn config(&self) -> &ToolExecutorConfig {
        &self.config
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Categorize an error message into an ErrorCategory.
///
/// # Examples
///
/// ```rust,no_run
/// # use coding_agent_cli::tools::{ToolError, ErrorCategory};
/// // Automatically categorizes different error types
/// let error = ToolError::new("cannot find crate `serde_json`");
/// // => ErrorCategory::Code { error_type: "missing_dependency" }
///
/// let error = ToolError::new("Permission denied: '/etc/passwd'");
/// // => ErrorCategory::Permission { resource: "/etc/passwd" }
///
/// let error = ToolError::new("Connection timed out");
/// // => ErrorCategory::Network { is_transient: true }
/// ```
fn categorize_error(message: &str) -> (ErrorCategory, bool, Option<String>) {
    let lower = message.to_lowercase();

    // Code errors - often fixable
    if lower.contains("cannot find crate")
        || lower.contains("can't find crate")
        || lower.contains("unresolved import")
        || lower.contains("no such crate")
        || lower.contains("could not find")
            && (lower.contains("crate") || lower.contains("module") || lower.contains("package"))
    {
        return (
            ErrorCategory::Code {
                error_type: "missing_dependency".to_string(),
            },
            false,
            Some("Add the missing dependency to Cargo.toml or package.json".to_string()),
        );
    }

    if lower.contains("type mismatch")
        || lower.contains("mismatched types")
        || lower.contains("expected")
            && (lower.contains("found") || lower.contains("type"))
            && !lower.contains("file")
    {
        return (
            ErrorCategory::Code {
                error_type: "type_error".to_string(),
            },
            false,
            Some("Fix the type annotation or conversion".to_string()),
        );
    }

    if lower.contains("syntax error")
        || lower.contains("unexpected token")
        || lower.contains("expected")
            && (lower.contains("`;`")
                || lower.contains("`}`")
                || lower.contains("`)`")
                || lower.contains("expression"))
    {
        return (
            ErrorCategory::Code {
                error_type: "syntax_error".to_string(),
            },
            false,
            Some("Fix the syntax error".to_string()),
        );
    }

    if lower.contains("cannot find") && lower.contains("in this scope")
        || lower.contains("not found in scope")
        || lower.contains("use of undeclared")
    {
        return (
            ErrorCategory::Code {
                error_type: "missing_import".to_string(),
            },
            false,
            Some("Add the missing import statement".to_string()),
        );
    }

    // Permission errors
    if lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("operation not permitted")
        || lower.contains("eacces")
    {
        // Try to extract the resource path
        let resource = extract_path_from_error(message).unwrap_or_else(|| "unknown".to_string());
        return (
            ErrorCategory::Permission { resource },
            false,
            Some("Check file permissions or request access".to_string()),
        );
    }

    // Network errors - often transient
    if lower.contains("connection refused")
        || lower.contains("connection reset")
        || lower.contains("network unreachable")
        || lower.contains("host unreachable")
    {
        return (
            ErrorCategory::Network { is_transient: true },
            true,
            Some("Check network connectivity and retry".to_string()),
        );
    }

    if lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("deadline exceeded")
    {
        return (
            ErrorCategory::Network { is_transient: true },
            true,
            Some("Operation timed out, will retry".to_string()),
        );
    }

    if lower.contains("dns") || lower.contains("name resolution") || lower.contains("getaddrinfo") {
        return (
            ErrorCategory::Network {
                is_transient: false,
            },
            false,
            Some("DNS resolution failed, check the hostname".to_string()),
        );
    }

    // Resource errors
    if lower.contains("no space left")
        || lower.contains("disk full")
        || lower.contains("enospc")
        || lower.contains("out of disk")
    {
        return (
            ErrorCategory::Resource {
                resource_type: "disk_full".to_string(),
            },
            false,
            Some("Free up disk space".to_string()),
        );
    }

    if lower.contains("out of memory")
        || lower.contains("cannot allocate")
        || lower.contains("enomem")
    {
        return (
            ErrorCategory::Resource {
                resource_type: "out_of_memory".to_string(),
            },
            false,
            Some("Reduce memory usage or increase available memory".to_string()),
        );
    }

    if lower.contains("no such file")
        || lower.contains("file not found")
        || lower.contains("enoent")
        || lower.contains("does not exist")
    {
        let resource = extract_path_from_error(message).unwrap_or_else(|| "file".to_string());
        return (
            ErrorCategory::Resource {
                resource_type: "not_found".to_string(),
            },
            false,
            Some(format!("File or directory '{}' does not exist", resource)),
        );
    }

    // Unknown error
    (ErrorCategory::Unknown, false, None)
}

/// Try to extract a file path from an error message.
fn extract_path_from_error(message: &str) -> Option<String> {
    // Look for quoted paths
    if let Some(start) = message.find('\'') {
        if let Some(end) = message[start + 1..].find('\'') {
            let path = &message[start + 1..start + 1 + end];
            if path.contains('/') || path.contains('\\') {
                return Some(path.to_string());
            }
        }
    }

    // Look for double-quoted paths
    if let Some(start) = message.find('"') {
        if let Some(end) = message[start + 1..].find('"') {
            let path = &message[start + 1..start + 1 + end];
            if path.contains('/') || path.contains('\\') {
                return Some(path.to_string());
            }
        }
    }

    // Look for paths starting with / or containing common path segments
    for word in message.split_whitespace() {
        if word.starts_with('/')
            || word.contains("/src/")
            || word.contains("/home/")
            || word.contains("/Users/")
        {
            // Clean up trailing punctuation
            let cleaned = word.trim_end_matches(|c| c == ':' || c == ',' || c == '.' || c == ')');
            return Some(cleaned.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization_missing_dependency() {
        let error = ToolError::new("error[E0463]: can't find crate for `serde_json`");
        assert!(matches!(
            error.category,
            ErrorCategory::Code {
                error_type: ref t
            } if t == "missing_dependency"
        ));
        assert!(!error.retriable);
        assert!(error.is_auto_fixable());
        assert!(error.suggested_fix.is_some());
    }

    #[test]
    fn test_error_categorization_permission() {
        let error = ToolError::new("Permission denied: '/etc/passwd'");
        assert!(matches!(
            error.category,
            ErrorCategory::Permission { ref resource } if resource == "/etc/passwd"
        ));
        assert!(!error.retriable);
        assert!(!error.is_auto_fixable());
    }

    #[test]
    fn test_error_categorization_network_timeout() {
        let error = ToolError::new("Connection timed out");
        assert!(matches!(
            error.category,
            ErrorCategory::Network { is_transient: true }
        ));
        assert!(error.retriable);
        assert!(!error.is_auto_fixable());
    }

    #[test]
    fn test_error_categorization_network_connection_refused() {
        let error = ToolError::new("Connection refused: could not connect to localhost:5432");
        assert!(matches!(
            error.category,
            ErrorCategory::Network { is_transient: true }
        ));
        assert!(error.retriable);
    }

    #[test]
    fn test_error_categorization_resource_not_found() {
        let error = ToolError::new("No such file or directory: '/tmp/missing.txt'");
        assert!(matches!(
            error.category,
            ErrorCategory::Resource {
                resource_type: ref t
            } if t == "not_found"
        ));
        assert!(!error.retriable);
    }

    #[test]
    fn test_error_categorization_disk_full() {
        let error = ToolError::new("No space left on device (ENOSPC)");
        assert!(matches!(
            error.category,
            ErrorCategory::Resource {
                resource_type: ref t
            } if t == "disk_full"
        ));
        assert!(!error.retriable);
    }

    #[test]
    fn test_error_categorization_type_error() {
        let error =
            ToolError::new("error[E0308]: mismatched types: expected `&str`, found `String`");
        assert!(matches!(
            error.category,
            ErrorCategory::Code {
                error_type: ref t
            } if t == "type_error"
        ));
        assert!(error.is_auto_fixable());
    }

    #[test]
    fn test_error_categorization_syntax_error() {
        let error = ToolError::new("syntax error: unexpected token, expected `;`");
        assert!(matches!(
            error.category,
            ErrorCategory::Code {
                error_type: ref t
            } if t == "syntax_error"
        ));
        assert!(error.is_auto_fixable());
    }

    #[test]
    fn test_error_categorization_missing_import() {
        let error = ToolError::new("error[E0425]: cannot find value `HashMap` in this scope");
        assert!(matches!(
            error.category,
            ErrorCategory::Code {
                error_type: ref t
            } if t == "missing_import"
        ));
        assert!(error.is_auto_fixable());
    }

    #[test]
    fn test_error_categorization_unknown() {
        let error = ToolError::new("something went wrong");
        assert!(matches!(error.category, ErrorCategory::Unknown));
        assert!(!error.retriable);
        assert!(!error.is_auto_fixable());
    }

    #[test]
    fn test_tool_error_with_raw_output() {
        let error = ToolError::new("error").with_raw_output("full compiler output here");
        assert_eq!(
            error.raw_output,
            Some("full compiler output here".to_string())
        );
    }

    #[test]
    fn test_tool_error_with_suggested_fix() {
        let error = ToolError::new("error").with_suggested_fix("try this");
        assert_eq!(error.suggested_fix, Some("try this".to_string()));
    }

    #[test]
    fn test_tool_executor_register_and_has_tool() {
        let mut executor = ToolExecutor::with_defaults();

        fn dummy_tool(_: Value) -> Result<String, String> {
            Ok("ok".to_string())
        }

        assert!(!executor.has_tool("test_tool"));
        executor.register_tool("test_tool", dummy_tool);
        assert!(executor.has_tool("test_tool"));
    }

    #[test]
    fn test_tool_executor_tool_names() {
        let mut executor = ToolExecutor::with_defaults();

        fn tool_a(_: Value) -> Result<String, String> {
            Ok("a".to_string())
        }
        fn tool_b(_: Value) -> Result<String, String> {
            Ok("b".to_string())
        }

        executor.register_tool("tool_a", tool_a);
        executor.register_tool("tool_b", tool_b);

        let names = executor.tool_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"tool_a"));
        assert!(names.contains(&"tool_b"));
    }

    #[test]
    fn test_tool_executor_execute_success() {
        let mut executor = ToolExecutor::with_defaults();

        fn echo_tool(input: Value) -> Result<String, String> {
            Ok(format!("echo: {}", input))
        }

        executor.register_tool("echo", echo_tool);

        let result = executor.execute("call_1", "echo", serde_json::json!({"msg": "hello"}));

        assert!(result.is_success());
        assert_eq!(result.tool_name, "echo");
        assert_eq!(result.call_id, "call_1");
        assert_eq!(result.retries, 0);
        assert!(result.result.unwrap().contains("hello"));
    }

    #[test]
    fn test_tool_executor_execute_unknown_tool() {
        let executor = ToolExecutor::with_defaults();

        let result = executor.execute("call_1", "nonexistent", serde_json::json!({}));

        assert!(!result.is_success());
        let error = result.error().unwrap();
        assert!(error.message.contains("Unknown tool"));
        assert!(matches!(
            error.category,
            ErrorCategory::Resource { resource_type: ref t } if t == "tool_not_found"
        ));
    }

    #[test]
    fn test_tool_executor_execute_error() {
        let mut executor = ToolExecutor::with_defaults();

        fn failing_tool(_: Value) -> Result<String, String> {
            Err("Permission denied: '/etc/shadow'".to_string())
        }

        executor.register_tool("fail", failing_tool);

        let result = executor.execute("call_1", "fail", serde_json::json!({}));

        assert!(!result.is_success());
        assert!(!result.is_auto_fixable());
        let error = result.error().unwrap();
        assert!(matches!(error.category, ErrorCategory::Permission { .. }));
    }

    #[test]
    fn test_tool_executor_config_defaults() {
        let config = ToolExecutorConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_retry_delay_ms, 1000);
        assert_eq!(config.max_retry_delay_ms, 10000);
        assert!(config.auto_fix_enabled);
        assert_eq!(config.execution_timeout_ms, 300000);
    }

    #[test]
    fn test_tool_executor_with_custom_config() {
        let config = ToolExecutorConfig {
            max_retries: 5,
            base_retry_delay_ms: 500,
            max_retry_delay_ms: 5000,
            auto_fix_enabled: false,
            execution_timeout_ms: 60000,
        };

        let executor = ToolExecutor::new(config.clone());
        assert_eq!(executor.config().max_retries, 5);
        assert_eq!(executor.config().base_retry_delay_ms, 500);
        assert!(!executor.config().auto_fix_enabled);
    }

    #[test]
    fn test_tool_execution_result_accessors() {
        let success_result = ToolExecutionResult {
            tool_name: "test".to_string(),
            call_id: "1".to_string(),
            result: Ok("output".to_string()),
            duration: Duration::from_millis(100),
            retries: 0,
        };

        assert!(success_result.is_success());
        assert!(!success_result.is_auto_fixable());
        assert!(success_result.error().is_none());

        let error_result = ToolExecutionResult {
            tool_name: "test".to_string(),
            call_id: "2".to_string(),
            result: Err(ToolError::new("cannot find crate `foo`")),
            duration: Duration::from_millis(50),
            retries: 0,
        };

        assert!(!error_result.is_success());
        assert!(error_result.is_auto_fixable());
        assert!(error_result.error().is_some());
    }

    #[test]
    fn test_extract_path_single_quoted() {
        let path = extract_path_from_error("Cannot open '/tmp/test.txt': No such file");
        assert_eq!(path, Some("/tmp/test.txt".to_string()));
    }

    #[test]
    fn test_extract_path_double_quoted() {
        let path = extract_path_from_error("Permission denied: \"/etc/shadow\"");
        assert_eq!(path, Some("/etc/shadow".to_string()));
    }

    #[test]
    fn test_extract_path_bare() {
        let path = extract_path_from_error("Error reading /home/user/file.txt");
        assert_eq!(path, Some("/home/user/file.txt".to_string()));
    }

    #[test]
    fn test_extract_path_none() {
        let path = extract_path_from_error("Something went wrong");
        assert_eq!(path, None);
    }

    #[test]
    fn test_tool_error_display() {
        let error = ToolError::new("test error message");
        assert_eq!(format!("{}", error), "test error message");
    }

    #[test]
    fn test_tool_error_with_explicit_category() {
        let error = ToolError::with_category(
            "custom error",
            ErrorCategory::Code {
                error_type: "custom".to_string(),
            },
        );
        assert!(matches!(
            error.category,
            ErrorCategory::Code { error_type: ref t } if t == "custom"
        ));
    }

    #[test]
    fn test_retry_delay_calculation() {
        let config = ToolExecutorConfig {
            base_retry_delay_ms: 1000,
            max_retry_delay_ms: 10000,
            ..Default::default()
        };
        let executor = ToolExecutor::new(config);

        // retry 1: 1000 * 2^0 = 1000
        assert_eq!(
            executor.calculate_retry_delay(1),
            Duration::from_millis(1000)
        );
        // retry 2: 1000 * 2^1 = 2000
        assert_eq!(
            executor.calculate_retry_delay(2),
            Duration::from_millis(2000)
        );
        // retry 3: 1000 * 2^2 = 4000
        assert_eq!(
            executor.calculate_retry_delay(3),
            Duration::from_millis(4000)
        );
        // retry 4: 1000 * 2^3 = 8000
        assert_eq!(
            executor.calculate_retry_delay(4),
            Duration::from_millis(8000)
        );
        // retry 5: 1000 * 2^4 = 16000, capped at 10000
        assert_eq!(
            executor.calculate_retry_delay(5),
            Duration::from_millis(10000)
        );
    }

    #[test]
    fn test_retry_backoff_timing() {
        // Test that exponential backoff works correctly (mirrors spec requirement)
        let config = ToolExecutorConfig {
            base_retry_delay_ms: 100, // Use small delays for test speed
            max_retry_delay_ms: 1000,
            ..Default::default()
        };
        let executor = ToolExecutor::new(config);

        // Verify backoff sequence: 100, 200, 400, 800, capped at 1000
        assert_eq!(
            executor.calculate_retry_delay(1),
            Duration::from_millis(100)
        );
        assert_eq!(
            executor.calculate_retry_delay(2),
            Duration::from_millis(200)
        );
        assert_eq!(
            executor.calculate_retry_delay(3),
            Duration::from_millis(400)
        );
        assert_eq!(
            executor.calculate_retry_delay(4),
            Duration::from_millis(800)
        );
        assert_eq!(
            executor.calculate_retry_delay(5),
            Duration::from_millis(1000)
        ); // capped
        assert_eq!(
            executor.calculate_retry_delay(10),
            Duration::from_millis(1000)
        ); // still capped
    }

    #[test]
    fn test_max_retries_exceeded() {
        use std::sync::atomic::{AtomicU32, Ordering};

        // Track how many times the tool is called
        static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

        fn network_fail_tool(_: Value) -> Result<String, String> {
            CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            // Return a retriable network error
            Err("Connection refused: server not available".to_string())
        }

        // Reset counter before test
        CALL_COUNT.store(0, Ordering::SeqCst);

        let config = ToolExecutorConfig {
            max_retries: 3,
            base_retry_delay_ms: 1, // Very short delay for test speed
            max_retry_delay_ms: 10,
            ..Default::default()
        };
        let mut executor = ToolExecutor::new(config);
        executor.register_tool("network_fail", network_fail_tool);

        let result = executor.execute("call_1", "network_fail", serde_json::json!({}));

        // Should have failed after max_retries attempts
        assert!(!result.is_success());
        assert_eq!(result.retries, 3); // Should have retried 3 times

        // Total calls = 1 initial + 3 retries = 4
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 4);

        // Error should still be the network error
        let error = result.error().unwrap();
        assert!(error.message.contains("Connection refused"));
        assert!(matches!(
            error.category,
            ErrorCategory::Network { is_transient: true }
        ));
    }

    #[test]
    fn test_non_retriable_error_no_retry() {
        use std::sync::atomic::{AtomicU32, Ordering};

        static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

        fn permission_fail_tool(_: Value) -> Result<String, String> {
            CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            // Return a non-retriable permission error
            Err("Permission denied: '/etc/passwd'".to_string())
        }

        CALL_COUNT.store(0, Ordering::SeqCst);

        let config = ToolExecutorConfig {
            max_retries: 3,
            base_retry_delay_ms: 1,
            max_retry_delay_ms: 10,
            ..Default::default()
        };
        let mut executor = ToolExecutor::new(config);
        executor.register_tool("permission_fail", permission_fail_tool);

        let result = executor.execute("call_1", "permission_fail", serde_json::json!({}));

        // Should have failed immediately without retries
        assert!(!result.is_success());
        assert_eq!(result.retries, 0); // No retries for non-retriable errors

        // Only called once
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_retry_succeeds_on_second_attempt() {
        use std::sync::atomic::{AtomicU32, Ordering};

        static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

        fn flaky_network_tool(_: Value) -> Result<String, String> {
            let count = CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                // First call fails with retriable error
                Err("Connection timed out".to_string())
            } else {
                // Subsequent calls succeed
                Ok("Success!".to_string())
            }
        }

        CALL_COUNT.store(0, Ordering::SeqCst);

        let config = ToolExecutorConfig {
            max_retries: 3,
            base_retry_delay_ms: 1,
            max_retry_delay_ms: 10,
            ..Default::default()
        };
        let mut executor = ToolExecutor::new(config);
        executor.register_tool("flaky", flaky_network_tool);

        let result = executor.execute("call_1", "flaky", serde_json::json!({}));

        // Should have succeeded on retry
        assert!(result.is_success());
        assert_eq!(result.retries, 1); // Succeeded on first retry
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2); // Called twice total
        assert_eq!(result.result.unwrap(), "Success!");
    }
}

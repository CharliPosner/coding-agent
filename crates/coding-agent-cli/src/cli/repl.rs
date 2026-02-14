//! REPL (Read-Eval-Print Loop) for the coding-agent CLI
//!
//! This module implements the main loop: read → parse → execute → display → repeat

use super::commands::{
    parse_command, CollapsedResults, CommandContext, CommandRegistry, CommandResult,
};
use super::input::{InputHandler, InputResult};
use super::modes::Mode;
use super::terminal::Terminal;
use crate::agents::manager::AgentManager;
use crate::config::Config;
use crate::integrations::{Session, SessionManager};
use crate::permissions::{
    OperationType, PermissionChecker, PermissionDecision, PermissionPrompt, PermissionResponse,
    TrustedPaths,
};
use crate::tokens::{CostTracker, ModelPricing, TokenCounter};
use crate::tools::{
    create_tool_definitions, tool_definitions_to_api, ToolExecutor, ToolExecutorConfig,
};
use crate::ui::{
    ContextBar, FunFactClient, LongWaitDetector, MarkdownRenderer, StatusBar, Theme,
    ThinkingMessages, ToolExecutionSpinner, ToolResultFormatter,
};
use coding_agent_core::{
    ContentBlock, Message, MessageRequest, MessageResponse, Tool, ToolDefinition,
};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// REPL configuration
pub struct ReplConfig {
    /// Whether to show verbose output
    pub verbose: bool,
    /// Path for session history (relative to project root)
    pub history_path: Option<PathBuf>,
    /// Whether session persistence is enabled
    pub persistence_enabled: bool,
    /// Whether to show the context bar
    pub show_context_bar: bool,
    /// Context window size in tokens
    pub context_window: u64,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            history_path: None,
            persistence_enabled: true,
            show_context_bar: true,
            context_window: 200_000,
        }
    }
}

impl ReplConfig {
    /// Create a ReplConfig from the application config
    pub fn from_config(config: &Config, verbose: bool) -> Self {
        Self {
            verbose,
            history_path: Some(PathBuf::from(&config.persistence.path)),
            persistence_enabled: config.persistence.enabled,
            show_context_bar: config.behavior.show_context_bar,
            context_window: config.model.context_window as u64,
        }
    }
}

/// The main REPL loop
pub struct Repl {
    config: ReplConfig,
    registry: CommandRegistry,
    input_handler: InputHandler,
    session: Session,
    session_manager: Option<SessionManager>,
    /// Token counter for tracking context usage
    token_counter: TokenCounter,
    /// Context bar for displaying token usage
    context_bar: ContextBar,
    /// Cost tracker for detailed token and cost tracking
    cost_tracker: CostTracker,
    /// API key for Claude
    api_key: Option<String>,
    /// Conversation history for API calls
    conversation: Vec<Message>,
    /// Tool definitions for Claude to use
    tool_definitions: Vec<ToolDefinition>,
    /// Tool definitions in API format
    tools_api: Vec<Tool>,
    /// Tool result formatter for displaying results
    tool_result_formatter: ToolResultFormatter,
    /// Permission checker for file operations
    permission_checker: Option<PermissionChecker>,
    /// Application config (needed for updating trusted paths)
    app_config: Option<Config>,
    /// Current mode (normal or planning)
    mode: Mode,
    /// Thinking messages manager for rotating messages
    thinking_messages: ThinkingMessages,
    /// Fun fact client for entertaining content during long waits
    fun_fact_client: Option<FunFactClient>,
    /// Whether fun facts are enabled
    fun_facts_enabled: bool,
    /// Delay before showing fun facts (in seconds)
    fun_fact_delay: u32,
    /// Agent manager for spawning and tracking autonomous agents
    agent_manager: Arc<AgentManager>,
    /// Tool executor for executing tools with error handling and retry logic
    tool_executor: ToolExecutor,
    /// Theme for styling UI components
    theme: Theme,
    /// Status bar for displaying multi-agent progress
    status_bar: StatusBar,
    /// Track number of status bar lines rendered (for clearing)
    status_bar_lines: usize,
    /// Markdown renderer for agent responses
    markdown_renderer: MarkdownRenderer,
    /// Last collapsed results for /results command
    collapsed_results: Arc<Mutex<CollapsedResults>>,
}

impl Repl {
    /// Create a new REPL with the given configuration
    pub fn new(config: ReplConfig) -> Self {
        Self::new_with_app_config(config, None)
    }

    /// Create a new REPL with the given configuration and app config
    pub fn new_with_app_config(config: ReplConfig, app_config: Option<&Config>) -> Self {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        // Get API key from environment
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();

        // Initialize session manager if persistence is enabled
        let session_manager = if config.persistence_enabled {
            let base_dir = config
                .history_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(".specstory/history"));

            // Use current working directory as base
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Some(SessionManager::new(cwd.join(base_dir)))
        } else {
            None
        };

        // Initialize token counter and context bar
        let token_counter = TokenCounter::default();
        let context_bar = ContextBar::new(config.context_window);

        // Initialize cost tracker with default model pricing
        let cost_tracker = CostTracker::new(ModelPricing::default_pricing());

        // Initialize tool definitions
        let tool_definitions = create_tool_definitions();
        let tools_api = tool_definitions_to_api(&tool_definitions);

        // Initialize tool result formatter
        let tool_result_formatter = ToolResultFormatter::new();

        // Initialize permission checker if app config is provided
        let permission_checker = app_config.map(|cfg| {
            let trusted_paths = TrustedPaths::new(&cfg.permissions.trusted_paths)
                .unwrap_or_else(|_| TrustedPaths::new(&[]).unwrap());
            PermissionChecker::new(trusted_paths, cfg.permissions.auto_read)
        });

        // Initialize thinking messages and fun facts
        let thinking_messages = ThinkingMessages::new();
        let fun_facts_enabled = app_config.map(|cfg| cfg.behavior.fun_facts).unwrap_or(true);
        let fun_fact_delay = app_config
            .map(|cfg| cfg.behavior.fun_fact_delay)
            .unwrap_or(10);

        // Try to create fun fact client if enabled
        let fun_fact_client = if fun_facts_enabled {
            FunFactClient::new().ok()
        } else {
            None
        };

        // Initialize theme based on config
        let theme_style = app_config
            .and_then(|cfg| crate::ui::theme::ThemeStyle::from_str(&cfg.theme.style))
            .unwrap_or(crate::ui::theme::ThemeStyle::Minimal);
        let theme = Theme::new(theme_style);

        // Initialize agent manager
        let agent_manager = Arc::new(AgentManager::new());

        // Initialize tool executor with default configuration
        let tool_executor_config = app_config
            .and_then(|cfg| {
                if cfg.error_recovery.auto_fix {
                    Some(ToolExecutorConfig {
                        max_retries: cfg.error_recovery.max_retry_attempts,
                        auto_fix_enabled: cfg.error_recovery.auto_fix,
                        ..Default::default()
                    })
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let mut tool_executor = ToolExecutor::new(tool_executor_config);

        // Register all tool functions with permission checking wrapper
        // Note: We register the raw functions directly since permission checking
        // will be added as a separate layer in Phase 14.1
        for tool_def in &tool_definitions {
            tool_executor.register_tool(&tool_def.name, tool_def.function);
        }

        // Initialize status bar with the same theme
        let status_bar = StatusBar::with_theme(theme.clone());

        // Initialize markdown renderer
        let markdown_renderer = MarkdownRenderer::new();

        Self {
            config,
            registry: CommandRegistry::with_defaults(),
            input_handler: InputHandler::new(),
            session: Session::new(),
            session_manager,
            token_counter,
            context_bar,
            cost_tracker,
            api_key,
            conversation: Vec::new(),
            tool_definitions,
            tools_api,
            tool_result_formatter,
            permission_checker,
            app_config: app_config.cloned(),
            mode: Mode::default(),
            thinking_messages,
            fun_fact_client,
            fun_facts_enabled,
            fun_fact_delay,
            agent_manager,
            tool_executor,
            theme,
            status_bar,
            status_bar_lines: 0,
            markdown_renderer,
            collapsed_results: Arc::new(Mutex::new(CollapsedResults::default())),
        }
    }

    /// Get the current session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get a mutable reference to the current session
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Save the current session to disk
    pub fn save_session(&mut self) -> Result<(), String> {
        if let Some(ref manager) = self.session_manager {
            if !self.session.is_empty() {
                manager.save(&mut self.session).map_err(|e| e.to_string())?;
                if self.config.verbose {
                    if let Some(ref path) = self.session.file_path {
                        eprintln!("[verbose] Session saved to: {:?}", path);
                    }
                }
            }
        }
        Ok(())
    }

    /// Start a new session, optionally saving the current one first
    pub fn new_session(&mut self, save_current: bool) -> Result<(), String> {
        if save_current {
            self.save_session()?;
        }
        self.session = Session::new();
        Ok(())
    }

    /// Load a session by filename
    pub fn load_session(&mut self, filename: &str) -> Result<(), String> {
        if let Some(ref manager) = self.session_manager {
            self.session = manager.load(filename).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Session persistence is disabled".to_string())
        }
    }

    /// Get the session manager
    pub fn session_manager(&self) -> Option<&SessionManager> {
        self.session_manager.as_ref()
    }

    /// Get the current context bar
    pub fn context_bar(&self) -> &ContextBar {
        &self.context_bar
    }

    /// Get mutable access to the context bar
    pub fn context_bar_mut(&mut self) -> &mut ContextBar {
        &mut self.context_bar
    }

    /// Update context bar and cost tracker with tokens from a message
    fn update_context_tokens(&mut self, role: &str, content: &str) {
        let token_count = self.token_counter.count_message(role, content);
        self.context_bar.add_tokens(token_count.tokens as u64);

        // Also update the cost tracker with separate input/output tracking
        if role == "user" {
            self.cost_tracker
                .add_input_tokens(token_count.tokens as u64);
        } else {
            self.cost_tracker
                .add_output_tokens(token_count.tokens as u64);
        }
        self.cost_tracker.add_message();
    }

    /// Display the context bar if enabled
    fn display_context_bar(&self) {
        if self.config.show_context_bar {
            self.print_line(&self.context_bar.render());
        }
    }

    /// Get the current mode
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// Set the current mode
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Reset context tracking (for /clear)
    pub fn reset_context(&mut self) {
        self.context_bar.reset();
        self.cost_tracker.reset();
        self.conversation.clear();
        self.mode = Mode::default(); // Reset to normal mode
    }

    /// Call the Claude API with the current conversation
    fn call_claude(&self, messages: &[Message]) -> Result<MessageResponse, String> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            "ANTHROPIC_API_KEY not set. Please set it in your environment or .env file.".to_string()
        })?;

        let request = MessageRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            messages: messages.to_vec(),
            tools: self.tools_api.clone(),
            system: Some(self.mode.system_prompt()),
        };

        let response = ureq::post("https://api.anthropic.com/v1/messages")
            .set("Content-Type", "application/json")
            .set("x-api-key", api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(&request)
            .map_err(|e| format!("API request failed: {}", e))?;

        let msg_response: MessageResponse = response
            .into_json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(msg_response)
    }

    /// Process a conversation turn, handling tool use in a loop until done
    fn process_conversation(&mut self) -> Result<(), String> {
        const MAX_TOOL_ITERATIONS: usize = 10;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > MAX_TOOL_ITERATIONS {
                return Err(
                    "Maximum tool iterations reached. Stopping to prevent infinite loop."
                        .to_string(),
                );
            }

            // Show thinking indicator with potential fun fact during long waits
            self.print_newline();

            // Get a thinking message
            let thinking_msg = self.thinking_messages.current();
            self.print_line(thinking_msg);

            // Pre-fetch a fun fact if enabled (using sync method to avoid async in this context)
            let fun_fact = if self.fun_facts_enabled && self.fun_fact_client.is_some() {
                self.fun_fact_client
                    .as_ref()
                    .map(|client| client.get_fact_sync())
            } else {
                None
            };

            // Start long wait detector
            let mut detector =
                LongWaitDetector::with_threshold(Duration::from_secs(self.fun_fact_delay as u64));
            detector.start();

            // Call Claude API
            let response = match self.call_claude(&self.conversation) {
                Ok(r) => r,
                Err(e) => {
                    // Clear the "Thinking..." line
                    print!("\x1b[A\x1b[2K\r");
                    let _ = std::io::stdout().flush();
                    return Err(e);
                }
            };

            // Check if call took longer than threshold
            let show_fun_fact = detector.has_exceeded_threshold();

            // If call took long enough and we have a fun fact, display it
            if show_fun_fact && fun_fact.is_some() {
                let fact = fun_fact.unwrap();

                // Clear thinking line and display fun fact
                print!("\x1b[A\x1b[2K\r");
                let _ = std::io::stdout().flush();

                self.print_newline();
                self.print_line(&format!("\x1b[36mDid you know?\x1b[0m {}", fact.text));
                self.print_newline();

                // Brief pause to let user see the fact
                thread::sleep(Duration::from_millis(1500));

                // Clear the fun fact
                for _ in 0..3 {
                    print!("\x1b[A\x1b[2K\r");
                }
                let _ = std::io::stdout().flush();
            } else {
                // Just clear the thinking line
                print!("\x1b[A\x1b[2K\r");
                let _ = std::io::stdout().flush();
            }

            // Rotate to next thinking message for next iteration
            self.thinking_messages.next();

            // Process the response
            let mut response_text = String::new();
            let mut tool_uses: Vec<(String, String, serde_json::Value)> = Vec::new();

            for block in &response.content {
                match block {
                    ContentBlock::Text { text } => {
                        response_text.push_str(text);
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        tool_uses.push((id.clone(), name.clone(), input.clone()));
                    }
                    _ => {}
                }
            }

            // Display any text response with markdown rendering
            if !response_text.is_empty() {
                self.print_newline();
                self.markdown_renderer.print(&response_text);
                self.print_newline();
            }

            // Add assistant response to conversation history
            self.conversation
                .push(Message::assistant(response.content.clone()));

            // Update token counts
            if !response_text.is_empty() {
                self.session.add_agent_message(&response_text);
                self.update_context_tokens("assistant", &response_text);
            }

            // If there are no tool uses, we're done
            if tool_uses.is_empty() {
                break;
            }

            // Execute tools and collect results
            let mut tool_results: Vec<ContentBlock> = Vec::new();
            for (id, name, input) in tool_uses {
                // Create spinner with target if available
                let spinner = if let Some(target) = self.extract_target(&name, &input) {
                    ToolExecutionSpinner::with_target(&name, target, self.theme.clone())
                } else {
                    ToolExecutionSpinner::new(&name, self.theme.clone())
                };

                // Execute the tool using ToolExecutor
                // Note: Permission checking is still done by execute_tool_with_permissions
                // which is wrapped inside the registered tool functions
                let execution_result = self.tool_executor.execute(id.clone(), &name, input.clone());

                // Handle retry attempts
                if execution_result.retries > 0 {
                    // The spinner will show retry info in the final message
                    // No need to display intermediate retry messages
                }

                match &execution_result.result {
                    Ok(output) => {
                        // Finish spinner with success
                        let summary = self.summarize_tool_result(&name, output);
                        spinner.finish_success_with_message(&summary);

                        // Display formatted result with collapsible support
                        let formatted = self
                            .tool_result_formatter
                            .format_result_collapsible(&name, output);

                        // Store collapsed content if any
                        if let Some(ref collapsed_content) = formatted.collapsed_content {
                            let mut collapsed = self.collapsed_results.lock().unwrap();
                            collapsed.content = Some(collapsed_content.clone());
                            collapsed.tool_name = formatted.tool_name.clone();
                            collapsed.count = formatted.collapsed_count;
                        }

                        for line in formatted.display.lines() {
                            self.print_line(line);
                        }
                        self.print_newline();

                        tool_results.push(ContentBlock::ToolResult {
                            tool_use_id: id,
                            content: output.clone(),
                            is_error: None,
                        });
                    }
                    Err(tool_error) => {
                        // Finish spinner with failure
                        spinner.finish_failed(&tool_error.message);

                        // Check if this error is auto-fixable
                        if execution_result.is_auto_fixable() {
                            self.print_line("\x1b[33m  → Diagnosing issue...\x1b[0m");

                            // Attempt to spawn a fix-agent
                            if let Some(fix_result) = self.attempt_auto_fix(
                                execution_result.clone(),
                                &name,
                                input.clone(),
                            ) {
                                if fix_result.is_success() {
                                    // Fix succeeded! Show what happened
                                    self.print_line(&format!(
                                        "\x1b[32m  ✓ Auto-fixed: {}\x1b[0m",
                                        fix_result.original_error
                                    ));

                                    // Show modified files
                                    let files = fix_result.all_modified_files();
                                    if !files.is_empty() {
                                        for file in files {
                                            self.print_line(&format!("    + Modified: {}", file));
                                        }
                                    }

                                    // Show test generation
                                    if let Some(test) = &fix_result.generated_test {
                                        self.print_line(&format!(
                                            "    + Generated regression test: {}",
                                            test.suggested_path.display()
                                        ));
                                    }

                                    // Re-run the original tool
                                    self.print_line("  → Re-running original tool...");
                                    self.print_newline();

                                    let retry_spinner =
                                        if let Some(target) = self.extract_target(&name, &input) {
                                            ToolExecutionSpinner::with_target(
                                                &name,
                                                target,
                                                self.theme.clone(),
                                            )
                                        } else {
                                            ToolExecutionSpinner::new(&name, self.theme.clone())
                                        };

                                    let retry_result = self.tool_executor.execute(
                                        id.clone(),
                                        &name,
                                        input.clone(),
                                    );

                                    match retry_result.result {
                                        Ok(output) => {
                                            let summary =
                                                self.summarize_tool_result(&name, &output);
                                            retry_spinner.finish_success_with_message(&summary);

                                            let formatted = self
                                                .tool_result_formatter
                                                .format_result_collapsible(&name, &output);

                                            // Store collapsed content if any
                                            if let Some(ref collapsed_content) =
                                                formatted.collapsed_content
                                            {
                                                let mut collapsed =
                                                    self.collapsed_results.lock().unwrap();
                                                collapsed.content = Some(collapsed_content.clone());
                                                collapsed.tool_name = formatted.tool_name.clone();
                                                collapsed.count = formatted.collapsed_count;
                                            }

                                            for line in formatted.display.lines() {
                                                self.print_line(line);
                                            }
                                            self.print_newline();

                                            tool_results.push(ContentBlock::ToolResult {
                                                tool_use_id: id,
                                                content: output,
                                                is_error: None,
                                            });

                                            // Continue to next tool (skip error handling below)
                                            continue;
                                        }
                                        Err(retry_error) => {
                                            retry_spinner.finish_failed(&retry_error.message);
                                            self.print_line(&format!(
                                                "\x1b[31m  ✗ Retry failed: {}\x1b[0m",
                                                retry_error.message
                                            ));
                                            self.print_newline();

                                            // Fall through to normal error handling
                                            tool_results.push(ContentBlock::ToolResult {
                                                tool_use_id: id,
                                                content: retry_error.message,
                                                is_error: Some(true),
                                            });
                                            continue;
                                        }
                                    }
                                } else {
                                    // Fix failed after all attempts
                                    self.print_line(&format!(
                                        "\x1b[31m  ✗ Auto-fix failed after {} attempts\x1b[0m",
                                        fix_result.attempt_count()
                                    ));

                                    // Show the attempts that were made
                                    for attempt in &fix_result.attempts {
                                        if let Some(error_msg) = &attempt.error_message {
                                            self.print_line(&format!(
                                                "    Attempt {}: {}",
                                                attempt.attempt_number, error_msg
                                            ));
                                        }
                                    }
                                }
                            }
                        }

                        // Check if this is a permission error that we can handle
                        if let crate::tools::ErrorCategory::Permission { ref resource } =
                            tool_error.category
                        {
                            if let Some(handled) =
                                self.handle_permission_error(resource, &id, &name, input.clone())
                            {
                                // Permission was handled (either granted or denied)
                                tool_results.push(handled);
                                continue;
                            }
                        }

                        // Check if this is a resource error that we can suggest alternatives for
                        if let crate::tools::ErrorCategory::Resource { ref resource_type } =
                            tool_error.category
                        {
                            self.handle_resource_error(resource_type, &tool_error.message);
                        }

                        // Show suggested fix if available and no auto-fix was attempted
                        if let Some(suggested_fix) = &tool_error.suggested_fix {
                            self.print_line(&format!(
                                "\x1b[33m  💡 Suggestion: {}\x1b[0m",
                                suggested_fix
                            ));
                        }
                        self.print_newline();

                        tool_results.push(ContentBlock::ToolResult {
                            tool_use_id: id,
                            content: tool_error.message.clone(),
                            is_error: Some(true),
                        });
                    }
                }
            }

            // Add tool results as a user message
            self.conversation.push(Message {
                role: "user".to_string(),
                content: tool_results,
            });

            // Check if Claude wants to stop
            if response.stop_reason.as_deref() == Some("end_turn") {
                break;
            }
        }

        Ok(())
    }

    /// Extract the target (e.g., file path) from a tool call input
    fn extract_target(&self, name: &str, input: &serde_json::Value) -> Option<String> {
        match name {
            "read_file" | "write_file" | "edit_file" => input
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "list_files" => {
                let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                Some(path.to_string())
            }
            "bash" => {
                input.get("command").and_then(|v| v.as_str()).map(|cmd| {
                    // Truncate long commands for display
                    if cmd.len() > 50 {
                        format!("{}...", &cmd[..47])
                    } else {
                        cmd.to_string()
                    }
                })
            }
            "code_search" => input
                .get("pattern")
                .and_then(|v| v.as_str())
                .map(|s| format!("'{}'", s)),
            _ => None,
        }
    }

    /// Format a tool call for display
    fn format_tool_call(&self, name: &str, input: &serde_json::Value) -> String {
        match name {
            "read_file" => {
                let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                format!("Reading {}", path)
            }
            "write_file" => {
                let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                format!("Writing {}", path)
            }
            "edit_file" => {
                let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                format!("Editing {}", path)
            }
            "list_files" => {
                let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                format!("Listing files in {}", path)
            }
            "bash" => {
                let command = input.get("command").and_then(|v| v.as_str()).unwrap_or("?");
                // Truncate long commands
                if command.len() > 50 {
                    format!("Running: {}...", &command[..47])
                } else {
                    format!("Running: {}", command)
                }
            }
            "code_search" => {
                let pattern = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
                format!("Searching for '{}'", pattern)
            }
            _ => format!("Executing {}", name),
        }
    }

    /// Summarize a tool result for display
    fn summarize_tool_result(&self, name: &str, output: &str) -> String {
        match name {
            "read_file" => {
                let lines = output.lines().count();
                let bytes = output.len();
                format!("Read {} lines ({} bytes)", lines, bytes)
            }
            "write_file" => output.to_string(),
            "edit_file" => {
                if output == "OK" {
                    "Edit applied".to_string()
                } else {
                    output.to_string()
                }
            }
            "list_files" => {
                // Count items in the JSON array
                if let Ok(files) = serde_json::from_str::<Vec<String>>(output) {
                    format!("Found {} files/directories", files.len())
                } else {
                    "Listed files".to_string()
                }
            }
            "bash" => {
                let lines = output.lines().count();
                if lines == 0 {
                    "Command completed (no output)".to_string()
                } else if lines == 1 {
                    // Show short output directly
                    if output.len() <= 60 {
                        output.to_string()
                    } else {
                        format!("{}...", &output[..57])
                    }
                } else {
                    format!("Output: {} lines", lines)
                }
            }
            "code_search" => {
                if output == "No matches found" {
                    output.to_string()
                } else {
                    let lines = output.lines().count();
                    format!("Found {} matches", lines)
                }
            }
            _ => {
                let lines = output.lines().count();
                format!("Result: {} lines", lines)
            }
        }
    }

    /// Attempt to auto-fix an error using the FixAgent.
    ///
    /// Returns Some(FixResult) if a fix was attempted, None if the error is not fixable.
    fn attempt_auto_fix(
        &self,
        execution_result: crate::tools::ToolExecutionResult,
        _tool_name: &str,
        _tool_input: serde_json::Value,
    ) -> Option<crate::agents::FixResult> {
        use crate::agents::{FixAgent, FixAgentConfig};

        // Spawn a fix-agent
        let mut agent = FixAgent::spawn(execution_result, FixAgentConfig::default())?;

        // Try to diagnose and fix
        let fix_result = agent.attempt_fix(
            |fix_type, error_category| {
                // Apply the fix based on the type
                self.apply_fix_for_error(fix_type, error_category)
            },
            || {
                // Verify the fix by re-executing the tool
                // For now, we just return Ok - the actual verification happens
                // when we re-run the tool after this method returns
                Ok(())
            },
        );

        Some(fix_result)
    }

    /// Apply a fix for a specific error category.
    ///
    /// This is a simplified implementation that handles the most common cases.
    /// In a real implementation, this would use more sophisticated analysis.
    fn apply_fix_for_error(
        &self,
        fix_type: &str,
        error_category: &crate::tools::ErrorCategory,
    ) -> Result<Vec<String>, String> {
        match fix_type {
            "missing_dependency" => {
                // Extract the crate name from the error
                if let crate::tools::ErrorCategory::Code { error_type } = error_category {
                    if error_type == "missing_dependency" {
                        // For now, just return success with Cargo.toml
                        // A real implementation would parse the error and add the dependency
                        return Ok(vec!["Cargo.toml".to_string()]);
                    }
                }
                Err("Could not determine missing dependency".to_string())
            }
            "missing_import" => {
                // A real implementation would add the missing import
                Err("Auto-fix for missing imports not yet implemented".to_string())
            }
            "type_error" => {
                // Type errors are harder to fix automatically
                Err("Auto-fix for type errors not yet implemented".to_string())
            }
            "syntax_error" => {
                // Syntax errors need careful analysis
                Err("Auto-fix for syntax errors not yet implemented".to_string())
            }
            _ => Err(format!("Unknown fix type: {}", fix_type)),
        }
    }

    /// Handle a permission error by prompting the user for permission.
    ///
    /// Returns Some(ContentBlock) with the result if the permission was handled,
    /// or None if the permission system is not available or the user denied permission.
    fn handle_permission_error(
        &mut self,
        resource: &str,
        tool_use_id: &str,
        tool_name: &str,
        tool_input: serde_json::Value,
    ) -> Option<ContentBlock> {
        use std::path::Path;

        // Create a permission prompt
        let prompt = PermissionPrompt::new(self.theme.clone());

        // Parse the resource path
        let path = Path::new(resource);

        // Determine operation type from tool name
        let operation = match tool_name {
            "write_file" => OperationType::Write,
            "edit_file" => OperationType::Modify,
            "bash" => OperationType::Write, // Conservative default
            _ => OperationType::Write,
        };

        self.print_newline();

        // Prompt the user
        match prompt.prompt(path, operation) {
            Ok(response) => {
                match response {
                    PermissionResponse::Yes => {
                        // Allow once - record in session
                        if let Some(ref mut checker) = self.permission_checker {
                            checker.record_decision(path, operation, PermissionDecision::Allowed);
                        }

                        self.print_line("  → Permission granted for this operation");

                        // Re-run the tool
                        self.print_line("  → Re-running tool...");
                        self.print_newline();

                        let spinner = if let Some(target) =
                            self.extract_target(tool_name, &tool_input)
                        {
                            ToolExecutionSpinner::with_target(tool_name, target, self.theme.clone())
                        } else {
                            ToolExecutionSpinner::new(tool_name, self.theme.clone())
                        };

                        let result = self
                            .tool_executor
                            .execute(tool_use_id, tool_name, tool_input);

                        match result.result {
                            Ok(output) => {
                                let summary = self.summarize_tool_result(tool_name, &output);
                                spinner.finish_success_with_message(&summary);

                                let formatted =
                                    self.tool_result_formatter.format_result(tool_name, &output);
                                for line in formatted.lines() {
                                    self.print_line(line);
                                }
                                self.print_newline();

                                Some(ContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.to_string(),
                                    content: output,
                                    is_error: None,
                                })
                            }
                            Err(error) => {
                                spinner.finish_failed(&error.message);
                                self.print_line(&format!(
                                    "\x1b[31m  ✗ Still failed: {}\x1b[0m",
                                    error.message
                                ));
                                self.print_newline();

                                Some(ContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.to_string(),
                                    content: format!(
                                        "Permission granted but operation failed: {}",
                                        error.message
                                    ),
                                    is_error: Some(true),
                                })
                            }
                        }
                    }
                    PermissionResponse::No => {
                        self.print_line("  → Permission denied for this operation");
                        self.print_newline();

                        Some(ContentBlock::ToolResult {
                            tool_use_id: tool_use_id.to_string(),
                            content: "Permission denied by user".to_string(),
                            is_error: Some(true),
                        })
                    }
                    PermissionResponse::Always => {
                        // Add to trusted paths and update config
                        // Collect the message first, then print after releasing the borrow
                        let message = if let Some(ref mut checker) = self.permission_checker {
                            match checker.add_trusted_path(path) {
                                Ok(path_str) => {
                                    // Update the config file with the new trusted path
                                    if let Some(ref mut config) = self.app_config {
                                        match config.add_trusted_path(&path_str) {
                                            Ok(_) => {
                                                // Successfully added to config
                                                checker.record_decision(
                                                    path,
                                                    operation,
                                                    PermissionDecision::Allowed,
                                                );
                                                format!("  → Added '{}' to trusted paths and saved to config", path_str)
                                            }
                                            Err(e) => {
                                                // Config update failed, but still allow for this session
                                                checker.record_decision(
                                                    path,
                                                    operation,
                                                    PermissionDecision::Allowed,
                                                );
                                                format!("\x1b[33m  ⚠ Added to session but failed to save to config: {}\x1b[0m", e)
                                            }
                                        }
                                    } else {
                                        // No app config available, just record in session
                                        checker.record_decision(
                                            path,
                                            operation,
                                            PermissionDecision::Allowed,
                                        );
                                        format!(
                                            "  → Added '{}' to trusted paths for this session",
                                            path_str
                                        )
                                    }
                                }
                                Err(e) => {
                                    // Still allow for this session
                                    checker.record_decision(
                                        path,
                                        operation,
                                        PermissionDecision::Allowed,
                                    );
                                    format!(
                                        "\x1b[33m  ⚠ Could not add to trusted paths: {}\x1b[0m",
                                        e
                                    )
                                }
                            }
                        } else {
                            "  → Permission checker not available".to_string()
                        };

                        // Now print the message
                        self.print_line(&message);

                        // Re-run the tool
                        self.print_line("  → Re-running tool...");
                        self.print_newline();

                        let spinner = if let Some(target) =
                            self.extract_target(tool_name, &tool_input)
                        {
                            ToolExecutionSpinner::with_target(tool_name, target, self.theme.clone())
                        } else {
                            ToolExecutionSpinner::new(tool_name, self.theme.clone())
                        };

                        let result = self
                            .tool_executor
                            .execute(tool_use_id, tool_name, tool_input);

                        match result.result {
                            Ok(output) => {
                                let summary = self.summarize_tool_result(tool_name, &output);
                                spinner.finish_success_with_message(&summary);

                                let formatted =
                                    self.tool_result_formatter.format_result(tool_name, &output);
                                for line in formatted.lines() {
                                    self.print_line(line);
                                }
                                self.print_newline();

                                Some(ContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.to_string(),
                                    content: output,
                                    is_error: None,
                                })
                            }
                            Err(error) => {
                                spinner.finish_failed(&error.message);
                                self.print_line(&format!(
                                    "\x1b[31m  ✗ Still failed: {}\x1b[0m",
                                    error.message
                                ));
                                self.print_newline();

                                Some(ContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.to_string(),
                                    content: format!(
                                        "Permission granted but operation failed: {}",
                                        error.message
                                    ),
                                    is_error: Some(true),
                                })
                            }
                        }
                    }
                    PermissionResponse::Never => {
                        // Record never for this session
                        if let Some(ref mut checker) = self.permission_checker {
                            checker.record_decision(path, operation, PermissionDecision::Denied);
                        }

                        self.print_line("  → Permission permanently denied for this session");
                        self.print_newline();

                        Some(ContentBlock::ToolResult {
                            tool_use_id: tool_use_id.to_string(),
                            content: "Permission permanently denied by user".to_string(),
                            is_error: Some(true),
                        })
                    }
                }
            }
            Err(e) => {
                self.print_line(&format!(
                    "\x1b[31m  ✗ Error reading permission response: {}\x1b[0m",
                    e
                ));
                self.print_newline();

                Some(ContentBlock::ToolResult {
                    tool_use_id: tool_use_id.to_string(),
                    content: format!("Permission prompt failed: {}", e),
                    is_error: Some(true),
                })
            }
        }
    }

    /// Handle a resource error by suggesting alternatives.
    ///
    /// This provides actionable alternatives when a resource-related error occurs,
    /// such as disk full, file not found, or tool not found.
    fn handle_resource_error(&mut self, resource_type: &str, _error_message: &str) {
        match resource_type {
            "disk_full" => {
                self.print_line("\x1b[33m  💡 Alternatives:\x1b[0m");
                self.print_line("     • Use 'df -h' to check disk usage");
                self.print_line("     • Clean up temporary files (e.g., cargo clean, rm /tmp/*)");
                self.print_line("     • Move files to a different disk with more space");
                self.print_line("     • Compress old files to free up space");
            }
            "out_of_memory" => {
                self.print_line("\x1b[33m  💡 Alternatives:\x1b[0m");
                self.print_line("     • Process data in smaller chunks");
                self.print_line("     • Close other applications to free up memory");
                self.print_line("     • Use streaming or incremental processing");
                self.print_line("     • Consider using a machine with more RAM");
            }
            "not_found" => {
                self.print_line("\x1b[33m  💡 Alternatives:\x1b[0m");
                self.print_line("     • Check the file path for typos");
                self.print_line("     • Use 'ls' to list available files in the directory");
                self.print_line("     • Create the file if it should exist");
                self.print_line("     • Check if the file was moved or deleted");
            }
            "tool_not_found" => {
                self.print_line("\x1b[33m  💡 Available tools:\x1b[0m");
                let tool_names = self.tool_executor.tool_names();
                for tool_name in tool_names {
                    self.print_line(&format!("     • {}", tool_name));
                }
            }
            _ => {
                // For unknown resource types, just show a generic message
                self.print_line(&format!(
                    "\x1b[33m  💡 Resource error ({}): Consider alternative approaches\x1b[0m",
                    resource_type
                ));
            }
        }
    }

    /// Update and render the status bar showing active agents
    fn update_status_bar(&mut self) -> Result<(), String> {
        // Process any pending progress updates from agents
        self.agent_manager.process_progress_updates();

        // Get all agent statuses
        let statuses = self.agent_manager.get_all_statuses();

        // Filter to only show active agents (not complete or cancelled)
        let active_statuses: Vec<_> = statuses
            .into_iter()
            .filter(|s| {
                s.state.is_active() || matches!(s.state, crate::agents::status::AgentState::Queued)
            })
            .collect();

        // Clear previous status bar if it was rendered
        if self.status_bar_lines > 0 {
            self.status_bar
                .clear(self.status_bar_lines)
                .map_err(|e| e.to_string())?;
            self.status_bar_lines = 0;
        }

        // Render new status bar if there are active agents
        if !active_statuses.is_empty() {
            self.status_bar_lines = self
                .status_bar
                .render(&active_statuses)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Run the REPL loop
    pub async fn run(&mut self, _terminal: &mut Terminal) -> Result<(), String> {
        self.print_welcome();

        loop {
            // Update and render status bar showing active agents
            if let Err(e) = self.update_status_bar() {
                if self.config.verbose {
                    eprintln!("[verbose] Warning: Failed to update status bar: {}", e);
                }
            }

            // Show mode indicator in prompt if in planning mode
            if let Some(indicator) = self.mode.indicator() {
                print!("{} > ", indicator);
            } else {
                print!("> ");
            }
            std::io::stdout().flush().map_err(|e| e.to_string())?;

            match self.input_handler.read_input().await {
                Ok(InputResult::Submitted(text)) => {
                    if text.is_empty() {
                        continue;
                    }

                    if self.config.verbose {
                        eprintln!("[verbose] Input: {:?}", text);
                    }

                    match self.process_input(&text) {
                        ReplAction::Continue => continue,
                        ReplAction::Exit => {
                            // Save session before exiting
                            if let Err(e) = self.save_session() {
                                eprint!("Warning: Failed to save session: {}\r\n", e);
                            }
                            self.print_newline();
                            self.print_line("Goodbye!");
                            self.print_newline();
                            break;
                        }
                        ReplAction::Clear => {
                            // Save session before clearing
                            if let Err(e) = self.save_session() {
                                eprint!("Warning: Failed to save session: {}\r\n", e);
                            }
                            // Start a new session and reset context tracking
                            self.session = Session::new();
                            self.reset_context();
                            Terminal::clear().map_err(|e| e.to_string())?;
                            self.print_welcome();
                        }
                        ReplAction::Output(output) => {
                            self.print_newline();
                            // Print output line by line with proper \r\n
                            for line in output.lines() {
                                self.print_line(line);
                            }
                            self.print_newline();
                        }
                        ReplAction::Error(error) => {
                            self.print_newline();
                            self.print_line(&format!("Error: {}", error));
                            self.print_newline();
                        }
                        ReplAction::ModeChange { mode, output } => {
                            // Set the new mode
                            self.mode = mode.clone();

                            // Show mode indicator if available
                            if let Some(indicator) = mode.indicator() {
                                self.print_newline();
                                self.print_line(&indicator);
                            }

                            // Display output if provided
                            if let Some(output) = output {
                                self.print_newline();
                                for line in output.lines() {
                                    self.print_line(line);
                                }
                            }
                            self.print_newline();
                        }
                        ReplAction::Message(input) => {
                            // Record the user message and update token count
                            self.session.add_user_message(&input);
                            self.update_context_tokens("user", &input);

                            // Add user message to conversation history
                            self.conversation.push(Message::user(&input));

                            // Process the conversation with tool use loop
                            if let Err(e) = self.process_conversation() {
                                self.print_newline();
                                self.print_line(&format!("Error: {}", e));
                                self.print_newline();
                            }

                            // Display the context bar after the exchange
                            self.display_context_bar();
                            self.print_newline();

                            // Auto-save after each exchange
                            if let Err(e) = self.save_session() {
                                if self.config.verbose {
                                    eprintln!(
                                        "[verbose] Warning: Failed to auto-save session: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(InputResult::Cancelled) => {
                    self.print_newline();
                    self.print_line("[Input cleared]");
                    self.print_newline();
                }
                Ok(InputResult::Exit) => {
                    // Save session before exiting
                    if let Err(e) = self.save_session() {
                        eprint!("Warning: Failed to save session: {}\r\n", e);
                    }
                    self.print_newline();
                    self.print_line("Goodbye!");
                    self.print_newline();
                    break;
                }
                Err(e) => {
                    self.print_newline();
                    self.print_line(&format!("Error reading input: {}", e));
                    self.print_newline();
                }
            }
        }

        Ok(())
    }

    /// Print a line with proper raw mode handling (\r\n instead of just \n)
    fn print_line(&self, text: &str) {
        print!("{}\r\n", text);
        let _ = std::io::stdout().flush();
    }

    /// Print an empty line
    fn print_newline(&self) {
        print!("\r\n");
        let _ = std::io::stdout().flush();
    }

    /// Print the welcome message
    fn print_welcome(&self) {
        self.print_line("coding-agent v0.1.0");
        self.print_line("Type your message and press Enter twice to submit.");
        self.print_line("Use /help for available commands.");
        self.print_newline();
    }

    /// Process a line of input, returning the action to take
    fn process_input(&self, input: &str) -> ReplAction {
        // Check if this is a command
        if let Some((cmd_name, args)) = parse_command(input) {
            return self.execute_command(cmd_name, &args);
        }

        // Check if user tried to enter a command but it was invalid (just "/" or "/ ")
        if input.trim().starts_with('/') {
            return ReplAction::Error(
                "Invalid command. Type /help for available commands.".to_string(),
            );
        }

        // Regular message
        ReplAction::Message(input.to_string())
    }

    /// Execute a slash command
    fn execute_command(&self, name: &str, args: &[&str]) -> ReplAction {
        let mut ctx = CommandContext {
            registry: self.registry.clone(),
            cost_tracker: self.cost_tracker.clone(),
            agent_manager: Some(Arc::clone(&self.agent_manager)),
            config: Arc::new(self.app_config.clone().unwrap_or_default()),
            collapsed_results: Arc::clone(&self.collapsed_results),
        };

        match self.registry.get(name) {
            Some(cmd) => match cmd.execute(args, &mut ctx) {
                CommandResult::Continue => ReplAction::Continue,
                CommandResult::Exit => ReplAction::Exit,
                CommandResult::Cleared => ReplAction::Clear,
                CommandResult::Output(output) => ReplAction::Output(output),
                CommandResult::Error(error) => ReplAction::Error(error),
                CommandResult::ModeChange { mode, output } => {
                    ReplAction::ModeChange { mode, output }
                }
            },
            None => ReplAction::Error(format!(
                "Unknown command: /{}. Try /help for available commands.",
                name
            )),
        }
    }

    /// Get a reference to the cost tracker
    pub fn cost_tracker(&self) -> &CostTracker {
        &self.cost_tracker
    }
}

/// Action to take after processing input
enum ReplAction {
    /// Continue the REPL loop
    Continue,
    /// Exit the REPL
    Exit,
    /// Clear the screen
    Clear,
    /// Display output
    Output(String),
    /// Display an error
    Error(String),
    /// A regular message (not a command)
    Message(String),
    /// Change mode with optional output
    ModeChange { mode: Mode, output: Option<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_input_command() {
        let repl = Repl::new(ReplConfig::default());

        // Help command
        let action = repl.process_input("/help");
        assert!(matches!(action, ReplAction::Output(_)));

        // Exit command
        let action = repl.process_input("/exit");
        assert!(matches!(action, ReplAction::Exit));

        // Clear command
        let action = repl.process_input("/clear");
        assert!(matches!(action, ReplAction::Clear));

        // Unknown command
        let action = repl.process_input("/unknown");
        assert!(matches!(action, ReplAction::Error(_)));
    }

    #[test]
    fn test_process_input_message() {
        let repl = Repl::new(ReplConfig::default());

        // Regular message (not a command)
        let action = repl.process_input("Hello, world!");
        match action {
            ReplAction::Message(msg) => assert_eq!(msg, "Hello, world!"),
            _ => panic!("Expected Message action"),
        }
    }

    #[test]
    fn test_execute_unknown_command() {
        let repl = Repl::new(ReplConfig::default());

        let action = repl.execute_command("nonexistent", &[]);
        match action {
            ReplAction::Error(msg) => {
                assert!(msg.contains("Unknown command"));
                assert!(msg.contains("/help"));
            }
            _ => panic!("Expected Error action"),
        }
    }

    #[test]
    fn test_slash_only_is_invalid_command() {
        let repl = Repl::new(ReplConfig::default());

        // Just "/" should be an error, not a message
        let action = repl.process_input("/");
        match action {
            ReplAction::Error(msg) => {
                assert!(msg.contains("Invalid command"));
                assert!(msg.contains("/help"));
            }
            _ => panic!("Expected Error action for just '/'"),
        }

        // "/ " (slash with space) should also be an error
        let action = repl.process_input("/ ");
        match action {
            ReplAction::Error(msg) => {
                assert!(msg.contains("Invalid command"));
            }
            _ => panic!("Expected Error action for '/ '"),
        }
    }

    #[test]
    fn test_context_bar_initial_state() {
        let repl = Repl::new(ReplConfig::default());

        assert_eq!(repl.context_bar().current_tokens(), 0);
        assert_eq!(repl.context_bar().max_tokens(), 200_000);
        assert_eq!(repl.context_bar().percent(), 0);
    }

    #[test]
    fn test_context_bar_custom_window_size() {
        let config = ReplConfig {
            context_window: 100_000,
            ..ReplConfig::default()
        };
        let repl = Repl::new(config);

        assert_eq!(repl.context_bar().max_tokens(), 100_000);
    }

    #[test]
    fn test_update_context_tokens() {
        let mut repl = Repl::new(ReplConfig::default());

        // Initial state
        assert_eq!(repl.context_bar().current_tokens(), 0);

        // Update with a user message
        repl.update_context_tokens("user", "Hello, world!");
        let tokens_after_user = repl.context_bar().current_tokens();
        assert!(tokens_after_user > 0, "Should have counted tokens");

        // Update with an assistant message
        repl.update_context_tokens("assistant", "Hi there! How can I help?");
        let tokens_after_assistant = repl.context_bar().current_tokens();
        assert!(
            tokens_after_assistant > tokens_after_user,
            "Should have accumulated more tokens"
        );
    }

    #[test]
    fn test_reset_context() {
        let mut repl = Repl::new(ReplConfig::default());

        // Add some tokens
        repl.update_context_tokens("user", "Hello!");
        assert!(repl.context_bar().current_tokens() > 0);

        // Reset context
        repl.reset_context();
        assert_eq!(repl.context_bar().current_tokens(), 0);
    }

    #[test]
    fn test_context_bar_renders_correctly() {
        let mut repl = Repl::new(ReplConfig::default());

        // Add some tokens
        repl.update_context_tokens("user", "Test message");

        let rendered = repl.context_bar().render();
        assert!(rendered.contains("Context:"));
        assert!(rendered.contains("%"));
        assert!(rendered.contains("tokens"));
    }

    #[test]
    fn test_show_context_bar_config() {
        // Test with context bar enabled (default)
        let config_enabled = ReplConfig {
            show_context_bar: true,
            ..ReplConfig::default()
        };
        let repl_enabled = Repl::new(config_enabled);
        assert!(repl_enabled.config.show_context_bar);

        // Test with context bar disabled
        let config_disabled = ReplConfig {
            show_context_bar: false,
            ..ReplConfig::default()
        };
        let repl_disabled = Repl::new(config_disabled);
        assert!(!repl_disabled.config.show_context_bar);
    }

    #[test]
    fn test_format_tool_call_read_file() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({"path": "src/main.rs"});
        let formatted = repl.format_tool_call("read_file", &input);
        assert_eq!(formatted, "Reading src/main.rs");
    }

    #[test]
    fn test_format_tool_call_write_file() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({"path": "output.txt", "content": "hello"});
        let formatted = repl.format_tool_call("write_file", &input);
        assert_eq!(formatted, "Writing output.txt");
    }

    #[test]
    fn test_format_tool_call_edit_file() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({"path": "test.rs", "search": "old", "replace": "new"});
        let formatted = repl.format_tool_call("edit_file", &input);
        assert_eq!(formatted, "Editing test.rs");
    }

    #[test]
    fn test_format_tool_call_list_files() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({"path": "/home/user"});
        let formatted = repl.format_tool_call("list_files", &input);
        assert_eq!(formatted, "Listing files in /home/user");
    }

    #[test]
    fn test_format_tool_call_list_files_current_dir() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({});
        let formatted = repl.format_tool_call("list_files", &input);
        assert_eq!(formatted, "Listing files in .");
    }

    #[test]
    fn test_format_tool_call_bash_short() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({"command": "ls -la"});
        let formatted = repl.format_tool_call("bash", &input);
        assert_eq!(formatted, "Running: ls -la");
    }

    #[test]
    fn test_format_tool_call_bash_long() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({"command": "find . -name '*.rs' -type f -exec grep -l 'pattern' {} +"});
        let formatted = repl.format_tool_call("bash", &input);
        // Should truncate long commands
        assert!(formatted.starts_with("Running: find . -name '*.rs' -type f -exec grep"));
        assert!(formatted.ends_with("..."));
        assert!(formatted.len() <= 60);
    }

    #[test]
    fn test_format_tool_call_code_search() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({"pattern": "fn main"});
        let formatted = repl.format_tool_call("code_search", &input);
        assert_eq!(formatted, "Searching for 'fn main'");
    }

    #[test]
    fn test_format_tool_call_unknown_tool() {
        let repl = Repl::new(ReplConfig::default());
        let input = serde_json::json!({});
        let formatted = repl.format_tool_call("unknown_tool", &input);
        assert_eq!(formatted, "Executing unknown_tool");
    }

    #[test]
    fn test_summarize_tool_result_read_file() {
        let repl = Repl::new(ReplConfig::default());
        let output = "line1\nline2\nline3";
        let summary = repl.summarize_tool_result("read_file", output);
        assert_eq!(summary, "Read 3 lines (17 bytes)");
    }

    #[test]
    fn test_summarize_tool_result_write_file() {
        let repl = Repl::new(ReplConfig::default());
        let output = "File written successfully";
        let summary = repl.summarize_tool_result("write_file", output);
        assert_eq!(summary, "File written successfully");
    }

    #[test]
    fn test_summarize_tool_result_edit_file_ok() {
        let repl = Repl::new(ReplConfig::default());
        let summary = repl.summarize_tool_result("edit_file", "OK");
        assert_eq!(summary, "Edit applied");
    }

    #[test]
    fn test_summarize_tool_result_edit_file_error() {
        let repl = Repl::new(ReplConfig::default());
        let summary = repl.summarize_tool_result("edit_file", "Pattern not found");
        assert_eq!(summary, "Pattern not found");
    }

    #[test]
    fn test_summarize_tool_result_list_files() {
        let repl = Repl::new(ReplConfig::default());
        let output = r#"["file1.rs", "file2.rs", "dir1"]"#;
        let summary = repl.summarize_tool_result("list_files", output);
        assert_eq!(summary, "Found 3 files/directories");
    }

    #[test]
    fn test_summarize_tool_result_bash_no_output() {
        let repl = Repl::new(ReplConfig::default());
        let summary = repl.summarize_tool_result("bash", "");
        assert_eq!(summary, "Command completed (no output)");
    }

    #[test]
    fn test_summarize_tool_result_bash_short() {
        let repl = Repl::new(ReplConfig::default());
        let output = "total 42";
        let summary = repl.summarize_tool_result("bash", output);
        assert_eq!(summary, "total 42");
    }

    #[test]
    fn test_summarize_tool_result_bash_long() {
        let repl = Repl::new(ReplConfig::default());
        let output =
            "This is a very long output line that exceeds sixty characters and should be truncated";
        let summary = repl.summarize_tool_result("bash", output);
        assert!(summary.ends_with("..."));
        assert!(summary.len() <= 63); // 60 chars + "..."
    }

    #[test]
    fn test_summarize_tool_result_bash_multiline() {
        let repl = Repl::new(ReplConfig::default());
        let output = "line1\nline2\nline3\nline4\nline5";
        let summary = repl.summarize_tool_result("bash", output);
        assert_eq!(summary, "Output: 5 lines");
    }

    #[test]
    fn test_summarize_tool_result_code_search_found() {
        let repl = Repl::new(ReplConfig::default());
        let output = "src/main.rs:10\nsrc/lib.rs:25";
        let summary = repl.summarize_tool_result("code_search", output);
        assert_eq!(summary, "Found 2 matches");
    }

    #[test]
    fn test_summarize_tool_result_code_search_no_matches() {
        let repl = Repl::new(ReplConfig::default());
        let output = "No matches found";
        let summary = repl.summarize_tool_result("code_search", output);
        assert_eq!(summary, "No matches found");
    }
}

use crate::types::{ContentBlock, Message};
use serde_json::Value;

/// Tracks the status of a single tool execution
#[derive(Debug, Clone, PartialEq)]
pub enum ToolExecutionStatus {
    Pending {
        call_id: String,
        tool_name: String,
        input: Value,
    },
    Running {
        call_id: String,
        tool_name: String,
    },
    Completed {
        call_id: String,
        result: Result<String, String>,
    },
}

/// The agent's current state
#[derive(Debug, Clone)]
pub enum AgentState {
    /// Idle, waiting for user to type a message
    WaitingForUserInput { conversation: Vec<Message> },

    /// Making a request to Claude
    CallingLlm {
        conversation: Vec<Message>,
        retries: u32,
    },

    /// Processing a completed LLM response (transient state)
    ProcessingLlmResponse {
        conversation: Vec<Message>,
        response_content: Vec<ContentBlock>,
        stop_reason: String,
    },

    /// Executing one or more tool calls
    ExecutingTools {
        conversation: Vec<Message>,
        executions: Vec<ToolExecutionStatus>,
    },

    /// Post-tool-execution hook point for quality gates, context checks, etc.
    PostToolsHook {
        conversation: Vec<Message>,
        pending_tool_results: Vec<Message>,
    },

    /// Recoverable error with retry capability
    Error {
        conversation: Vec<Message>,
        error_message: String,
        retries: u32,
    },

    /// Terminal state - agent is shutting down
    ShuttingDown,
}

impl AgentState {
    /// Returns the state name for logging
    pub fn name(&self) -> &'static str {
        match self {
            Self::WaitingForUserInput { .. } => "WaitingForUserInput",
            Self::CallingLlm { .. } => "CallingLlm",
            Self::ProcessingLlmResponse { .. } => "ProcessingLlmResponse",
            Self::ExecutingTools { .. } => "ExecutingTools",
            Self::PostToolsHook { .. } => "PostToolsHook",
            Self::Error { .. } => "Error",
            Self::ShuttingDown => "ShuttingDown",
        }
    }

    /// Returns the conversation if this state has one
    pub fn conversation(&self) -> Option<&Vec<Message>> {
        match self {
            Self::WaitingForUserInput { conversation } => Some(conversation),
            Self::CallingLlm { conversation, .. } => Some(conversation),
            Self::ProcessingLlmResponse { conversation, .. } => Some(conversation),
            Self::ExecutingTools { conversation, .. } => Some(conversation),
            Self::PostToolsHook { conversation, .. } => Some(conversation),
            Self::Error { conversation, .. } => Some(conversation),
            Self::ShuttingDown => None,
        }
    }

    /// Returns true if this is the ShuttingDown state
    pub fn is_shutting_down(&self) -> bool {
        matches!(self, Self::ShuttingDown)
    }
}

/// Events that can be sent to the state machine
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// User submitted a message
    UserInput(String),

    /// LLM request completed successfully
    LlmCompleted {
        content: Vec<ContentBlock>,
        stop_reason: String,
    },

    /// LLM request failed
    LlmError(String),

    /// A tool execution completed
    ToolCompleted {
        call_id: String,
        result: Result<String, String>,
    },

    /// Post-tool hooks have completed
    HooksCompleted {
        proceed: bool,
        warning: Option<String>,
    },

    /// Retry timer expired (for error recovery)
    RetryTimeout,

    /// User requested shutdown (e.g., Ctrl+C or "/quit")
    ShutdownRequested,
}

/// Actions the caller must perform
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAction {
    /// Send a request to Claude
    SendLlmRequest { messages: Vec<Message> },

    /// Execute the specified tool calls
    ExecuteTools { calls: Vec<ToolCall> },

    /// Display text to the user
    DisplayText(String),

    /// Display an error to the user
    DisplayError(String),

    /// Prompt for user input
    PromptForInput,

    /// Wait for next event (no action needed)
    WaitForEvent,

    /// Schedule a retry after a delay
    ScheduleRetry { delay_ms: u64 },

    /// Run post-tool hooks (caller decides what to run)
    RunPostToolsHooks { tool_names: Vec<String> },

    /// Display a warning to the user (non-blocking)
    DisplayWarning(String),

    /// Terminate the agent
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub input: Value,
}

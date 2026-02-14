use crate::state::{AgentAction, AgentEvent, AgentState, ToolCall, ToolExecutionStatus};
use crate::types::{ContentBlock, Message};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

pub struct StateMachine {
    state: AgentState,
    verbose: bool,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: AgentState::WaitingForUserInput {
                conversation: Vec::new(),
            },
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn state(&self) -> &AgentState {
        &self.state
    }

    /// Process an event and return the action to perform
    pub fn handle_event(&mut self, event: AgentEvent) -> AgentAction {
        let old_state_name = self.state.name();

        let action = self.transition(event);

        if self.verbose {
            eprintln!("[STATE] {} -> {}", old_state_name, self.state.name());
        }

        action
    }

    fn transition(&mut self, event: AgentEvent) -> AgentAction {
        // Handle shutdown from any state
        if matches!(event, AgentEvent::ShutdownRequested) {
            self.state = AgentState::ShuttingDown;
            return AgentAction::Shutdown;
        }

        match (&self.state, event) {
            // === WaitingForUserInput ===
            (AgentState::WaitingForUserInput { conversation }, AgentEvent::UserInput(text)) => {
                let mut conv = conversation.clone();
                conv.push(Message::user(text));

                self.state = AgentState::CallingLlm {
                    conversation: conv.clone(),
                    retries: 0,
                };

                AgentAction::SendLlmRequest { messages: conv }
            }

            // === CallingLlm ===
            (
                AgentState::CallingLlm { conversation, .. },
                AgentEvent::LlmCompleted {
                    content,
                    stop_reason,
                },
            ) => {
                self.state = AgentState::ProcessingLlmResponse {
                    conversation: conversation.clone(),
                    response_content: content,
                    stop_reason,
                };

                self.process_llm_response()
            }

            (
                AgentState::CallingLlm {
                    conversation,
                    retries,
                },
                AgentEvent::LlmError(error),
            ) => {
                let retries = *retries;
                if retries < MAX_RETRIES {
                    self.state = AgentState::Error {
                        conversation: conversation.clone(),
                        error_message: error.clone(),
                        retries,
                    };

                    AgentAction::ScheduleRetry {
                        delay_ms: RETRY_DELAY_MS * (retries + 1) as u64,
                    }
                } else {
                    self.state = AgentState::WaitingForUserInput {
                        conversation: conversation.clone(),
                    };

                    AgentAction::DisplayError(format!(
                        "LLM request failed after {} retries: {}",
                        MAX_RETRIES, error
                    ))
                }
            }

            // === Error (retry) ===
            (
                AgentState::Error {
                    conversation,
                    retries,
                    ..
                },
                AgentEvent::RetryTimeout,
            ) => {
                let conv = conversation.clone();
                let new_retries = *retries + 1;

                self.state = AgentState::CallingLlm {
                    conversation: conv.clone(),
                    retries: new_retries,
                };

                AgentAction::SendLlmRequest { messages: conv }
            }

            // === ExecutingTools ===
            (
                AgentState::ExecutingTools {
                    conversation,
                    executions,
                },
                AgentEvent::ToolCompleted { call_id, result },
            ) => {
                let mut execs = executions.clone();

                // Update the completed tool (from Pending or Running)
                for exec in &mut execs {
                    match exec {
                        ToolExecutionStatus::Pending { call_id: id, .. }
                        | ToolExecutionStatus::Running { call_id: id, .. } => {
                            if id == &call_id {
                                *exec = ToolExecutionStatus::Completed {
                                    call_id: call_id.clone(),
                                    result: result.clone(),
                                };
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                // Check if all tools are done
                let all_done = execs
                    .iter()
                    .all(|e| matches!(e, ToolExecutionStatus::Completed { .. }));

                if all_done {
                    // Build tool result messages
                    let mut tool_result_messages = Vec::new();
                    for exec in &execs {
                        if let ToolExecutionStatus::Completed { call_id, result } = exec {
                            match result {
                                Ok(output) => {
                                    tool_result_messages.push(Message::tool_result(
                                        call_id.clone(),
                                        output.clone(),
                                    ));
                                }
                                Err(err) => {
                                    tool_result_messages.push(Message::tool_result_error(
                                        call_id.clone(),
                                        format!("Error: {}", err),
                                    ));
                                }
                            }
                        }
                    }

                    // Collect tool names for hook decision (from original executions)
                    let tool_names: Vec<String> = executions
                        .iter()
                        .filter_map(|e| match e {
                            ToolExecutionStatus::Pending { tool_name, .. } => {
                                Some(tool_name.clone())
                            }
                            ToolExecutionStatus::Running { tool_name, .. } => {
                                Some(tool_name.clone())
                            }
                            ToolExecutionStatus::Completed { .. } => None,
                        })
                        .collect();

                    // Transition to PostToolsHook instead of CallingLlm
                    self.state = AgentState::PostToolsHook {
                        conversation: conversation.clone(),
                        pending_tool_results: tool_result_messages,
                    };

                    AgentAction::RunPostToolsHooks { tool_names }
                } else {
                    self.state = AgentState::ExecutingTools {
                        conversation: conversation.clone(),
                        executions: execs,
                    };

                    AgentAction::WaitForEvent
                }
            }

            // === PostToolsHook ===
            (
                AgentState::PostToolsHook {
                    conversation,
                    pending_tool_results,
                },
                AgentEvent::HooksCompleted { proceed, warning },
            ) => {
                if proceed {
                    // Continue to LLM with tool results
                    let mut conv = conversation.clone();
                    conv.extend(pending_tool_results.clone());

                    self.state = AgentState::CallingLlm {
                        conversation: conv.clone(),
                        retries: 0,
                    };

                    // If there's a warning, return DisplayWarning action
                    // The caller should handle it and then call SendLlmRequest
                    if let Some(w) = warning {
                        AgentAction::DisplayWarning(w)
                    } else {
                        AgentAction::SendLlmRequest { messages: conv }
                    }
                } else {
                    // Stop loop, return to user
                    self.state = AgentState::WaitingForUserInput {
                        conversation: conversation.clone(),
                    };

                    if let Some(w) = warning {
                        AgentAction::DisplayWarning(w)
                    } else {
                        AgentAction::PromptForInput
                    }
                }
            }

            // === Invalid transition ===
            (state, event) => {
                if self.verbose {
                    eprintln!(
                        "[WARN] Invalid transition: {:?} in state {}",
                        event,
                        state.name()
                    );
                }
                AgentAction::WaitForEvent
            }
        }
    }

    /// Helper: Process an LLM response (called from ProcessingLlmResponse state)
    fn process_llm_response(&mut self) -> AgentAction {
        let (conversation, content, _stop_reason) = match &self.state {
            AgentState::ProcessingLlmResponse {
                conversation,
                response_content,
                stop_reason,
            } => (
                conversation.clone(),
                response_content.clone(),
                stop_reason.clone(),
            ),
            _ => return AgentAction::WaitForEvent,
        };

        // Extract text and tool calls from response
        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for block in &content {
            match block {
                ContentBlock::Text { text } => text_parts.push(text.clone()),
                ContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        call_id: id.clone(),
                        tool_name: name.clone(),
                        input: input.clone(),
                    });
                }
                _ => {}
            }
        }

        // Update conversation with assistant message
        let mut conv = conversation;
        conv.push(Message::assistant(content));

        if !tool_calls.is_empty() {
            // Transition to ExecutingTools
            let executions = tool_calls
                .iter()
                .map(|tc| ToolExecutionStatus::Pending {
                    call_id: tc.call_id.clone(),
                    tool_name: tc.tool_name.clone(),
                    input: tc.input.clone(),
                })
                .collect();

            self.state = AgentState::ExecutingTools {
                conversation: conv,
                executions,
            };

            AgentAction::ExecuteTools { calls: tool_calls }
        } else {
            // No tool calls - display text and wait for user
            self.state = AgentState::WaitingForUserInput { conversation: conv };

            let display_text = text_parts.join("");
            if !display_text.is_empty() {
                AgentAction::DisplayText(display_text)
            } else {
                AgentAction::PromptForInput
            }
        }
    }

    /// For testing: set the initial state
    #[cfg(test)]
    pub fn with_state(mut self, state: AgentState) -> Self {
        self.state = state;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_machine_starts_waiting_for_input() {
        let machine = StateMachine::new();
        assert_eq!(machine.state().name(), "WaitingForUserInput");
    }

    #[test]
    fn test_user_input_transitions_to_calling_llm() {
        let mut machine = StateMachine::new();
        let action = machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        assert!(matches!(action, AgentAction::SendLlmRequest { .. }));
        assert_eq!(machine.state().name(), "CallingLlm");

        // Verify the message was added to conversation
        if let AgentState::CallingLlm { conversation, .. } = machine.state() {
            assert_eq!(conversation.len(), 1);
            assert_eq!(conversation[0].role, "user");
        } else {
            panic!("Expected CallingLlm state");
        }
    }

    #[test]
    fn test_llm_completed_with_text_transitions_to_waiting() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        let action = machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::Text {
                text: "Hi there!".to_string(),
            }],
            stop_reason: "end_turn".to_string(),
        });

        assert!(matches!(action, AgentAction::DisplayText(ref text) if text == "Hi there!"));
        assert_eq!(machine.state().name(), "WaitingForUserInput");
    }

    #[test]
    fn test_llm_completed_with_tool_use_transitions_to_executing() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run a command".to_string()));

        let action = machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "echo hello"}),
            }],
            stop_reason: "tool_use".to_string(),
        });

        assert!(matches!(action, AgentAction::ExecuteTools { .. }));
        assert_eq!(machine.state().name(), "ExecutingTools");
    }

    #[test]
    fn test_llm_error_triggers_retry() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        let action = machine.handle_event(AgentEvent::LlmError("timeout".to_string()));

        assert!(matches!(
            action,
            AgentAction::ScheduleRetry { delay_ms: 1000 }
        ));
        assert_eq!(machine.state().name(), "Error");
    }

    #[test]
    fn test_retry_timeout_retries_llm_call() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));
        machine.handle_event(AgentEvent::LlmError("timeout".to_string()));

        let action = machine.handle_event(AgentEvent::RetryTimeout);

        assert!(matches!(action, AgentAction::SendLlmRequest { .. }));
        assert_eq!(machine.state().name(), "CallingLlm");

        // Verify retries counter incremented
        if let AgentState::CallingLlm { retries, .. } = machine.state() {
            assert_eq!(*retries, 1);
        } else {
            panic!("Expected CallingLlm state");
        }
    }

    #[test]
    fn test_max_retries_returns_to_waiting() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        // Exhaust retries (0, 1, 2 are the retries, error on retry 3 exceeds MAX_RETRIES)
        for i in 0..MAX_RETRIES {
            let action = machine.handle_event(AgentEvent::LlmError("timeout".to_string()));
            assert!(
                matches!(action, AgentAction::ScheduleRetry { .. }),
                "Expected ScheduleRetry on attempt {}",
                i
            );
            machine.handle_event(AgentEvent::RetryTimeout);
        }

        // Final error should return to waiting
        let action = machine.handle_event(AgentEvent::LlmError("timeout".to_string()));

        assert!(matches!(action, AgentAction::DisplayError(_)));
        assert_eq!(machine.state().name(), "WaitingForUserInput");
    }

    #[test]
    fn test_tool_completed_single_tool() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run a command".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "echo hello"}),
            }],
            stop_reason: "tool_use".to_string(),
        });

        let action = machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_1".to_string(),
            result: Ok("hello".to_string()),
        });

        // Should transition to PostToolsHook (not directly to CallingLlm)
        assert!(matches!(action, AgentAction::RunPostToolsHooks { .. }));
        assert_eq!(machine.state().name(), "PostToolsHook");

        // After hooks complete, should transition to CallingLlm
        let action = machine.handle_event(AgentEvent::HooksCompleted {
            proceed: true,
            warning: None,
        });
        assert!(matches!(action, AgentAction::SendLlmRequest { .. }));
        assert_eq!(machine.state().name(), "CallingLlm");
    }

    #[test]
    fn test_tool_completed_multiple_tools() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run commands".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool_1".to_string(),
                    name: "bash".to_string(),
                    input: json!({"command": "echo one"}),
                },
                ContentBlock::ToolUse {
                    id: "tool_2".to_string(),
                    name: "bash".to_string(),
                    input: json!({"command": "echo two"}),
                },
            ],
            stop_reason: "tool_use".to_string(),
        });

        // Complete first tool - should wait for more
        let action = machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_1".to_string(),
            result: Ok("one".to_string()),
        });
        assert!(matches!(action, AgentAction::WaitForEvent));
        assert_eq!(machine.state().name(), "ExecutingTools");

        // Complete second tool - should proceed to PostToolsHook
        let action = machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_2".to_string(),
            result: Ok("two".to_string()),
        });
        assert!(matches!(action, AgentAction::RunPostToolsHooks { .. }));
        assert_eq!(machine.state().name(), "PostToolsHook");

        // After hooks complete, should transition to CallingLlm
        let action = machine.handle_event(AgentEvent::HooksCompleted {
            proceed: true,
            warning: None,
        });
        assert!(matches!(action, AgentAction::SendLlmRequest { .. }));
        assert_eq!(machine.state().name(), "CallingLlm");
    }

    #[test]
    fn test_tool_completed_with_error() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run a command".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "invalid"}),
            }],
            stop_reason: "tool_use".to_string(),
        });

        let action = machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_1".to_string(),
            result: Err("command not found".to_string()),
        });

        // Should proceed to PostToolsHook (even with error)
        assert!(matches!(action, AgentAction::RunPostToolsHooks { .. }));
        assert_eq!(machine.state().name(), "PostToolsHook");

        // After hooks complete, should transition to CallingLlm
        let action = machine.handle_event(AgentEvent::HooksCompleted {
            proceed: true,
            warning: None,
        });
        assert!(matches!(action, AgentAction::SendLlmRequest { .. }));
        assert_eq!(machine.state().name(), "CallingLlm");
    }

    #[test]
    fn test_shutdown_from_waiting() {
        let mut machine = StateMachine::new();
        let action = machine.handle_event(AgentEvent::ShutdownRequested);

        assert!(matches!(action, AgentAction::Shutdown));
        assert_eq!(machine.state().name(), "ShuttingDown");
    }

    #[test]
    fn test_shutdown_from_calling_llm() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        let action = machine.handle_event(AgentEvent::ShutdownRequested);

        assert!(matches!(action, AgentAction::Shutdown));
        assert_eq!(machine.state().name(), "ShuttingDown");
    }

    #[test]
    fn test_shutdown_from_executing_tools() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({}),
            }],
            stop_reason: "tool_use".to_string(),
        });

        let action = machine.handle_event(AgentEvent::ShutdownRequested);

        assert!(matches!(action, AgentAction::Shutdown));
        assert_eq!(machine.state().name(), "ShuttingDown");
    }

    #[test]
    fn test_shutdown_from_error() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));
        machine.handle_event(AgentEvent::LlmError("error".to_string()));

        let action = machine.handle_event(AgentEvent::ShutdownRequested);

        assert!(matches!(action, AgentAction::Shutdown));
        assert_eq!(machine.state().name(), "ShuttingDown");
    }

    #[test]
    fn test_invalid_transition_returns_wait() {
        let mut machine = StateMachine::new();

        // Try to send ToolCompleted when waiting for input (invalid)
        let action = machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "x".to_string(),
            result: Ok("y".to_string()),
        });

        assert!(matches!(action, AgentAction::WaitForEvent));
        // State should remain unchanged
        assert_eq!(machine.state().name(), "WaitingForUserInput");
    }

    #[test]
    fn test_conversation_preserved_across_transitions() {
        let mut machine = StateMachine::new();

        // User input
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        // LLM response
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::Text {
                text: "Hi!".to_string(),
            }],
            stop_reason: "end_turn".to_string(),
        });

        // Check conversation has both messages
        if let AgentState::WaitingForUserInput { conversation } = machine.state() {
            assert_eq!(conversation.len(), 2);
            assert_eq!(conversation[0].role, "user");
            assert_eq!(conversation[1].role, "assistant");
        } else {
            panic!("Expected WaitingForUserInput state");
        }
    }

    #[test]
    fn test_empty_text_response_prompts_for_input() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        let action = machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![],
            stop_reason: "end_turn".to_string(),
        });

        assert!(matches!(action, AgentAction::PromptForInput));
        assert_eq!(machine.state().name(), "WaitingForUserInput");
    }

    #[test]
    fn test_exponential_backoff_delay() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Hello".to_string()));

        // First error: delay = 1000 * 1 = 1000
        let action = machine.handle_event(AgentEvent::LlmError("error".to_string()));
        assert!(matches!(
            action,
            AgentAction::ScheduleRetry { delay_ms: 1000 }
        ));

        machine.handle_event(AgentEvent::RetryTimeout);

        // Second error: delay = 1000 * 2 = 2000
        let action = machine.handle_event(AgentEvent::LlmError("error".to_string()));
        assert!(matches!(
            action,
            AgentAction::ScheduleRetry { delay_ms: 2000 }
        ));

        machine.handle_event(AgentEvent::RetryTimeout);

        // Third error: delay = 1000 * 3 = 3000
        let action = machine.handle_event(AgentEvent::LlmError("error".to_string()));
        assert!(matches!(
            action,
            AgentAction::ScheduleRetry { delay_ms: 3000 }
        ));
    }

    #[test]
    fn test_post_tools_hook_with_warning() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run a command".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "echo hello"}),
            }],
            stop_reason: "tool_use".to_string(),
        });
        machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_1".to_string(),
            result: Ok("hello".to_string()),
        });

        // HooksCompleted with warning
        let action = machine.handle_event(AgentEvent::HooksCompleted {
            proceed: true,
            warning: Some("Context at 65%".to_string()),
        });

        // Should return DisplayWarning (caller handles it then sends LLM request)
        assert!(matches!(action, AgentAction::DisplayWarning(ref w) if w == "Context at 65%"));
        assert_eq!(machine.state().name(), "CallingLlm");
    }

    #[test]
    fn test_post_tools_hook_stops_on_proceed_false() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run a command".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "echo hello"}),
            }],
            stop_reason: "tool_use".to_string(),
        });
        machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_1".to_string(),
            result: Ok("hello".to_string()),
        });

        // HooksCompleted with proceed=false (stop the loop)
        let action = machine.handle_event(AgentEvent::HooksCompleted {
            proceed: false,
            warning: None,
        });

        // Should return to WaitingForUserInput
        assert!(matches!(action, AgentAction::PromptForInput));
        assert_eq!(machine.state().name(), "WaitingForUserInput");
    }

    #[test]
    fn test_post_tools_hook_stops_with_warning() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run a command".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "echo hello"}),
            }],
            stop_reason: "tool_use".to_string(),
        });
        machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_1".to_string(),
            result: Ok("hello".to_string()),
        });

        // HooksCompleted with proceed=false and warning
        let action = machine.handle_event(AgentEvent::HooksCompleted {
            proceed: false,
            warning: Some("Critical context level".to_string()),
        });

        // Should return DisplayWarning and go to WaitingForUserInput
        assert!(
            matches!(action, AgentAction::DisplayWarning(ref w) if w == "Critical context level")
        );
        assert_eq!(machine.state().name(), "WaitingForUserInput");
    }

    #[test]
    fn test_shutdown_from_post_tools_hook() {
        let mut machine = StateMachine::new();
        machine.handle_event(AgentEvent::UserInput("Run".to_string()));
        machine.handle_event(AgentEvent::LlmCompleted {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".to_string(),
                name: "bash".to_string(),
                input: json!({}),
            }],
            stop_reason: "tool_use".to_string(),
        });
        machine.handle_event(AgentEvent::ToolCompleted {
            call_id: "tool_1".to_string(),
            result: Ok("done".to_string()),
        });

        // Now in PostToolsHook state
        assert_eq!(machine.state().name(), "PostToolsHook");

        let action = machine.handle_event(AgentEvent::ShutdownRequested);

        assert!(matches!(action, AgentAction::Shutdown));
        assert_eq!(machine.state().name(), "ShuttingDown");
    }
}

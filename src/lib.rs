use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};

// ============================================================================
// API Types - Request/Response structures for Anthropic Messages API
// ============================================================================

#[derive(Debug, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// ============================================================================
// Agent Types - Tool definitions and agent struct
// ============================================================================

pub type ToolFunction = fn(Value) -> Result<String, String>;

pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub function: ToolFunction,
}

pub struct Agent {
    api_key: String,
    tools: Vec<ToolDefinition>,
    verbose: bool,
}

impl Agent {
    pub fn new(api_key: String, tools: Vec<ToolDefinition>, verbose: bool) -> Self {
        Agent {
            api_key,
            tools,
            verbose,
        }
    }

    /// Main event loop - reads user input, calls API, handles tool use
    pub fn run(&self) -> Result<(), String> {
        let mut conversation: Vec<Message> = Vec::new();
        let stdin = io::stdin();
        let mut reader = stdin.lock();

        if self.verbose {
            eprintln!("[verbose] Starting chat session");
        }
        println!("Chat with Claude (use 'ctrl-c' to quit)");

        loop {
            print!("\x1b[94mYou\x1b[0m: ");
            io::stdout().flush().ok();

            let mut input = String::new();
            match reader.read_line(&mut input) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(_) => break,
            }

            let input = input.trim();
            if input.is_empty() {
                if self.verbose {
                    eprintln!("[verbose] Skipping empty message");
                }
                continue;
            }

            if self.verbose {
                eprintln!("[verbose] User input: {:?}", input);
            }

            // Add user message to conversation
            conversation.push(Message {
                role: "user".to_string(),
                content: vec![ContentBlock::Text {
                    text: input.to_string(),
                }],
            });

            // Call API
            let response = self.call_api(&conversation)?;
            conversation.push(Message {
                role: "assistant".to_string(),
                content: response.content.clone(),
            });

            // Process response and handle tool use loop
            let mut current_response = response;
            loop {
                let mut tool_results: Vec<ContentBlock> = Vec::new();
                let mut has_tool_use = false;

                for block in &current_response.content {
                    match block {
                        ContentBlock::Text { text } => {
                            println!("\x1b[93mClaude\x1b[0m: {}", text);
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            has_tool_use = true;
                            if self.verbose {
                                eprintln!("[verbose] Tool use: {} with input: {}", name, input);
                            }
                            println!("\x1b[96mtool\x1b[0m: {}({})", name, input);

                            // Find and execute tool
                            let result = self.execute_tool(name, input.clone());
                            match &result {
                                Ok(output) => println!("\x1b[92mresult\x1b[0m: {}", output),
                                Err(err) => println!("\x1b[91merror\x1b[0m: {}", err),
                            }

                            tool_results.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: result.clone().unwrap_or_else(|e| e),
                                is_error: if result.is_err() { Some(true) } else { None },
                            });
                        }
                        _ => {}
                    }
                }

                if !has_tool_use {
                    break;
                }

                // Send tool results back
                conversation.push(Message {
                    role: "user".to_string(),
                    content: tool_results,
                });

                current_response = self.call_api(&conversation)?;
                conversation.push(Message {
                    role: "assistant".to_string(),
                    content: current_response.content.clone(),
                });
            }
        }

        if self.verbose {
            eprintln!("[verbose] Chat session ended");
        }
        Ok(())
    }

    fn execute_tool(&self, name: &str, input: Value) -> Result<String, String> {
        for tool in &self.tools {
            if tool.name == name {
                return (tool.function)(input);
            }
        }
        Err(format!("tool '{}' not found", name))
    }

    fn call_api(&self, conversation: &[Message]) -> Result<MessageResponse, String> {
        let tools: Vec<Tool> = self
            .tools
            .iter()
            .map(|t| Tool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect();

        let request = MessageRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            messages: conversation.to_vec(),
            tools,
        };

        if self.verbose {
            eprintln!(
                "[verbose] API request: {}",
                serde_json::to_string_pretty(&request).unwrap_or_default()
            );
        }

        let response = ureq::post("https://api.anthropic.com/v1/messages")
            .set("Content-Type", "application/json")
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(&request)
            .map_err(|e| format!("API request failed: {}", e))?;

        let msg_response: MessageResponse = response
            .into_json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if self.verbose {
            eprintln!(
                "[verbose] API response: {:?}",
                msg_response.stop_reason
            );
        }

        Ok(msg_response)
    }
}

// ============================================================================
// Helper - Generate JSON Schema from a struct
// ============================================================================

pub fn generate_schema<T: JsonSchema>() -> Value {
    let schema = schema_for!(T);
    serde_json::to_value(schema).unwrap_or(Value::Null)
}

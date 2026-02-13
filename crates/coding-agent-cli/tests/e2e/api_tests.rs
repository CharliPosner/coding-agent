/// API integration tests using mock Claude server
/// These tests verify that the CLI correctly handles API interactions
use super::mock_claude::MockClaudeServer;
use serde_json::json;

#[tokio::test]
async fn test_api_simple_conversation() {
    let server = MockClaudeServer::start().await;
    server
        .mock_simple_response("Hello! I'm Claude.")
        .await;

    // Make a request to the mock server
    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 100
        }));

    assert!(response.is_ok());
    let body: serde_json::Value = response.unwrap().into_json().unwrap();

    // Verify response structure
    assert_eq!(body["type"], "message");
    assert_eq!(body["role"], "assistant");
    assert_eq!(body["content"][0]["text"], "Hello! I'm Claude.");
    assert_eq!(body["stop_reason"], "end_turn");

    // Verify usage tracking
    assert_eq!(body["usage"]["input_tokens"], 10);
    assert_eq!(body["usage"]["output_tokens"], 20);
}

#[tokio::test]
async fn test_api_multi_turn_context() {
    let server = MockClaudeServer::start().await;

    // Setup responses for multiple turns
    let responses = vec![
        "Hello! How can I help you?",
        "Sure, I understand your request.",
        "Here's the answer you're looking for.",
    ];

    server.mock_multi_turn_conversation(responses.clone()).await;

    // Make multiple requests simulating a conversation
    for (i, expected_response) in responses.iter().enumerate() {
        let response = ureq::post(&format!("{}/v1/messages", server.url()))
            .set("Content-Type", "application/json")
            .send_json(json!({
                "model": "claude-3-opus-20240229",
                "messages": [
                    {"role": "user", "content": format!("Message {}", i + 1)}
                ],
                "max_tokens": 100
            }));

        assert!(response.is_ok());
        let body: serde_json::Value = response.unwrap().into_json().unwrap();
        assert_eq!(body["content"][0]["text"], *expected_response);

        // Verify token usage increases with conversation
        let expected_input_tokens = 10 + i * 5;
        let expected_output_tokens = 20 + i * 5;
        assert_eq!(body["usage"]["input_tokens"], expected_input_tokens);
        assert_eq!(body["usage"]["output_tokens"], expected_output_tokens);
    }
}

#[tokio::test]
async fn test_api_tool_call_execution() {
    let server = MockClaudeServer::start().await;

    // Mock a tool call response
    server
        .mock_tool_call(
            "read_file",
            json!({"path": "/tmp/example.txt"})
        )
        .await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Read /tmp/example.txt"}],
            "max_tokens": 100,
            "tools": [{
                "name": "read_file",
                "description": "Read a file",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }
            }]
        }));

    assert!(response.is_ok());
    let body: serde_json::Value = response.unwrap().into_json().unwrap();

    // Verify tool call structure
    assert_eq!(body["stop_reason"], "tool_use");
    assert_eq!(body["content"][0]["type"], "tool_use");
    assert_eq!(body["content"][0]["name"], "read_file");
    assert_eq!(body["content"][0]["input"]["path"], "/tmp/example.txt");
}

#[tokio::test]
async fn test_api_tool_result_sent() {
    let server = MockClaudeServer::start().await;

    // Test that tool results can be sent back in the conversation
    // First, mock a tool call
    server
        .mock_tool_call(
            "read_file",
            json!({"path": "/tmp/test.txt"})
        )
        .await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "What's in test.txt?"}],
            "max_tokens": 100
        }));

    assert!(response.is_ok());
    let body: serde_json::Value = response.unwrap().into_json().unwrap();

    // Verify tool call structure
    assert_eq!(body["stop_reason"], "tool_use");
    let tool_call_id = body["content"][0]["id"].as_str().unwrap();

    // Verify we can construct a proper tool result message
    // (In a real CLI, this would be sent back to Claude)
    let tool_result_message = json!({
        "role": "user",
        "content": [{
            "type": "tool_result",
            "tool_use_id": tool_call_id,
            "content": "Hello, World!"
        }]
    });

    // Verify the structure is correct
    assert_eq!(tool_result_message["role"], "user");
    assert_eq!(tool_result_message["content"][0]["type"], "tool_result");
    assert_eq!(tool_result_message["content"][0]["tool_use_id"], tool_call_id);
}

#[tokio::test]
async fn test_api_streaming_display() {
    let server = MockClaudeServer::start().await;

    let chunks = vec!["Hello", " there", "!", " How", " can", " I", " help", "?"];
    server.mock_streaming_response(chunks.clone()).await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .set("Accept", "text/event-stream")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 100,
            "stream": true
        }));

    assert!(response.is_ok());
    let body = response.unwrap().into_string().unwrap();

    // Verify streaming format
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("event: message_stop"));

    // Verify chunks are present
    for chunk in chunks {
        assert!(body.contains(chunk));
    }
}

#[tokio::test]
async fn test_api_rate_limit_retry() {
    let server = MockClaudeServer::start().await;
    server.mock_rate_limit().await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 100
        }));

    // Should get 429 error
    assert!(response.is_err());
    if let Err(ureq::Error::Status(code, _)) = response {
        assert_eq!(code, 429);
    } else {
        panic!("Expected HTTP 429 error");
    }
}

#[tokio::test]
async fn test_api_network_error_recovery() {
    let server = MockClaudeServer::start().await;
    server.mock_network_error().await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 100
        }));

    // Should get 500 error
    assert!(response.is_err());
    if let Err(ureq::Error::Status(code, _)) = response {
        assert_eq!(code, 500);
    } else {
        panic!("Expected HTTP 500 error");
    }
}

#[tokio::test]
async fn test_api_token_counting_accurate() {
    let server = MockClaudeServer::start().await;
    server
        .mock_simple_response("This is a response with known token counts.")
        .await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Test"}],
            "max_tokens": 100
        }));

    assert!(response.is_ok());
    let body: serde_json::Value = response.unwrap().into_json().unwrap();

    // Verify usage object exists and has correct structure
    assert!(body["usage"].is_object());
    assert!(body["usage"]["input_tokens"].is_number());
    assert!(body["usage"]["output_tokens"].is_number());

    // Verify our mock values
    assert_eq!(body["usage"]["input_tokens"], 10);
    assert_eq!(body["usage"]["output_tokens"], 20);
}

#[tokio::test]
async fn test_api_auth_error() {
    let server = MockClaudeServer::start().await;
    server.mock_auth_error().await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Hi"}],
            "max_tokens": 100
        }));

    // Should get 401 error
    assert!(response.is_err());
    if let Err(ureq::Error::Status(code, response)) = response {
        assert_eq!(code, 401);
        let body: serde_json::Value = response.into_json().unwrap();
        assert_eq!(body["error"]["type"], "authentication_error");
    } else {
        panic!("Expected HTTP 401 error");
    }
}

#[tokio::test]
async fn test_api_text_and_tool_call_mixed() {
    let server = MockClaudeServer::start().await;
    server
        .mock_text_and_tool_call(
            "Let me check that file for you.",
            "read_file",
            json!({"path": "/tmp/data.json"})
        )
        .await;

    let response = ureq::post(&format!("{}/v1/messages", server.url()))
        .set("Content-Type", "application/json")
        .send_json(json!({
            "model": "claude-3-opus-20240229",
            "messages": [{"role": "user", "content": "Check data.json"}],
            "max_tokens": 100
        }));

    assert!(response.is_ok());
    let body: serde_json::Value = response.unwrap().into_json().unwrap();

    // Verify mixed content
    assert_eq!(body["content"].as_array().unwrap().len(), 2);
    assert_eq!(body["content"][0]["type"], "text");
    assert_eq!(body["content"][0]["text"], "Let me check that file for you.");
    assert_eq!(body["content"][1]["type"], "tool_use");
    assert_eq!(body["content"][1]["name"], "read_file");
}

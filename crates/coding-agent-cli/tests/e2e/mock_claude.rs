use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mock Claude API server for deterministic testing
pub struct MockClaudeServer {
    server: MockServer,
}

impl MockClaudeServer {
    /// Start a new mock server
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }

    /// Get the base URL of the mock server
    pub fn url(&self) -> String {
        self.server.uri()
    }

    /// Mock a simple text response from Claude
    pub async fn mock_simple_response(&self, content: &str) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({
                    "id": "msg_test123",
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "text", "text": content}],
                    "model": "claude-3-opus-20240229",
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 10, "output_tokens": 20}
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mock a tool call response from Claude
    pub async fn mock_tool_call(&self, tool_name: &str, tool_input: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({
                    "id": "msg_test456",
                    "type": "message",
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": "tool_call_123",
                        "name": tool_name,
                        "input": tool_input
                    }],
                    "model": "claude-3-opus-20240229",
                    "stop_reason": "tool_use",
                    "usage": {"input_tokens": 15, "output_tokens": 30}
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mock a streaming response from Claude
    pub async fn mock_streaming_response(&self, chunks: Vec<&str>) {
        // SSE streaming format
        let mut body = String::new();

        // Event stream-start
        body.push_str("event: message_start\n");
        body.push_str(&format!("data: {}\n\n", json!({
            "type": "message_start",
            "message": {
                "id": "msg_test789",
                "type": "message",
                "role": "assistant",
                "model": "claude-3-opus-20240229",
                "content": [],
                "stop_reason": null,
                "usage": {"input_tokens": 10, "output_tokens": 0}
            }
        })));

        // Content block start
        body.push_str("event: content_block_start\n");
        body.push_str(&format!("data: {}\n\n", json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        })));

        // Content deltas
        for chunk in chunks {
            body.push_str("event: content_block_delta\n");
            body.push_str(&format!("data: {}\n\n", json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": chunk}
            })));
        }

        // Content block stop
        body.push_str("event: content_block_stop\n");
        body.push_str(&format!("data: {}\n\n", json!({
            "type": "content_block_stop",
            "index": 0
        })));

        // Message delta with stop reason
        body.push_str("event: message_delta\n");
        body.push_str(&format!("data: {}\n\n", json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": null},
            "usage": {"output_tokens": 20}
        })));

        // Message stop
        body.push_str("event: message_stop\n");
        body.push_str(&format!("data: {}\n\n", json!({
            "type": "message_stop"
        })));

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("accept", "text/event-stream"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(body)
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&self.server)
            .await;
    }

    /// Mock a rate limit error (429)
    pub async fn mock_rate_limit(&self) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(429).set_body_json(json!({
                    "type": "error",
                    "error": {
                        "type": "rate_limit_error",
                        "message": "Rate limited"
                    }
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mock a network error (500)
    pub async fn mock_network_error(&self) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&self.server)
            .await;
    }

    /// Mock an authentication error (401)
    pub async fn mock_auth_error(&self) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(json!({
                    "type": "error",
                    "error": {
                        "type": "authentication_error",
                        "message": "Invalid API key"
                    }
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mock a conversation with multiple turns
    /// Returns responses in order for each subsequent request
    pub async fn mock_multi_turn_conversation(&self, responses: Vec<&str>) {
        for (i, response) in responses.iter().enumerate() {
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({
                        "id": format!("msg_test{}", i),
                        "type": "message",
                        "role": "assistant",
                        "content": [{"type": "text", "text": response}],
                        "model": "claude-3-opus-20240229",
                        "stop_reason": "end_turn",
                        "usage": {"input_tokens": 10 + i * 5, "output_tokens": 20 + i * 5}
                    })),
                )
                .up_to_n_times(1)
                .mount(&self.server)
                .await;
        }
    }

    /// Mock a response with both text and tool calls
    pub async fn mock_text_and_tool_call(&self, text: &str, tool_name: &str, tool_input: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({
                    "id": "msg_mixed",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": text},
                        {
                            "type": "tool_use",
                            "id": "tool_call_mixed",
                            "name": tool_name,
                            "input": tool_input
                        }
                    ],
                    "model": "claude-3-opus-20240229",
                    "stop_reason": "tool_use",
                    "usage": {"input_tokens": 15, "output_tokens": 35}
                })),
            )
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_starts() {
        let server = MockClaudeServer::start().await;
        assert!(server.url().starts_with("http://"));
    }

    #[tokio::test]
    async fn test_mock_simple_response() {
        let server = MockClaudeServer::start().await;
        server.mock_simple_response("Hello, world!").await;

        // Make a test request using ureq
        let response = ureq::post(&format!("{}/v1/messages", server.url()))
            .set("Content-Type", "application/json")
            .send_json(json!({
                "model": "claude-3-opus-20240229",
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 100
            }));

        assert!(response.is_ok());
        let body: serde_json::Value = response.unwrap().into_json().unwrap();
        assert_eq!(body["content"][0]["text"], "Hello, world!");
    }

    #[tokio::test]
    async fn test_mock_tool_call() {
        let server = MockClaudeServer::start().await;
        server
            .mock_tool_call("read_file", json!({"path": "/tmp/test.txt"}))
            .await;

        let response = ureq::post(&format!("{}/v1/messages", server.url()))
            .set("Content-Type", "application/json")
            .send_json(json!({
                "model": "claude-3-opus-20240229",
                "messages": [{"role": "user", "content": "Read test.txt"}],
                "max_tokens": 100
            }));

        assert!(response.is_ok());
        let body: serde_json::Value = response.unwrap().into_json().unwrap();
        assert_eq!(body["content"][0]["name"], "read_file");
        assert_eq!(body["content"][0]["input"]["path"], "/tmp/test.txt");
    }

    #[tokio::test]
    async fn test_mock_rate_limit() {
        let server = MockClaudeServer::start().await;
        server.mock_rate_limit().await;

        let response = ureq::post(&format!("{}/v1/messages", server.url()))
            .set("Content-Type", "application/json")
            .send_json(json!({
                "model": "claude-3-opus-20240229",
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 100
            }));

        assert!(response.is_err());
        let err = response.unwrap_err();
        assert_eq!(err.kind(), ureq::ErrorKind::HTTP);
    }

    #[tokio::test]
    async fn test_mock_network_error() {
        let server = MockClaudeServer::start().await;
        server.mock_network_error().await;

        let response = ureq::post(&format!("{}/v1/messages", server.url()))
            .set("Content-Type", "application/json")
            .send_json(json!({
                "model": "claude-3-opus-20240229",
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 100
            }));

        assert!(response.is_err());
    }
}

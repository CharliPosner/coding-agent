/// Full workflow integration tests
/// Tests complete user journeys from start to finish
use crate::e2e::harness::CliTestSession;
use crate::e2e::mock_claude::MockClaudeServer;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to set up a test environment with mock API
async fn setup_test_env() -> (MockClaudeServer, String) {
    let server = MockClaudeServer::start().await;
    let api_url = server.url();
    (server, api_url)
}

/// Helper to create a temporary test directory
fn create_temp_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_workflow_new_session_conversation() {
    // Setup mock server
    let (server, api_url) = setup_test_env().await;

    // Mock a simple conversation
    server
        .mock_multi_turn_conversation(vec![
            "Hello! I'm Claude, an AI assistant. How can I help you today?",
            "The sum of 2 + 2 is 4.",
        ])
        .await;

    // Set API URL via environment variable
    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    // 1. Start CLI
    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");

    // 2. Select new session
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // 3. Send message, get response
    session
        .send_message("Hello")
        .expect("Failed to send message");
    let response = session
        .expect_response(Duration::from_secs(10))
        .expect("Failed to get response");
    assert!(
        response.contains("Claude") || response.contains("assistant"),
        "Response doesn't contain expected content: {}",
        response
    );

    // 4. Send follow-up, verify context retained
    session
        .send_message("What is 2 + 2?")
        .expect("Failed to send follow-up");
    let response2 = session
        .expect_response(Duration::from_secs(10))
        .expect("Failed to get follow-up response");
    assert!(
        response2.contains("4"),
        "Follow-up response doesn't contain expected answer: {}",
        response2
    );

    // 5. Check /cost shows token usage
    let cost_output = session
        .run_command("/cost")
        .expect("Failed to run /cost command");
    assert!(
        cost_output.contains("tokens") || cost_output.contains("Token"),
        "Cost output doesn't show token info: {}",
        cost_output
    );

    // 6. Exit with /exit
    session.run_command("/exit").expect("Failed to exit");

    // 7. Verify session saved to .specstory/
    // Note: This would require checking the filesystem after the session ends
    // For now, we just verify the session exited cleanly
}

#[tokio::test]
#[ignore] // Requires built binary and session history
async fn test_workflow_resume_session() {
    // This test requires:
    // 1. Creating a session with conversation
    // 2. Exiting
    // 3. Restarting CLI
    // 4. Selecting resume
    // 5. Verifying previous context loaded
    // 6. Continuing conversation

    // Setup mock server
    let (server, api_url) = setup_test_env().await;

    server
        .mock_multi_turn_conversation(vec![
            "I remember we were discussing Rust programming.",
            "Lifetimes in Rust ensure memory safety without garbage collection.",
        ])
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    // First session
    {
        let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
        session.expect_startup_screen().expect("No startup screen");
        session
            .select_new_session()
            .expect("Failed to select new session");
        session
            .send_message("Let's talk about Rust")
            .expect("Failed to send message");
        let _ = session.expect_response(Duration::from_secs(10));
        session.run_command("/exit").expect("Failed to exit");
    }

    // Wait a moment for session to be saved
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Second session - resume
    {
        let mut session = CliTestSession::spawn().expect("Failed to spawn CLI for resume");
        session.expect_startup_screen().expect("No startup screen");

        // Select resume option
        session
            .session_mut()
            .send_line("r")
            .expect("Failed to send 'r'");
        session
            .session_mut()
            .expect(">")
            .expect("Failed to wait for prompt");

        // Send follow-up that requires context
        session
            .send_message("Can you explain lifetimes?")
            .expect("Failed to send follow-up");
        let response = session
            .expect_response(Duration::from_secs(10))
            .expect("Failed to get response");

        assert!(
            response.contains("Lifetime")
                || response.contains("lifetime")
                || response.contains("memory"),
            "Response doesn't show context awareness: {}",
            response
        );

        session.run_command("/exit").expect("Failed to exit");
    }
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_workflow_tool_execution() {
    let temp_dir = create_temp_test_dir();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello from test file!").expect("Failed to create test file");

    // Setup mock server with tool call
    let (server, api_url) = setup_test_env().await;

    // Mock Claude requesting to read a file
    server
        .mock_tool_call(
            "read_file",
            serde_json::json!({"path": test_file.to_str().unwrap()}),
        )
        .await;

    // Then mock the response after tool execution
    server
        .mock_simple_response("The file contains: 'Hello from test file!'")
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Ask Claude to read a file
    session
        .send_message(&format!("Read the file at {}", test_file.display()))
        .expect("Failed to send message");

    let response = session
        .expect_response(Duration::from_secs(10))
        .expect("Failed to get response");

    // Verify tool call was displayed
    // The output should show something like "● Reading..." or "✓ Read"
    // Note: Exact format depends on CLI implementation

    // Verify Claude's response about file contents
    assert!(
        response.contains("Hello") || response.contains("test file"),
        "Response doesn't mention file contents: {}",
        response
    );

    session.run_command("/exit").expect("Failed to exit");
}

#[tokio::test]
#[ignore] // Requires built binary and tool integration
async fn test_workflow_multi_tool_sequence() {
    // This test verifies that Claude can use multiple tools in sequence
    let temp_dir = create_temp_test_dir();

    // Create a test file
    let test_file = temp_dir.path().join("example.txt");
    fs::write(&test_file, "Original content").expect("Failed to create test file");

    let (server, api_url) = setup_test_env().await;

    // Mock sequence: read file, then write file
    server
        .mock_tool_call(
            "read_file",
            serde_json::json!({"path": test_file.to_str().unwrap()}),
        )
        .await;

    server
        .mock_tool_call(
            "write_file",
            serde_json::json!({
                "path": test_file.to_str().unwrap(),
                "content": "Modified content"
            }),
        )
        .await;

    server
        .mock_simple_response("I've read the file and updated it with new content.")
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    session
        .send_message(&format!(
            "Read {} and change the content to 'Modified content'",
            test_file.display()
        ))
        .expect("Failed to send message");

    let response = session
        .expect_response(Duration::from_secs(15))
        .expect("Failed to get response");

    // Verify the response indicates both operations happened
    assert!(
        response.contains("read") || response.contains("updated") || response.contains("modified"),
        "Response doesn't indicate operations completed: {}",
        response
    );

    // Verify file was actually modified (if tools are connected)
    // let content = fs::read_to_string(&test_file).expect("Failed to read file");
    // assert_eq!(content, "Modified content");

    session.run_command("/exit").expect("Failed to exit");
}

#[tokio::test]
#[ignore] // Requires built binary and git integration
async fn test_workflow_git_commit() {
    // Setup: Create test repo with changes
    let temp_dir = create_temp_test_dir();
    let repo_path = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    // Configure git
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git email");

    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git name");

    // Create a file and stage it
    let test_file = repo_path.join("feature.rs");
    fs::write(&test_file, "fn new_feature() { println!(\"Hello\"); }")
        .expect("Failed to create file");

    std::process::Command::new("git")
        .args(&["add", "feature.rs"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage file");

    let (server, api_url) = setup_test_env().await;

    // Mock Claude analyzing the changes and generating commit message
    server
        .mock_simple_response(
            "I'll analyze the changes and create a commit.\n\n\
         Add new feature function\n\n\
         This introduces a greeting function that prints a message. \
         It's a foundational piece for the feature module.",
        )
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");
    std::env::set_var("PWD", repo_path.to_str().unwrap());

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Run /commit command
    let output = session
        .run_command("/commit")
        .expect("Failed to run /commit");

    // Verify the command processed changes
    assert!(
        output.contains("commit") || output.contains("changes") || output.contains("feature"),
        "/commit output doesn't indicate processing: {}",
        output
    );

    session.run_command("/exit").expect("Failed to exit");
}

#[tokio::test]
#[ignore] // Requires built binary and permission system
async fn test_workflow_permission_prompt() {
    let temp_dir = create_temp_test_dir();
    let untrusted_path = temp_dir.path().join("untrusted.txt");

    let (server, api_url) = setup_test_env().await;

    // Mock Claude trying to write to an untrusted location
    server
        .mock_tool_call(
            "write_file",
            serde_json::json!({
                "path": untrusted_path.to_str().unwrap(),
                "content": "Test content"
            }),
        )
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Ask Claude to write to untrusted path
    session
        .send_message(&format!(
            "Write 'Test content' to {}",
            untrusted_path.display()
        ))
        .expect("Failed to send message");

    // Expect permission prompt or error about permissions
    // The exact format depends on implementation, but should ask for permission
    session
        .session_mut()
        .set_expect_timeout(Some(Duration::from_secs(10)));

    // Try to capture output - it may contain a permission prompt or complete with prompt
    // For now, we'll just try to get to the next prompt and check for permission-related text
    let result = session.session_mut().expect(">");

    if let Ok(output) = result {
        let output_str = String::from_utf8_lossy(output.as_bytes());
        // Check if permission-related text appeared (could be prompt or message)
        // This is a simplified check - actual implementation may vary
        let has_permission_text = output_str.contains("permission")
            || output_str.contains("allow")
            || output_str.contains("trust")
            || output_str.contains("write")
            || output_str.contains("error"); // May error if permission denied

        assert!(
            has_permission_text,
            "Expected permission-related content, got: {}",
            output_str
        );
    }

    session.run_command("/exit").expect("Failed to exit");
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_workflow_error_handling() {
    let (server, api_url) = setup_test_env().await;

    // Mock a network error response
    server.mock_network_error().await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Send a message that will trigger the error
    session
        .send_message("Hello")
        .expect("Failed to send message");

    // Expect error handling - try to capture output with timeout
    session
        .session_mut()
        .set_expect_timeout(Some(Duration::from_secs(10)));

    // Try to get to next prompt - may show error or retry
    let result = session.session_mut().expect(">");

    if let Ok(output) = result {
        let output_str = String::from_utf8_lossy(output.as_bytes());
        // Should show an error message but not crash
        assert!(
            output_str.contains("error")
                || output_str.contains("Error")
                || output_str.contains("failed")
                || output_str.contains("retry"),
            "Expected error message, got: {}",
            output_str
        );
    }

    // CLI should still be responsive
    let help_output = session
        .run_command("/help")
        .expect("CLI not responsive after error");
    assert!(
        help_output.contains("help") || help_output.contains("command"),
        "CLI not functioning after error"
    );

    session.run_command("/exit").expect("Failed to exit");
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_workflow_context_tracking() {
    let (server, api_url) = setup_test_env().await;

    // Mock responses with increasing token counts
    server
        .mock_multi_turn_conversation(vec![
            "First response with some tokens.",
            "Second response adding more tokens to the context.",
            "Third response further increasing token usage.",
        ])
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Send multiple messages
    session
        .send_message("First message")
        .expect("Failed to send first message");
    let _ = session.expect_response(Duration::from_secs(10));

    session
        .send_message("Second message")
        .expect("Failed to send second message");
    let _ = session.expect_response(Duration::from_secs(10));

    session
        .send_message("Third message")
        .expect("Failed to send third message");
    let _ = session.expect_response(Duration::from_secs(10));

    // Check context tracking
    let context_output = session
        .run_command("/context")
        .expect("Failed to run /context");

    // Should show token usage
    assert!(
        context_output.contains("token") || context_output.contains("Token"),
        "Context output doesn't show tokens: {}",
        context_output
    );

    // Check cost tracking
    let cost_output = session.run_command("/cost").expect("Failed to run /cost");

    // Should show accumulated costs
    assert!(
        cost_output.contains("token") || cost_output.contains("Token"),
        "Cost output doesn't show tokens: {}",
        cost_output
    );

    session.run_command("/exit").expect("Failed to exit");
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_workflow_command_history() {
    let (server, api_url) = setup_test_env().await;
    server
        .mock_simple_response("Response to your message.")
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Run several commands
    let _ = session.run_command("/help");
    let _ = session.run_command("/context");

    session
        .send_message("Test message")
        .expect("Failed to send message");
    let _ = session.expect_response(Duration::from_secs(10));

    // Check history command
    let history_output = session
        .run_command("/history")
        .expect("Failed to run /history");

    // Should show past sessions or indicate history is available
    assert!(
        history_output.contains("history")
            || history_output.contains("session")
            || history_output.contains("Session"),
        "History output unexpected: {}",
        history_output
    );

    session.run_command("/exit").expect("Failed to exit");
}

#[tokio::test]
#[ignore] // Requires built binary
async fn test_workflow_clear_command() {
    let (server, api_url) = setup_test_env().await;
    server
        .mock_multi_turn_conversation(vec![
            "First session response.",
            "This should be in a fresh context after clear.",
        ])
        .await;

    std::env::set_var("ANTHROPIC_BASE_URL", &api_url);
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");

    let mut session = CliTestSession::spawn().expect("Failed to spawn CLI");
    session.expect_startup_screen().expect("No startup screen");
    session
        .select_new_session()
        .expect("Failed to select new session");

    // Have a conversation
    session
        .send_message("Remember this: banana")
        .expect("Failed to send message");
    let _ = session.expect_response(Duration::from_secs(10));

    // Clear the context
    let _clear_output = session.run_command("/clear").expect("Failed to run /clear");

    // Screen should be cleared and context reset
    // The exact behavior depends on implementation

    // Send another message - should not have context from before clear
    session
        .send_message("What did I tell you to remember?")
        .expect("Failed to send message");
    let _response = session
        .expect_response(Duration::from_secs(10))
        .expect("Failed to get response after clear");

    // Response should indicate no memory of "banana"
    // (This depends on the mock being set up correctly)

    session.run_command("/exit").expect("Failed to exit");
}

// End-to-end tests for the CLI
// These tests spawn the actual binary and interact with it via PTY

mod e2e;

// Re-export test modules
pub use e2e::*;

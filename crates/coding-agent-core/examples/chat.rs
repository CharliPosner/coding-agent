//! Basic chat example - no tools, just conversation with Claude.
//!
//! Usage:
//!   cargo run --example chat
//!   cargo run --example chat -- --verbose
//!
//! Set ANTHROPIC_API_KEY environment variable before running.

use coding_agent_core::Agent;
use std::env;

fn main() {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Parse command line arguments for --verbose flag
    let args: Vec<String> = env::args().collect();
    let verbose = args.iter().any(|arg| arg == "--verbose" || arg == "-v");

    if verbose {
        eprintln!("[verbose] Verbose logging enabled");
    }

    // Get API key from environment
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: ANTHROPIC_API_KEY environment variable not set");
        std::process::exit(1);
    });

    if verbose {
        eprintln!("[verbose] Anthropic API key found");
    }

    // Create agent with no tools (empty vector)
    let agent = Agent::new(api_key, vec![], verbose);

    if verbose {
        eprintln!("[verbose] Agent initialized with no tools");
    }

    // Run the main chat loop
    if let Err(e) = agent.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

mod agents;
mod cli;
mod config;
mod integrations;
mod permissions;
mod tokens;
mod tools;
mod ui;

use clap::Parser;
use std::process::ExitCode;

/// AI-powered coding assistant
#[derive(Parser, Debug)]
#[command(name = "code")]
#[command(version, about, long_about = None)]
struct Args {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();

    match cli::run(args.verbose).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

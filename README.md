# Build Your Own Coding Agent in Rust

A step-by-step workshop for building an AI-powered coding assistant in Rust. Start from a basic chatbot and progressively add powerful tools like file reading, shell command execution, and code searching.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (1.70 or later) - Install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **ripgrep** - Required for the code search tool:
  ```bash
  # macOS
  brew install ripgrep

  # Ubuntu/Debian
  sudo apt install ripgrep

  # Arch Linux
  sudo pacman -S ripgrep

  # Windows (with Chocolatey)
  choco install ripgrep
  ```

- **Anthropic API Key** - Get one from [Anthropic](https://www.anthropic.com/product/claude)

## Setup

1. Clone the repository and navigate to the project directory:
   ```bash
   cd how-to-build-a-coding-agent-rust
   ```

2. Set your Anthropic API key:
   ```bash
   export ANTHROPIC_API_KEY="your-api-key-here"
   ```

3. Build the project:
   ```bash
   cargo build
   ```

## Running the Examples

Each example builds upon the previous one, adding more capabilities to your coding agent.

### 1. Basic Chat

A simple chatbot that communicates with Claude.

```bash
cargo run --example chat
```

**Try these prompts:**
- "Hello!"
- "What can you help me with?"

### 2. File Reader

Adds the ability for Claude to read files from your filesystem.

```bash
cargo run --example read
```

**Try these prompts:**
- "Read fizzbuzz.js"
- "What's in the Cargo.toml file?"

## Project Structure

```
how-to-build-a-coding-agent-rust/
├── Cargo.toml          # Project dependencies and configuration
├── src/                # Library source code
└── examples/           # Runnable example agents
    ├── chat.rs         # Basic chat agent
    └── read.rs         # Agent with file reading capability
```

## What You'll Learn

- Connect to the Anthropic Claude API using Rust
- Build a simple AI chatbot
- Add tools like reading files, editing code, and running commands
- Handle tool requests and errors
- Build an agent that grows smarter with each iteration

## Architecture

Each agent follows this event loop:

1. Wait for user input
2. Send input to Claude
3. Claude responds directly or requests a tool
4. Agent executes the requested tool
5. Send tool result back to Claude
6. Claude provides the final answer

## Troubleshooting

**API key not working?**
- Verify it's exported: `echo $ANTHROPIC_API_KEY`
- Check your quota on [Anthropic's dashboard](https://console.anthropic.com)

**Build errors?**
- Run `cargo clean && cargo build`
- Ensure Rust is up to date: `rustup update`

**ripgrep not found?**
- Verify installation: `rg --version`
- Ensure it's in your PATH

## License

See LICENSE file for details.

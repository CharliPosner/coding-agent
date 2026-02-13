# Makefile for coding-agent
# Build, run, and test targets

.PHONY: build build-release run-cli run-chat run-read run-list-files run-bash run-edit run-code-search run-test-image test test-verbose clean fmt fmt-check lint check help

# Package shortcuts
CORE = coding-agent-core
CLI = coding-agent-cli

# Default target
all: build

# Build everything
build:
	cargo build --workspace

# Build in release mode
build-release:
	cargo build --workspace --release

# Run CLI
run-cli:
	cargo run -p $(CLI)

# Run core examples
run-chat:
	cargo run -p $(CORE) --example chat

run-read:
	cargo run -p $(CORE) --example read

run-list-files:
	cargo run -p $(CORE) --example list_files

run-bash:
	cargo run -p $(CORE) --example bash

run-edit:
	cargo run -p $(CORE) --example edit

run-code-search:
	cargo run -p $(CORE) --example code_search

run-test-image:
	cargo run -p $(CORE) --example test_gemini_image

# Run all tests
test:
	cargo test --workspace

# Run tests with verbose output
test-verbose:
	cargo test --workspace -- --nocapture

# Clean build artifacts
clean:
	cargo clean

# Format code
fmt:
	cargo fmt --all

# Check formatting without modifying
fmt-check:
	cargo fmt --all -- --check

# Run clippy linter
lint:
	cargo clippy --workspace -- -D warnings

# Check compilation without building
check:
	cargo check --workspace

# Help target
help:
	@echo "Available targets:"
	@echo ""
	@echo "  Build:"
	@echo "    build          - Build all crates"
	@echo "    build-release  - Build all crates in release mode"
	@echo ""
	@echo "  Run:"
	@echo "    run-cli        - Run the CLI application"
	@echo "    run-chat       - Run the chat example"
	@echo "    run-read       - Run the read example"
	@echo "    run-list-files - Run the list_files example"
	@echo "    run-bash       - Run the bash example"
	@echo "    run-edit       - Run the edit example"
	@echo "    run-code-search- Run the code_search example"
	@echo "    run-test-image - Test Gemini image generation"
	@echo ""
	@echo "  Test & Quality:"
	@echo "    test           - Run all tests"
	@echo "    test-verbose   - Run tests with verbose output"
	@echo "    fmt            - Format code with rustfmt"
	@echo "    fmt-check      - Check code formatting"
	@echo "    lint           - Run clippy linter"
	@echo "    check          - Check compilation"
	@echo ""
	@echo "  Other:"
	@echo "    clean          - Remove build artifacts"
	@echo "    help           - Show this help message"

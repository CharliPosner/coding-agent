# Makefile for how-to-build-a-coding-agent-rust
# Build, run, and test targets for all examples

.PHONY: build run-chat run-read run-list-files run-bash run-edit run-code-search test clean help

# Default target
all: build

# Build all examples
build:
	cargo build --examples

# Build in release mode
build-release:
	cargo build --examples --release

# Run individual examples
run-chat:
	cargo run --example chat

run-read:
	cargo run --example read

run-list-files:
	cargo run --example list_files

run-bash:
	cargo run --example bash

run-edit:
	cargo run --example edit

run-code-search:
	cargo run --example code_search

# Run all tests
test:
	cargo test

# Run tests with verbose output
test-verbose:
	cargo test -- --nocapture

# Clean build artifacts
clean:
	cargo clean

# Format code
fmt:
	cargo fmt

# Check formatting without modifying
fmt-check:
	cargo fmt -- --check

# Run clippy linter
lint:
	cargo clippy --examples -- -D warnings

# Check compilation without building
check:
	cargo check --examples

# Help target
help:
	@echo "Available targets:"
	@echo "  build          - Build all examples"
	@echo "  build-release  - Build all examples in release mode"
	@echo "  run-chat       - Run the chat example"
	@echo "  run-read       - Run the read example"
	@echo "  run-list-files - Run the list_files example"
	@echo "  run-bash       - Run the bash example"
	@echo "  run-edit       - Run the edit example"
	@echo "  run-code-search- Run the code_search example"
	@echo "  test           - Run all tests"
	@echo "  test-verbose   - Run all tests with verbose output"
	@echo "  clean          - Remove build artifacts"
	@echo "  fmt            - Format code with rustfmt"
	@echo "  fmt-check      - Check code formatting"
	@echo "  lint           - Run clippy linter"
	@echo "  check          - Check compilation without building"
	@echo "  help           - Show this help message"

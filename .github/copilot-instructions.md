# Copilot Instructions for LSP Bridge

<!-- Use this file to provide workspace-specific custom instructions to Copilot. For more details, visit https://code.visualstudio.com/docs/copilot/copilot-customization#_use-a-githubcopilotinstructionsmd-file -->

## Project Overview

This is an LSP Bridge Rust crate project that provides a comprehensive bridge between Language Server Protocol (LSP) servers and clients. The crate simplifies LSP integration by handling protocol communication, lifecycle management, and feature negotiation.

## Coding Guidelines

- **Async/Await**: Use async/await patterns throughout the codebase with tokio runtime
- **Error Handling**: Use `thiserror` for error types and comprehensive error handling
- **Documentation**: All public APIs must have comprehensive documentation with examples
- **Testing**: Write comprehensive unit and integration tests for all functionality
- **Concurrency**: Use `DashMap` for thread-safe collections and proper async synchronization
- **Logging**: Use `tracing` for structured logging and diagnostics

## Architecture

- `protocol/`: LSP protocol implementation and types
- `server/`: Server lifecycle management and communication
- `client/`: Client-side bridge implementation
- `bridge/`: Main bridge interface coordinating client-server communication
- `config/`: Configuration types and validation
- `error/`: Comprehensive error types
- `utils/`: Shared utilities and helpers

## Key Dependencies

- `lsp-types`: Official LSP protocol types
- `tokio`: Async runtime and utilities
- `serde`: Serialization/deserialization
- `dashmap`: Concurrent hash maps
- `tracing`: Structured logging

## LSP Features to Support

- Document synchronization
- Completions
- Diagnostics
- Go to definition/references
- Hover information
- Document formatting
- Code actions
- Rename refactoring
- Workspace symbols

## Code Style

- Follow standard Rust conventions
- Use meaningful type names and clear module organization
- Prefer composition over inheritance
- Use builder patterns for complex configuration
- Handle all error cases explicitly

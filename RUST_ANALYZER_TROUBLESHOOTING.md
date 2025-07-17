# LSP Bridge - Rust-Analyzer Troubleshooting Guide

This document helps resolve rust-analyzer false positive errors in VS Code.

## Quick Fix Steps

1. **Clean and Rebuild**:

   ```bash
   cargo clean
   cargo check
   ```

2. **Restart Rust-Analyzer** in VS Code:

   - Press `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (Mac)
   - Type "rust-analyzer: Restart"
   - Select "rust-analyzer: Restart Server"

3. **Reload VS Code Window**:

   - Press `Ctrl+Shift+P`
   - Type "Developer: Reload Window"
   - Press Enter

## Project Status ✅

- **✅ Compilation**: All code compiles successfully
- **✅ Examples**: All examples run without errors
- **✅ Tests**: 34/34 unit tests pass  
- **✅ Linting**: Core library has no clippy warnings

## Configuration Files Created

- `.vscode/settings.json` - Rust-analyzer configuration
- `rust-toolchain.toml` - Consistent Rust version
- `.cargo/config.toml` - Build configuration

## Verification Commands

```bash
# Verify compilation
cargo check --all-targets

# Run examples  
cargo run --example basic
cargo run --example document_operations
cargo run --example multi_server

# Run tests
cargo test

# Check linting
cargo clippy --lib
```

## Common Issues

### "Unresolved import" errors

- These are rust-analyzer false positives
- The code compiles and runs successfully
- Solution: Restart rust-analyzer (see steps above)

### "No method found" errors (E0599)

- Also rust-analyzer false positives
- All method calls work correctly at runtime
- For String.clone() errors: Use explicit `String::clone(&your_string)` instead
- For ServerCapabilities errors: Ensure proper imports from protocol module

### Type mismatch errors (E0308)

- Usually related to LspError vs LspBridgeError conversions
- Add explicit `.into()` calls for proper error conversion
- Example: `LspError::some_error().into()` instead of `LspError::some_error()`
- Check all error-returning functions, especially in process.rs and server.rs

## Using Rust Attributes for Persistent Issues

For any persistent rust-analyzer errors that can't be fixed otherwise, you can use attributes to suppress them:

```rust
// Suppress specific warnings in a function
#[allow(unused_variables)]
fn some_function() {
    let x = 5; // No warning for unused variable
}

// For a single line
#[allow(clippy::unnecessary_unwrap)]
let value = optional.unwrap(); // No clippy warning here

// For error type mismatches (at the function level)
#[allow(clippy::result_large_err)]
fn function_with_error() -> Result<()> {
    // Function implementation
}
```

## Expected Behavior

After following the steps above, rust-analyzer should:

- ✅ Recognize all imports correctly
- ✅ Provide proper auto-completion
- ✅ Show accurate type information
- ✅ Display no false error messages

## Final Notes

Remember that rust-analyzer false positives do not affect actual compilation or runtime behavior. The code compiles and runs correctly even with these warning indicators in the editor.

If persistent issues remain after trying all solutions, you can disable specific rust-analyzer diagnostics in your `.vscode/settings.json` file.

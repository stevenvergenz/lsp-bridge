use lsp_bridge::prelude::*;
use std::time::Duration;

#[test]
fn test_server_creation() {
    let config = LspServerConfig::new().command("echo").arg("test");

    let server = LspServer::new("test_server", config);

    // Just test that we can create the server
    assert_eq!(server.id(), "test_server");
}

#[test]
fn test_bridge_creation() {
    let bridge = LspBridge::new();
    assert_eq!(bridge.server_count(), 0);
}

#[test]
fn test_config_validation() {
    // Test that config validates properly
    let config = LspServerConfig::new()
        .command("test_command")
        .startup_timeout(Duration::from_secs(10));

    assert_eq!(config.command, "test_command");
    assert_eq!(config.startup_timeout, Duration::from_secs(10));

    // Test validation
    assert!(config.validate().is_ok());
}

#[test]
fn test_malformed_messages() {
    // Test that the protocol layer handles malformed JSON gracefully
    let malformed_json = r#"{"jsonrpc": "2.0", "id": 1, "method": "test", invalid}"#;
    let result = serde_json::from_str::<LspMessage>(malformed_json);
    assert!(result.is_err());

    // Test incomplete message
    let incomplete_json = r#"{"jsonrpc": "2.0""#;
    let result = serde_json::from_str::<LspMessage>(incomplete_json);
    assert!(result.is_err());
}

#[test]
fn test_error_handling() {
    // Test that errors are created correctly
    let error = LspError::communication("Test error message".to_string());
    match error {
        LspError::Communication { message } => {
            assert_eq!(message, "Test error message");
        }
        _ => panic!("Expected communication error"),
    }
}

#[test]
fn test_config_builder_invalid() {
    // Test that invalid configs are rejected
    let invalid_config = LspServerConfig::new().startup_timeout(Duration::from_secs(30));
    // Should fail because command is empty
    assert!(invalid_config.validate().is_err());

    // Test valid config
    let valid_config = LspServerConfig::new().command("test_command");
    assert!(valid_config.validate().is_ok());
}

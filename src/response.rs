use dashmap::DashMap;
use lsp_types::{LogMessageParams, MessageType, ShowMessageParams};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::LspResponse;

pub trait LspMessageHandler {
    fn handle_notify(&self, method: &str, params: Option<Value>) -> impl Future<Output = Option<()>>;

    fn handle_request(&self, method: &str, params: Option<Value>) -> impl Future<Output = Option<LspResponse>>;
}

pub struct LogMessageHandler;
impl LspMessageHandler for LogMessageHandler {
    async fn handle_notify(&self, method: &str, params: Option<Value>) -> Option<()> {
        let param = match params {
            Some(p) => p,
            _ => return None,
        };

        let (typ, msg) = match method {
            "window/logMessage" => match serde_json::from_value(param) {
                Ok(LogMessageParams { typ, message }) => (typ, message),
                _ => return None,
            },
            "window/showMessage" => match serde_json::from_value(param) {
                Ok(ShowMessageParams { typ, message }) => (typ, message),
                _ => return None,
            },
            _ => return None,
        };

        match typ {
            MessageType::ERROR => error!("LSP Server Message: {msg}"),
            MessageType::WARNING => warn!("LSP Server Message: {msg}"),
            MessageType::INFO
                | MessageType::LOG => info!("LSP Server Message: {msg}"),
            _ => debug!("LSP Server Message: {msg}"),
        };

        Some(())
    }

    async fn handle_request(&self, _method: &str, _params: Option<Value>) -> Option<LspResponse> {
        None
    }
}

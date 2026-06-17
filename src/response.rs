use dashmap::DashMap;
use lsp_types::{ConfigurationParams, LogMessageParams, MessageType, ShowMessageParams};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::{LspResponse, protocol::{LspResponseError, RequestId}};

pub struct LspMessageHandler {
    notif_handlers: DashMap<String, Box<dyn FnMut(&str, Option<Value>) -> Option<()>>>,
    req_handlers: DashMap<String, Box<dyn FnMut(&str, Option<Value>) -> Result<Value, LspResponseError>>>,
}

impl LspMessageHandler {
    pub fn new() -> Self {
        Self {
            notif_handlers: DashMap::new(),
            req_handlers: DashMap::new(),
        }
    }

    pub fn register_notify(&mut self, method: impl Into<String>, handler: impl FnMut(&str, Option<Value>) -> Option<()>) {
        self.notif_handlers.insert(method.into(), Box::new(handler));
    }

    pub fn handle_notify(&self, method: &str, params: Option<Value>) -> Option<()> {
        if let Some(mut h) = self.notif_handlers.get_mut(method) {
            h(method, params)
        } else {
            None
        }
    }

    pub fn register_response(&mut self, method: impl Into<String>, handler: impl FnMut(&str, Option<Value>) -> Result<Value, LspResponseError>) {
        self.req_handlers.insert(method.into(), Box::new(handler));
    }

    pub fn handle_request(&self, method: &str, params: Option<Value>) -> Result<Value, LspResponseError> {
        if let Some(mut h) = self.req_handlers.get_mut(method) {
            h(method, params)
        } else {
            Ok(Value::Null)
        }
    }

    fn handle_log_notif(method: &str, params: Option<Value>) -> Option<()> {
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

    fn handle_config_request(method: &str, params: Option<Value>) -> Result<Value, LspResponseError> {
        if method == "workspace/configuration"
            && let Some(params) = params
            && let Ok(ConfigurationParams { items }) = serde_json::from_value(params) {
            debug!("Server requested {} configuration items", items.len());
                                
            // Create a response with empty configuration for each item
            // In a real implementation, this would pull from client's config
            let _config_items: Vec<Value> = items
                .iter()
                .map(|_| Value::Null)
                .collect();
                                
            // Return empty configuration values as default response
            // The actual handling of this would happen in the LspClient which
            // would need to register handlers for specific request types
            debug!("Responding with default configuration values");
            Ok(Value::Array(_config_items))
        } else {
            Ok(Value::Null)
        }
    }
}

impl Default for LspMessageHandler {
    fn default() -> Self {
        let mut s = Self::new();
        s.register_notify("window/logMessage", Self::handle_log_notif);
        s.register_notify("window/showMessage", Self::handle_log_notif);
        s.register_response("workspace/configuration", Self::handle_config_request);
        s
    }
}

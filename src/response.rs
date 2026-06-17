use dashmap::DashMap;
use lsp_types::{LogMessageParams, MessageType, ShowMessageParams};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::LspResponse;

pub struct LspMessageHandler {
    notif_handlers: DashMap<String, Box<dyn FnMut(&str, Option<&Value>) -> Option<()>>>,
    req_handlers: DashMap<String, Box<dyn FnMut(&str, Option<&Value>) -> Option<LspResponse>>>,
}

impl LspMessageHandler {
    pub fn new() -> Self {
        Self {
            notif_handlers: DashMap::new(),
            req_handlers: DashMap::new(),
        }
    }

    pub fn register_notify(&mut self, method: impl Into<String>, handler: impl FnMut(&str, Option<&Value>) -> Option<()>) {
        self.notif_handlers.insert(method.into(), Box::new(handler));
    }

    pub async fn handle_notify(&self, method: &str, params: Option<&Value>) -> Option<()> {
        if let Some(h) = self.notif_handlers.get(method) {
            h(method, params)
        } else {
            None
        }
    }

    pub async fn handle_request(&self, method: &str, params: Option<&Value>) -> Option<LspResponse> {
        if let Some(h) = self.req_handlers.get(method) {
            h(method, params)
        } else {
            None
        }
    }

    async fn handle_log_notif(&self, method: &str, params: Option<&Value>) -> Option<()> {
        let param = match params {
            Some(p) => p,
            _ => return None,
        };

        let (typ, msg) = match method {
            "window/logMessage" => match serde_json::from_value(param.clone()) {
                Ok(LogMessageParams { typ, message }) => (typ, message),
                _ => return None,
            },
            "window/showMessage" => match serde_json::from_value(param.clone()) {
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
}

impl Default for LspMessageHandler {
    fn default() -> Self {
        let mut s = Self::new();
        s.register_notify("window/logMessage", Self::handle_log_notif);
        s.register_notify("window/showMessage", Self::handle_log_notif);
        s
    }
}

//! Client-side bridge implementation for LSP interactions.

use crate::error::{LspError, Result};
use crate::protocol::DocumentState;
use crate::server::{LspServer, ServerId};
use dashmap::DashMap;
use lsp_types::*;
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Client-side LSP bridge for handling document operations and server interactions.
pub struct LspClient {
    servers: Arc<DashMap<ServerId, Arc<RwLock<LspServer>>>>,
    documents: Arc<DashMap<String, DocumentState>>,
    active_server: Arc<RwLock<Option<ServerId>>>,
}

impl LspClient {
    /// Create a new LSP client.
    pub fn new() -> Self {
        Self {
            servers: Arc::new(DashMap::new()),
            documents: Arc::new(DashMap::new()),
            active_server: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a server with the client.
    pub async fn register_server(&self, server_id: ServerId, server: LspServer) -> Result<()> {
        info!("Registering server: {server_id}");

        let server = Arc::new(RwLock::new(server));
        self.servers.insert(server_id.clone(), server);

        // Set as active server if none is set
        if self.active_server.read().await.is_none() {
            *self.active_server.write().await = Some(server_id);
        }

        Ok(())
    }

    /// Unregister a server from the client.
    pub async fn unregister_server(&self, server_id: &str) -> Result<()> {
        info!("Unregistering server: {server_id}");

        // Stop the server first with timeout
        if let Some((_, server)) = self.servers.remove(server_id) {
            // Check server state before attempting shutdown
            let state = server.read().await.state().await;
            
            // Only attempt graceful shutdown for running servers
            if matches!(state, crate::server::ServerState::Ready | crate::server::ServerState::Starting | crate::server::ServerState::Initializing) {
                // Use shorter timeout to prevent hanging in tests
                let stop_result = tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    server.write().await.stop()
                ).await;
                
                match stop_result {
                    Ok(Ok(())) => debug!("Server {server_id} stopped cleanly"),
                    Ok(Err(e)) => warn!("Error stopping server {server_id}: {e}"),
                    Err(_) => warn!("Timeout stopping server {server_id}, forcing shutdown"),
                }
            } else {
                // Server was never started or already stopped, just clean up
                debug!("Server {server_id} was not running, skipping graceful shutdown");
            }
        }

        // Update active server if needed (avoid deadlock by dropping read lock first)
        let should_clear_active = {
            if let Some(active) = self.active_server.read().await.as_ref() {
                active == server_id
            } else {
                false
            }
        }; // Read lock is dropped here
        
        if should_clear_active {
            *self.active_server.write().await = None;
        }

        Ok(())
    }

    /// Get a server by ID.
    pub fn get_server(&self, server_id: &str) -> Option<Arc<RwLock<LspServer>>> {
        self.servers.get(server_id).map(|entry| entry.clone())
    }

    /// Set the active server.
    pub async fn set_active_server(&self, server_id: Option<ServerId>) -> Result<()> {
        if let Some(id) = &server_id {
            if !self.servers.contains_key(id) {
                return Err(LspError::server_not_found(id).into());
            }
        }

        *self.active_server.write().await = server_id;
        Ok(())
    }

    /// Get the active server ID.
    pub async fn active_server(&self) -> Option<ServerId> {
        self.active_server.read().await.clone()
    }

    /// Get the active server instance.
    pub async fn get_active_server(&self) -> Option<Arc<RwLock<LspServer>>> {
        if let Some(id) = self.active_server().await {
            self.get_server(&id)
        } else {
            None
        }
    }

    /// Open a document in the LSP server.
    pub async fn open_document(
        &self,
        server_id: &str,
        uri: &str,
        content: &str,
        language_id: &str,
    ) -> Result<()> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        // Create document state
        let document = DocumentState::new(uri, 1, content, language_id);
        self.documents.insert(uri.to_string(), document);

        // Send textDocument/didOpen notification
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.parse().map_err(|_| LspError::invalid_uri(uri))?,
                language_id: language_id.to_string(),
                version: 1,
                text: content.to_string(),
            },
        };

        server
            .read()
            .await
            .notify(
                "textDocument/didOpen".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        info!("Opened document: {uri}");
        Ok(())
    }

    /// Close a document in the LSP server.
    pub async fn close_document(&self, server_id: &str, uri: &str) -> Result<()> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        // Remove document state
        self.documents.remove(uri);

        // Send textDocument/didClose notification
        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
            },
        };

        server
            .read()
            .await
            .notify(
                "textDocument/didClose".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        info!("Closed document: {uri}");
        Ok(())
    }

    /// Update document content.
    pub async fn update_document(&self, server_id: &str, uri: &str, content: &str) -> Result<()> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        // Update document state
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.update_content(content);
            let version = doc.version;

            // Send textDocument/didChange notification
            let params = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: content.to_string(),
                }],
            };

            server
                .read()
                .await
                .notify(
                    "textDocument/didChange".to_string(),
                    Some(serde_json::to_value(params)?),
                )
                .await?;

            debug!("Updated document: {uri} (version {version})");
        } else {
            return Err(LspError::invalid_uri(uri).into());
        }

        Ok(())
    }

    /// Request completions for a document position.
    pub async fn get_completions(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Vec<CompletionItem>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        // Check if server supports completion
        let capabilities = server.read().await.capabilities().await;
        if let Some(caps) = capabilities {
            // Use the supports_completion method from the wrapper
            if !caps.supports_completion() {
                return Err(LspError::feature_not_supported("completion", server_id).into());
            }
        }

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/completion".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        // Parse completion response
        match response {
            serde_json::Value::Array(items) => {
                let completions: Result<Vec<CompletionItem>> = items
                    .into_iter()
                    .map(|item| serde_json::from_value(item).map_err(|e| LspError::from(e).into()))
                    .collect();
                completions
            }
            serde_json::Value::Object(_) => {
                // CompletionList format
                if let Some(items) = response.get("items") {
                    let completions: Result<Vec<CompletionItem>> =
                        serde_json::from_value(items.clone()).map_err(|e| LspError::from(e).into());
                    completions
                } else {
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }

    /// Get hover information for a document position.
    pub async fn get_hover(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<Hover>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/hover".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(None)
        } else {
            let hover: Hover = serde_json::from_value(response)?;
            Ok(Some(hover))
        }
    }

    /// Go to definition for a document position.
    pub async fn go_to_definition(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/definition".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(None)
        } else {
            let definition: GotoDefinitionResponse = serde_json::from_value(response)?;
            Ok(Some(definition))
        }
    }

    /// Find references for a document position.
    pub async fn find_references(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/references".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(vec![])
        } else {
            let references: Vec<Location> = serde_json::from_value(response)?;
            Ok(references)
        }
    }

    /// Format a document.
    pub async fn format_document(
        &self,
        server_id: &str,
        uri: &str,
        options: FormattingOptions,
    ) -> Result<Vec<TextEdit>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
            },
            options,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/formatting".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(vec![])
        } else {
            let edits: Vec<TextEdit> = serde_json::from_value(response)?;
            Ok(edits)
        }
    }

    /// Get diagnostics for a document.
    pub fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        if let Some(doc) = self.documents.get(uri) {
            doc.diagnostics.clone()
        } else {
            vec![]
        }
    }

    /// Get document content.
    pub fn get_document_content(&self, uri: &str) -> Option<String> {
        self.documents.get(uri).map(|doc| doc.content.clone())
    }

    /// Get document state.
    pub fn get_document_state(&self, uri: &str) -> Option<DocumentState> {
        self.documents.get(uri).map(|doc| doc.clone())
    }

    /// List all open documents.
    pub fn list_documents(&self) -> Vec<String> {
        self.documents
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get server count.
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// List all registered server IDs.
    pub fn list_servers(&self) -> Vec<String> {
        self.servers.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get document count.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Check if a server is ready.
    pub async fn is_server_ready(&self, server_id: &str) -> bool {
        if let Some(server) = self.get_server(server_id) {
            server.read().await.is_ready().await
        } else {
            false
        }
    }

    /// Wait for a server to become ready.
    pub async fn wait_server_ready(&self, server_id: &str, timeout_secs: u64) -> Result<()> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let timeout_duration = std::time::Duration::from_secs(timeout_secs);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            if server.read().await.is_ready().await {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Err(LspError::timeout(timeout_secs * 1000).into())
    }

    /// Get signature help for a document position.
    pub async fn get_signature_help(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<SignatureHelp>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            context: None,
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/signatureHelp".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(None)
        } else {
            let signature_help: SignatureHelp = serde_json::from_value(response)?;
            Ok(Some(signature_help))
        }
    }

    /// Get document symbols.
    #[allow(deprecated)]
    pub async fn get_document_symbols(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<Vec<DocumentSymbol>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/documentSymbol".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(vec![])
        } else {
            // LSP can return either DocumentSymbol[] or SymbolInformation[]
            // We'll try DocumentSymbol first, then fall back to SymbolInformation
            if let Ok(symbols) = serde_json::from_value::<Vec<DocumentSymbol>>(response.clone()) {
                Ok(symbols)
            } else if let Ok(symbol_info) = serde_json::from_value::<Vec<SymbolInformation>>(response) {
                // Convert SymbolInformation to DocumentSymbol
                Ok(symbol_info.into_iter().map(|info| {
                    // Convert deprecated field to tags if needed
                    let mut tags = info.tags.unwrap_or_default();
                    #[allow(deprecated)]
                    if info.deprecated.unwrap_or(false) {
                        tags.push(SymbolTag::DEPRECATED);
                    }
                    
                    DocumentSymbol {
                        name: info.name,
                        detail: None,
                        kind: info.kind,
                        tags: Some(tags),
                        deprecated: None,
                        range: info.location.range,
                        selection_range: info.location.range,
                        children: None,
                    }
                }).collect())
            } else {
                Ok(vec![])
            }
        }
    }

    /// Get workspace symbols.
    pub async fn get_workspace_symbols(
        &self,
        server_id: &str,
        query: &str,
    ) -> Result<Vec<SymbolInformation>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = server
            .read()
            .await
            .request(
                "workspace/symbol".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(vec![])
        } else {
            let symbols: Vec<SymbolInformation> = serde_json::from_value(response)?;
            Ok(symbols)
        }
    }

    /// Get code actions for a document range.
    pub async fn get_code_actions(
        &self,
        server_id: &str,
        uri: &str,
        range: Range,
        context: CodeActionContext,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
            },
            range,
            context,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/codeAction".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(vec![])
        } else {
            let actions: Vec<CodeActionOrCommand> = serde_json::from_value(response)?;
            Ok(actions)
        }
    }

    /// Rename a symbol.
    pub async fn rename_symbol(
        &self,
        server_id: &str,
        uri: &str,
        position: Position,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| LspError::server_not_found(server_id))?;

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str(uri).map_err(|_| LspError::invalid_uri(uri))?,
                },
                position,
            },
            new_name: new_name.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = server
            .read()
            .await
            .request(
                "textDocument/rename".to_string(),
                Some(serde_json::to_value(params)?),
            )
            .await?;

        if response.is_null() {
            Ok(None)
        } else {
            let edit: WorkspaceEdit = serde_json::from_value(response)?;
            Ok(Some(edit))
        }
    }

    // ...existing code...
}

impl Default for LspClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LspServerConfig;
    use crate::server::LspServer;

    #[tokio::test]
    async fn test_client_creation() {
        let client = LspClient::new();
        assert_eq!(client.server_count(), 0);
        assert_eq!(client.document_count(), 0);
        assert!(client.active_server().await.is_none());
    }

    #[tokio::test]
    async fn test_server_registration() {
        let client = LspClient::new();
        let config = LspServerConfig::new().command("test-server");
        let server = LspServer::new("test", config);

        client
            .register_server("test".to_string(), server)
            .await
            .unwrap();
        assert_eq!(client.server_count(), 1);
        assert_eq!(client.active_server().await, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_document_operations() {
        let client = LspClient::new();
        let uri = "file:///test.rs";
        let _content = "fn main() {}";

        // Note: This would require a running server for full testing
        assert_eq!(client.document_count(), 0);
        assert!(client.get_document_content(uri).is_none());
    }
}

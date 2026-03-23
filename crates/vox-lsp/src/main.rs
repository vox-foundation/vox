//! `vox-lsp` binary — Language Server Protocol frontend for Vox sources.
//!
//! Wraps lex/parse/typecheck using the same diagnostics path as the CLI.

use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;

use vox_lexer::lex;
use vox_parser::parser::parse;
use vox_typeck::diagnostics::Severity;
use vox_typeck::typecheck_module;

#[derive(Debug)]
struct Backend {
    client: Client,
    /// Latest full document text per URI (FULL sync) for hover and validation.
    documents: Mutex<HashMap<Url, String>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        info!("Vox LSP initializing...");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Vox LSP initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let tdp = params.text_document_position_params;
        let uri = &tdp.text_document.uri;
        let pos = tdp.position;
        let text = self.documents.lock().unwrap().get(uri).cloned();
        let Some(text) = text else {
            return Ok(None);
        };
        let Some(word) = vox_lsp::word_at_position(&text, pos.line, pos.character) else {
            return Ok(None);
        };
        let line_str = text.lines().nth(pos.line as usize).unwrap_or("");
        let Some(md) = vox_lsp::builtin_hover_markdown_in_line(line_str, &word) else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: md,
            }),
            range: None,
        }))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate_document(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // We assume FULL sync, so content_changes[0].text is the full document.
        if let Some(change) = params.content_changes.first() {
            self.validate_document(params.text_document.uri, change.text.clone())
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(text) = params.text {
            self.validate_document(uri, text).await;
            return;
        }
        let Ok(path) = uri.to_file_path() else {
            let _ = self
                .client
                .log_message(
                    MessageType::WARNING,
                    format!("did_save: cannot map URI to file path: {uri}"),
                )
                .await;
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => self.validate_document(uri, text).await,
            Err(e) => {
                let _ = self
                    .client
                    .log_message(
                        MessageType::WARNING,
                        format!("did_save: failed to read {}: {e}", path.display()),
                    )
                    .await;
            }
        }
    }
}

impl Backend {
    async fn validate_document(&self, uri: Url, text: String) {
        self.documents
            .lock()
            .unwrap()
            .insert(uri.clone(), text.clone());

        let mut diagnostics = Vec::new();

        // 1. Lex
        let tokens = lex(&text);

        // 2. Parse errors are now handled to position them properly
        match parse(tokens) {
            Ok(module) => {
                // 3. Type Check
                let type_errors = typecheck_module(&module, &text);

                for err in type_errors {
                    let (sl, sc) = vox_lsp::byte_index_to_line_col(&text, err.span.start);
                    let (el, ec) = vox_lsp::byte_index_to_line_col(&text, err.span.end);
                    let start = Position {
                        line: sl,
                        character: sc,
                    };
                    let end = Position {
                        line: el,
                        character: ec,
                    };

                    diagnostics.push(Diagnostic {
                        range: Range { start, end },
                        severity: Some(match err.severity {
                            Severity::Error => DiagnosticSeverity::ERROR,
                            Severity::Warning => DiagnosticSeverity::WARNING,
                        }),
                        code: None,
                        code_description: None,
                        source: Some("vox-lsp".to_string()),
                        message: err.message,
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
            Err(parse_errors) => {
                // Convert ParseError to Diagnostic
                for err in parse_errors {
                    let (sl, sc) = vox_lsp::byte_index_to_line_col(&text, err.span.start);
                    let (el, ec) = vox_lsp::byte_index_to_line_col(&text, err.span.end);
                    let start = Position {
                        line: sl,
                        character: sc,
                    };
                    let end = Position {
                        line: el,
                        character: ec,
                    };
                    diagnostics.push(Diagnostic {
                        range: Range { start, end },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        message: err.to_string(),
                        source: Some("vox-lsp".to_string()),
                        ..Default::default()
                    });
                }
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Mutex::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

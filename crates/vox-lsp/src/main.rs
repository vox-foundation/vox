//! `vox-lsp` binary — Language Server Protocol frontend for Vox sources.
//!
//! Wraps lex/parse/typecheck using the same diagnostics path as the CLI.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;

use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::diagnostics::TypeckSeverity;
use vox_compiler::typeck::typecheck_module;

static LUDUS_PROJECT_DB: OnceLock<Mutex<Option<Arc<vox_db::VoxDb>>>> = OnceLock::new();

fn ludus_lsp_events_disabled() -> bool {
    matches!(
        std::env::var("VOX_LSP_LUDUS_EVENTS")
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "0" | "false" | "no" | "off"
    )
}

async fn cached_project_db() -> Option<Arc<vox_db::VoxDb>> {
    let cell = LUDUS_PROJECT_DB.get_or_init(|| Mutex::new(None));
    let need_open = cell.lock().ok()?.is_none();
    if need_open && let Ok(db) = vox_db::open_project_db().await {
        let mut g = cell.lock().ok()?;
        if g.is_none() {
            *g = Some(Arc::new(db));
        }
    }
    let g = cell.lock().ok()?;
    g.as_ref().map(Arc::clone)
}

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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                        SemanticTokensRegistrationOptions {
                            text_document_registration_options: TextDocumentRegistrationOptions {
                                document_selector: Some(vec![DocumentFilter {
                                    language: Some("vox".to_string()),
                                    scheme: Some("file".to_string()),
                                    pattern: None,
                                }]),
                            },
                            semantic_tokens_options: SemanticTokensOptions {
                                work_done_progress_options: WorkDoneProgressOptions {
                                    work_done_progress: None,
                                },
                                range: None,
                                full: Some(SemanticTokensFullOptions::Bool(true)),
                                legend: SemanticTokensLegend {
                                    token_types: vox_lsp::grammar::SEMANTIC_TOKEN_TYPES.to_vec(),
                                    token_modifiers: vec![
                                        SemanticTokenModifier::DECLARATION,
                                        SemanticTokenModifier::DEFINITION,
                                        SemanticTokenModifier::READONLY,
                                    ],
                                },
                            },
                            static_registration_options: StaticRegistrationOptions { id: None },
                        },
                    ),
                ),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["@".to_string(), ".".to_string()]),
                    ..Default::default()
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
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
        let text = match self.documents.lock() {
            Ok(g) => g.get(uri).cloned(),
            Err(e) => {
                tracing::error!("hover: documents mutex poisoned: {e}");
                return Ok(None);
            }
        };
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

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::List(
            vox_lsp::completions::CompletionEngine::completions(params),
        )))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let text = match self.documents.lock() {
            Ok(g) => g.get(uri).cloned(),
            Err(e) => {
                tracing::error!("document_symbol: documents mutex poisoned: {e}");
                return Ok(None);
            }
        };
        let Some(text) = text else {
            return Ok(None);
        };

        let tokens = lex(&text);
        if let Ok(module) = parse(tokens) {
            let symbols = vox_lsp::symbols::SymbolEngine::symbols(&module, &text);
            Ok(Some(DocumentSymbolResponse::Nested(symbols)))
        } else {
            Ok(None)
        }
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = &params.text_document.uri;
        let text = match self.documents.lock() {
            Ok(g) => g.get(uri).cloned(),
            Err(e) => {
                tracing::error!("code_lens: documents mutex poisoned: {e}");
                return Ok(None);
            }
        };
        let Some(text) = text else {
            return Ok(Some(vec![]));
        };
        let tokens = lex(&text);
        if let Ok(module) = parse(tokens) {
            let lenses = vox_lsp::code_lens::code_lenses_for_module(&module, &text);
            Ok(Some(lenses))
        } else {
            Ok(Some(vec![]))
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let text = match self.documents.lock() {
            Ok(g) => g.get(uri).cloned(),
            Err(e) => {
                tracing::error!("semantic_tokens_full: documents mutex poisoned: {e}");
                return Ok(None);
            }
        };
        let Some(text) = text else {
            return Ok(None);
        };

        let tokens = lex(&text);
        let mut last_line = 0;
        let mut last_char = 0;
        let mut data = Vec::new();

        for token in tokens {
            if let Some(token_type) = vox_lsp::grammar::token_to_semantic_type(&token.token) {
                let (line, col) = vox_lsp::byte_index_to_line_col(&text, token.span.start);
                let length = (token.span.end - token.span.start) as u32;

                let delta_line = line - last_line;
                let delta_char = if delta_line == 0 {
                    col - last_char
                } else {
                    col
                };

                data.push(SemanticToken {
                    delta_line,
                    delta_start: delta_char,
                    length,
                    token_type,
                    token_modifiers_bitset: 0,
                });

                last_line = line;
                last_char = col;
            }
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
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
        {
            let mut guard = match self.documents.lock() {
                Ok(g) => g,
                Err(e) => {
                    tracing::error!("validate_document: documents mutex poisoned: {e}");
                    return;
                }
            };
            guard.insert(uri.clone(), text.clone());
        }

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
                            TypeckSeverity::Error => DiagnosticSeverity::ERROR,
                            TypeckSeverity::Warning => DiagnosticSeverity::WARNING,
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

        let err_n = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .count();
        let warn_n = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
            .count();

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;

        if !ludus_lsp_events_disabled() {
            let uri_s = uri.to_string();
            tokio::spawn(async move {
                let Some(db) = cached_project_db().await else {
                    return;
                };
                vox_ludus::lsp_telemetry::after_diagnostic_publish(
                    db.as_ref(),
                    &uri_s,
                    err_n,
                    warn_n,
                )
                .await;
            });
        }
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

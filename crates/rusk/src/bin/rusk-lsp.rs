use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::Result,
    lsp_types::{
        Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse,
        InitializeParams, InitializeResult, InitializedParams, MessageType, OneOf, Position, Range,
        ServerCapabilities, SymbolKind, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    },
};

use rusk::{SourceMapNode, transpile};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "rusk language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.publish_diagnostics(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params
            .content_changes
            .into_iter()
            .last()
            .map(|change| change.text)
            .unwrap_or_default();
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.publish_diagnostics(uri, text).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let Ok(output) = transpile(text) else {
            return Ok(None);
        };

        Ok(Some(DocumentSymbolResponse::Nested(
            output
                .source_tree
                .iter()
                .filter_map(symbol_from_node)
                .collect(),
        )))
    }
}

impl Backend {
    async fn publish_diagnostics(&self, uri: Url, text: String) {
        let diagnostics = match transpile(&text) {
            Ok(_) => Vec::new(),
            Err(error) => vec![Diagnostic {
                range: line_range(&text, error.line),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("rusk".to_string()),
                message: error.to_string(),
                ..Diagnostic::default()
            }],
        };
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[allow(deprecated)]
fn symbol_from_node(node: &SourceMapNode) -> Option<DocumentSymbol> {
    let kind = symbol_kind(&node.kind)?;
    Some(DocumentSymbol {
        name: symbol_name(node),
        detail: Some(node.kind.clone()),
        kind,
        tags: None,
        deprecated: None,
        range: node_range(node),
        selection_range: node_range(node),
        children: Some(node.children.iter().filter_map(symbol_from_node).collect()),
    })
}

fn symbol_kind(kind: &str) -> Option<SymbolKind> {
    match kind {
        "struct" => Some(SymbolKind::STRUCT),
        "enum" => Some(SymbolKind::ENUM),
        "trait" => Some(SymbolKind::INTERFACE),
        "impl" => Some(SymbolKind::OBJECT),
        "module" => Some(SymbolKind::MODULE),
        "function" => Some(SymbolKind::FUNCTION),
        "member" | "field" => Some(SymbolKind::FIELD),
        _ => None,
    }
}

fn symbol_name(node: &SourceMapNode) -> String {
    let text = node.source_text.trim();
    if let Some(name) = item_name(text, "struct")
        .or_else(|| item_name(text, "enum"))
        .or_else(|| item_name(text, "trait"))
        .or_else(|| item_name(text, "mod"))
        .or_else(|| item_name(text, "fn"))
    {
        name
    } else if let Some(rest) = text.strip_prefix("impl") {
        format!("impl{}", rest)
    } else {
        text.split([':', '=', '(', ' '])
            .next()
            .unwrap_or(text)
            .to_string()
    }
}

fn item_name(text: &str, keyword: &str) -> Option<String> {
    let index = text.find(keyword)? + keyword.len();
    let rest = text[index..].trim_start();
    let name = rest
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .find(|part| !part.is_empty())?;
    Some(name.to_string())
}

fn node_range(node: &SourceMapNode) -> Range {
    let line = node.source_line.saturating_sub(1) as u32;
    Range {
        start: Position {
            line,
            character: node.source_indent as u32,
        },
        end: Position {
            line,
            character: (node.source_indent + node.source_text.len()) as u32,
        },
    }
}

fn line_range(text: &str, line: usize) -> Range {
    let line_index = line.saturating_sub(1);
    let length = text
        .lines()
        .nth(line_index)
        .map(str::len)
        .unwrap_or_default() as u32;
    Range {
        start: Position {
            line: line_index as u32,
            character: 0,
        },
        end: Position {
            line: line_index as u32,
            character: length,
        },
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(RwLock::new(HashMap::new())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

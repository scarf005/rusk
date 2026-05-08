use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use tokio::sync::RwLock;
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::Result,
    lsp_types::{
        CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams,
        CompletionResponse, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbol,
        DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
        Hover, HoverContents, HoverParams, InitializeParams, InitializeResult, InitializedParams,
        Location, MarkupContent, MarkupKind, MessageType, OneOf, Position, Range,
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
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
                }),
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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&params.text_document_position_params.text_document.uri)
        else {
            return Ok(None);
        };

        Ok(hover_at_position(
            text,
            params.text_document_position_params.position,
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&uri) else {
            return Ok(None);
        };
        let Some(range) =
            definition_range_at_position(text, params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range,
        })))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };

        Ok(Some(CompletionResponse::Array(completion_items(text))))
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

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "do", "else", "enum", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "return",
    "self", "Self", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while",
];

const BUILTIN_TYPES: &[&str] = &[
    "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "str", "u8", "u16",
    "u32", "u64", "u128", "usize",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Word {
    text: String,
    range: Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Declaration {
    name: String,
    detail: String,
    kind: CompletionItemKind,
    range: Range,
}

fn hover_at_position(text: &str, position: Position) -> Option<Hover> {
    let word = word_at_position(text, position)?;
    let contents = if BUILTIN_TYPES.contains(&word.text.as_str()) {
        format!("builtin type `{}`", word.text)
    } else if KEYWORDS.contains(&word.text.as_str()) {
        format!("keyword `{}`", word.text)
    } else {
        declaration_at_position(text, &word)?.detail
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: contents,
        }),
        range: Some(word.range),
    })
}

fn definition_range_at_position(text: &str, position: Position) -> Option<Range> {
    let word = word_at_position(text, position)?;
    declaration_at_position(text, &word).map(|declaration| declaration.range)
}

fn completion_items(text: &str) -> Vec<CompletionItem> {
    let mut seen = BTreeSet::new();
    KEYWORDS
        .iter()
        .map(|keyword| CompletionItem {
            label: (*keyword).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..CompletionItem::default()
        })
        .chain(BUILTIN_TYPES.iter().map(|builtin| CompletionItem {
            label: (*builtin).to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("builtin type".to_string()),
            ..CompletionItem::default()
        }))
        .chain(
            collect_declarations(text)
                .into_iter()
                .map(|declaration| CompletionItem {
                    label: declaration.name,
                    kind: Some(declaration.kind),
                    detail: Some(declaration.detail),
                    ..CompletionItem::default()
                }),
        )
        .filter(|item| seen.insert(item.label.clone()))
        .collect()
}

fn declaration_at_position(text: &str, word: &Word) -> Option<Declaration> {
    let declarations = collect_declarations(text);
    declarations
        .iter()
        .rev()
        .find(|declaration| {
            declaration.name == word.text && declaration.range.start.line <= word.range.start.line
        })
        .or_else(|| {
            declarations
                .iter()
                .find(|declaration| declaration.name == word.text)
        })
        .cloned()
}

fn collect_declarations(text: &str) -> Vec<Declaration> {
    text.lines()
        .enumerate()
        .flat_map(|(index, raw)| declarations_from_line(index as u32, raw))
        .collect()
}

fn declarations_from_line(line: u32, raw: &str) -> Vec<Declaration> {
    let text = raw.trim_start();
    if text.is_empty() || text.starts_with("//") {
        return Vec::new();
    }

    let indent = raw.len() - text.len();
    let mut declarations = Vec::new();
    if let Some(declaration) =
        item_declaration(line, indent, text, "struct", CompletionItemKind::STRUCT)
            .or_else(|| item_declaration(line, indent, text, "enum", CompletionItemKind::ENUM))
            .or_else(|| {
                item_declaration(line, indent, text, "trait", CompletionItemKind::INTERFACE)
            })
            .or_else(|| item_declaration(line, indent, text, "mod", CompletionItemKind::MODULE))
            .or_else(|| item_declaration(line, indent, text, "fn", CompletionItemKind::FUNCTION))
    {
        if declaration.kind == CompletionItemKind::FUNCTION {
            declarations.extend(parameter_declarations(line, indent, text));
        }
        declarations.push(declaration);
        return declarations;
    }

    if let Some(declaration) = let_declaration(line, indent, text) {
        declarations.push(declaration);
    } else if let Some(declaration) = field_declaration(line, indent, text) {
        declarations.push(declaration);
    }
    declarations
}

fn item_declaration(
    line: u32,
    indent: usize,
    text: &str,
    keyword: &str,
    kind: CompletionItemKind,
) -> Option<Declaration> {
    let keyword_start = keyword_position(text, keyword)?;
    let name_start = text[keyword_start + keyword.len()..]
        .find(is_identifier_start)
        .map(|offset| keyword_start + keyword.len() + offset)?;
    let name_end = identifier_end(text, name_start);
    let name = text[name_start..name_end].to_string();
    Some(Declaration {
        name,
        detail: trim_assignment_tail(text).to_string(),
        kind,
        range: range_for_span(line, indent + name_start, indent + name_end),
    })
}

fn parameter_declarations(line: u32, indent: usize, text: &str) -> Vec<Declaration> {
    let Some(fn_start) = keyword_position(text, "fn") else {
        return Vec::new();
    };
    let Some(open) = text[fn_start..].find('(').map(|offset| fn_start + offset) else {
        return Vec::new();
    };
    let Some(close) = matching_delimiter(text, open, '(', ')') else {
        return Vec::new();
    };

    split_top_level_spans(&text[open + 1..close], ',')
        .into_iter()
        .filter_map(|(start, end)| {
            let parameter = &text[open + 1 + start..open + 1 + end];
            let trimmed_start = parameter.len() - parameter.trim_start().len();
            let trimmed = parameter.trim();
            let colon = trimmed.find(':')?;
            let name = trimmed[..colon].trim();
            if name == "self" || name == "&self" || name == "&mut self" {
                return None;
            }
            let name_start = open + 1 + start + trimmed_start + trimmed[..colon].find(name)?;
            let name_end = name_start + name.len();
            Some(Declaration {
                name: name.to_string(),
                detail: format!("parameter {name}: {}", trimmed[colon + 1..].trim()),
                kind: CompletionItemKind::VARIABLE,
                range: range_for_span(line, indent + name_start, indent + name_end),
            })
        })
        .collect()
}

fn let_declaration(line: u32, indent: usize, text: &str) -> Option<Declaration> {
    let rest = text.strip_prefix("let ")?;
    let rest_offset = text.len() - rest.len();
    let rest = rest.strip_prefix("mut ").unwrap_or(rest);
    let rest_offset = if rest.as_ptr() == text[rest_offset..].as_ptr() {
        rest_offset
    } else {
        rest_offset + "mut ".len()
    };
    let name_start = rest.find(is_identifier_start)?;
    let name_end = identifier_end(rest, name_start);
    let name = &rest[name_start..name_end];
    let after_name = &rest[name_end..];
    let detail = after_name
        .trim_start()
        .strip_prefix(':')
        .and_then(|after_colon| after_colon.split('=').next())
        .map(str::trim)
        .filter(|annotation| !annotation.is_empty())
        .map(|annotation| format!("let {name}: {annotation}"))
        .unwrap_or_else(|| format!("let {name}"));

    Some(Declaration {
        name: name.to_string(),
        detail,
        kind: CompletionItemKind::VARIABLE,
        range: range_for_span(
            line,
            indent + rest_offset + name_start,
            indent + rest_offset + name_end,
        ),
    })
}

fn field_declaration(line: u32, indent: usize, text: &str) -> Option<Declaration> {
    let (text, offset) = text
        .strip_prefix("pub ")
        .map(|text| (text, "pub ".len()))
        .unwrap_or((text, 0));
    let colon = text.find(':')?;
    if text[..colon].contains([' ', '(', ')', '=']) {
        return None;
    }
    let name = text[..colon].trim();
    if name.is_empty() || !name.chars().next().is_some_and(is_identifier_start) {
        return None;
    }
    Some(Declaration {
        name: name.to_string(),
        detail: format!("field {name}: {}", text[colon + 1..].trim()),
        kind: CompletionItemKind::FIELD,
        range: range_for_span(line, indent + offset, indent + offset + name.len()),
    })
}

fn word_at_position(text: &str, position: Position) -> Option<Word> {
    let raw = text.lines().nth(position.line as usize)?;
    let character = (position.character as usize).min(raw.len());
    let bytes = raw.as_bytes();
    let mut start = character;
    if start == raw.len() && start > 0 {
        start -= 1;
    }
    if start < raw.len()
        && !is_identifier_byte(bytes[start])
        && start > 0
        && is_identifier_byte(bytes[start - 1])
    {
        start -= 1;
    }
    if start >= raw.len() || !is_identifier_byte(bytes[start]) {
        return None;
    }
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < raw.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }

    Some(Word {
        text: raw[start..end].to_string(),
        range: range_for_span(position.line, start, end),
    })
}

fn keyword_position(text: &str, keyword: &str) -> Option<usize> {
    text.match_indices(keyword)
        .find(|(index, _)| {
            let before = index
                .checked_sub(1)
                .and_then(|before| text.as_bytes().get(before))
                .is_none_or(|byte| !is_identifier_byte(*byte));
            let after = text
                .as_bytes()
                .get(index + keyword.len())
                .is_none_or(|byte| !is_identifier_byte(*byte));
            before && after
        })
        .map(|(index, _)| index)
}

fn identifier_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut end = start;
    while end < text.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }
    end
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn trim_assignment_tail(text: &str) -> &str {
    text.split(" =")
        .next()
        .unwrap_or(text)
        .trim_end_matches('=')
        .trim()
}

fn matching_delimiter(text: &str, open: usize, open_char: char, close_char: char) -> Option<usize> {
    let mut depth = 0usize;
    for (index, character) in text[open..].char_indices() {
        if character == open_char {
            depth += 1;
        } else if character == close_char {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(open + index);
            }
        }
    }
    None
}

fn split_top_level_spans(text: &str, delimiter: char) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut braces = 0usize;
    for (index, character) in text.char_indices() {
        match character {
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            character if character == delimiter && parens == 0 && brackets == 0 && braces == 0 => {
                spans.push((start, index));
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    spans.push((start, text.len()));
    spans
}

fn range_for_span(line: u32, start: usize, end: usize) -> Range {
    Range {
        start: Position {
            line,
            character: start as u32,
        },
        end: Position {
            line,
            character: end as u32,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_reports_function_signature() {
        let hover = hover_at_position(
            "pub fn answer(value: i32) -> i32 = value\n",
            Position {
                line: 0,
                character: 8,
            },
        )
        .unwrap();

        assert_eq!(
            hover.contents,
            HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "pub fn answer(value: i32) -> i32".to_string(),
            })
        );
    }

    #[test]
    fn hover_reports_parameter_type() {
        let hover = hover_at_position(
            "pub fn answer(value: i32) -> i32 = value\n",
            Position {
                line: 0,
                character: 14,
            },
        )
        .unwrap();

        assert_eq!(
            hover.contents,
            HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "parameter value: i32".to_string(),
            })
        );
    }

    #[test]
    fn definition_finds_nearest_declaration() {
        let range = definition_range_at_position(
            "pub fn answer() -> i32 = 42\npub fn main() = answer()\n",
            Position {
                line: 1,
                character: 16,
            },
        )
        .unwrap();

        assert_eq!(range, range_for_span(0, 7, 13));
    }

    #[test]
    fn completions_include_document_symbols() {
        let items = completion_items("pub struct User\npub fn greet(user: User) = user\n");

        assert!(items.iter().any(|item| item.label == "User"));
        assert!(items.iter().any(|item| item.label == "greet"));
        assert!(items.iter().any(|item| item.label == "user"));
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

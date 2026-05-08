use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapEntry {
    pub source_line: usize,
    pub generated_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapNode {
    pub kind: String,
    pub source_line: usize,
    pub source_indent: usize,
    pub source_text: String,
    pub generated_start_line: usize,
    pub generated_end_line: usize,
    pub children: Vec<SourceMapNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranspileOutput {
    pub rust: String,
    pub source_map: Vec<SourceMapEntry>,
    pub source_tree: Vec<SourceMapNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranspileError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for TranspileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for TranspileError {}

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    use super::{TranspileError, source_map_json, transpile};

    #[wasm_bindgen]
    pub fn transpile_to_rust(source: &str) -> Result<String, JsValue> {
        transpile(source)
            .map(|output| output.rust)
            .map_err(error_to_js_value)
    }

    #[wasm_bindgen]
    pub fn transpile_syntax_tree_json(source: &str) -> Result<String, JsValue> {
        transpile(source)
            .map(|output| source_map_json(&output))
            .map_err(error_to_js_value)
    }

    fn error_to_js_value(error: TranspileError) -> JsValue {
        JsValue::from_str(&error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceLine {
    line: usize,
    indent: usize,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    line: usize,
    indent: usize,
    text: String,
    children: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Root,
    Block,
    Struct,
    Enum,
    Trait,
    Match,
    StructLiteral,
}

#[derive(Debug, Default)]
struct Emitter {
    lines: Vec<String>,
    source_map: Vec<SourceMapEntry>,
    source_tree: Vec<SourceMapNode>,
    tree_stack: Vec<SourceMapNode>,
}

pub fn transpile(source: &str) -> Result<TranspileOutput, TranspileError> {
    let lines = source_lines(source)?;
    let (nodes, index) = parse_nodes(&lines, 0, 0)?;
    if index != lines.len() {
        return Err(TranspileError {
            line: lines[index].line,
            message: "unexpected indentation".to_string(),
        });
    }

    let mut emitter = Emitter::default();
    emit_nodes(&nodes, Context::Root, &mut emitter);
    Ok(TranspileOutput {
        rust: ensure_trailing_newline(&emitter.lines.join("\n")),
        source_map: emitter.source_map,
        source_tree: emitter.source_tree,
    })
}

pub fn source_map_json(output: &TranspileOutput) -> String {
    format!(
        "{{\n  \"version\": 2,\n  \"format\": \"rusk.syntax-tree\",\n  \"tree\": [\n{}\n  ]\n}}\n",
        source_map_nodes_json(&output.source_tree, 2)
    )
}

fn source_map_nodes_json(nodes: &[SourceMapNode], indent: usize) -> String {
    nodes
        .iter()
        .map(|node| source_map_node_json(node, indent))
        .collect::<Vec<_>>()
        .join(",\n")
}

fn source_map_node_json(node: &SourceMapNode, indent: usize) -> String {
    let pad = spaces(indent * 2);
    let child_pad = spaces((indent + 1) * 2);
    let children = if node.children.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[\n{}\n{child_pad}]",
            source_map_nodes_json(&node.children, indent + 2)
        )
    };

    format!(
        "{pad}{{\n{child_pad}\"kind\": \"{}\",\n{child_pad}\"source\": {{ \"line\": {}, \"indent\": {}, \"text\": \"{}\" }},\n{child_pad}\"generated\": {{ \"start_line\": {}, \"end_line\": {} }},\n{child_pad}\"children\": {}\n{pad}}}",
        escape_json(&node.kind),
        node.source_line,
        node.source_indent,
        escape_json(&node.source_text),
        node.generated_start_line,
        node.generated_end_line,
        children
    )
}

fn escape_json(text: &str) -> String {
    let mut escaped = String::new();
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn source_lines(source: &str) -> Result<Vec<SourceLine>, TranspileError> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            let line = index + 1;
            let without_comment = raw.trim_end();
            if without_comment.trim().is_empty() {
                return None;
            }
            if without_comment.contains('\t') {
                return Some(Err(TranspileError {
                    line,
                    message: "tabs are not supported; use spaces".to_string(),
                }));
            }
            let indent = without_comment
                .chars()
                .take_while(|character| *character == ' ')
                .count();
            Some(Ok(SourceLine {
                line,
                indent,
                text: without_comment.trim_start().to_string(),
            }))
        })
        .collect()
}

fn parse_nodes(
    lines: &[SourceLine],
    mut index: usize,
    indent: usize,
) -> Result<(Vec<Node>, usize), TranspileError> {
    let mut nodes = Vec::new();
    while index < lines.len() {
        let line = &lines[index];
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(TranspileError {
                line: line.line,
                message: "indentation must belong to the previous line".to_string(),
            });
        }

        index += 1;
        let mut children = Vec::new();
        if index < lines.len() && lines[index].indent > indent {
            let child_indent = lines[index].indent;
            let parsed = parse_nodes(lines, index, child_indent)?;
            children = parsed.0;
            index = parsed.1;
        }
        nodes.push(Node {
            line: line.line,
            indent: line.indent,
            text: line.text.clone(),
            children,
        });
    }
    Ok((nodes, index))
}

fn emit_nodes(nodes: &[Node], context: Context, emitter: &mut Emitter) {
    for (index, node) in nodes.iter().enumerate() {
        emit_node(node, context, index + 1 == nodes.len(), emitter);
    }
}

fn emit_node(node: &Node, context: Context, is_last: bool, emitter: &mut Emitter) {
    emitter.begin_node(node, classify_node(node, context));
    emit_node_body(node, context, is_last, emitter);
    emitter.end_node();
}

fn emit_node_body(node: &Node, context: Context, is_last: bool, emitter: &mut Emitter) {
    let text = node.text.trim();
    if text.starts_with("//") {
        emitter.push(node, text.to_string());
        return;
    }
    if let Some(attribute) = lower_attribute(text) {
        emitter.push(node, format!("{}{}", spaces(node.indent), attribute));
        return;
    }

    match context {
        Context::Struct => emit_struct_member(node, emitter),
        Context::Enum => emit_enum_member(node, emitter),
        Context::Trait => emit_trait_member(node, emitter),
        Context::Match => emit_match_arm(node, emitter),
        Context::StructLiteral => emit_struct_literal_field(node, emitter),
        Context::Root | Context::Block => emit_general(node, context, is_last, emitter),
    }
}

fn classify_node(node: &Node, context: Context) -> String {
    let text = node.text.trim();
    if text.starts_with("//") {
        "comment".to_string()
    } else if lower_attribute(text).is_some() {
        "attribute".to_string()
    } else if context == Context::Match {
        "match_arm".to_string()
    } else if context == Context::StructLiteral {
        "field".to_string()
    } else if matches!(context, Context::Struct | Context::Enum) && !is_function(text) {
        "member".to_string()
    } else if is_struct_item(text) {
        "struct".to_string()
    } else if is_enum_item(text) {
        "enum".to_string()
    } else if is_trait_item(text) {
        "trait".to_string()
    } else if is_function(text) {
        "function".to_string()
    } else if starts_item(text, "impl") {
        "impl".to_string()
    } else if starts_item(text, "mod") {
        "module".to_string()
    } else if is_match(text) {
        "match".to_string()
    } else if is_inline_if_expression(text) {
        "if_expression".to_string()
    } else if is_control(text) {
        "control".to_string()
    } else if looks_like_struct_literal(node) || is_inline_struct_literal(text) {
        "struct_literal".to_string()
    } else if is_let(text) {
        "let".to_string()
    } else if let Some(expr) = text.strip_prefix("do ") {
        format!("do_{}", classify_expression(expr.trim()))
    } else {
        classify_expression(text)
    }
}

fn classify_expression(text: &str) -> String {
    if is_inline_if_expression(text) {
        "if_expression".to_string()
    } else if is_inline_struct_literal(text) {
        "struct_literal".to_string()
    } else if is_assignment(text) {
        "assignment".to_string()
    } else if is_jump_statement(text) {
        "jump".to_string()
    } else {
        "expression".to_string()
    }
}

fn emit_general(node: &Node, context: Context, is_last: bool, emitter: &mut Emitter) {
    let text = node.text.trim();
    if is_struct_item(text) {
        emit_braced_item(node, Context::Struct, emitter, lower_signature(text));
    } else if is_enum_item(text) {
        emit_braced_item(node, Context::Enum, emitter, lower_signature(text));
    } else if is_trait_item(text) {
        emit_braced_item(node, Context::Trait, emitter, lower_signature(text));
    } else if is_function(text) {
        emit_function(node, emitter);
    } else if is_impl_or_mod_item(text) {
        emit_braced_item(node, Context::Block, emitter, lower_signature(text));
    } else if is_match(text) {
        emit_braced_item(node, Context::Match, emitter, lower_expr(text));
    } else if is_control(text) {
        emit_braced_item(node, Context::Block, emitter, lower_expr(text));
    } else if looks_like_struct_literal(node) {
        emit_struct_literal_expr(node, emitter, is_last);
    } else {
        emit_statement(node, context, is_last, emitter);
    }
}

fn emit_braced_item(node: &Node, child_context: Context, emitter: &mut Emitter, header: String) {
    let indent = spaces(node.indent);
    if node.children.is_empty() && matches!(child_context, Context::Struct | Context::Enum) {
        emitter.push(node, format!("{}{};", indent, header));
        return;
    }
    emitter.push(node, format!("{}{} {{", indent, header));
    emit_nodes(&node.children, child_context, emitter);
    emitter.push_generated(format!("{}}}", indent));
}

fn emit_function(node: &Node, emitter: &mut Emitter) {
    let text = node.text.trim();
    let Some((signature, body)) = split_once_top_level(text, '=') else {
        emitter.push(
            node,
            format!("{}{}", spaces(node.indent), lower_signature(text)),
        );
        return;
    };
    let indent = spaces(node.indent);
    emitter.push(
        node,
        format!("{}{} {{", indent, lower_signature(signature.trim())),
    );
    let body = body.trim();
    if body.is_empty() {
        emit_nodes(&node.children, Context::Block, emitter);
    } else if let Some(expr) = body.strip_prefix("do ") {
        emitter.push_generated(format!(
            "{}{};",
            spaces(node.indent + 4),
            lower_expr(expr.trim())
        ));
    } else {
        emitter.push_generated(format!("{}{}", spaces(node.indent + 4), lower_expr(body)));
    }
    emitter.push_generated(format!("{}}}", indent));
}

fn emit_statement(node: &Node, _context: Context, is_last: bool, emitter: &mut Emitter) {
    let indent = spaces(node.indent);
    let text = node.text.trim();
    if let Some(expr) = text.strip_prefix("do ") {
        emitter.push(node, format!("{}{};", indent, lower_expr(expr.trim())));
    } else if is_use_or_extern_crate(text) {
        emitter.push(node, format!("{}{};", indent, lower_signature(text)));
    } else if is_let(text) || is_assignment(text) || is_jump_statement(text) {
        emitter.push(node, format!("{}{};", indent, lower_expr(text)));
    } else if !node.children.is_empty() && is_control(text) {
        emit_braced_item(node, Context::Block, emitter, lower_expr(text));
    } else if !node.children.is_empty() && looks_like_struct_literal(node) {
        emit_struct_literal_expr(node, emitter, is_last);
    } else {
        let suffix = if is_last { "" } else { ";" };
        emitter.push(node, format!("{}{}{}", indent, lower_expr(text), suffix));
    }
}

fn emit_struct_member(node: &Node, emitter: &mut Emitter) {
    let text = node.text.trim();
    if is_function(text) || is_impl_or_mod_item(text) {
        emit_general(node, Context::Block, true, emitter);
    } else {
        emitter.push(
            node,
            format!("{}{},", spaces(node.indent), lower_field_or_variant(text)),
        );
    }
}

fn emit_enum_member(node: &Node, emitter: &mut Emitter) {
    if node.children.is_empty() {
        emitter.push(
            node,
            format!(
                "{}{},",
                spaces(node.indent),
                lower_field_or_variant(node.text.trim())
            ),
        );
    } else {
        emitter.push(
            node,
            format!(
                "{}{} {{",
                spaces(node.indent),
                lower_field_or_variant(node.text.trim())
            ),
        );
        emit_nodes(&node.children, Context::Struct, emitter);
        emitter.push_generated(format!("{}}},", spaces(node.indent)));
    }
}

fn emit_trait_member(node: &Node, emitter: &mut Emitter) {
    let text = node.text.trim();
    if is_function(text) && text.contains('=') {
        emit_function(node, emitter);
    } else if is_function(text) {
        emitter.push(
            node,
            format!("{}{};", spaces(node.indent), lower_signature(text)),
        );
    } else {
        emit_general(node, Context::Trait, true, emitter);
    }
}

fn emit_match_arm(node: &Node, emitter: &mut Emitter) {
    let indent = spaces(node.indent);
    if let Some((pattern, expr)) = split_arrow(node.text.trim()) {
        let pattern = lower_expr(pattern.trim());
        let expr = expr.trim();
        if expr.is_empty() {
            emitter.push(node, format!("{}{} => {{", indent, pattern));
            emit_nodes(&node.children, Context::Block, emitter);
            emitter.push_generated(format!("{}}},", indent));
        } else if let Some(expr) = expr.strip_prefix("do ") {
            emitter.push(node, format!("{}{} => {{", indent, pattern));
            emitter.push_generated(format!(
                "{}{};",
                spaces(node.indent + 4),
                lower_expr(expr.trim())
            ));
            emitter.push_generated(format!("{}}},", indent));
        } else {
            emitter.push(
                node,
                format!("{}{} => {},", indent, pattern, lower_expr(expr)),
            );
        }
    } else if node.children.is_empty() {
        emitter.push(node, format!("{}{},", indent, lower_expr(node.text.trim())));
    } else {
        emitter.push(
            node,
            format!("{}{} => {{", indent, lower_expr(node.text.trim())),
        );
        emit_nodes(&node.children, Context::Block, emitter);
        emitter.push_generated(format!("{}}},", indent));
    }
}

fn emit_struct_literal_expr(node: &Node, emitter: &mut Emitter, is_last: bool) {
    let indent = spaces(node.indent);
    emitter.push(
        node,
        format!("{}{} {{", indent, lower_expr(node.text.trim())),
    );
    emit_nodes(&node.children, Context::StructLiteral, emitter);
    emitter.push_generated(format!("{}}}{}", indent, if is_last { "" } else { ";" }));
}

fn emit_struct_literal_field(node: &Node, emitter: &mut Emitter) {
    let text = node.text.trim();
    if let Some((field, expr)) = split_once_top_level(text, '=') {
        emitter.push(
            node,
            format!(
                "{}{}: {},",
                spaces(node.indent),
                field.trim(),
                lower_expr(expr.trim())
            ),
        );
    } else {
        emitter.push(
            node,
            format!("{}{},", spaces(node.indent), lower_expr(text)),
        );
    }
}

fn split_arrow(text: &str) -> Option<(&str, &str)> {
    text.split_once("=>")
}

fn split_once_top_level(text: &str, needle: char) -> Option<(&str, &str)> {
    let mut round = 0usize;
    let mut square = 0usize;
    let mut angle = 0usize;
    let mut previous = '\0';
    for (index, character) in text.char_indices() {
        match character {
            '(' => round += 1,
            ')' => round = round.saturating_sub(1),
            '[' => square += 1,
            ']' => square = square.saturating_sub(1),
            '<' if previous == ':' => angle += 1,
            '>' if angle > 0 => angle -= 1,
            character if character == needle && round == 0 && square == 0 && angle == 0 => {
                return Some((&text[..index], &text[index + character.len_utf8()..]));
            }
            _ => {}
        }
        previous = character;
    }
    None
}

fn lower_attribute(text: &str) -> Option<String> {
    if let Some(rest) = text.strip_prefix("#!") {
        if rest.starts_with('[') {
            Some(format!("#!{}", process_attribute_body(rest)))
        } else {
            Some(format!("#![{}]", process_attribute_body(rest)))
        }
    } else if let Some(rest) = text.strip_prefix('#') {
        if rest.starts_with('[') {
            Some(process_attribute_body(text))
        } else {
            Some(format!("#[{}]", process_attribute_body(rest)))
        }
    } else {
        None
    }
}

fn process_attribute_body(text: &str) -> String {
    replace_dotted_paths(
        &replace_square_generics(text, GenericMode::Type),
        PathMode::Type,
    )
}

fn lower_signature(text: &str) -> String {
    replace_dotted_paths(
        &replace_square_generics(text, GenericMode::Type),
        PathMode::Type,
    )
}

fn lower_field_or_variant(text: &str) -> String {
    lower_signature(text)
}

fn lower_expr(text: &str) -> String {
    lower_if_then_else(text).unwrap_or_else(|| lower_basic_expr(text))
}

fn lower_basic_expr(text: &str) -> String {
    replace_dotted_paths(
        &replace_square_generics(text, GenericMode::Expr),
        PathMode::Expr,
    )
}

fn lower_if_then_else(text: &str) -> Option<String> {
    let (prefix, condition_and_body) = if let Some(rest) = text.strip_prefix("if ") {
        ("if", rest)
    } else if let Some(rest) = text.strip_prefix("else if ") {
        ("else if", rest)
    } else {
        return None;
    };

    let Some(then_index) = find_top_level_keyword(condition_and_body, "then") else {
        return None;
    };

    let condition = condition_and_body[..then_index].trim();
    let body = condition_and_body[then_index + "then".len()..].trim();
    if body.is_empty() {
        return Some(format!("{} {}", prefix, lower_basic_expr(condition)));
    }

    let condition = lower_basic_expr(condition);
    if let Some(else_index) = find_else_for_then_body(body) {
        let then_expr = body[..else_index].trim();
        let else_expr = body[else_index + "else".len()..].trim();
        Some(format!(
            "{} {} {{ {} }} else {{ {} }}",
            prefix,
            condition,
            lower_expr(then_expr),
            lower_expr(else_expr)
        ))
    } else {
        Some(format!(
            "{} {} {{ {} }}",
            prefix,
            condition,
            lower_expr(body)
        ))
    }
}

fn find_else_for_then_body(text: &str) -> Option<usize> {
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut nested_if_count = 0usize;
    let mut index = 0usize;

    while index < text.len() {
        let character = text[index..].chars().next()?;
        match character {
            '(' => round += 1,
            ')' => round = round.saturating_sub(1),
            '[' => square += 1,
            ']' => square = square.saturating_sub(1),
            '{' => curly += 1,
            '}' => curly = curly.saturating_sub(1),
            _ => {}
        }

        if round == 0 && square == 0 && curly == 0 {
            if text[index..].starts_with("if") && is_keyword_boundary(text, index, "if".len()) {
                nested_if_count += 1;
            } else if text[index..].starts_with("else")
                && is_keyword_boundary(text, index, "else".len())
            {
                if nested_if_count == 0 {
                    return Some(index);
                }
                nested_if_count -= 1;
            }
        }

        index += character.len_utf8();
    }

    None
}

fn find_top_level_keyword(text: &str, keyword: &str) -> Option<usize> {
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut index = 0usize;

    while index < text.len() {
        let character = text[index..].chars().next()?;
        match character {
            '(' => round += 1,
            ')' => round = round.saturating_sub(1),
            '[' => square += 1,
            ']' => square = square.saturating_sub(1),
            '{' => curly += 1,
            '}' => curly = curly.saturating_sub(1),
            _ => {}
        }

        if round == 0
            && square == 0
            && curly == 0
            && text[index..].starts_with(keyword)
            && is_keyword_boundary(text, index, keyword.len())
        {
            return Some(index);
        }

        index += character.len_utf8();
    }

    None
}

fn is_keyword_boundary(text: &str, index: usize, len: usize) -> bool {
    let before = text[..index].chars().next_back();
    let after = text[index + len..].chars().next();
    before.is_none_or(|character| !is_ident_continue(character))
        && after.is_none_or(|character| !is_ident_continue(character))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenericMode {
    Type,
    Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathMode {
    Type,
    Expr,
}

fn replace_square_generics(text: &str, mode: GenericMode) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '['
            && has_ident_before(&chars, index)
            && let Some(close) = matching_bracket(&chars, index, '[', ']')
        {
            let content: String = chars[index + 1..close].iter().collect();
            let next = chars.get(close + 1).copied();
            let should_convert = match mode {
                GenericMode::Type => true,
                GenericMode::Expr => next == Some('(') || type_like_generic(&content),
            };
            if should_convert {
                let inner = replace_square_generics(&content, GenericMode::Type);
                if mode == GenericMode::Expr && next == Some('(') {
                    output.push_str("::<");
                } else {
                    output.push('<');
                }
                output.push_str(&inner);
                output.push('>');
                index = close + 1;
                continue;
            }
        }
        output.push(chars[index]);
        index += 1;
    }
    output
}

fn replace_dotted_paths(text: &str, mode: PathMode) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        if is_ident_start(chars[index]) {
            let start = index;
            let mut segments = Vec::new();
            let first_end = read_ident(&chars, index);
            segments.push(chars[start..first_end].iter().collect::<String>());
            index = first_end;
            while index + 1 < chars.len() && chars[index] == '.' && is_ident_start(chars[index + 1])
            {
                let segment_start = index + 1;
                let segment_end = read_ident(&chars, segment_start);
                segments.push(chars[segment_start..segment_end].iter().collect::<String>());
                index = segment_end;
            }
            if segments.len() > 1 && should_convert_path(&segments, mode, chars.get(index).copied())
            {
                output.push_str(&segments.join("::"));
            } else {
                output.push_str(&chars[start..index].iter().collect::<String>());
            }
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }
    output
}

fn matching_bracket(chars: &[char], open_index: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (index, character) in chars.iter().enumerate().skip(open_index) {
        if *character == open {
            depth += 1;
        } else if *character == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn has_ident_before(chars: &[char], index: usize) -> bool {
    index > 0
        && (is_ident_continue(chars[index - 1])
            || chars[index - 1] == ']'
            || chars[index - 1] == '>')
}

fn type_like_generic(content: &str) -> bool {
    let trimmed = content.trim();
    !trimmed.is_empty()
        && !trimmed.chars().all(|character| character.is_ascii_digit())
        && generic_tokens(trimmed).into_iter().any(|token| {
            token == "_"
                || is_builtin_type(token)
                || token
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_uppercase())
        })
}

fn generic_tokens(content: &str) -> Vec<&str> {
    content
        .split(|character: char| !is_ident_continue(character))
        .filter(|token| !token.is_empty())
        .collect()
}

fn is_builtin_type(token: &str) -> bool {
    matches!(
        token,
        "bool"
            | "char"
            | "str"
            | "String"
            | "usize"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "f32"
            | "f64"
    )
}

fn should_convert_path(segments: &[String], mode: PathMode, next: Option<char>) -> bool {
    match mode {
        PathMode::Type => true,
        PathMode::Expr => {
            let first = segments.first().map(String::as_str).unwrap_or_default();
            matches!(first, "std" | "core" | "alloc" | "crate" | "super" | "Self")
                || first.chars().next().is_some_and(char::is_uppercase)
                || next == Some('!')
        }
    }
}

fn is_ident_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_ident_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn read_ident(chars: &[char], mut index: usize) -> usize {
    while index < chars.len() && is_ident_continue(chars[index]) {
        index += 1;
    }
    index
}

fn is_struct_item(text: &str) -> bool {
    starts_item(text, "struct")
}

fn is_enum_item(text: &str) -> bool {
    starts_item(text, "enum")
}

fn is_trait_item(text: &str) -> bool {
    starts_item(text, "trait")
}

fn is_impl_or_mod_item(text: &str) -> bool {
    starts_item(text, "impl") || starts_item(text, "mod")
}

fn is_function(text: &str) -> bool {
    text.starts_with("fn ") || text.starts_with("pub fn ") || text.contains(" fn ")
}

fn is_match(text: &str) -> bool {
    text.starts_with("match ")
}

fn is_control(text: &str) -> bool {
    (text.starts_with("if ") && !is_inline_if_expression(text))
        || (text.starts_with("else if ") && !is_inline_if_expression(text))
        || (text.starts_with("else") && !text.starts_with("else if "))
        || text == "loop"
        || text.starts_with("while ")
        || text.starts_with("for ")
        || text == "unsafe"
        || text == "async"
}

fn is_inline_if_expression(text: &str) -> bool {
    let Some(rest) = text
        .strip_prefix("if ")
        .or_else(|| text.strip_prefix("else if "))
    else {
        return false;
    };
    let Some(then_index) = find_top_level_keyword(rest, "then") else {
        return false;
    };
    !rest[then_index + "then".len()..].trim().is_empty()
}

fn starts_item(text: &str, keyword: &str) -> bool {
    text == keyword
        || text.starts_with(&format!("{} ", keyword))
        || text.starts_with(&format!("{}[", keyword))
        || text.starts_with(&format!("pub {} ", keyword))
        || text.starts_with(&format!("pub {}[", keyword))
        || text.starts_with(&format!("pub(crate) {} ", keyword))
        || text.starts_with(&format!("pub(crate) {}[", keyword))
}

fn is_let(text: &str) -> bool {
    text.starts_with("let ")
}

fn is_use_or_extern_crate(text: &str) -> bool {
    text.starts_with("use ") || text.starts_with("extern crate ")
}

fn is_jump_statement(text: &str) -> bool {
    text == "return"
        || text.starts_with("return ")
        || text == "break"
        || text.starts_with("break ")
        || text == "continue"
        || text.starts_with("continue ")
}

fn is_assignment(text: &str) -> bool {
    [" += ", " -= ", " *= ", " /= ", " %= ", " = "]
        .iter()
        .any(|operator| text.contains(operator))
        && !text.contains(" == ")
        && !text.contains(" != ")
        && !text.contains(" <= ")
        && !text.contains(" >= ")
        && !text.contains(" => ")
}

fn looks_like_struct_literal(node: &Node) -> bool {
    !node.children.is_empty()
        && node
            .text
            .trim()
            .chars()
            .next()
            .is_some_and(char::is_uppercase)
        && node
            .children
            .iter()
            .all(|child| split_once_top_level(child.text.trim(), '=').is_some())
}

fn is_inline_struct_literal(text: &str) -> bool {
    let Some(open_index) = text.find('{') else {
        return false;
    };
    text.ends_with('}')
        && text[..open_index]
            .trim_end()
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
}

impl Emitter {
    fn begin_node(&mut self, node: &Node, kind: String) {
        self.tree_stack.push(SourceMapNode {
            kind,
            source_line: node.line,
            source_indent: node.indent,
            source_text: node.text.clone(),
            generated_start_line: self.lines.len() + 1,
            generated_end_line: self.lines.len(),
            children: Vec::new(),
        });
    }

    fn end_node(&mut self) {
        let Some(mut node) = self.tree_stack.pop() else {
            return;
        };
        node.generated_end_line = self.lines.len();
        if let Some(parent) = self.tree_stack.last_mut() {
            parent.children.push(node);
        } else {
            self.source_tree.push(node);
        }
    }

    fn push(&mut self, node: &Node, line: String) {
        self.source_map.push(SourceMapEntry {
            source_line: node.line,
            generated_line: self.lines.len() + 1,
        });
        self.lines.push(line);
    }

    fn push_generated(&mut self, line: String) {
        self.lines.push(line);
    }
}

fn spaces(count: usize) -> String {
    " ".repeat(count)
}

fn ensure_trailing_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{}\n", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust(source: &str) -> String {
        transpile(source).unwrap().rust
    }

    #[test]
    fn lowers_struct_impl_and_inline_functions() {
        let source = r#"
#[derive(Debug, Clone)]
pub struct User
    pub id: u64
    pub name: String

impl User
    pub fn new(id: u64, name: String) -> Self = Self{ id, name }

    pub fn display_name(&self) -> &str = &self.name
"#;

        assert_eq!(
            rust(source),
            r#"#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
}
impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self{ id, name }
    }
    pub fn display_name(&self) -> &str {
        &self.name
    }
}
"#
        );
    }

    #[test]
    fn lowers_do_and_match_arms() {
        let source = r#"
fn parse(line: &str) -> Result[i32, String] =
    match line.parse[i32]()
        Ok(value)
            do println!("{}", value)
            Ok(value)
        Err(error) => Err(error.to_string())
"#;

        assert_eq!(
            rust(source),
            r#"fn parse(line: &str) -> Result<i32, String> {
    match line.parse::<i32>() {
        Ok(value) => {
            println!("{}", value);
            Ok(value)
        },
        Err(error) => Err(error.to_string()),
    }
}
"#
        );
    }

    #[test]
    fn lowers_generic_impl_and_inline_do_match_arm() {
        let source = r#"
pub struct Boxed[T]
    pub value: T

impl[T] Boxed[T]
    pub fn new(value: T) -> Self = Self{ value }

fn log(value: Result[i32, String]) =
    match value
        Ok(number) => do println!("{}", number)
        Err(error) => error
"#;

        assert_eq!(
            rust(source),
            r#"pub struct Boxed<T> {
    pub value: T,
}
impl<T> Boxed<T> {
    pub fn new(value: T) -> Self {
        Self{ value }
    }
}
fn log(value: Result<i32, String>) {
    match value {
        Ok(number) => {
            println!("{}", number);
        },
        Err(error) => error,
    }
}
"#
        );
    }

    #[test]
    fn preserves_value_dots_and_lowers_path_dots() {
        let source = r#"
fn test(iter: Iter) =
    do Foo.new()
    do Foo::bar()
    do iter.collect::<Vec[_]>()
    do std.io.read()
"#;

        assert_eq!(
            rust(source),
            r#"fn test(iter: Iter) {
    Foo::new();
    Foo::bar();
    iter.collect::<Vec<_>>();
    std::io::read();
}
"#
        );
    }

    #[test]
    fn keeps_index_expressions_numeric() {
        let source = r#"
fn example(xs: &[i32], index: usize) =
    let a = [Foo]
    let b = [3]
    let c = xs[3]
    let d = xs[index]
    c + d
"#;

        assert_eq!(
            rust(source),
            r#"fn example(xs: &[i32], index: usize) {
    let a = [Foo];
    let b = [3];
    let c = xs[3];
    let d = xs[index];
    c + d
}
"#
        );
    }

    #[test]
    fn lowers_if_then_else_expression() {
        let source = r#"
fn clamp(value: i32, min: i32, max: i32) -> i32 =
    if value < min then min else if value > max then max else value
"#;

        assert_eq!(
            rust(source),
            r#"fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min { min } else { if value > max { max } else { value } }
}
"#
        );

        assert_eq!(
            rust("fn choose(a: bool, b: bool) -> i32 = if a then if b then 1 else 2 else 3\n"),
            r#"fn choose(a: bool, b: bool) -> i32 {
    if a { if b { 1 } else { 2 } } else { 3 }
}
"#
        );
    }

    #[test]
    fn emits_source_map_json() {
        let output = transpile("fn id(x: i32) -> i32 = x\n").unwrap();
        assert_eq!(output.source_map[0].source_line, 1);
        assert_eq!(output.source_tree[0].kind, "function");
        let json = source_map_json(&output);
        assert!(json.contains("\"version\": 2"));
        assert!(json.contains("\"kind\": \"function\""));
        assert!(json.contains("\"generated\": { \"start_line\": 1, \"end_line\": 3 }"));
    }
}

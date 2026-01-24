use std::collections::HashSet;

use crate::lexer::{self, TokenKind};
use tree_sitter::{Node, Parser};

pub fn collect_declared_identifiers(input: &str) -> Option<HashSet<String>> {
    let mut parser = Parser::new();
    let language = tree_sitter_cpp::LANGUAGE;
    if parser.set_language(&language.into()).is_err() {
        return None;
    }
    let tree = parser.parse(input, None)?;
    let mut declared = HashSet::new();
    collect_from_node(tree.root_node(), input.as_bytes(), &mut declared);
    Some(declared)
}

#[derive(Debug, Clone)]
struct FunctionDefinition {
    name: String,
    decl_range: (usize, usize),
    def_range: (usize, usize),
}

pub fn strip_unused_functions(input: &str) -> Option<String> {
    let mut parser = Parser::new();
    let language = tree_sitter_cpp::LANGUAGE;
    if parser.set_language(&language.into()).is_err() {
        return None;
    }
    let tree = parser.parse(input, None)?;
    let source = input.as_bytes();

    let mut definitions = Vec::new();
    collect_function_definitions(tree.root_node(), source, false, &mut definitions);

    if definitions.is_empty() {
        return Some(input.to_string());
    }

    let decl_ranges: Vec<(usize, usize)> = definitions
        .iter()
        .map(|def| def.decl_range)
        .collect();

    let mut used = HashSet::new();
    collect_used_identifiers(tree.root_node(), source, &decl_ranges, &mut used);

    for token in lexer::tokenize(input) {
        if token.kind == TokenKind::Preprocessor {
            scan_identifiers(&token.text, |ident| {
                used.insert(ident.to_string());
            });
        }
    }

    let mut remove_ranges: Vec<(usize, usize)> = definitions
        .into_iter()
        .filter(|def| !used.contains(&def.name))
        .map(|def| def.def_range)
        .collect();

    if remove_ranges.is_empty() {
        return Some(input.to_string());
    }

    remove_ranges.sort_by_key(|range| range.0);
    let mut out = String::with_capacity(input.len());
    let mut last = 0;
    for (start, end) in remove_ranges {
        if start > last {
            out.push_str(&input[last..start]);
        }
        if end > last {
            last = end;
        }
    }
    if last < input.len() {
        out.push_str(&input[last..]);
    }

    Some(out)
}

fn collect_from_node(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    match node.kind() {
        "function_definition" | "parameter_declaration" => {
            collect_declarator_field(node, source, declared);
        }
        "declaration" | "field_declaration" | "type_definition" => {
            collect_repeated_declarators(node, source, declared);
        }
        "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier" => {
            collect_named_field(node, source, declared);
        }
        "enumerator" => {
            collect_named_field(node, source, declared);
        }
        "namespace_definition" | "namespace_alias_definition" => {
            collect_namespace_name(node, source, declared);
        }
        "alias_declaration" => {
            collect_named_field(node, source, declared);
        }
        "type_parameter_declaration" | "variadic_type_parameter_declaration" => {
            collect_type_parameter_name(node, source, declared);
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_from_node(child, source, declared);
    }
}

fn collect_declarator_field(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    if let Some(declarator) = node.child_by_field_name("declarator") {
        collect_declarator_names(declarator, source, declared);
    }
}

fn collect_repeated_declarators(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    let count = node.child_count();
    for i in 0..count {
        let index = i as u32;
        if node.field_name_for_child(index) == Some("declarator") {
            if let Some(child) = node.child(index) {
                collect_declarator_names(child, source, declared);
            }
        }
    }
}

fn collect_named_field(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    if let Some(name_node) = node.child_by_field_name("name") {
        collect_declarator_names(name_node, source, declared);
    }
}

fn collect_namespace_name(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    if let Some(name_node) = node.child_by_field_name("name") {
        collect_namespace_identifiers(name_node, source, declared);
    }
}

fn collect_namespace_identifiers(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    if node.kind() == "namespace_identifier" {
        if let Ok(text) = node.utf8_text(source) {
            declared.insert(text.to_string());
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_namespace_identifiers(child, source, declared);
    }
}

fn collect_type_parameter_name(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "type_identifier" {
            if let Ok(text) = child.utf8_text(source) {
                declared.insert(text.to_string());
            }
        }
    }
}

fn collect_declarator_names(node: Node, source: &[u8], declared: &mut HashSet<String>) {
    match node.kind() {
        "init_declarator" => {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                collect_declarator_names(declarator, source, declared);
            }
        }
        "identifier" | "field_identifier" | "type_identifier" | "namespace_identifier" => {
            if let Ok(text) = node.utf8_text(source) {
                declared.insert(text.to_string());
            }
        }
        "qualified_identifier" => {
            if let Some(name) = rightmost_identifier(node, source) {
                declared.insert(name);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_declarator_names(child, source, declared);
            }
        }
    }
}

fn collect_function_definitions(
    node: Node,
    source: &[u8],
    in_type_scope: bool,
    definitions: &mut Vec<FunctionDefinition>,
) {
    let kind = node.kind();
    let in_type_scope =
        in_type_scope || matches!(kind, "class_specifier" | "struct_specifier" | "union_specifier");

    if kind == "function_definition" && !in_type_scope {
        if let Some(declarator) = node.child_by_field_name("declarator") {
            if let Some(name) = extract_declarator_identifier(declarator, source) {
                if name != "main" {
                    definitions.push(FunctionDefinition {
                        name,
                        decl_range: (declarator.start_byte(), declarator.end_byte()),
                        def_range: (node.start_byte(), node.end_byte()),
                    });
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_function_definitions(child, source, in_type_scope, definitions);
    }
}

fn extract_declarator_identifier(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" | "namespace_identifier" | "type_identifier" => node
            .utf8_text(source)
            .ok()
            .map(|text| text.to_string()),
        "qualified_identifier" => rightmost_identifier(node, source),
        "operator_name" => None,
        _ => {
            if let Some(inner) = node.child_by_field_name("declarator") {
                return extract_declarator_identifier(inner, source);
            }
            None
        }
    }
}

fn collect_used_identifiers(
    node: Node,
    source: &[u8],
    decl_ranges: &[(usize, usize)],
    used: &mut HashSet<String>,
) {
    let kind = node.kind();
    if matches!(kind, "identifier" | "field_identifier" | "namespace_identifier") {
        let start = node.start_byte();
        if !is_in_ranges(start, decl_ranges) {
            if let Ok(text) = node.utf8_text(source) {
                used.insert(text.to_string());
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_used_identifiers(child, source, decl_ranges, used);
    }
}

fn is_in_ranges(pos: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| pos >= *start && pos < *end)
}

fn scan_identifiers<F: FnMut(&str)>(text: &str, mut f: F) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_ident_start_byte(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_char_byte(bytes[i]) {
                i += 1;
            }
            f(&text[start..i]);
            continue;
        }
        i += 1;
    }
}

fn is_ident_start_byte(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'_')
}

fn is_ident_char_byte(b: u8) -> bool {
    is_ident_start_byte(b) || matches!(b, b'0'..=b'9')
}

fn rightmost_identifier(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" | "namespace_identifier" => {
            node.utf8_text(source).ok().map(|text| text.to_string())
        }
        _ => {
            let mut cursor = node.walk();
            let mut last = None;
            for child in node.named_children(&mut cursor) {
                if let Some(name) = rightmost_identifier(child, source) {
                    last = Some(name);
                }
            }
            last
        }
    }
}

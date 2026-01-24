use std::collections::HashSet;

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

use std::collections::HashSet;

use tree_sitter::{Node, Parser, Tree};

use crate::ObfuscationError;

pub(crate) fn parse(source: &str) -> Result<Tree, ObfuscationError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .map_err(|_| ObfuscationError::ParserUnavailable)?;

    let tree = parser
        .parse(source, None)
        .ok_or(ObfuscationError::ParserUnavailable)?;
    let root = tree.root_node();
    if root.has_error() {
        let statement_macros = statement_macro_names(root, source);
        if let Some(node) = first_unsupported_error(root, source, &statement_macros) {
            let position = node.start_position();
            return Err(ObfuscationError::Syntax {
                line: position.row + 1,
                column: position.column + 1,
                kind: node.kind().to_string(),
            });
        }
    }

    Ok(tree)
}

pub(crate) fn signature(root: Node<'_>) -> String {
    let mut output = String::new();
    append_signature(root, None, &mut output);
    output
}

pub(crate) fn ensure_same_structure(
    expected: &str,
    actual_root: Node<'_>,
    message: &str,
) -> Result<(), ObfuscationError> {
    if signature(actual_root) == expected {
        Ok(())
    } else {
        Err(ObfuscationError::Validation(message.to_string()))
    }
}

fn append_signature(node: Node<'_>, field: Option<&str>, output: &mut String) {
    if node.kind() == "comment" {
        return;
    }

    output.push('(');
    if let Some(field) = field {
        output.push_str(field);
        output.push('=');
    }
    if node.is_error() {
        output.push('!');
    }
    if node.is_missing() {
        output.push('?');
    }
    output.push_str(node.kind());

    for index in 0..node.child_count() {
        let child = node.child(index as u32).expect("valid child index");
        append_signature(child, node.field_name_for_child(index as u32), output);
    }
    output.push(')');
}

fn first_unsupported_error<'tree>(
    node: Node<'tree>,
    source: &str,
    statement_macros: &HashSet<String>,
) -> Option<Node<'tree>> {
    if node.is_error()
        || (node.is_missing() && !is_statement_macro_semicolon(node, source, statement_macros))
    {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(error) = first_unsupported_error(child, source, statement_macros) {
            return Some(error);
        }
    }
    None
}

fn is_statement_macro_semicolon(
    node: Node<'_>,
    source: &str,
    statement_macros: &HashSet<String>,
) -> bool {
    if node.kind() != ";" {
        return false;
    }

    let Some(parent) = node.parent() else {
        return false;
    };
    if parent.kind() != "expression_statement" {
        return false;
    }

    let mut cursor = parent.walk();
    let Some(call) = parent.named_children(&mut cursor).next() else {
        return false;
    };
    if call.kind() != "call_expression" {
        return false;
    }

    call.child_by_field_name("function")
        .filter(|function| function.kind() == "identifier")
        .and_then(|function| function.utf8_text(source.as_bytes()).ok())
        .is_some_and(|name| statement_macros.contains(name))
}

fn statement_macro_names(root: Node<'_>, source: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_statement_macro_names(root, source, &mut names);
    names
}

fn collect_statement_macro_names(node: Node<'_>, source: &str, names: &mut HashSet<String>) {
    if node.kind() == "preproc_function_def" {
        let name = node
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(source.as_bytes()).ok());
        let value = node
            .child_by_field_name("value")
            .and_then(|value| value.utf8_text(source.as_bytes()).ok());
        if let (Some(name), Some(value)) = (name, value)
            && starts_statement(value)
        {
            names.insert(name.to_string());
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_statement_macro_names(child, source, names);
    }
}

fn starts_statement(value: &str) -> bool {
    let value = value.trim_start();
    ["for", "if", "while", "switch"].iter().any(|keyword| {
        value.strip_prefix(keyword).is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|character| character != '_' && !character.is_alphanumeric())
        })
    })
}

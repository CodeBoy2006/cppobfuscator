use tree_sitter::Node;

use crate::integer::IntegerLiteral;
use crate::rewrite::{self, TextEdit};
use crate::{ObfuscationError, syntax};

const MAX_SIMPLIFICATION_PASSES: usize = 16;

pub(crate) fn simplify(source: &str) -> Result<String, ObfuscationError> {
    let expected_newlines = newline_count(source);
    let mut current = source.to_string();

    for _ in 0..MAX_SIMPLIFICATION_PASSES {
        let tree = syntax::parse(&current)?;
        let mut edits = Vec::new();
        collect_edits(tree.root_node(), &current, &mut edits);
        if edits.is_empty() {
            return Ok(current);
        }

        current = rewrite::apply(&current, &edits)?;
        syntax::parse(&current)?;
        if newline_count(&current) != expected_newlines {
            return Err(ObfuscationError::Validation(
                "code simplification changed the physical line count".to_string(),
            ));
        }
    }

    Err(ObfuscationError::Validation(
        "code simplification did not reach a fixed point".to_string(),
    ))
}

fn collect_edits(node: Node<'_>, source: &str, edits: &mut Vec<TextEdit>) {
    if node.kind() == "if_statement"
        && let Some(edit) = constant_if_edit(node, source)
    {
        edits.push(edit);
        return;
    }

    if node.kind() == "while_statement"
        && let Some(edit) = false_while_edit(node, source)
    {
        edits.push(edit);
        return;
    }

    if node.kind() == "compound_statement" {
        collect_compound_edits(node, source, edits);
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_edits(child, source, edits);
    }
}

fn collect_compound_edits(node: Node<'_>, source: &str, edits: &mut Vec<TextEdit>) {
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect();

    let mut process_count = children.len();
    for (index, child) in children.iter().enumerate() {
        if !is_unconditional_terminator(*child) || index + 1 == children.len() {
            continue;
        }

        let tail = &children[index + 1..];
        if tail.iter().any(|child| contains_removal_barrier(*child)) {
            break;
        }
        let start = tail.first().expect("non-empty tail").start_byte();
        let end = tail.last().expect("non-empty tail").end_byte();
        edits.push(TextEdit {
            start,
            end,
            replacement: preserve_newlines(&source[start..end]),
        });
        process_count = index + 1;
        break;
    }

    for child in children.into_iter().take(process_count) {
        if is_empty_statement(child) {
            edits.push(TextEdit {
                start: child.start_byte(),
                end: child.end_byte(),
                replacement: preserve_newlines(node_text(child, source)),
            });
        } else {
            collect_edits(child, source, edits);
        }
    }
}

fn constant_if_edit(node: Node<'_>, source: &str) -> Option<TextEdit> {
    // A goto or switch entry can enter a dead-looking branch without
    // evaluating its condition; preprocessing nodes act unconditionally.
    if contains_control_entry_barrier(node) {
        return None;
    }

    let condition = node.child_by_field_name("condition")?;
    let truth = constant_truth(condition, source)?;
    let replacement = if truth {
        let consequence = node.child_by_field_name("consequence")?;
        scoped_statement(consequence, source)
    } else {
        node.child_by_field_name("alternative")
            .and_then(first_statement_child)
            .map_or_else(
                || "{}".to_string(),
                |statement| scoped_statement(statement, source),
            )
    };
    line_preserving_edit(node, source, replacement)
}

fn false_while_edit(node: Node<'_>, source: &str) -> Option<TextEdit> {
    // A label can make a false loop body externally reachable through goto.
    if contains_control_entry_barrier(node) {
        return None;
    }
    let condition = node.child_by_field_name("condition")?;
    if constant_truth(condition, source)? {
        return None;
    }
    line_preserving_edit(node, source, "{}".to_string())
}

fn line_preserving_edit(node: Node<'_>, source: &str, mut replacement: String) -> Option<TextEdit> {
    let original_newlines = newline_count(node_text(node, source));
    let replacement_newlines = newline_count(&replacement);
    if replacement_newlines > original_newlines {
        return None;
    }
    for _ in replacement_newlines..original_newlines {
        replacement.push('\n');
    }
    Some(TextEdit {
        start: node.start_byte(),
        end: node.end_byte(),
        replacement,
    })
}

fn scoped_statement(node: Node<'_>, source: &str) -> String {
    let text = node_text(node, source);
    if node.kind() == "compound_statement" {
        text.to_string()
    } else {
        format!("{{{text}}}")
    }
}

fn constant_truth(node: Node<'_>, source: &str) -> Option<bool> {
    let node = unwrap_single_expression(node)?;
    match node.kind() {
        "true" => Some(true),
        "false" => Some(false),
        "number_literal" => {
            let literal = IntegerLiteral::parse(node_text(node, source))?;
            Some(literal.value != 0)
        }
        _ => None,
    }
}

fn unwrap_single_expression(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        if !matches!(node.kind(), "condition_clause" | "parenthesized_expression") {
            return Some(node);
        }

        let mut cursor = node.walk();
        let mut children = node
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment");
        let child = children.next()?;
        if children.next().is_some() {
            return None;
        }
        node = child;
    }
}

fn first_statement_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() != "comment")
}

fn is_empty_statement(node: Node<'_>) -> bool {
    if node.kind() != "expression_statement" {
        return false;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .all(|child| child.kind() == "comment")
}

fn is_unconditional_terminator(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "return_statement"
            | "co_return_statement"
            | "break_statement"
            | "continue_statement"
            | "goto_statement"
    )
}

fn contains_control_entry_barrier(node: Node<'_>) -> bool {
    contains_kind(node, &["labeled_statement", "case_statement"]) || contains_preprocessor(node)
}

fn contains_removal_barrier(node: Node<'_>) -> bool {
    contains_control_entry_barrier(node)
        || contains_kind(
            node,
            &[
                "static_assert_declaration",
                "type_definition",
                "alias_declaration",
            ],
        )
}

fn contains_preprocessor(node: Node<'_>) -> bool {
    if node.kind().starts_with("preproc_") {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(contains_preprocessor)
}

fn contains_kind(node: Node<'_>, kinds: &[&str]) -> bool {
    if kinds.contains(&node.kind()) {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| contains_kind(child, kinds))
}

fn preserve_newlines(text: &str) -> String {
    text.chars()
        .filter(|character| *character == '\n')
        .collect()
}

fn newline_count(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count()
}

fn node_text<'source>(node: Node<'_>, source: &'source str) -> &'source str {
    node.utf8_text(source.as_bytes()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_literal_branches_and_removes_empty_statements() {
        let source = "int main(){;;if(true){return 1;}else{return 2;}}\n";
        let output = simplify(source).unwrap();

        assert!(!output.contains("if(true)"));
        assert!(!output.contains("return 2"));
        assert!(!output.contains(";;"));
        assert!(output.contains("return 1"));
        assert_eq!(newline_count(&output), newline_count(source));
    }

    #[test]
    fn removes_false_loops_and_unreachable_runtime_statements() {
        let source = "int main(){while(false){return 1;}int value=2;return value;value++;}\n";
        let output = simplify(source).unwrap();

        assert!(!output.contains("while(false)"));
        assert!(!output.contains("value++"));
        assert!(output.contains("return value"));
    }

    #[test]
    fn preserves_branches_with_jump_targets() {
        let source = "int main(){goto done;if(false){done:return 1;}return 0;}\n";
        let output = simplify(source).unwrap();

        assert!(output.contains("if(false)"));
        assert!(output.contains("done:"));
    }

    #[test]
    fn preserves_preprocessor_effects_in_constant_branches() {
        let source = "int main(){if(false){\n#define VALUE 1\nreturn VALUE;}return 0;}\n";
        let output = simplify(source).unwrap();

        assert!(output.contains("if(false)"));
        assert!(output.contains("#define VALUE"));
    }
}

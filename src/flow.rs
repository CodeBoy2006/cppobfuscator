use tree_sitter::Node;

use crate::random::SplitMix64;
use crate::rewrite::{self, TextEdit};
use crate::{ObfuscationError, Profile, syntax};

const FLOW_SEED_SALT: u64 = 0xF10A_5EED_1A11_1E55;
const MAX_STATEMENTS_PER_SHARD: usize = 3;

pub(crate) fn split(
    source: &str,
    root: Node<'_>,
    profile: Profile,
    seed: u64,
) -> Result<String, ObfuscationError> {
    if !profile.splits_execution_flow() {
        return Ok(source.to_string());
    }

    let expected_newlines = newline_count(source);
    let mut edits = Vec::new();
    let mut rng = SplitMix64::new(seed ^ FLOW_SEED_SALT);
    collect_function_edits(root, source, &mut rng, &mut edits);
    if edits.is_empty() {
        return Ok(source.to_string());
    }

    let rewritten = rewrite::apply_tracked(source, &edits)?;
    if newline_count(&rewritten.output) != expected_newlines {
        return Err(ObfuscationError::Validation(
            "inline flow splitting changed the physical line count".to_string(),
        ));
    }

    let tree = syntax::parse(&rewritten.output)?;
    validate_replacements(
        &rewritten.output,
        tree.root_node(),
        &rewritten.replacement_ranges,
    )?;
    Ok(rewritten.output)
}

fn collect_function_edits(
    node: Node<'_>,
    source: &str,
    rng: &mut SplitMix64,
    edits: &mut Vec<TextEdit>,
) {
    if node.kind() == "function_definition" {
        if function_is_eligible(node, source)
            && let Some(body) = node.child_by_field_name("body")
            && body.kind() == "compound_statement"
        {
            collect_compound_edits(body, source, rng, edits);
        }
        return;
    }

    if node.kind().starts_with("preproc_") {
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_function_edits(child, source, rng, edits);
    }
}

fn collect_compound_edits(
    node: Node<'_>,
    source: &str,
    rng: &mut SplitMix64,
    edits: &mut Vec<TextEdit>,
) {
    if matches!(node.kind(), "lambda_expression" | "function_definition") {
        return;
    }

    if node.kind() == "compound_statement" {
        collect_statement_runs(node, source, rng, edits);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "expression_statement" {
            collect_compound_edits(child, source, rng, edits);
        }
    }
}

fn collect_statement_runs(
    compound: Node<'_>,
    source: &str,
    rng: &mut SplitMix64,
    edits: &mut Vec<TextEdit>,
) {
    let mut run = Vec::new();
    let mut cursor = compound.walk();
    for child in compound.named_children(&mut cursor) {
        if expression_statement_is_eligible(child, source) {
            run.push(child);
        } else {
            flush_run(&run, source, rng, edits);
            run.clear();
        }
    }
    flush_run(&run, source, rng, edits);
}

fn flush_run(run: &[Node<'_>], source: &str, rng: &mut SplitMix64, edits: &mut Vec<TextEdit>) {
    let mut offset = 0;
    while offset < run.len() {
        let remaining = run.len() - offset;
        let maximum = remaining.min(MAX_STATEMENTS_PER_SHARD);
        let chunk_size = 1 + (rng.next() as usize % maximum);
        let first = run[offset];
        let last = run[offset + chunk_size - 1];
        let original = &source[first.start_byte()..last.end_byte()];
        let replacement = if rng.one_in(2) {
            format!("([&](){{{original}}}());")
        } else {
            format!("([&]()->void{{{original}}}());")
        };
        edits.push(TextEdit {
            start: first.start_byte(),
            end: last.end_byte(),
            replacement,
        });
        offset += chunk_size;
    }
}

fn function_is_eligible(node: Node<'_>, source: &str) -> bool {
    let Some(body) = node.child_by_field_name("body") else {
        return false;
    };
    if body.kind() != "compound_statement" {
        return false;
    }

    for index in 0..node.child_count() {
        let Some(child) = node.child(index as u32) else {
            continue;
        };
        if child == body {
            continue;
        }
        if matches!(child.kind(), "constexpr" | "consteval")
            || (matches!(child.kind(), "storage_class_specifier" | "type_qualifier")
                && matches!(node_text(child, source).trim(), "constexpr" | "consteval"))
        {
            return false;
        }
    }

    !contains_function_barrier(node, source)
}

fn contains_function_barrier(node: Node<'_>, source: &str) -> bool {
    if node.kind().starts_with("preproc_")
        || is_anonymous_union(node)
        || is_gnu_label_address(node, source)
        || matches!(
            node.kind(),
            "co_await"
                | "co_await_expression"
                | "co_return"
                | "co_return_statement"
                | "co_yield"
                | "co_yield_statement"
                | "structured_binding_declarator"
                | "variadic_parameter_declaration"
                | "seh_leave_statement"
                | "seh_try_statement"
        )
    {
        return true;
    }

    if node.kind() == "storage_class_specifier" && node_text(node, source).trim() == "register" {
        return true;
    }

    if matches!(
        node.kind(),
        "identifier" | "field_identifier" | "namespace_identifier" | "type_identifier"
    ) && frame_sensitive_identifier(node_text(node, source))
    {
        return true;
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| contains_function_barrier(child, source))
}

fn expression_statement_is_eligible(node: Node<'_>, source: &str) -> bool {
    if node.kind() != "expression_statement" {
        return false;
    }

    let mut cursor = node.walk();
    if node
        .named_children(&mut cursor)
        .all(|child| child.kind() == "comment")
    {
        return false;
    }

    !contains_expression_barrier(node, source)
}

fn contains_expression_barrier(node: Node<'_>, source: &str) -> bool {
    if node.kind().starts_with("preproc_")
        || matches!(
            node.kind(),
            "lambda_expression"
                | "co_await"
                | "co_await_expression"
                | "co_yield"
                | "co_yield_statement"
                | "gnu_asm_expression"
                | "requires_expression"
                | "statement_expression"
        )
    {
        return true;
    }

    if matches!(
        node.kind(),
        "identifier" | "field_identifier" | "namespace_identifier" | "type_identifier"
    ) && frame_sensitive_identifier(node_text(node, source))
    {
        return true;
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| contains_expression_barrier(child, source))
}

fn frame_sensitive_identifier(name: &str) -> bool {
    matches!(
        name,
        "__func__"
            | "__FUNCTION__"
            | "__PRETTY_FUNCTION__"
            | "__FUNCSIG__"
            | "__FUNCDNAME__"
            | "__builtin_FUNCTION"
            | "assert"
            | "source_location"
            | "alloca"
            | "_alloca"
            | "__builtin_alloca"
            | "__builtin_alloca_with_align"
            | "setjmp"
            | "_setjmp"
            | "sigsetjmp"
            | "longjmp"
            | "_longjmp"
            | "siglongjmp"
            | "va_start"
            | "va_arg"
            | "va_end"
            | "va_copy"
            | "__builtin_va_start"
            | "__builtin_va_arg"
            | "__builtin_va_end"
            | "__builtin_va_copy"
            | "__builtin_next_arg"
            | "__builtin_saveregs"
            | "__builtin_frame_address"
            | "__builtin_return_address"
            | "__builtin_stack_address"
            | "__builtin_dwarf_cfa"
            | "__builtin_apply_args"
            | "__builtin_apply"
            | "__builtin_return"
            | "_AddressOfReturnAddress"
            | "_ReturnAddress"
    )
}

fn is_anonymous_union(node: Node<'_>) -> bool {
    node.kind() == "union_specifier" && node.child_by_field_name("name").is_none()
}

fn is_gnu_label_address(node: Node<'_>, source: &str) -> bool {
    node.kind() == "pointer_expression" && node_text(node, source).trim_start().starts_with("&&")
}

fn validate_replacements(
    source: &str,
    root: Node<'_>,
    replacement_ranges: &[(usize, usize)],
) -> Result<(), ObfuscationError> {
    for &(start, end) in replacement_ranges {
        let Some(statement) = syntax::exact_named_node(root, start, end) else {
            return Err(ObfuscationError::Validation(format!(
                "inline flow range {start}..{end} is not an exact AST node"
            )));
        };
        if statement.kind() != "expression_statement"
            || !contains_generated_lambda(statement, source)
        {
            return Err(ObfuscationError::Validation(format!(
                "inline flow range {start}..{end} did not parse as a lambda invocation"
            )));
        }
    }
    Ok(())
}

fn contains_generated_lambda(node: Node<'_>, source: &str) -> bool {
    if node.kind() == "lambda_expression" {
        return node
            .child_by_field_name("captures")
            .is_some_and(|captures| node_text(captures, source) == "[&]");
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| contains_generated_lambda(child, source))
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
    fn splits_expression_runs_and_preserves_lines() {
        let source = "int f(int x) {\n  x += 1;\n  x *= 2;\n  return x;\n}\n";
        let tree = syntax::parse(source).unwrap();
        let output = split(source, tree.root_node(), Profile::Maximum, 7).unwrap();

        assert!(output.contains("[&]"));
        assert_eq!(newline_count(&output), newline_count(source));
        syntax::parse(&output).unwrap();
    }

    #[test]
    fn leaves_non_maximum_profiles_unchanged() {
        let source = "int f(int x){x++;return x;}\n";
        let tree = syntax::parse(source).unwrap();

        assert_eq!(
            split(source, tree.root_node(), Profile::Balanced, 7).unwrap(),
            source
        );
    }

    #[test]
    fn skips_functions_with_capture_or_frame_sensitive_semantics() {
        let source = r#"
constexpr int first(int value) { value++; return value; }
int second() { auto [left, right] = std::pair<int, int>{1, 2}; left++; return left + right; }
int third() { void* pointer = __builtin_alloca(16); pointer = nullptr; return pointer == nullptr; }
"#;
        let tree = syntax::parse(source).unwrap();
        let output = split(source, tree.root_node(), Profile::Maximum, 7).unwrap();

        assert!(!output.contains("[&]"));
    }

    #[test]
    fn skips_functions_with_anonymous_unions_or_gnu_label_addresses() {
        let source = r#"
int first() { union { int value; }; value = 1; return value; }
int second() { void* target = nullptr; target = &&done; goto done; done: return target != nullptr; }
"#;
        let tree = syntax::parse(source).unwrap();
        let output = split(source, tree.root_node(), Profile::Maximum, 7).unwrap();

        assert!(!output.contains("[&]"));
    }
}

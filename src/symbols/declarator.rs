use tree_sitter::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityKind {
    Unknown,
    Variable,
    Function,
}

pub(super) struct DeclaratorInfo<'tree> {
    pub name: Node<'tree>,
    pub entity: EntityKind,
    pub qualified: bool,
    pub function_declarator: Option<Node<'tree>>,
}

pub(super) fn analyze_declarator(node: Node<'_>) -> Option<DeclaratorInfo<'_>> {
    if node.kind() == "structured_binding_declarator" {
        return None;
    }

    match node.kind() {
        "identifier" | "field_identifier" => Some(DeclaratorInfo {
            name: node,
            entity: EntityKind::Unknown,
            qualified: false,
            function_declarator: None,
        }),
        "qualified_identifier" => rightmost_identifier(node).map(|name| DeclaratorInfo {
            name,
            entity: EntityKind::Unknown,
            qualified: true,
            function_declarator: None,
        }),
        "operator_name" | "operator_cast" | "destructor_name" => None,
        _ => {
            let child = declarator_child(node)?;
            let mut info = analyze_declarator(child)?;
            if info.entity == EntityKind::Unknown {
                match node.kind() {
                    "function_declarator" => {
                        info.entity = EntityKind::Function;
                        info.function_declarator = Some(node);
                    }
                    "pointer_declarator" | "reference_declarator" | "array_declarator" => {
                        info.entity = EntityKind::Variable;
                    }
                    _ => {}
                }
            }
            Some(info)
        }
    }
}

pub(super) fn declarator_fields(node: Node<'_>) -> Vec<Node<'_>> {
    let mut output = Vec::new();
    for index in 0..node.child_count() {
        if node.field_name_for_child(index as u32) == Some("declarator")
            && let Some(child) = node.child(index as u32)
        {
            output.push(child);
        }
    }
    output
}

pub(super) fn declarator_name_is_qualified(node: Node<'_>) -> bool {
    if node.kind() == "qualified_identifier" {
        return true;
    }

    declarator_child(node).is_some_and(declarator_name_is_qualified)
}

pub(super) fn innermost_function_declarator(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(child) = declarator_child(node)
        && let Some(function) = innermost_function_declarator(child)
    {
        return Some(function);
    }
    (node.kind() == "function_declarator").then_some(node)
}

pub(super) fn declaration_point(declarator: Node<'_>) -> usize {
    if find_descendant(declarator, "structured_binding_declarator").is_some() {
        return declarator.end_byte();
    }

    if declarator.kind() == "init_declarator" {
        return declarator
            .child_by_field_name("declarator")
            .map_or(declarator.end_byte(), |inner| inner.end_byte());
    }

    declarator.end_byte()
}

pub(super) fn find_descendant<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    if node.kind() == kind {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(found) = find_descendant(child, kind) {
            return Some(found);
        }
    }
    None
}

fn declarator_child(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(child) = node.child_by_field_name("declarator") {
        return Some(child);
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| is_declarator_node(child.kind()))
}

fn is_declarator_node(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "field_identifier"
            | "qualified_identifier"
            | "function_declarator"
            | "pointer_declarator"
            | "reference_declarator"
            | "array_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator"
            | "variadic_declarator"
            | "init_declarator"
            | "structured_binding_declarator"
    )
}

fn rightmost_identifier(node: Node<'_>) -> Option<Node<'_>> {
    if matches!(node.kind(), "identifier" | "field_identifier") {
        return Some(node);
    }

    let mut cursor = node.walk();
    let mut result = None;
    for child in node.named_children(&mut cursor) {
        if let Some(identifier) = rightmost_identifier(child) {
            result = Some(identifier);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_function_pointer_declarators() {
        let tree =
            crate::syntax::parse("int f(int); int (*pointer)(int); int *factory(int value);")
                .unwrap();
        let root = tree.root_node();
        let mut cursor = root.walk();
        let declarations: Vec<Node<'_>> = root.named_children(&mut cursor).collect();

        let first =
            analyze_declarator(declarations[0].child_by_field_name("declarator").unwrap()).unwrap();
        let second =
            analyze_declarator(declarations[1].child_by_field_name("declarator").unwrap()).unwrap();
        let third =
            analyze_declarator(declarations[2].child_by_field_name("declarator").unwrap()).unwrap();

        assert_eq!(first.entity, EntityKind::Function);
        assert_eq!(second.entity, EntityKind::Variable);
        assert_eq!(third.entity, EntityKind::Function);
    }
}

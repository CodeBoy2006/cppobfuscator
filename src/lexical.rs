use tree_sitter::Node;

use crate::Profile;
use crate::integer::{IntegerLiteral, Radix};
use crate::random::SplitMix64;
use crate::rewrite::TextEdit;

pub(crate) fn obfuscation_edits(
    source: &str,
    root: Node<'_>,
    profile: Profile,
    seed: u64,
) -> Vec<TextEdit> {
    if !profile.obfuscates_lexical_tokens() {
        return Vec::new();
    }

    let mut edits = Vec::new();
    let mut rng = SplitMix64::new(seed ^ 0xA11C_E5ED_1EAF_BA5E);
    collect_edits(root, source, profile, false, &mut rng, &mut edits);
    edits
}

fn collect_edits(
    node: Node<'_>,
    source: &str,
    profile: Profile,
    in_preprocessor: bool,
    rng: &mut SplitMix64,
    edits: &mut Vec<TextEdit>,
) {
    let in_preprocessor = in_preprocessor || node.kind().starts_with("preproc_");
    if in_preprocessor {
        return;
    }

    let replacement = match node.kind() {
        "number_literal" => integer_replacement(node_text(node, source), rng),
        "string_content" if !is_syntax_sensitive_string(node) => {
            string_content_replacement(node_text(node, source), profile, rng)
        }
        "character" => character_replacement(node_text(node, source)),
        _ if node.child_count() == 0 => operator_replacement(node),
        _ => None,
    };

    if let Some(replacement) = replacement {
        edits.push(TextEdit {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_edits(child, source, profile, false, rng, edits);
    }
}

fn is_syntax_sensitive_string(node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if matches!(
            parent.kind(),
            "static_assert_declaration"
                | "linkage_specification"
                | "attribute"
                | "attribute_declaration"
                | "attribute_specifier"
                | "gnu_asm_expression"
        ) {
            return true;
        }
        ancestor = parent.parent();
    }
    false
}

fn integer_replacement(text: &str, rng: &mut SplitMix64) -> Option<String> {
    let literal = IntegerLiteral::parse(text)?;
    // Below INT_MAX every supported radix selects the same first standard type.
    if literal.value > i32::MAX as u128 {
        return None;
    }

    let radix = match rng.next() % 3 {
        0 => Radix::Binary,
        1 => Radix::Octal,
        _ => Radix::Hexadecimal,
    };
    let replacement = literal.render(radix);
    (replacement != text).then_some(replacement)
}

fn string_content_replacement(
    text: &str,
    profile: Profile,
    rng: &mut SplitMix64,
) -> Option<String> {
    let mut output = String::with_capacity(text.len());
    let mut changed = false;

    for character in text.chars() {
        let encode = character.is_ascii()
            && !character.is_ascii_control()
            && (profile == Profile::Maximum || !rng.one_in(3));
        if encode {
            // Three octal digits cannot consume characters from the next segment.
            output.push_str(&format!("\\{:03o}", character as u8));
            changed = true;
        } else {
            output.push(character);
        }
    }

    changed.then_some(output)
}

fn character_replacement(text: &str) -> Option<String> {
    let mut characters = text.chars();
    let character = characters.next()?;
    if characters.next().is_some() || !character.is_ascii() || character.is_ascii_control() {
        return None;
    }
    Some(format!("\\{:03o}", character as u8))
}

fn operator_replacement(node: Node<'_>) -> Option<String> {
    let parent = node.parent()?;
    if !matches!(
        parent.kind(),
        "assignment_expression" | "binary_expression" | "fold_expression" | "unary_expression"
    ) {
        // Pointer/reference uses of `&` are valid C++ alternative tokens, but
        // tree-sitter-cpp does not accept those spellings in declarators.
        return None;
    }
    let replacement = match node.kind() {
        "&&" => " and ",
        "||" => " or ",
        "!" => "not ",
        "&" => " bitand ",
        "|" => " bitor ",
        "^" => " xor ",
        "~" => "compl ",
        "&=" => " and_eq ",
        "|=" => " or_eq ",
        "^=" => " xor_eq ",
        "!=" => " not_eq ",
        _ => return None,
    };
    Some(replacement.to_string())
}

fn node_text<'source>(node: Node<'_>, source: &'source str) -> &'source str {
    node.utf8_text(source.as_bytes()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_ascii_literal_content() {
        let mut rng = SplitMix64::new(1);
        assert_eq!(
            string_content_replacement("Az", Profile::Maximum, &mut rng),
            Some("\\101\\172".to_string())
        );
        assert_eq!(character_replacement("A"), Some("\\101".to_string()));
    }
}

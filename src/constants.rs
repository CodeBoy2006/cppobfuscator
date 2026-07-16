use tree_sitter::Node;

use crate::integer::{IntegerLiteral, IntegerType};
use crate::random::SplitMix64;
use crate::rewrite::TextEdit;
use crate::syntax;
use crate::{ObfuscationError, Profile};

const WORD_MASK: u32 = u32::MAX;
const WORD_MASK_LITERAL: &str = "0xffffffffULL";

pub(crate) fn encoding_edits(
    source: &str,
    root: Node<'_>,
    profile: Profile,
    seed: u64,
) -> Result<Vec<TextEdit>, ObfuscationError> {
    if !profile.obfuscates_arithmetic_constants() {
        return Ok(Vec::new());
    }

    let mut edits = Vec::new();
    let mut rng = SplitMix64::new(seed ^ 0xC057_A17C_3E71_5EED);
    collect_edits(root, source, false, &mut rng, &mut edits)?;
    Ok(edits)
}

pub(crate) fn validate_replacements(
    source: &str,
    root: Node<'_>,
    replacement_ranges: &[(usize, usize)],
) -> Result<(), ObfuscationError> {
    for &(start, end) in replacement_ranges {
        let Some(node) = syntax::exact_named_node(root, start, end) else {
            return Err(ObfuscationError::Validation(format!(
                "constant rewrite range {start}..{end} is not an exact AST node"
            )));
        };
        if node.kind() != "parenthesized_expression" {
            return Err(ObfuscationError::Validation(format!(
                "constant rewrite range {start}..{end} parsed as {}",
                node.kind()
            )));
        }

        let mut cursor = node.walk();
        let mut children = node.named_children(&mut cursor);
        let generated_cast = children.next();
        if children.next().is_some()
            || !generated_cast.is_some_and(|cast| is_generated_cast(cast, source))
        {
            return Err(ObfuscationError::Validation(format!(
                "constant rewrite range {start}..{end} is not a generated static cast"
            )));
        }
    }
    Ok(())
}

fn is_generated_cast(node: Node<'_>, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "template_function" {
        return false;
    }
    function
        .child_by_field_name("name")
        .and_then(|name| name.utf8_text(source.as_bytes()).ok())
        == Some("static_cast")
        && node
            .child_by_field_name("arguments")
            .is_some_and(|arguments| arguments.kind() == "argument_list")
}

fn collect_edits(
    node: Node<'_>,
    source: &str,
    in_preprocessor: bool,
    rng: &mut SplitMix64,
    edits: &mut Vec<TextEdit>,
) -> Result<(), ObfuscationError> {
    let in_preprocessor = in_preprocessor || node.kind().starts_with("preproc_");
    if in_preprocessor {
        return Ok(());
    }

    if node.kind() == "number_literal" {
        if let Some(replacement) = integer_replacement(node_text(node, source), rng)? {
            edits.push(TextEdit {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement,
            });
        }
        return Ok(());
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_edits(child, source, false, rng, edits)?;
    }
    Ok(())
}

fn integer_replacement(
    text: &str,
    rng: &mut SplitMix64,
) -> Result<Option<String>, ObfuscationError> {
    let Some(literal) = IntegerLiteral::parse(text) else {
        return Ok(None);
    };
    let Some(integer_type) = literal.exact_arithmetic_type() else {
        return Ok(None);
    };
    // Since C++11, an integer literal with value zero has special null-pointer
    // semantics that an equivalent integral constant expression does not have.
    if literal.value == 0 {
        return Ok(None);
    }

    let value = u32::try_from(literal.value).map_err(|_| {
        ObfuscationError::Validation("eligible integer did not fit the 32-bit encoder".to_string())
    })?;
    let encoding = Encoding::new(value, rng);
    if encoding.evaluate() != value {
        return Err(ObfuscationError::Validation(
            "generated arithmetic constant did not round-trip".to_string(),
        ));
    }
    Ok(Some(encoding.render(integer_type)))
}

fn node_text<'source>(node: Node<'_>, source: &'source str) -> &'source str {
    node.utf8_text(source.as_bytes()).unwrap_or_default()
}

#[derive(Debug, Clone, Copy)]
enum Encoding {
    Affine {
        encoded: u32,
        inverse: u32,
        addend: u32,
        key: u32,
    },
    RotatedAffine {
        encoded: u32,
        inverse: u32,
        addend: u32,
        key: u32,
        rotation: u32,
    },
    Split {
        left: u32,
        left_key: u32,
        right: u32,
        right_key: u32,
        mask: u32,
    },
}

impl Encoding {
    fn new(value: u32, rng: &mut SplitMix64) -> Self {
        let family = match rng.next() % 3 {
            0 => EncodingFamily::Affine,
            1 => EncodingFamily::RotatedAffine,
            _ => EncodingFamily::Split,
        };
        Self::new_with_family(value, family, rng)
    }

    fn new_with_family(value: u32, family: EncodingFamily, rng: &mut SplitMix64) -> Self {
        match family {
            EncodingFamily::Affine => {
                let (encoded, inverse, addend, key) = affine_parameters(value, rng);
                Self::Affine {
                    encoded,
                    inverse,
                    addend,
                    key,
                }
            }
            EncodingFamily::RotatedAffine => {
                let (encoded, inverse, addend, key) = affine_parameters(value, rng);
                let rotation = 5 + (rng.next() % 23) as u32;
                Self::RotatedAffine {
                    encoded: encoded.rotate_left(rotation),
                    inverse,
                    addend,
                    key,
                    rotation,
                }
            }
            EncodingFamily::Split => {
                let left_key = random_word(rng);
                let right_key = random_word(rng);
                let mut mask = random_word(rng);
                if matches!(mask, 0 | WORD_MASK) {
                    mask = 0x5a5a_a5a5;
                }
                Self::Split {
                    left: value ^ left_key,
                    left_key,
                    right: value ^ right_key,
                    right_key,
                    mask,
                }
            }
        }
    }

    fn evaluate(self) -> u32 {
        match self {
            Self::Affine {
                encoded,
                inverse,
                addend,
                key,
            } => decode_affine(encoded, inverse, addend, key),
            Self::RotatedAffine {
                encoded,
                inverse,
                addend,
                key,
                rotation,
            } => decode_affine(encoded.rotate_right(rotation), inverse, addend, key),
            Self::Split {
                left,
                left_key,
                right,
                right_key,
                mask,
            } => ((left ^ left_key) & mask) | ((right ^ right_key) & !mask),
        }
    }

    fn render(self, integer_type: IntegerType) -> String {
        match self {
            Self::Affine {
                encoded,
                inverse,
                addend,
                key,
            } => render_affine(integer_type, word_literal(encoded), inverse, addend, key),
            Self::RotatedAffine {
                encoded,
                inverse,
                addend,
                key,
                rotation,
            } => {
                let encoded = word_literal(encoded);
                let rotated = format!(
                    "((({encoded} >> {rotation}) | ({encoded} << {})) & {WORD_MASK_LITERAL})",
                    32 - rotation
                );
                render_affine(integer_type, rotated, inverse, addend, key)
            }
            Self::Split {
                left,
                left_key,
                right,
                right_key,
                mask,
            } => format!(
                "(static_cast<{}>((({} ^ {}) & {}) | (({} ^ {}) & {})))",
                integer_type.cpp_name(),
                word_literal(left),
                word_literal(left_key),
                word_literal(mask),
                word_literal(right),
                word_literal(right_key),
                word_literal(!mask)
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EncodingFamily {
    Affine,
    RotatedAffine,
    Split,
}

fn affine_parameters(value: u32, rng: &mut SplitMix64) -> (u32, u32, u32, u32) {
    let mut multiplier = random_word(rng) | 1;
    if multiplier == 1 {
        multiplier = 0x9e37_79b1;
    }
    let inverse = modular_inverse(multiplier);
    let addend = random_word(rng);
    let key = random_word(rng);
    let encoded = (value ^ key).wrapping_sub(addend).wrapping_mul(multiplier);
    (encoded, inverse, addend, key)
}

fn decode_affine(encoded: u32, inverse: u32, addend: u32, key: u32) -> u32 {
    encoded.wrapping_mul(inverse).wrapping_add(addend) ^ key
}

fn modular_inverse(value: u32) -> u32 {
    debug_assert!(value % 2 == 1);
    let mut inverse = 1u32;
    for _ in 0..6 {
        inverse = inverse.wrapping_mul(2u32.wrapping_sub(value.wrapping_mul(inverse)));
    }
    inverse
}

fn random_word(rng: &mut SplitMix64) -> u32 {
    (rng.next() as u32) ^ 0xa5a5_5a5a
}

fn render_affine(
    integer_type: IntegerType,
    encoded: String,
    inverse: u32,
    addend: u32,
    key: u32,
) -> String {
    format!(
        "(static_cast<{}>((((({encoded} * {}) & {WORD_MASK_LITERAL}) + {}) & {WORD_MASK_LITERAL}) ^ {}))",
        integer_type.cpp_name(),
        word_literal(inverse),
        word_literal(addend),
        word_literal(key)
    )
}

fn word_literal(value: u32) -> String {
    format!("0x{value:08x}ULL")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modular_inverses_round_trip_all_odd_samples() {
        for value in [1, 3, 5, 0x9e37_79b1, u32::MAX] {
            assert_eq!(value.wrapping_mul(modular_inverse(value)), 1);
        }
    }

    #[test]
    fn every_encoding_family_round_trips() {
        let values = [1, 2, 42, 1_000_000_007, i32::MAX as u32];
        let families = [
            EncodingFamily::Affine,
            EncodingFamily::RotatedAffine,
            EncodingFamily::Split,
        ];

        for (seed, value) in values.into_iter().enumerate() {
            for family in families {
                let mut rng = SplitMix64::new(seed as u64 + 1);
                let encoding = Encoding::new_with_family(value, family, &mut rng);
                assert_eq!(encoding.evaluate(), value);
                let source = format!(
                    "constexpr int value = {};",
                    encoding.render(IntegerType::Int)
                );
                crate::syntax::parse(&source).unwrap();
            }
        }
    }

    #[test]
    fn zero_is_not_replaced_because_it_may_be_a_null_pointer_constant() {
        let mut rng = SplitMix64::new(1);
        assert_eq!(integer_replacement("0", &mut rng).unwrap(), None);
    }
}

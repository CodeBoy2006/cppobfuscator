use tree_sitter::Node;

use crate::ObfuscationError;
use crate::Profile;
use crate::random::SplitMix64;
use crate::rewrite::{self, TextEdit};

pub(crate) fn render(
    source: &str,
    root: Node<'_>,
    compact: bool,
    strip_comments: bool,
    profile: Profile,
    seed: u64,
) -> Result<String, ObfuscationError> {
    if compact {
        compact_source(
            source,
            root,
            strip_comments,
            profile.inserts_separator_comments(),
            seed,
        )
    } else if strip_comments {
        strip_comments_preserving_layout(source, root)
    } else {
        Ok(source.to_string())
    }
}

fn compact_source(
    source: &str,
    root: Node<'_>,
    strip_comments: bool,
    insert_separator_comments: bool,
    seed: u64,
) -> Result<String, ObfuscationError> {
    let mut leaves = Vec::new();
    collect_leaves(root, false, &mut leaves);

    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    let mut pending = PendingSeparator::default();
    let mut previous_preprocessor = false;
    let mut previous_alternative_operator = false;
    let mut previous_slash = false;
    let mut rng = SplitMix64::new(seed ^ 0xC0A9_AC71_C0DE_5EED);

    for leaf in leaves {
        pending.feed_gap(&source[cursor..leaf.node.start_byte()])?;
        let text = &source[leaf.node.start_byte()..leaf.node.end_byte()];
        cursor = leaf.node.end_byte();

        if strip_comments && leaf.node.kind() == "comment" {
            pending.feed_comment(text);
            continue;
        }

        let current_alternative_operator = is_alternative_operator(leaf.node.kind());
        // Tree-sitter has stricter comment boundaries around alternative tokens,
        // and a comment immediately after `/` would begin with `//` in raw text.
        let noise_allowed = insert_separator_comments
            && !previous_preprocessor
            && !leaf.in_preprocessor
            && !previous_alternative_operator
            && !current_alternative_operator
            && !previous_slash;
        pending.flush(&mut output, noise_allowed, &mut rng);
        output.push_str(text);
        previous_preprocessor = leaf.in_preprocessor;
        previous_alternative_operator = current_alternative_operator;
        previous_slash = leaf.node.kind() == "/";
    }

    pending.feed_gap(&source[cursor..])?;
    pending.finish(&mut output, &mut rng);
    Ok(output)
}

fn is_alternative_operator(kind: &str) -> bool {
    matches!(
        kind,
        "and"
            | "or"
            | "not"
            | "bitand"
            | "bitor"
            | "xor"
            | "compl"
            | "and_eq"
            | "or_eq"
            | "xor_eq"
            | "not_eq"
    )
}

fn strip_comments_preserving_layout(
    source: &str,
    root: Node<'_>,
) -> Result<String, ObfuscationError> {
    let mut edits = Vec::new();
    collect_comment_edits(root, source, &mut edits);
    rewrite::apply(source, &edits)
}

struct Leaf<'tree> {
    node: Node<'tree>,
    in_preprocessor: bool,
}

fn collect_leaves<'tree>(node: Node<'tree>, in_preprocessor: bool, leaves: &mut Vec<Leaf<'tree>>) {
    let in_preprocessor = in_preprocessor || node.kind().starts_with("preproc_");
    if node.child_count() == 0 {
        leaves.push(Leaf {
            node,
            in_preprocessor,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_leaves(child, in_preprocessor, leaves);
    }
}

fn collect_comment_edits(node: Node<'_>, source: &str, edits: &mut Vec<TextEdit>) {
    if node.kind() == "comment" {
        let text = &source[node.start_byte()..node.end_byte()];
        let newline_count = text.bytes().filter(|byte| *byte == b'\n').count();
        let replacement = if newline_count == 0 {
            " ".to_string()
        } else {
            "\n".repeat(newline_count)
        };
        edits.push(TextEdit {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_comment_edits(child, source, edits);
    }
}

#[derive(Default)]
struct PendingSeparator {
    saw_space: bool,
    newline_count: usize,
    line_continuations: usize,
}

impl PendingSeparator {
    fn feed_gap(&mut self, gap: &str) -> Result<(), ObfuscationError> {
        let bytes = gap.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' if bytes.get(index + 1) == Some(&b'\n') => {
                    self.line_continuations += 1;
                    index += 2;
                }
                b'\\'
                    if bytes.get(index + 1) == Some(&b'\r')
                        && bytes.get(index + 2) == Some(&b'\n') =>
                {
                    self.line_continuations += 1;
                    index += 3;
                }
                b'\n' => {
                    self.newline_count += 1;
                    index += 1;
                }
                b'\r' | b' ' | b'\t' | 0x0B | 0x0C => {
                    self.saw_space = true;
                    index += 1;
                }
                _ => {
                    return Err(ObfuscationError::Validation(
                        "Tree-sitter left non-trivia bytes between syntax leaves".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn feed_comment(&mut self, comment: &str) {
        let newline_count = comment.bytes().filter(|byte| *byte == b'\n').count();
        if newline_count > 0 {
            self.newline_count += newline_count;
        } else {
            self.saw_space = true;
        }
    }

    fn flush(&mut self, output: &mut String, noise_allowed: bool, rng: &mut SplitMix64) {
        if output.is_empty() {
            for _ in 0..self.line_continuations {
                output.push_str("\\\n");
            }
            for _ in 0..self.newline_count {
                output.push('\n');
            }
        } else {
            if self.line_continuations > 0 {
                output.push(' ');
                for _ in 0..self.line_continuations {
                    output.push_str("\\\n");
                }
            }
            if self.newline_count > 0 {
                for _ in 0..self.newline_count {
                    output.push('\n');
                }
            } else if self.saw_space && self.line_continuations == 0 {
                if noise_allowed {
                    output.push_str(if rng.one_in(2) { "/**/" } else { "/*_*/" });
                } else {
                    output.push(' ');
                }
            }
        }
        self.saw_space = false;
        self.newline_count = 0;
        self.line_continuations = 0;
    }

    fn finish(&mut self, output: &mut String, rng: &mut SplitMix64) {
        if self.line_continuations > 0 {
            self.flush(output, false, rng);
        } else {
            for _ in 0..self.newline_count {
                output.push('\n');
            }
        }
    }
}

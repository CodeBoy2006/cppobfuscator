use tree_sitter::Node;

use crate::ObfuscationError;
use crate::rewrite::{self, TextEdit};

pub(crate) fn render(
    source: &str,
    root: Node<'_>,
    compact: bool,
    strip_comments: bool,
) -> Result<String, ObfuscationError> {
    if compact {
        compact_source(source, root, strip_comments)
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
) -> Result<String, ObfuscationError> {
    let mut leaves = Vec::new();
    collect_leaves(root, &mut leaves);

    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    let mut pending = PendingSeparator::default();

    for leaf in leaves {
        pending.feed_gap(&source[cursor..leaf.start_byte()])?;
        let text = &source[leaf.start_byte()..leaf.end_byte()];
        cursor = leaf.end_byte();

        if strip_comments && leaf.kind() == "comment" {
            pending.feed_comment(text);
            continue;
        }

        pending.flush(&mut output);
        output.push_str(text);
    }

    pending.feed_gap(&source[cursor..])?;
    pending.finish(&mut output);
    Ok(output)
}

fn strip_comments_preserving_layout(
    source: &str,
    root: Node<'_>,
) -> Result<String, ObfuscationError> {
    let mut edits = Vec::new();
    collect_comment_edits(root, source, &mut edits);
    rewrite::apply(source, &edits)
}

fn collect_leaves<'tree>(node: Node<'tree>, leaves: &mut Vec<Node<'tree>>) {
    if node.child_count() == 0 {
        leaves.push(node);
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_leaves(child, leaves);
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
    saw_newline: bool,
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
                    self.saw_newline = true;
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
        if comment.contains('\n') {
            self.saw_newline = true;
        } else {
            self.saw_space = true;
        }
    }

    fn flush(&mut self, output: &mut String) {
        if !output.is_empty() {
            if self.line_continuations > 0 {
                output.push(' ');
                for _ in 0..self.line_continuations {
                    output.push_str("\\\n");
                }
            }
            if self.saw_newline {
                output.push('\n');
            } else if self.saw_space && self.line_continuations == 0 {
                output.push(' ');
            }
        }
        self.saw_space = false;
        self.saw_newline = false;
        self.line_continuations = 0;
    }

    fn finish(&mut self, output: &mut String) {
        if self.line_continuations > 0 {
            self.flush(output);
        } else if self.saw_newline && !output.ends_with('\n') {
            output.push('\n');
        }
    }
}

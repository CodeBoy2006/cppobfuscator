use std::collections::BTreeMap;

use crate::ObfuscationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

pub(crate) fn apply(source: &str, edits: &[TextEdit]) -> Result<String, ObfuscationError> {
    let mut normalized = BTreeMap::<(usize, usize), &str>::new();
    for edit in edits {
        validate_range(source, edit.start, edit.end)?;
        match normalized.insert((edit.start, edit.end), &edit.replacement) {
            Some(existing) if existing != edit.replacement => {
                return Err(ObfuscationError::InvalidEdit(format!(
                    "conflicting replacements for byte range {}..{}",
                    edit.start, edit.end
                )));
            }
            _ => {}
        }
    }

    let mut previous_end = 0;
    for &(start, end) in normalized.keys() {
        if start < previous_end {
            return Err(ObfuscationError::InvalidEdit(format!(
                "overlapping byte ranges near {start}..{end}"
            )));
        }
        previous_end = end;
    }

    let mut output = source.to_string();
    for (&(start, end), replacement) in normalized.iter().rev() {
        output.replace_range(start..end, replacement);
    }
    Ok(output)
}

fn validate_range(source: &str, start: usize, end: usize) -> Result<(), ObfuscationError> {
    if start > end || end > source.len() {
        return Err(ObfuscationError::InvalidEdit(format!(
            "byte range {start}..{end} is outside the source"
        )));
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err(ObfuscationError::InvalidEdit(format!(
            "byte range {start}..{end} splits a UTF-8 code point"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_edits_from_the_end() {
        let output = apply(
            "alpha beta",
            &[
                TextEdit {
                    start: 0,
                    end: 5,
                    replacement: "x".to_string(),
                },
                TextEdit {
                    start: 6,
                    end: 10,
                    replacement: "yy".to_string(),
                },
            ],
        )
        .unwrap();

        assert_eq!(output, "x yy");
    }

    #[test]
    fn rejects_overlapping_edits() {
        let error = apply(
            "abcdef",
            &[
                TextEdit {
                    start: 1,
                    end: 4,
                    replacement: "x".to_string(),
                },
                TextEdit {
                    start: 3,
                    end: 5,
                    replacement: "y".to_string(),
                },
            ],
        )
        .unwrap_err();

        assert!(matches!(error, ObfuscationError::InvalidEdit(_)));
    }
}

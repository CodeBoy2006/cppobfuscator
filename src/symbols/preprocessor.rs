pub(super) fn has_token_paste(text: &str) -> bool {
    scan_macro_operators(text).token_paste
}

pub(super) fn has_stringification(text: &str) -> bool {
    scan_macro_operators(text).stringification
}

fn scan_macro_operators(text: &str) -> MacroOperators {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut operators = MacroOperators::default();

    while index < bytes.len() {
        if let Some(end) = raw_string_end(bytes, index) {
            index = end;
            continue;
        }

        match bytes[index] {
            b'"' | b'\'' => {
                index = quoted_end(bytes, index, bytes[index]);
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = line_comment_end(bytes, index + 2);
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = block_comment_end(bytes, index + 2);
            }
            b'#' if bytes.get(index + 1) == Some(&b'#') => {
                operators.token_paste = true;
                index += 2;
            }
            b'#' => {
                operators.stringification = true;
                index += 1;
            }
            b'%' if bytes[index..].starts_with(b"%:%:") => {
                operators.token_paste = true;
                index += 4;
            }
            b'%' if bytes[index..].starts_with(b"%:") => {
                operators.stringification = true;
                index += 2;
            }
            _ => index += 1,
        }
    }

    operators
}

#[derive(Default)]
struct MacroOperators {
    token_paste: bool,
    stringification: bool,
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    const PREFIXES: [&[u8]; 5] = [b"u8R\"", b"uR\"", b"UR\"", b"LR\"", b"R\""];
    let prefix = PREFIXES
        .iter()
        .find(|prefix| bytes[start..].starts_with(prefix))?;
    let delimiter_start = start + prefix.len();
    let mut opening_parenthesis = delimiter_start;

    while opening_parenthesis < bytes.len()
        && opening_parenthesis - delimiter_start <= 16
        && bytes[opening_parenthesis] != b'('
    {
        if matches!(
            bytes[opening_parenthesis],
            b' ' | b'\t' | b'\n' | b'\r' | b'\\' | b')'
        ) {
            return None;
        }
        opening_parenthesis += 1;
    }
    if bytes.get(opening_parenthesis) != Some(&b'(') {
        return None;
    }

    let delimiter = &bytes[delimiter_start..opening_parenthesis];
    let mut cursor = opening_parenthesis + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b')' {
            let delimiter_end = cursor + 1 + delimiter.len();
            if bytes
                .get(cursor + 1..delimiter_end)
                .is_some_and(|candidate| candidate == delimiter)
                && bytes.get(delimiter_end) == Some(&b'"')
            {
                return Some(delimiter_end + 1);
            }
        }
        cursor += 1;
    }

    Some(bytes.len())
}

fn quoted_end(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            character if character == quote => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

fn line_comment_end(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    index
}

fn block_comment_end(bytes: &[u8], mut index: usize) -> usize {
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_both_token_paste_spellings() {
        assert!(has_token_paste("left ## right"));
        assert!(has_token_paste("left %:%: right"));
    }

    #[test]
    fn ignores_hashes_inside_literals_and_comments() {
        assert!(!has_token_paste(r###""##" '##' R"tag(##)tag""###));
        assert!(!has_token_paste("/* ## */ value // ##"));
    }

    #[test]
    fn finds_stringification_but_ignores_literals() {
        assert!(has_stringification("#value"));
        assert!(has_stringification("%: value"));
        assert!(!has_stringification(r###""#value" R"tag(%:)tag""###));
        assert!(!has_stringification("/* #value */ item"));
    }
}

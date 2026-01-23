use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    Keyword,
    Number,
    StringLiteral,
    CharLiteral,
    Operator,
    Whitespace,
    Comment,
    Preprocessor,
    Other,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

impl Token {
    pub fn is_whitespace_or_comment(&self) -> bool {
        matches!(self.kind, TokenKind::Whitespace | TokenKind::Comment)
    }

    pub fn is_word_like(&self) -> bool {
        matches!(self.kind, TokenKind::Identifier | TokenKind::Keyword | TokenKind::Number)
    }
}

pub fn cpp_keywords() -> HashSet<&'static str> {
    [
        "alignas",
        "alignof",
        "and",
        "and_eq",
        "asm",
        "atomic_cancel",
        "atomic_commit",
        "atomic_noexcept",
        "auto",
        "bitand",
        "bitor",
        "bool",
        "break",
        "case",
        "catch",
        "char",
        "char8_t",
        "char16_t",
        "char32_t",
        "class",
        "compl",
        "concept",
        "const",
        "consteval",
        "constexpr",
        "constinit",
        "const_cast",
        "continue",
        "co_await",
        "co_return",
        "co_yield",
        "decltype",
        "default",
        "delete",
        "do",
        "double",
        "dynamic_cast",
        "else",
        "enum",
        "explicit",
        "export",
        "extern",
        "false",
        "float",
        "for",
        "friend",
        "goto",
        "if",
        "inline",
        "int",
        "long",
        "mutable",
        "namespace",
        "new",
        "noexcept",
        "not",
        "not_eq",
        "nullptr",
        "operator",
        "or",
        "or_eq",
        "private",
        "protected",
        "public",
        "register",
        "reinterpret_cast",
        "requires",
        "return",
        "short",
        "signed",
        "sizeof",
        "static",
        "static_assert",
        "static_cast",
        "struct",
        "switch",
        "synchronized",
        "template",
        "this",
        "thread_local",
        "throw",
        "true",
        "try",
        "typedef",
        "typeid",
        "typename",
        "union",
        "unsigned",
        "using",
        "virtual",
        "void",
        "volatile",
        "wchar_t",
        "while",
        "xor",
        "xor_eq",
    ]
    .into_iter()
    .collect()
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let keywords = cpp_keywords();
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line_start = true;

    while i < bytes.len() {
        let ch = bytes[i] as char;

        if line_start {
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c == ' ' || c == '\t' || c == '\r' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j < bytes.len() && bytes[j] as char == '#' {
                let start = i;
                let mut k = j;
                while k < bytes.len() && bytes[k] as char != '\n' {
                    k += 1;
                }
                if k < bytes.len() {
                    k += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Preprocessor,
                    text: input[start..k].to_string(),
                });
                i = k;
                line_start = true;
                continue;
            }
        }

        if ch.is_ascii_whitespace() {
            let start = i;
            let mut has_newline = false;
            while i < bytes.len() {
                let c = bytes[i] as char;
                if c.is_ascii_whitespace() {
                    if c == '\n' {
                        has_newline = true;
                    }
                    i += 1;
                } else {
                    break;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Whitespace,
                text: input[start..i].to_string(),
            });
            line_start = has_newline;
            continue;
        }

        if ch == '/' && i + 1 < bytes.len() {
            let next = bytes[i + 1] as char;
            if next == '/' {
                let start = i;
                i += 2;
                while i < bytes.len() && bytes[i] as char != '\n' {
                    i += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    text: input[start..i].to_string(),
                });
                line_start = false;
                continue;
            }
            if next == '*' {
                let start = i;
                i += 2;
                while i + 1 < bytes.len() {
                    if bytes[i] as char == '*' && bytes[i + 1] as char == '/' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                let text = input[start..i].to_string();
                let has_newline = text.contains('\n');
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    text,
                });
                line_start = has_newline;
                continue;
            }
        }

        if let Some((lit, end)) = parse_string_literal(input, i) {
            tokens.push(Token {
                kind: TokenKind::StringLiteral,
                text: lit,
            });
            i = end;
            line_start = false;
            continue;
        }

        if let Some((lit, end)) = parse_char_literal(input, i) {
            tokens.push(Token {
                kind: TokenKind::CharLiteral,
                text: lit,
            });
            i = end;
            line_start = false;
            continue;
        }

        if is_identifier_start(ch) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_identifier_char(bytes[i] as char) {
                i += 1;
            }
            let text = &input[start..i];
            let kind = if keywords.contains(text) {
                TokenKind::Keyword
            } else {
                TokenKind::Identifier
            };
            tokens.push(Token {
                kind,
                text: text.to_string(),
            });
            line_start = false;
            continue;
        }

        if ch.is_ascii_digit() || (ch == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit()) {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let c = bytes[i] as char;
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-') {
                    i += 1;
                } else {
                    break;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: input[start..i].to_string(),
            });
            line_start = false;
            continue;
        }

        if let Some((op, end)) = parse_operator(input, i) {
            tokens.push(Token {
                kind: TokenKind::Operator,
                text: op,
            });
            i = end;
            line_start = false;
            continue;
        }

        tokens.push(Token {
            kind: TokenKind::Other,
            text: ch.to_string(),
        });
        i += 1;
        line_start = false;
    }

    tokens
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn parse_string_literal(input: &str, start: usize) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    let prefixes = ["u8R\"", "uR\"", "UR\"", "LR\"", "R\"", "u8\"", "u\"", "U\"", "L\""];
    for prefix in prefixes.iter() {
        if input[start..].starts_with(prefix) {
            if prefix.ends_with("R\"") {
                return parse_raw_string(input, start, prefix.len());
            }
            return parse_escaped_string(input, start, prefix.len());
        }
    }

    if bytes[start] as char == '"' {
        return parse_escaped_string(input, start, 1);
    }

    None
}

fn parse_char_literal(input: &str, start: usize) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    let prefixes = ["u8'", "u'", "U'", "L'"];
    for prefix in prefixes.iter() {
        if input[start..].starts_with(prefix) {
            return parse_escaped_char(input, start, prefix.len());
        }
    }

    if bytes[start] as char == '\'' {
        return parse_escaped_char(input, start, 1);
    }

    None
}

fn parse_raw_string(input: &str, start: usize, prefix_len: usize) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    let mut i = start + prefix_len;
    let len = bytes.len();
    let mut delim = String::new();

    while i < len {
        let c = bytes[i] as char;
        if c == '(' {
            i += 1;
            break;
        }
        delim.push(c);
        i += 1;
    }

    let needle = format!("){}\"", delim);
    while i + needle.len() <= len {
        if input[i..].starts_with(&needle) {
            let end = i + needle.len();
            return Some((input[start..end].to_string(), end));
        }
        i += 1;
    }

    None
}

fn parse_escaped_string(input: &str, start: usize, prefix_len: usize) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    let mut i = start + prefix_len;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' {
            i += 2;
            continue;
        }
        if c == '"' {
            i += 1;
            return Some((input[start..i].to_string(), i));
        }
        i += 1;
    }
    None
}

fn parse_escaped_char(input: &str, start: usize, prefix_len: usize) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    let mut i = start + prefix_len;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' {
            i += 2;
            continue;
        }
        if c == '\'' {
            i += 1;
            return Some((input[start..i].to_string(), i));
        }
        i += 1;
    }
    None
}

fn parse_operator(input: &str, start: usize) -> Option<(String, usize)> {
    let ops = [
        "<<=", ">>=", "->*", "...", "::", "++", "--", "->", "&&", "||", "<=", ">=", "==", "!=",
        "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<", ">>", "##", "::*", ".*",
        "+", "-", "*", "/", "%", "&", "|", "^", "!", "~", "<", ">", "=", "?", ":", ";", ",",
        "(", ")", "{", "}", "[", "]", ".",
    ];

    for op in ops.iter() {
        if input[start..].starts_with(op) {
            return Some((op.to_string(), start + op.len()));
        }
    }
    None
}

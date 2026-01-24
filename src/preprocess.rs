use std::collections::HashMap;

use crate::lexer::{self, Token, TokenKind};

#[derive(Clone)]
struct MacroDef {
    name: String,
    params: Vec<String>,
    replacement: Vec<Token>,
    function_like: bool,
}

pub fn expand_macros(input: &str) -> String {
    let tokens = lexer::tokenize(input);
    let macros = collect_macro_definitions(&tokens);
    if macros.is_empty() {
        return input.to_string();
    }
    let expanded = expand_tokens(&tokens, &macros, &mut Vec::new());
    render_tokens(&expanded)
}

fn collect_macro_definitions(tokens: &[Token]) -> HashMap<String, MacroDef> {
    let mut macros = HashMap::new();
    for token in tokens {
        if token.kind == TokenKind::Preprocessor {
            if let Some(def) = parse_define_line(&token.text) {
                macros.insert(def.name.clone(), def);
            }
        }
    }
    macros
}

fn parse_define_line(text: &str) -> Option<MacroDef> {
    let mut rest = text.trim_start();
    if !rest.starts_with('#') {
        return None;
    }
    rest = &rest[1..];
    rest = rest.trim_start();
    if !rest.starts_with("define") {
        return None;
    }
    rest = &rest["define".len()..];
    rest = rest.trim_start();
    if rest.is_empty() {
        return None;
    }

    let bytes = rest.as_bytes();
    if !is_ident_start_byte(bytes[0]) {
        return None;
    }
    let mut end = 1;
    while end < bytes.len() && is_ident_char_byte(bytes[end]) {
        end += 1;
    }
    let name = rest[..end].to_string();
    let mut replacement = &rest[end..];
    let mut params = Vec::new();
    let mut function_like = false;

    if replacement.starts_with('(') {
        if let Some((offset, parsed_params)) = parse_macro_params(replacement) {
            function_like = true;
            params = parsed_params;
            replacement = &replacement[offset..];
        } else {
            return None;
        }
    }

    let replacement = replacement.trim_start();
    let replacement_tokens = lexer::tokenize(replacement);
    if contains_unsupported_ops(&replacement_tokens) {
        return None;
    }

    Some(MacroDef {
        name,
        params,
        replacement: replacement_tokens,
        function_like,
    })
}

fn contains_unsupported_ops(tokens: &[Token]) -> bool {
    tokens.iter().any(|token| {
        token.kind == TokenKind::Operator && (token.text == "#" || token.text == "##")
    })
}

fn parse_macro_params(text: &str) -> Option<(usize, Vec<String>)> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'(') {
        return None;
    }
    let mut params = Vec::new();
    let mut i = 1;
    let mut start: Option<usize> = None;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b')' {
            if let Some(begin) = start {
                params.push(text[begin..i].to_string());
            }
            return Some((i + 1, params));
        }

        if let Some(begin) = start {
            if is_ident_char_byte(b) {
                i += 1;
                continue;
            }
            params.push(text[begin..i].to_string());
            start = None;
            i += 1;
            continue;
        }

        if is_ident_start_byte(b) {
            start = Some(i);
            i += 1;
            continue;
        }

        if b == b'.' {
            return None;
        }

        i += 1;
    }

    None
}

fn expand_tokens(
    tokens: &[Token],
    macros: &HashMap<String, MacroDef>,
    stack: &mut Vec<String>,
) -> Vec<Token> {
    let mut out = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];
        if token.kind == TokenKind::Preprocessor {
            out.push(token.clone());
            i += 1;
            continue;
        }

        if token.kind == TokenKind::Identifier {
            if let Some(def) = macros.get(&token.text) {
                if stack.iter().any(|name| name == &def.name) {
                    out.push(token.clone());
                    i += 1;
                    continue;
                }

                if def.function_like {
                    if let Some((mut args, next_idx)) = parse_macro_args(tokens, i + 1) {
                        args = normalize_args(args);
                        if def.params.is_empty()
                            && args.len() == 1
                            && args[0].is_empty()
                        {
                            args.clear();
                        }
                        if args.len() == def.params.len() {
                            stack.push(def.name.clone());
                            let expanded = expand_replacement(def, &args, macros, stack);
                            stack.pop();
                            out.extend(expanded);
                            i = next_idx;
                            continue;
                        }
                    }
                } else {
                    stack.push(def.name.clone());
                    let expanded = expand_replacement(def, &[], macros, stack);
                    stack.pop();
                    out.extend(expanded);
                    i += 1;
                    continue;
                }
            }
        }

        out.push(token.clone());
        i += 1;
    }

    out
}

fn expand_replacement(
    def: &MacroDef,
    args: &[Vec<Token>],
    macros: &HashMap<String, MacroDef>,
    stack: &mut Vec<String>,
) -> Vec<Token> {
    let mut out = Vec::new();
    for token in &def.replacement {
        if token.kind == TokenKind::Identifier {
            if let Some(pos) = def.params.iter().position(|param| param == &token.text) {
                out.extend(args[pos].clone());
                continue;
            }
        }
        out.push(token.clone());
    }

    expand_tokens(&out, macros, stack)
}

fn parse_macro_args(tokens: &[Token], start: usize) -> Option<(Vec<Vec<Token>>, usize)> {
    let mut i = skip_trivia(tokens, start);
    if i >= tokens.len() {
        return None;
    }
    if tokens[i].kind != TokenKind::Operator || tokens[i].text != "(" {
        return None;
    }
    i += 1;

    let mut args = vec![Vec::new()];
    let mut depth = 1usize;
    while i < tokens.len() {
        let token = &tokens[i];
        if token.kind == TokenKind::Operator {
            if token.text == "(" {
                depth += 1;
                args.last_mut().unwrap().push(token.clone());
                i += 1;
                continue;
            }
            if token.text == ")" {
                depth -= 1;
                if depth == 0 {
                    return Some((args, i + 1));
                }
                args.last_mut().unwrap().push(token.clone());
                i += 1;
                continue;
            }
            if token.text == "," && depth == 1 {
                args.push(Vec::new());
                i += 1;
                continue;
            }
        }

        args.last_mut().unwrap().push(token.clone());
        i += 1;
    }

    None
}

fn normalize_args(args: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    args.into_iter().map(trim_trivia).collect()
}

fn trim_trivia(tokens: Vec<Token>) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && tokens[start].is_whitespace_or_comment() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_whitespace_or_comment() {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

fn skip_trivia(tokens: &[Token], mut i: usize) -> usize {
    while i < tokens.len() {
        if tokens[i].is_whitespace_or_comment() {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn render_tokens(tokens: &[Token]) -> String {
    let mut out = String::new();
    for token in tokens {
        out.push_str(&token.text);
    }
    out
}

fn is_ident_start_byte(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'_')
}

fn is_ident_char_byte(b: u8) -> bool {
    is_ident_start_byte(b) || matches!(b, b'0'..=b'9')
}

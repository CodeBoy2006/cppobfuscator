use std::collections::{BTreeSet, HashMap, HashSet};

use crate::lexer::{Token, TokenKind};
use crate::semantics;

#[derive(Debug, Clone)]
pub struct ObfuscateConfig {
    pub seed: u64,
    pub rename: bool,
    pub minify: bool,
    pub inline: bool,
    pub constlift: bool,
    pub strip_comments: bool,
    pub preserve: Vec<String>,
}

#[derive(Debug, Clone)]
struct SimpleFunction {
    name: String,
    params: Vec<String>,
    expr_tokens: Vec<Token>,
}

pub fn obfuscate(input: &str, config: &ObfuscateConfig) -> String {
    let mut tokens = crate::lexer::tokenize(input);
    let preserve = build_preserve_set(&tokens, &config.preserve);

    if config.inline {
        let defs = extract_simple_functions(&tokens);
        if !defs.is_empty() {
            tokens = inline_calls(&tokens, &defs);
        }
    }

    if config.rename {
        let declared = semantics::collect_declared_identifiers(input);
        apply_renames(&mut tokens, config.seed, &preserve, declared.as_ref());
    }

    if config.constlift {
        apply_constlift(&mut tokens, config.seed);
    }

    if config.minify {
        render_minified(&tokens, config.strip_comments)
    } else {
        render_preserve(&tokens, config.strip_comments)
    }
}

fn build_preserve_set(tokens: &[Token], extra: &[String]) -> HashSet<String> {
    let mut preserve: HashSet<String> = default_preserve_set()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    for item in extra {
        preserve.insert(item.to_string());
    }

    for macro_name in collect_defined_macros(tokens) {
        preserve.insert(macro_name);
    }

    preserve
}

fn collect_defined_macros(tokens: &[Token]) -> HashSet<String> {
    let mut macros = HashSet::new();
    for token in tokens {
        if token.kind == TokenKind::Preprocessor {
            let line = token.text.trim_start();
            if let Some(rest) = line.strip_prefix("#define") {
                let mut parts = rest.split_whitespace();
                if let Some(name) = parts.next() {
                    let name = name.trim();
                    let name = name.split('(').next().unwrap_or(name);
                    if !name.is_empty() {
                        macros.insert(name.to_string());
                    }
                }
            }
        }
    }
    macros
}

fn default_preserve_set() -> Vec<&'static str> {
    vec![
        "main",
        "std",
        "cin",
        "cout",
        "cerr",
        "clog",
        "endl",
        "ios",
        "ios_base",
        "istream",
        "ostream",
        "ifstream",
        "ofstream",
        "stringstream",
        "istringstream",
        "ostringstream",
        "tie",
        "fixed",
        "setprecision",
        "setw",
        "setfill",
        "vector",
        "string",
        "map",
        "unordered_map",
        "set",
        "unordered_set",
        "multiset",
        "multimap",
        "queue",
        "stack",
        "priority_queue",
        "deque",
        "array",
        "bitset",
        "pair",
        "tuple",
        "make_pair",
        "make_tuple",
        "get",
        "complex",
        "optional",
        "variant",
        "any",
        "sort",
        "stable_sort",
        "partial_sort",
        "nth_element",
        "reverse",
        "unique",
        "lower_bound",
        "upper_bound",
        "equal_range",
        "binary_search",
        "min",
        "max",
        "min_element",
        "max_element",
        "accumulate",
        "iota",
        "fill",
        "fill_n",
        "count",
        "count_if",
        "find",
        "find_if",
        "find_if_not",
        "all_of",
        "any_of",
        "none_of",
        "next_permutation",
        "prev_permutation",
        "rotate",
        "copy",
        "copy_n",
        "copy_if",
        "move",
        "swap",
        "swap_ranges",
        "merge",
        "inplace_merge",
        "set_union",
        "set_intersection",
        "set_difference",
        "set_symmetric_difference",
        "partition",
        "stable_partition",
        "remove",
        "remove_if",
        "erase",
        "transform",
        "adjacent_find",
        "lexicographical_compare",
        "distance",
        "advance",
        "gcd",
        "lcm",
        "abs",
        "fabs",
        "sqrt",
        "cbrt",
        "sin",
        "cos",
        "tan",
        "asin",
        "acos",
        "atan",
        "atan2",
        "log",
        "log2",
        "log10",
        "exp",
        "pow",
        "printf",
        "scanf",
        "getchar",
        "putchar",
        "puts",
        "fgets",
        "fread",
        "fwrite",
        "memset",
        "memcpy",
        "memmove",
        "strlen",
        "strcmp",
        "strncmp",
        "strcpy",
        "strncpy",
        "strcat",
        "strncat",
        "atoi",
        "atoll",
        "strtol",
        "strtoll",
        "rand",
        "srand",
        "time",
        "NULL",
        "stdin",
        "stdout",
        "stderr",
    ]
}

fn apply_renames(
    tokens: &mut [Token],
    seed: u64,
    preserve: &HashSet<String>,
    declared: Option<&HashSet<String>>,
) {
    let mut identifiers = BTreeSet::new();
    let mut existing = HashSet::new();

    for token in tokens.iter() {
        if token.kind == TokenKind::Identifier {
            existing.insert(token.text.clone());
            if should_rename(&token.text, preserve, declared) {
                identifiers.insert(token.text.clone());
            }
        }
    }

    let mut used = preserve.clone();
    used.extend(existing.into_iter());

    let mut mapping = HashMap::new();
    for name in identifiers {
        let new_name = generate_name(&name, seed, &mut used);
        mapping.insert(name, new_name);
    }

    for token in tokens.iter_mut() {
        if token.kind == TokenKind::Identifier {
            if let Some(new_name) = mapping.get(&token.text) {
                token.text = new_name.clone();
            }
        }
    }
}

fn should_rename(name: &str, preserve: &HashSet<String>, declared: Option<&HashSet<String>>) -> bool {
    if let Some(declared) = declared {
        if !declared.contains(name) {
            return false;
        }
    }
    if preserve.contains(name) {
        return false;
    }
    if name.starts_with("__") {
        return false;
    }
    if name.starts_with('_') {
        if name.chars().nth(1).map_or(false, |c| c.is_ascii_uppercase()) {
            return false;
        }
    }
    if name.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) {
        return false;
    }
    true
}

fn generate_name(original: &str, seed: u64, used: &mut HashSet<String>) -> String {
    let mut h = hash64(original, seed);
    loop {
        let suffix = to_base36(h);
        let candidate = format!("v{suffix}");
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
        h = h.wrapping_add(0x9E3779B97F4A7C15);
    }
}

fn hash64(text: &str, seed: u64) -> u64 {
    let mut h = seed ^ 0x9E3779B97F4A7C15;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001B3);
    }
    h
}

fn to_base36(mut value: u64) -> String {
    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let idx = (value % 36) as usize;
        out.push(CHARS[idx] as char);
        value /= 36;
    }
    out.iter().rev().collect()
}

fn extract_simple_functions(tokens: &[Token]) -> Vec<SimpleFunction> {
    let mut functions = Vec::new();
    let mut brace_depth: usize = 0;
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];
        if token.kind == TokenKind::Operator {
            if token.text == "{" {
                brace_depth += 1;
            } else if token.text == "}" {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
            }
        }

        if brace_depth == 0 && token.kind == TokenKind::Identifier {
            if let Some(open_paren) = next_significant(tokens, i + 1) {
                if tokens[open_paren].text == "(" {
                    if let Some(close_paren) = find_matching_paren(tokens, open_paren) {
                        let params = match parse_param_names(&tokens[open_paren + 1..close_paren]) {
                            Some(params) => params,
                            None => {
                                i += 1;
                                continue;
                            }
                        };
                        let mut j = close_paren + 1;
                        j = skip_trivia(tokens, j);

                        while j < tokens.len() {
                            let t = &tokens[j];
                            if t.kind == TokenKind::Operator && t.text == "{" {
                                if let Some(close_brace) = find_matching_brace(tokens, j) {
                                    let body_tokens = &tokens[j + 1..close_brace];
                                    if let Some(expr_tokens) = extract_return_expr(body_tokens, &params) {
                                        functions.push(SimpleFunction {
                                            name: token.text.clone(),
                                            params,
                                            expr_tokens,
                                        });
                                    }
                                }
                                break;
                            }
                            if t.kind == TokenKind::Operator && t.text == ";" {
                                break;
                            }
                            j = skip_trivia(tokens, j + 1);
                        }
                    }
                }
            }
        }
        i += 1;
    }

    functions
}

fn parse_param_names(tokens: &[Token]) -> Option<Vec<String>> {
    let mut params = Vec::new();
    let mut current = Vec::new();
    let mut depth_paren: usize = 0;
    let mut depth_angle: usize = 0;
    let mut depth_bracket: usize = 0;
    let mut has_default = false;

    for token in tokens.iter().filter(|t| !t.is_whitespace_or_comment()) {
        if token.kind == TokenKind::Operator {
            match token.text.as_str() {
                "(" => depth_paren += 1,
                ")" => depth_paren = depth_paren.saturating_sub(1),
                "<" => depth_angle += 1,
                ">" => depth_angle = depth_angle.saturating_sub(1),
                ">>" => {
                    depth_angle = depth_angle.saturating_sub(2);
                }
                "[" => depth_bracket += 1,
                "]" => depth_bracket = depth_bracket.saturating_sub(1),
                "=" if depth_paren == 0 && depth_angle == 0 && depth_bracket == 0 => {
                    has_default = true;
                }
                "..." if depth_paren == 0 && depth_angle == 0 && depth_bracket == 0 => {
                    return None;
                }
                "," if depth_paren == 0 && depth_angle == 0 && depth_bracket == 0 => {
                    if let Some(name) = extract_param_name(&current) {
                        params.push(name);
                    } else if !current.is_empty() {
                        return None;
                    }
                    current.clear();
                    continue;
                }
                _ => {}
            }
        }
        current.push(token.clone());
    }

    if let Some(name) = extract_param_name(&current) {
        params.push(name);
    } else if !current.is_empty() {
        return None;
    }

    if has_default {
        None
    } else {
        Some(params)
    }
}

fn extract_param_name(tokens: &[Token]) -> Option<String> {
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        let token = &tokens[idx];
        if token.kind == TokenKind::Identifier {
            if let Some(prev_idx) = prev_significant(tokens, idx) {
                if tokens[prev_idx].text == "::" {
                    continue;
                }
            }
            return Some(token.text.clone());
        }
    }
    None
}

fn extract_return_expr(body: &[Token], params: &[String]) -> Option<Vec<Token>> {
    let significant: Vec<&Token> = body.iter().filter(|t| !t.is_whitespace_or_comment()).collect();
    if significant.len() < 2 {
        return None;
    }
    if significant[0].text != "return" {
        return None;
    }
    if significant.iter().filter(|t| t.text == ";").count() != 1 {
        return None;
    }
    if significant.last()?.text != ";" {
        return None;
    }

    let mut expr_tokens = Vec::new();
    for token in significant.iter().skip(1).take(significant.len() - 2) {
        if token.kind == TokenKind::Identifier && !params.contains(&token.text) {
            return None;
        }
        expr_tokens.push((*token).clone());
    }

    Some(expr_tokens)
}

fn inline_calls(tokens: &[Token], defs: &[SimpleFunction]) -> Vec<Token> {
    let mut map = HashMap::new();
    for def in defs {
        map.insert(def.name.clone(), def.clone());
    }

    let mut out = Vec::new();
    let mut i = 0;
    let mut brace_depth: usize = 0;

    while i < tokens.len() {
        let token = &tokens[i];
        if token.kind == TokenKind::Operator {
            if token.text == "{" {
                brace_depth += 1;
            } else if token.text == "}" {
                brace_depth = brace_depth.saturating_sub(1);
            }
        }

        if token.kind == TokenKind::Identifier {
            if let Some(def) = map.get(&token.text) {
                if is_definition_site(tokens, i, brace_depth) {
                    out.push(token.clone());
                    i += 1;
                    continue;
                }

                if is_disallowed_call_context(tokens, i) {
                    out.push(token.clone());
                    i += 1;
                    continue;
                }

                if let Some(open_paren) = next_significant(tokens, i + 1) {
                    if tokens[open_paren].text == "(" {
                        if let Some(close_paren) = find_matching_paren(tokens, open_paren) {
                            let arg_tokens = &tokens[open_paren + 1..close_paren];
                            let args = split_args(arg_tokens);
                            if args.len() == def.params.len() {
                                let substituted = substitute_expr(def, &args);
                                out.push(Token {
                                    kind: TokenKind::Operator,
                                    text: "(".to_string(),
                                });
                                out.extend(substituted);
                                out.push(Token {
                                    kind: TokenKind::Operator,
                                    text: ")".to_string(),
                                });
                                i = close_paren + 1;
                                continue;
                            }
                        }
                    }
                }
            }
        }

        out.push(token.clone());
        i += 1;
    }

    out
}

fn is_definition_site(tokens: &[Token], idx: usize, brace_depth: usize) -> bool {
    if brace_depth != 0 {
        return false;
    }
    let open_paren = match next_significant(tokens, idx + 1) {
        Some(i) if tokens[i].text == "(" => i,
        _ => return false,
    };
    let close_paren = match find_matching_paren(tokens, open_paren) {
        Some(i) => i,
        None => return false,
    };
    let mut j = skip_trivia(tokens, close_paren + 1);
    while j < tokens.len() {
        let t = &tokens[j];
        if t.kind == TokenKind::Operator && t.text == "{" {
            return true;
        }
        if t.kind == TokenKind::Operator && t.text == ";" {
            return false;
        }
        j = skip_trivia(tokens, j + 1);
    }
    false
}

fn is_disallowed_call_context(tokens: &[Token], idx: usize) -> bool {
    if let Some(prev_idx) = prev_significant(tokens, idx) {
        let prev = &tokens[prev_idx];
        if prev.kind == TokenKind::Operator {
            let op = prev.text.as_str();
            if op == "." || op == "->" || op == "::" || op == "&" {
                return true;
            }
        }
    }
    false
}

fn split_args(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut args = Vec::new();
    let mut current = Vec::new();
    let mut depth_paren: usize = 0;
    let mut depth_angle: usize = 0;
    let mut depth_bracket: usize = 0;

    for token in tokens.iter().filter(|t| !t.is_whitespace_or_comment()) {
        if token.kind == TokenKind::Operator {
            match token.text.as_str() {
                "(" => depth_paren += 1,
                ")" => depth_paren = depth_paren.saturating_sub(1),
                "<" => depth_angle += 1,
                ">" => depth_angle = depth_angle.saturating_sub(1),
                ">>" => {
                    depth_angle = depth_angle.saturating_sub(2);
                }
                "[" => depth_bracket += 1,
                "]" => depth_bracket = depth_bracket.saturating_sub(1),
                "," if depth_paren == 0 && depth_angle == 0 && depth_bracket == 0 => {
                    args.push(current);
                    current = Vec::new();
                    continue;
                }
                _ => {}
            }
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn substitute_expr(def: &SimpleFunction, args: &[Vec<Token>]) -> Vec<Token> {
    let mut out = Vec::new();
    for token in &def.expr_tokens {
        if token.kind == TokenKind::Identifier {
            if let Some(pos) = def.params.iter().position(|p| p == &token.text) {
                out.push(Token {
                    kind: TokenKind::Operator,
                    text: "(".to_string(),
                });
                out.extend(args[pos].clone());
                out.push(Token {
                    kind: TokenKind::Operator,
                    text: ")".to_string(),
                });
                continue;
            }
        }
        out.push(token.clone());
    }
    out
}

fn apply_constlift(tokens: &mut [Token], seed: u64) {
    for token in tokens.iter_mut() {
        if token.kind == TokenKind::Number {
            if is_integer_literal(&token.text) {
                let mut key = (hash64(&token.text, seed) ^ 0xA5A5A5A5A5A5A5A5) & 0xFFFF;
                if key == 0 {
                    key = 0x5A5A;
                }
                token.text = format!("(({})^0x{:x}^0x{:x})", token.text, key, key);
            }
        }
    }
}

fn is_integer_literal(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if text.starts_with('.') {
        return false;
    }
    let mut has_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            has_digit = true;
        }
        if matches!(ch, '.' | 'e' | 'E' | 'p' | 'P') {
            return false;
        }
    }
    if text.ends_with('f') || text.ends_with('F') {
        return false;
    }
    has_digit
}

fn render_minified(tokens: &[Token], strip_comments: bool) -> String {
    let mut out = String::new();
    let mut prev: Option<&Token> = None;

    for token in tokens {
        if token.kind == TokenKind::Preprocessor {
            out.push_str(&token.text);
            if !token.text.ends_with('\n') {
                out.push('\n');
            }
            prev = None;
            continue;
        }
        if token.kind == TokenKind::Whitespace {
            continue;
        }
        if token.kind == TokenKind::Comment {
            if strip_comments {
                continue;
            }
            if token.text.starts_with("//") {
                out.push_str(&token.text);
                out.push('\n');
                prev = None;
                continue;
            }
        }

        if let Some(prev_token) = prev {
            if needs_space(prev_token, token) {
                out.push(' ');
            }
        }
        out.push_str(&token.text);
        prev = Some(token);
    }

    out
}

fn render_preserve(tokens: &[Token], strip_comments: bool) -> String {
    let mut out = String::new();
    for token in tokens {
        if token.kind == TokenKind::Comment && strip_comments {
            out.push(' ');
            continue;
        }
        out.push_str(&token.text);
    }
    out
}

fn needs_space(prev: &Token, next: &Token) -> bool {
    if prev.is_word_like() && next.is_word_like() {
        return true;
    }
    if prev.is_word_like() && matches!(next.kind, TokenKind::StringLiteral | TokenKind::CharLiteral) {
        return true;
    }
    if matches!(prev.kind, TokenKind::StringLiteral | TokenKind::CharLiteral) && next.is_word_like() {
        return true;
    }
    if prev.kind == TokenKind::Operator && next.kind == TokenKind::Operator {
        if could_merge_operator(&prev.text, &next.text) {
            return true;
        }
        if prev.text == "/" && (next.text == "/" || next.text == "*") {
            return true;
        }
    }
    if matches!(prev.text.as_str(), ")" | "]" | "}") && next.is_word_like() {
        return true;
    }
    false
}

fn could_merge_operator(a: &str, b: &str) -> bool {
    matches!(
        (a, b),
        ("+", "+")
            | ("-", "-")
            | ("-", ">")
            | ("<", "<")
            | (">", ">")
            | ("<", "=")
            | (">", "=")
            | ("=", "=")
            | ("!", "=")
            | ("&", "&")
            | ("|", "|")
            | ("+", "=")
            | ("-", "=")
            | ("*", "=")
            | ("/", "=")
            | ("%", "=")
            | ("&", "=")
            | ("|", "=")
            | ("^", "=")
            | (":", ":")
            | ("#", "#")
            | (".", "*")
            | (".", ".")
    )
}

fn next_significant(tokens: &[Token], start: usize) -> Option<usize> {
    let mut i = start;
    while i < tokens.len() {
        if !tokens[i].is_whitespace_or_comment() {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn prev_significant(tokens: &[Token], start: usize) -> Option<usize> {
    let mut i = start;
    while i > 0 {
        i -= 1;
        if !tokens[i].is_whitespace_or_comment() {
            return Some(i);
        }
    }
    None
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

fn find_matching_paren(tokens: &[Token], open_idx: usize) -> Option<usize> {
    let mut depth = 0;
    for i in open_idx..tokens.len() {
        if tokens[i].kind == TokenKind::Operator {
            if tokens[i].text == "(" {
                depth += 1;
            } else if tokens[i].text == ")" {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn find_matching_brace(tokens: &[Token], open_idx: usize) -> Option<usize> {
    let mut depth = 0;
    for i in open_idx..tokens.len() {
        if tokens[i].kind == TokenKind::Operator {
            if tokens[i].text == "{" {
                depth += 1;
            } else if tokens[i].text == "}" {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

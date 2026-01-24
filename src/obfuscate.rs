use std::collections::{BTreeSet, HashMap, HashSet};

use crate::lexer::{cpp_keywords, Token, TokenKind};
use crate::semantics;

#[derive(Debug, Clone)]
pub struct ObfuscateConfig {
    pub seed: u64,
    pub rename: bool,
    pub minify: bool,
    pub inline: bool,
    pub constlift: bool,
    pub strip_comments: bool,
    pub strip_unused_macros: bool,
    pub strip_unused_functions: bool,
    pub strip_unused_globals: bool,
    pub preserve: Vec<String>,
    pub simple_names: bool,
}

#[derive(Debug, Clone)]
struct SimpleFunction {
    name: String,
    params: Vec<String>,
    expr_tokens: Vec<Token>,
}

pub fn obfuscate(input: &str, config: &ObfuscateConfig) -> String {
    let mut raw = input.to_string();
    if config.strip_unused_functions {
        if let Some(stripped) = semantics::strip_unused_functions(&raw) {
            raw = stripped;
        }
    }
    if config.strip_unused_globals {
        if let Some(stripped) = semantics::strip_unused_globals(&raw) {
            raw = stripped;
        }
    }
    let mut tokens = crate::lexer::tokenize(&raw);
    if config.strip_unused_macros {
        tokens = remove_unused_macros(&tokens);
    }
    let preserve = build_preserve_set(&tokens, &config.preserve);

    if config.inline {
        let defs = extract_simple_functions(&tokens);
        if !defs.is_empty() {
            tokens = inline_calls(&tokens, &defs);
        }
    }

    if config.rename {
        let declared = semantics::collect_declared_identifiers(&raw);
        apply_renames(
            &mut tokens,
            config.seed,
            &preserve,
            declared.as_ref(),
            config.simple_names,
        );
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

#[derive(Debug)]
struct MacroDefinition<'a> {
    name: &'a str,
    replacement: &'a str,
    params: Vec<String>,
}

fn remove_unused_macros(tokens: &[Token]) -> Vec<Token> {
    let mut defined: HashSet<String> = HashSet::new();
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    let mut uses: HashSet<String> = HashSet::new();

    for token in tokens {
        match token.kind {
            TokenKind::Preprocessor => {
                if let Some(def) = parse_define_line(&token.text) {
                    let MacroDefinition {
                        name,
                        replacement,
                        params,
                    } = def;
                    let name = name.to_string();
                    defined.insert(name.clone());
                    let mut local_deps = HashSet::new();
                    for_each_identifier(replacement, |ident| {
                        if ident != name && !params.iter().any(|param| param == ident) {
                            local_deps.insert(ident.to_string());
                        }
                    });
                    deps.insert(name, local_deps);
                } else {
                    for_each_identifier(&token.text, |ident| {
                        uses.insert(ident.to_string());
                    });
                }
            }
            TokenKind::Identifier => {
                uses.insert(token.text.clone());
            }
            _ => {}
        }
    }

    let mut reachable: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = uses
        .into_iter()
        .filter(|name| defined.contains(name))
        .collect();

    while let Some(name) = stack.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(local_deps) = deps.get(&name) {
            for dep in local_deps {
                if defined.contains(dep) && !reachable.contains(dep) {
                    stack.push(dep.clone());
                }
            }
        }
    }

    let unused: HashSet<String> = defined
        .into_iter()
        .filter(|name| !reachable.contains(name))
        .collect();

    if unused.is_empty() {
        return tokens.to_vec();
    }

    tokens
        .iter()
        .filter_map(|token| {
            if token.kind == TokenKind::Preprocessor {
                if let Some(def) = parse_define_line(&token.text) {
                    if unused.contains(def.name) {
                        return None;
                    }
                }
            }
            Some(token.clone())
        })
        .collect()
}

fn parse_define_line(text: &str) -> Option<MacroDefinition<'_>> {
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
    let name = &rest[..end];
    let mut replacement = &rest[end..];
    let mut params = Vec::new();

    if replacement.starts_with('(') {
        if let Some((offset, parsed_params)) = parse_macro_params(replacement) {
            replacement = &replacement[offset..];
            params = parsed_params;
        }
    }

    Some(MacroDefinition {
        name,
        replacement,
        params,
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

        i += 1;
    }

    None
}

fn for_each_identifier<F: FnMut(&str)>(text: &str, mut f: F) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_ident_start_byte(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_char_byte(bytes[i]) {
                i += 1;
            }
            f(&text[start..i]);
            continue;
        }
        i += 1;
    }
}

fn is_ident_start_byte(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'_')
}

fn is_ident_char_byte(b: u8) -> bool {
    is_ident_start_byte(b) || matches!(b, b'0'..=b'9')
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
    simple_names: bool,
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
    if simple_names {
        let reserved: HashSet<String> = cpp_keywords().into_iter().map(|kw| kw.to_string()).collect();
        let mut counter = 0usize;
        for name in identifiers {
            let new_name = next_short_name(&mut counter, &mut used, &reserved);
            mapping.insert(name, new_name);
        }
    } else {
        for name in identifiers {
            let new_name = generate_name(&name, seed, &mut used);
            mapping.insert(name, new_name);
        }
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

fn next_short_name(
    counter: &mut usize,
    used: &mut HashSet<String>,
    reserved: &HashSet<String>,
) -> String {
    loop {
        let candidate = short_name(*counter);
        *counter += 1;
        if !used.contains(&candidate) && !reserved.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
    }
}

fn short_name(mut index: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const BASE: usize = 52;
    if index < BASE {
        return (ALPHABET[index] as char).to_string();
    }
    index -= BASE;
    if index < BASE * BASE {
        let first = ALPHABET[index / BASE] as char;
        let second = ALPHABET[index % BASE] as char;
        return format!("{first}{second}");
    }
    index -= BASE * BASE;
    let mut chars = Vec::new();
    loop {
        chars.push(ALPHABET[index % BASE] as char);
        index /= BASE;
        if index == 0 {
            break;
        }
    }
    while chars.len() < 3 {
        chars.push('a');
    }
    chars.reverse();
    chars.into_iter().collect()
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
            if let Some((value, suffix)) = parse_integer_literal(&token.text) {
                let mut rng = ConstRng::new(hash64(&token.text, seed));
                let depth = choose_const_depth(value, &mut rng);
                token.text = obfuscate_const(value, depth, &mut rng, &suffix);
            }
        }
    }
}

fn parse_integer_literal(text: &str) -> Option<(u64, String)> {
    if text.is_empty() || text.starts_with('.') {
        return None;
    }
    if text.chars().any(|ch| matches!(ch, '.' | 'e' | 'E' | 'p' | 'P')) {
        return None;
    }

    let mut suffix_start = text.len();
    let bytes = text.as_bytes();
    while suffix_start > 0 {
        let b = bytes[suffix_start - 1];
        if matches!(b, b'u' | b'U' | b'l' | b'L') {
            suffix_start -= 1;
        } else {
            break;
        }
    }
    let (number, suffix) = text.split_at(suffix_start);
    if suffix.contains('f') || suffix.contains('F') {
        return None;
    }

    let num = number.replace('_', "");
    let (base, digits) = if num.starts_with("0x") || num.starts_with("0X") {
        (16, &num[2..])
    } else if num.starts_with("0b") || num.starts_with("0B") {
        (2, &num[2..])
    } else if num.starts_with('0') && num.len() > 1 {
        (8, &num[1..])
    } else {
        (10, num.as_str())
    };

    if digits.is_empty() {
        return Some((0, suffix.to_string()));
    }

    let value = u64::from_str_radix(digits, base).ok()?;
    Some((value, suffix.to_string()))
}

fn choose_const_depth(value: u64, rng: &mut ConstRng) -> u8 {
    if value <= 3 {
        return 1;
    }
    let base: u8 = if value <= 0xFF { 2 } else { 3 };
    let tweak = (rng.next_u64() & 1) as u8;
    base.saturating_sub(tweak).max(1)
}

fn obfuscate_const(value: u64, depth: u8, rng: &mut ConstRng, suffix: &str) -> String {
    if depth == 0 {
        return leaf_expr(value, rng, suffix);
    }

    let mut ops = Vec::new();
    if value > 0 {
        ops.push(ConstOp::Add);
    }
    if value < u64::MAX {
        ops.push(ConstOp::Sub);
    }
    ops.push(ConstOp::Xor);
    if value % 2 == 0 {
        ops.push(ConstOp::Shift);
    }

    let op = ops[(rng.next_u64() as usize) % ops.len()];
    let mut expr = match op {
        ConstOp::Add => {
            let r = bounded_rand(rng, value.min(0xFFFF).max(1));
            let left = obfuscate_const(value - r, depth - 1, rng, suffix);
            let right = obfuscate_const(r, depth - 1, rng, suffix);
            format!("({left}+{right})")
        }
        ConstOp::Sub => {
            let max_r = (u64::MAX - value).min(0xFFFF).max(1);
            let r = bounded_rand(rng, max_r);
            let left = obfuscate_const(value.wrapping_add(r), depth - 1, rng, suffix);
            let right = obfuscate_const(r, depth - 1, rng, suffix);
            format!("({left}-{right})")
        }
        ConstOp::Xor => {
            let r = bounded_rand(rng, 0xFFFF).max(1);
            let left = obfuscate_const(value ^ r, depth - 1, rng, suffix);
            let right = obfuscate_const(r, depth - 1, rng, suffix);
            format!("({left}^{right})")
        }
        ConstOp::Shift => {
            let shift = ((rng.next_u64() % 3) + 1) as u32;
            if shift >= 64 || (value >> shift) == 0 {
                leaf_expr(value, rng, suffix)
            } else {
                let left = obfuscate_const(value >> shift, depth - 1, rng, suffix);
                format!("({left}<<{shift})")
            }
        }
    };

    if (rng.next_u64() & 1) == 0 {
        expr = format!("(~(~{expr}))");
    }
    expr
}

fn bounded_rand(rng: &mut ConstRng, max_inclusive: u64) -> u64 {
    if max_inclusive <= 1 {
        return 1;
    }
    (rng.next_u64() % max_inclusive) + 1
}

fn leaf_expr(value: u64, rng: &mut ConstRng, suffix: &str) -> String {
    let mut choices = Vec::new();
    choices.push(format!("{}{}", value, suffix));
    choices.push(format!("0x{:x}{}", value, suffix));
    choices.push(format!("0{:o}{}", value, suffix));

    if value == 0 {
        choices.push("(!1)".to_string());
        choices.push("(sizeof(char)-sizeof(char))".to_string());
    }
    if value == 1 {
        choices.push("(!0)".to_string());
        choices.push("sizeof(char)".to_string());
    }
    if (2..=32).contains(&value) {
        choices.push(format!("sizeof(char[{}])", value));
    }
    if (33..=126).contains(&value) {
        let ch = value as u8 as char;
        if ch != '\\' && ch != '\'' {
            choices.push(format!("'{}'", ch));
        }
    }

    choices[(rng.next_u64() as usize) % choices.len()].clone()
}

#[derive(Copy, Clone)]
enum ConstOp {
    Add,
    Sub,
    Xor,
    Shift,
}

struct ConstRng {
    state: u64,
}

impl ConstRng {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x9E3779B97F4A7C15 } else { seed };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

fn render_minified(tokens: &[Token], strip_comments: bool) -> String {
    let mut out = String::new();
    let mut prev: Option<&Token> = None;

    for token in tokens {
        if token.kind == TokenKind::Preprocessor {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
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

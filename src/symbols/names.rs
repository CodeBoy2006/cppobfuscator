use std::collections::HashSet;

use crate::random::SplitMix64;

pub(super) fn scan_identifiers(text: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut start = None;

    for (index, character) in text.char_indices() {
        let is_start = character == '_' || character.is_alphabetic();
        let is_continue = is_start || character.is_alphanumeric();

        match start {
            Some(begin) if !is_continue => {
                identifiers.push(text[begin..index].to_string());
                start = is_start.then_some(index);
            }
            None if is_start => start = Some(index),
            _ => {}
        }
    }

    if let Some(begin) = start {
        identifiers.push(text[begin..].to_string());
    }
    identifiers
}

pub(super) fn is_reserved_identifier(name: &str) -> bool {
    name == "main"
        || name.starts_with("__")
        || name
            .strip_prefix('_')
            .and_then(|rest| rest.chars().next())
            .is_some_and(char::is_uppercase)
}

pub(super) struct NameGenerator {
    index: usize,
    first: Vec<char>,
    rest: Vec<char>,
    used: HashSet<String>,
}

impl NameGenerator {
    pub fn new(seed: u64, used: HashSet<String>, ambiguous: bool) -> Self {
        let mut rng = SplitMix64::new(seed);
        let (mut first, mut rest): (Vec<char>, Vec<char>) = if ambiguous {
            ("ilo".chars().collect(), "ilo01".chars().collect())
        } else {
            let first: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
            let rest = first.clone();
            (first, rest)
        };
        shuffle(&mut first, &mut rng);
        shuffle(&mut rest, &mut rng);
        if !ambiguous {
            rest.extend("0123456789".chars());
        }
        Self {
            index: 0,
            first,
            rest,
            used,
        }
    }

    pub fn next_name(&mut self) -> String {
        loop {
            let candidate = self.name_at(self.index);
            self.index += 1;
            if !self.used.contains(&candidate) && !is_cpp_keyword(&candidate) {
                self.used.insert(candidate.clone());
                return candidate;
            }
        }
    }

    fn name_at(&self, mut index: usize) -> String {
        if index < self.first.len() {
            return self.first[index].to_string();
        }
        index -= self.first.len();

        let mut suffix_len = 1usize;
        loop {
            let suffix_count = self.rest.len().saturating_pow(suffix_len as u32);
            let count = self.first.len().saturating_mul(suffix_count);
            if index < count {
                let first_index = index / suffix_count;
                let mut suffix_index = index % suffix_count;
                let mut output = String::with_capacity(suffix_len + 1);
                output.push(self.first[first_index]);

                let mut suffix = vec![self.rest[0]; suffix_len];
                for position in (0..suffix_len).rev() {
                    suffix[position] = self.rest[suffix_index % self.rest.len()];
                    suffix_index /= self.rest.len();
                }
                output.extend(suffix);
                return output;
            }
            index -= count;
            suffix_len += 1;
        }
    }
}

fn shuffle(values: &mut [char], rng: &mut SplitMix64) {
    for index in (1..values.len()).rev() {
        let other = (rng.next() as usize) % (index + 1);
        values.swap(index, other);
    }
}

fn is_cpp_keyword(name: &str) -> bool {
    CPP_KEYWORDS.contains(&name)
}

const CPP_KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
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
];

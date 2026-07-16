use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::Range;

use crate::ObfuscationError;

const MAX_EXPANSION_DEPTH: usize = 128;

pub(crate) fn expand_local_macros(source: &str) -> Result<String, ObfuscationError> {
    let mut analysis = Analysis::new(source);
    analysis.collect();

    let mut discovery = ExpansionState::default();
    analysis.render(source, &BTreeSet::new(), &mut discovery)?;

    let preserved = analysis.preserved_macros(&discovery);
    let removable = analysis
        .definitions
        .iter()
        .filter(|definition| definition.base_safe && !preserved.contains(&definition.name))
        .map(|definition| definition.id)
        .collect();

    let mut rendering = ExpansionState::default();
    let output = analysis.render(source, &removable, &mut rendering)?;
    if !rendering.residual.is_subset(&preserved) {
        return Err(ObfuscationError::Validation(
            "macro expansion decisions changed during final rendering".to_string(),
        ));
    }
    if newline_count(&output) != newline_count(source) {
        return Err(ObfuscationError::Validation(
            "macro expansion changed the physical line count".to_string(),
        ));
    }
    Ok(output)
}

pub(crate) fn has_token_paste(text: &str) -> bool {
    scan_macro_operators(text).token_paste
}

pub(crate) fn has_stringification(text: &str) -> bool {
    scan_macro_operators(text).stringification
}

struct Analysis {
    directives: Vec<Directive>,
    definitions: Vec<MacroDefinition>,
    definitions_by_name: BTreeMap<String, Vec<usize>>,
    unique_definitions: HashMap<String, usize>,
    direct_references: BTreeSet<String>,
    last_include_end: usize,
}

impl Analysis {
    fn new(source: &str) -> Self {
        Self {
            directives: scan_directives(source),
            definitions: Vec::new(),
            definitions_by_name: BTreeMap::new(),
            unique_definitions: HashMap::new(),
            direct_references: BTreeSet::new(),
            last_include_end: 0,
        }
    }

    fn collect(&mut self) {
        let mut conditional_depth = 0usize;

        for directive_index in 0..self.directives.len() {
            let directive = &mut self.directives[directive_index];
            let parsed = parse_directive(&directive.text);
            directive.keyword = parsed.keyword.clone();
            directive.payload = parsed.payload.clone();

            match parsed.keyword.as_str() {
                "endif" => conditional_depth = conditional_depth.saturating_sub(1),
                "include" | "include_next" | "import" => {
                    self.last_include_end = self.last_include_end.max(directive.range.end);
                }
                _ => {}
            }

            if parsed.keyword == "define"
                && let Some(mut definition) = parse_definition(
                    self.definitions.len(),
                    directive.range.clone(),
                    conditional_depth,
                    &parsed.payload,
                )
            {
                definition.dependencies = scan_identifier_set(&definition.replacement);
                directive.definition = Some(definition.id);
                self.definitions_by_name
                    .entry(definition.name.clone())
                    .or_default()
                    .push(definition.id);
                self.definitions.push(definition);
            }

            if matches!(parsed.keyword.as_str(), "if" | "ifdef" | "ifndef") {
                conditional_depth += 1;
            }
        }

        let local_names: BTreeSet<String> = self.definitions_by_name.keys().cloned().collect();
        for directive in &self.directives {
            if directive.keyword == "define" {
                continue;
            }
            self.direct_references.extend(
                scan_identifier_set(&directive.payload)
                    .into_iter()
                    .filter(|name| local_names.contains(name)),
            );
        }

        for (name, definitions) in &self.definitions_by_name {
            if definitions.len() == 1 {
                self.unique_definitions.insert(name.clone(), definitions[0]);
            }
        }

        for definition in &mut self.definitions {
            definition
                .dependencies
                .retain(|name| local_names.contains(name));
            // A later include can redefine or undefine a local macro through
            // header side effects that are intentionally outside this model.
            definition.base_safe = self
                .definitions_by_name
                .get(&definition.name)
                .is_some_and(|definitions| definitions.len() == 1)
                && definition.conditional_depth == 0
                && !self.direct_references.contains(&definition.name)
                && definition.range.end > self.last_include_end;
            definition.can_expand = definition.base_safe
                && !definition.variadic
                && !has_token_paste(&definition.replacement)
                && !has_stringification(&definition.replacement)
                && !definition.replacement.contains('\n');
        }
    }

    fn preserved_macros(&self, state: &ExpansionState) -> BTreeSet<String> {
        let mut preserved: BTreeSet<String> = self
            .definitions
            .iter()
            .filter(|definition| !definition.base_safe)
            .map(|definition| definition.name.clone())
            .collect();
        preserved.extend(state.residual.iter().cloned());

        // Definitions that survive must retain every local macro needed by
        // their untouched replacement text.
        loop {
            let mut changed = false;
            for definition in &self.definitions {
                if !preserved.contains(&definition.name) {
                    continue;
                }
                for dependency in &definition.dependencies {
                    changed |= preserved.insert(dependency.clone());
                }
            }
            if !changed {
                break;
            }
        }
        preserved
    }

    fn render(
        &self,
        source: &str,
        removable: &BTreeSet<usize>,
        state: &mut ExpansionState,
    ) -> Result<String, ObfuscationError> {
        let mut output = String::with_capacity(source.len());
        let mut cursor = 0usize;

        for directive in &self.directives {
            if cursor < directive.range.start {
                let fragment = &source[cursor..directive.range.start];
                output.push_str(&self.expand_fragment(
                    fragment,
                    Origin::Source(cursor),
                    &mut Vec::new(),
                    state,
                    0,
                )?);
            }

            let text = &source[directive.range.clone()];
            if directive
                .definition
                .is_some_and(|definition| removable.contains(&definition))
            {
                output.push_str(&only_line_endings(text));
            } else {
                output.push_str(text);
            }
            cursor = directive.range.end;
        }

        if cursor < source.len() {
            output.push_str(&self.expand_fragment(
                &source[cursor..],
                Origin::Source(cursor),
                &mut Vec::new(),
                state,
                0,
            )?);
        }

        Ok(output)
    }

    fn expand_fragment(
        &self,
        text: &str,
        origin: Origin,
        disabled: &mut Vec<String>,
        state: &mut ExpansionState,
        depth: usize,
    ) -> Result<String, ObfuscationError> {
        if depth > MAX_EXPANSION_DEPTH {
            return Err(ObfuscationError::Unsupported(
                "local macro expansion exceeded the recursion limit".to_string(),
            ));
        }

        let bytes = text.as_bytes();
        let mut output = String::with_capacity(text.len());
        let mut index = 0usize;

        while index < bytes.len() {
            if let Some(end) = literal_end(bytes, index) {
                output.push_str(&text[index..end]);
                index = end;
                continue;
            }
            if bytes[index..].starts_with(b"//") {
                let end = line_comment_end(bytes, index + 2);
                output.push_str(&text[index..end]);
                index = end;
                continue;
            }
            if bytes[index..].starts_with(b"/*") {
                let end = block_comment_end(bytes, index + 2);
                output.push_str(&text[index..end]);
                index = end;
                continue;
            }
            if let Some(end) = pp_number_end(bytes, index) {
                output.push_str(&text[index..end]);
                index = end;
                continue;
            }

            let Some(identifier_end) = identifier_end(text, index) else {
                let character = text[index..]
                    .chars()
                    .next()
                    .expect("index is inside UTF-8 source");
                output.push(character);
                index += character.len_utf8();
                continue;
            };

            let name = &text[index..identifier_end];
            let position = origin.position(index);
            let Some(&definition_id) = self.unique_definitions.get(name) else {
                output.push_str(name);
                index = identifier_end;
                continue;
            };
            let definition = &self.definitions[definition_id];
            if position < definition.range.end {
                output.push_str(name);
                index = identifier_end;
                continue;
            }

            let invocation = match &definition.kind {
                MacroKind::Object => Some(Invocation {
                    end: identifier_end,
                    arguments: Vec::new(),
                }),
                MacroKind::Function(parameters) => {
                    let open = skip_macro_trivia(text, identifier_end);
                    if bytes.get(open) != Some(&b'(') {
                        None
                    } else {
                        let parsed = parse_invocation(text, open)?;
                        if parameters.is_empty()
                            && parsed.arguments.len() == 1
                            && parsed.arguments[0].trim().is_empty()
                        {
                            Some(Invocation {
                                end: parsed.end,
                                arguments: Vec::new(),
                            })
                        } else {
                            Some(parsed)
                        }
                    }
                }
            };

            let Some(invocation) = invocation else {
                output.push_str(name);
                index = identifier_end;
                continue;
            };

            let consumed = &text[index..invocation.end];
            if let MacroKind::Function(parameters) = &definition.kind
                && parameters.len() != invocation.arguments.len()
            {
                state.residual.insert(name.to_string());
                self.mark_nested_references(consumed, position, state);
                output.push_str(consumed);
                index = invocation.end;
                continue;
            }
            if !definition.can_expand || disabled.iter().any(|disabled| disabled == name) {
                state.residual.insert(name.to_string());
                self.mark_nested_references(consumed, position, state);
                output.push_str(consumed);
                index = invocation.end;
                continue;
            }

            let expanded = self.expand_invocation(
                definition,
                &invocation.arguments,
                position,
                disabled,
                state,
                depth + 1,
            )?;
            let consumed_newlines = newline_count(consumed);
            let expanded_newlines = newline_count(&expanded);
            if expanded_newlines > consumed_newlines {
                // Duplicating a multiline argument would move every following
                // source line, so retain the invocation and its dependencies.
                state.residual.insert(name.to_string());
                self.mark_nested_references(consumed, position, state);
                output.push_str(consumed);
                index = invocation.end;
                continue;
            }

            // Expansion produces preprocessing tokens, which are not re-lexed
            // into larger operators. Spaces retain that boundary in emitted C++.
            output.push(' ');
            output.push_str(&expanded);
            output.push(' ');
            for _ in expanded_newlines..consumed_newlines {
                output.push('\n');
            }
            index = invocation.end;
        }

        Ok(output)
    }

    fn expand_invocation(
        &self,
        definition: &MacroDefinition,
        arguments: &[String],
        position: usize,
        disabled: &mut Vec<String>,
        state: &mut ExpansionState,
        depth: usize,
    ) -> Result<String, ObfuscationError> {
        disabled.push(definition.name.clone());

        let result = (|| {
            let replacement = match &definition.kind {
                MacroKind::Object => definition.replacement.clone(),
                MacroKind::Function(parameters) => {
                    let mut expanded_arguments = HashMap::new();
                    for (parameter, argument) in parameters.iter().zip(arguments) {
                        let normalized = normalize_macro_fragment(argument);
                        let expanded = self.expand_fragment(
                            &normalized,
                            Origin::Fixed(position),
                            disabled,
                            state,
                            depth,
                        )?;
                        expanded_arguments.insert(parameter.as_str(), expanded);
                    }
                    substitute_parameters(&definition.replacement, &expanded_arguments)
                }
            };

            self.expand_fragment(
                &replacement,
                Origin::Fixed(position),
                disabled,
                state,
                depth,
            )
        })();

        disabled.pop();
        result
    }

    fn mark_nested_references(&self, text: &str, position: usize, state: &mut ExpansionState) {
        for name in scan_identifier_set(text) {
            let Some(&definition_id) = self.unique_definitions.get(&name) else {
                continue;
            };
            if position >= self.definitions[definition_id].range.end {
                state.residual.insert(name);
            }
        }
    }
}

#[derive(Default)]
struct ExpansionState {
    residual: BTreeSet<String>,
}

#[derive(Clone, Copy)]
enum Origin {
    Source(usize),
    Fixed(usize),
}

impl Origin {
    fn position(self, offset: usize) -> usize {
        match self {
            Self::Source(base) => base + offset,
            Self::Fixed(position) => position,
        }
    }
}

#[derive(Debug)]
struct Directive {
    range: Range<usize>,
    text: String,
    keyword: String,
    payload: String,
    definition: Option<usize>,
}

#[derive(Debug)]
struct ParsedDirective {
    keyword: String,
    payload: String,
}

#[derive(Debug)]
struct MacroDefinition {
    id: usize,
    name: String,
    kind: MacroKind,
    replacement: String,
    range: Range<usize>,
    conditional_depth: usize,
    variadic: bool,
    dependencies: BTreeSet<String>,
    base_safe: bool,
    can_expand: bool,
}

#[derive(Debug)]
enum MacroKind {
    Object,
    Function(Vec<String>),
}

struct Invocation {
    end: usize,
    arguments: Vec<String>,
}

fn scan_directives(source: &str) -> Vec<Directive> {
    let bytes = source.as_bytes();
    let mut directives = Vec::new();
    let mut index = 0usize;
    let mut line_prefix_is_trivia = true;

    while index < bytes.len() {
        if line_prefix_is_trivia && (bytes[index] == b'#' || bytes[index..].starts_with(b"%:")) {
            let end = directive_end(bytes, index);
            directives.push(Directive {
                range: index..end,
                text: source[index..end].to_string(),
                keyword: String::new(),
                payload: String::new(),
                definition: None,
            });
            line_prefix_is_trivia = end > index && bytes.get(end.wrapping_sub(1)) == Some(&b'\n');
            index = end;
            continue;
        }

        if bytes[index..].starts_with(b"\\\r\n") {
            index += 3;
            continue;
        }
        if bytes[index..].starts_with(b"\\\n") {
            index += 2;
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            index = line_comment_end(bytes, index + 2);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            let end = block_comment_end(bytes, index + 2);
            if bytes[index..end].contains(&b'\n') {
                line_prefix_is_trivia = true;
            }
            index = end;
            continue;
        }
        if let Some(end) = literal_end(bytes, index) {
            if bytes[index..end].contains(&b'\n') {
                line_prefix_is_trivia = false;
            }
            index = end;
            continue;
        }

        match bytes[index] {
            b'\n' => {
                line_prefix_is_trivia = true;
                index += 1;
            }
            b' ' | b'\t' | b'\r' | 0x0B | 0x0C => index += 1,
            _ => {
                line_prefix_is_trivia = false;
                index += next_char_len(source, index);
            }
        }
    }

    directives
}

fn directive_end(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\\\r\n") {
            index += 3;
        } else if bytes[index..].starts_with(b"\\\n") {
            index += 2;
        } else if bytes[index] == b'\n' {
            return index + 1;
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn parse_directive(text: &str) -> ParsedDirective {
    let spliced = remove_line_splices(text);
    let clean = replace_comments(&spliced);
    let bytes = clean.as_bytes();
    let mut index = 0usize;

    if bytes.starts_with(b"%:") {
        index = 2;
    } else if bytes.first() == Some(&b'#') {
        index = 1;
    }
    index = skip_horizontal_whitespace(bytes, index);

    let keyword_start = index;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        index += 1;
    }
    let keyword = clean[keyword_start..index].to_string();
    let payload = clean[index..]
        .trim_matches([' ', '\t', '\r', '\n', '\u{000B}', '\u{000C}'])
        .to_string();
    ParsedDirective { keyword, payload }
}

fn parse_definition(
    id: usize,
    range: Range<usize>,
    conditional_depth: usize,
    payload: &str,
) -> Option<MacroDefinition> {
    let name_end = identifier_end(payload, 0)?;
    let name = payload[..name_end].to_string();
    let bytes = payload.as_bytes();
    let mut variadic = false;

    let (kind, replacement_start) = if bytes.get(name_end) == Some(&b'(') {
        let close = matching_parenthesis(payload, name_end)?;
        let parameter_text = &payload[name_end + 1..close];
        let mut parameters = Vec::new();
        let mut seen = HashSet::new();

        if !parameter_text.trim().is_empty() {
            for parameter in parameter_text.split(',') {
                let parameter = parameter.trim();
                if parameter == "..." || parameter.ends_with("...") {
                    variadic = true;
                    continue;
                }
                let end = identifier_end(parameter, 0)?;
                if end != parameter.len() || !seen.insert(parameter.to_string()) {
                    return None;
                }
                parameters.push(parameter.to_string());
            }
        }
        (MacroKind::Function(parameters), close + 1)
    } else {
        (MacroKind::Object, name_end)
    };

    let replacement = normalize_macro_fragment(&payload[replacement_start..]);
    Some(MacroDefinition {
        id,
        name,
        kind,
        replacement,
        range,
        conditional_depth,
        variadic,
        dependencies: BTreeSet::new(),
        base_safe: false,
        can_expand: false,
    })
}

fn matching_parenthesis(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = open + 1;
    while index < bytes.len() {
        if let Some(end) = literal_end(bytes, index) {
            index = end;
            continue;
        }
        match bytes[index] {
            b')' => return Some(index),
            b'(' => return None,
            _ => index += next_char_len(text, index),
        }
    }
    None
}

fn parse_invocation(text: &str, open: usize) -> Result<Invocation, ObfuscationError> {
    let bytes = text.as_bytes();
    let mut arguments = Vec::new();
    let mut depth = 1usize;
    let mut argument_start = open + 1;
    let mut index = open + 1;

    while index < bytes.len() {
        if let Some(end) = literal_end(bytes, index) {
            index = end;
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            index = line_comment_end(bytes, index + 2);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = block_comment_end(bytes, index + 2);
            continue;
        }

        match bytes[index] {
            b'(' => {
                depth += 1;
                index += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    arguments.push(text[argument_start..index].to_string());
                    return Ok(Invocation {
                        end: index + 1,
                        arguments,
                    });
                }
                index += 1;
            }
            b',' if depth == 1 => {
                // The preprocessor tracks only parenthesis depth here; braces
                // and template angle brackets do not protect commas.
                arguments.push(text[argument_start..index].to_string());
                argument_start = index + 1;
                index += 1;
            }
            _ => index += next_char_len(text, index),
        }
    }

    Err(ObfuscationError::Unsupported(
        "unterminated local function-like macro invocation".to_string(),
    ))
}

fn substitute_parameters(replacement: &str, arguments: &HashMap<&str, String>) -> String {
    let bytes = replacement.as_bytes();
    let mut output = String::with_capacity(replacement.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(end) = literal_end(bytes, index) {
            output.push_str(&replacement[index..end]);
            index = end;
            continue;
        }
        if let Some(end) = pp_number_end(bytes, index) {
            output.push_str(&replacement[index..end]);
            index = end;
            continue;
        }
        if let Some(end) = identifier_end(replacement, index) {
            let name = &replacement[index..end];
            if let Some(argument) = arguments.get(name) {
                output.push(' ');
                output.push_str(argument);
                output.push(' ');
            } else {
                output.push_str(name);
            }
            index = end;
            continue;
        }

        let character = replacement[index..]
            .chars()
            .next()
            .expect("index is inside UTF-8 replacement");
        output.push(character);
        index += character.len_utf8();
    }
    output
}

fn normalize_macro_fragment(text: &str) -> String {
    let spliced = remove_line_splices(text);
    let clean = replace_comments(&spliced);
    let bytes = clean.as_bytes();
    let mut output = String::with_capacity(clean.len());
    let mut index = 0usize;
    let mut pending_space = false;

    while index < bytes.len() {
        if let Some(end) = literal_end(bytes, index) {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            pending_space = false;
            output.push_str(&clean[index..end]);
            index = end;
            continue;
        }

        let character = clean[index..]
            .chars()
            .next()
            .expect("index is inside UTF-8 macro text");
        if character.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            pending_space = false;
            output.push(character);
        }
        index += character.len_utf8();
    }
    output.trim().to_string()
}

fn remove_line_splices(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\\\r\n") {
            index += 3;
        } else if bytes[index..].starts_with(b"\\\n") {
            index += 2;
        } else {
            let character = text[index..]
                .chars()
                .next()
                .expect("index is inside UTF-8 text");
            output.push(character);
            index += character.len_utf8();
        }
    }
    output
}

fn replace_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(end) = literal_end(bytes, index) {
            output.push_str(&text[index..end]);
            index = end;
        } else if bytes[index..].starts_with(b"//") {
            output.push(' ');
            index = line_comment_end(bytes, index + 2);
        } else if bytes[index..].starts_with(b"/*") {
            output.push(' ');
            index = block_comment_end(bytes, index + 2);
        } else {
            let character = text[index..]
                .chars()
                .next()
                .expect("index is inside UTF-8 text");
            output.push(character);
            index += character.len_utf8();
        }
    }
    output
}

fn scan_identifier_set(text: &str) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    let mut identifiers = BTreeSet::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(end) = literal_end(bytes, index) {
            index = end;
        } else if bytes[index..].starts_with(b"//") {
            index = line_comment_end(bytes, index + 2);
        } else if bytes[index..].starts_with(b"/*") {
            index = block_comment_end(bytes, index + 2);
        } else if let Some(end) = pp_number_end(bytes, index) {
            index = end;
        } else if let Some(end) = identifier_end(text, index) {
            identifiers.insert(text[index..end].to_string());
            index = end;
        } else {
            index += next_char_len(text, index);
        }
    }
    identifiers
}

fn scan_macro_operators(text: &str) -> MacroOperators {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut operators = MacroOperators::default();

    while index < bytes.len() {
        if let Some(end) = literal_end(bytes, index) {
            index = end;
            continue;
        }
        match bytes[index] {
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
            _ => index += next_char_len(text, index),
        }
    }
    operators
}

#[derive(Default)]
struct MacroOperators {
    token_paste: bool,
    stringification: bool,
}

fn literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    if let Some(end) = raw_string_end(bytes, start) {
        return Some(literal_suffix_end(bytes, end));
    }

    const PREFIXES: [(&[u8], u8); 10] = [
        (b"u8\"", b'"'),
        (b"u8'", b'\''),
        (b"u\"", b'"'),
        (b"u'", b'\''),
        (b"U\"", b'"'),
        (b"U'", b'\''),
        (b"L\"", b'"'),
        (b"L'", b'\''),
        (b"\"", b'"'),
        (b"'", b'\''),
    ];
    let (prefix, quote) = PREFIXES
        .iter()
        .find(|(prefix, _)| bytes[start..].starts_with(prefix))?;
    let end = quoted_end(bytes, start + prefix.len() - 1, *quote);
    Some(literal_suffix_end(bytes, end))
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

fn literal_suffix_end(bytes: &[u8], mut index: usize) -> usize {
    if bytes.get(index) != Some(&b'_') {
        return index;
    }
    index += 1;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        index += 1;
    }
    index
}

fn line_comment_end(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\\\r\n") {
            index += 3;
        } else if bytes[index..].starts_with(b"\\\n") {
            index += 2;
        } else if bytes[index] == b'\n' {
            return index;
        } else {
            index += 1;
        }
    }
    bytes.len()
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

fn identifier_end(text: &str, start: usize) -> Option<usize> {
    let mut characters = text[start..].char_indices();
    let (_, first) = characters.next()?;
    if first != '_' && !first.is_alphabetic() {
        return None;
    }

    let mut end = start + first.len_utf8();
    for (offset, character) in characters {
        if character != '_' && !character.is_alphanumeric() {
            break;
        }
        end = start + offset + character.len_utf8();
    }
    Some(end)
}

fn pp_number_end(bytes: &[u8], start: usize) -> Option<usize> {
    let starts_number = bytes.get(start).is_some_and(u8::is_ascii_digit)
        || (bytes.get(start) == Some(&b'.')
            && bytes.get(start + 1).is_some_and(u8::is_ascii_digit));
    if !starts_number {
        return None;
    }

    let mut index = start + 1;
    while index < bytes.len() {
        let byte = bytes[index];
        let exponent_sign = matches!(byte, b'+' | b'-')
            && index > start
            && matches!(bytes[index - 1], b'e' | b'E' | b'p' | b'P');
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'\'' | b'.') || exponent_sign {
            index += 1;
        } else {
            break;
        }
    }
    Some(index)
}

fn skip_macro_trivia(text: &str, mut index: usize) -> usize {
    let bytes = text.as_bytes();
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\\\r\n") {
            index += 3;
        } else if bytes[index..].starts_with(b"\\\n") {
            index += 2;
        } else if bytes[index..].starts_with(b"//") {
            index = line_comment_end(bytes, index + 2);
        } else if bytes[index..].starts_with(b"/*") {
            index = block_comment_end(bytes, index + 2);
        } else if bytes[index].is_ascii_whitespace() {
            index += 1;
        } else {
            break;
        }
    }
    index
}

fn skip_horizontal_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | 0x0B | 0x0C))
    {
        index += 1;
    }
    index
}

fn next_char_len(text: &str, index: usize) -> usize {
    text[index..].chars().next().map_or(1, char::len_utf8)
}

fn newline_count(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count()
}

fn only_line_endings(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut output = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\r\n") {
            output.push_str("\r\n");
            index += 2;
        } else if bytes[index] == b'\n' {
            output.push('\n');
            index += 1;
        } else {
            index += 1;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_object_and_function_macros_recursively() {
        let source = "#define ONE 1\n#define ADD(x,y) ((x)+(y))\nint main(){return ADD(ONE,2);}\n";
        let output = expand_local_macros(source).unwrap();

        assert!(!output.contains("#define"));
        assert!(!output.contains("ADD"));
        assert!(!output.contains("ONE"));
        assert!(output.contains("((  1  )+( 2 ))"));
        assert_eq!(newline_count(&output), newline_count(source));
    }

    #[test]
    fn leaves_literals_comments_and_pp_numbers_untouched() {
        let source = r#"#define X 7
int main(){auto a="X";auto b=R"tag(X)tag";/* X */return 1X + X;}
"#;
        let output = expand_local_macros(source).unwrap();

        assert!(output.contains("\"X\""));
        assert!(output.contains("R\"tag(X)tag\""));
        assert!(output.contains("/* X */"));
        assert!(output.contains("1X"));
        assert!(output.contains(" 7 "));
    }

    #[test]
    fn preserves_conditionally_visible_macros() {
        let source = "#define FLAG 1\n#if FLAG\nint value=FLAG;\n#endif\n";
        let output = expand_local_macros(source).unwrap();

        assert!(output.contains("#define FLAG 1"));
        assert!(output.contains("#if FLAG"));
    }

    #[test]
    fn removes_unused_unsupported_macros_but_preserves_used_ones() {
        let unused = "#define STRINGIFY(x) #x\nint main(){return 0;}\n";
        assert!(!expand_local_macros(unused).unwrap().contains("STRINGIFY"));

        let used = "#define STRINGIFY(x) #x\nint main(){return STRINGIFY(value)[0];}\n";
        let output = expand_local_macros(used).unwrap();
        assert!(output.contains("#define STRINGIFY"));
        assert!(output.contains("STRINGIFY(value)"));
    }

    #[test]
    fn does_not_expand_macros_defined_before_a_later_include() {
        let source = "#define VALUE 1\n#include <header.hpp>\nint main(){return VALUE;}\n";
        let output = expand_local_macros(source).unwrap();

        assert_eq!(output, source);
    }

    #[test]
    fn uses_only_definitions_visible_at_each_invocation() {
        let source = "#define FIRST SECOND\nint SECOND=5;\nint before=FIRST;\n#define SECOND 7\nint after=FIRST;\n";
        let output = expand_local_macros(source).unwrap();

        assert!(!output.contains("#define"));
        assert!(output.contains("int before= SECOND ;"));
        assert!(output.contains("int after=  7  ;"));
    }

    #[test]
    fn preserves_wrong_arity_invocations_and_their_definitions() {
        let source = "#define ADD(x,y) ((x)+(y))\nint main(){return ADD(1);}\n";
        let output = expand_local_macros(source).unwrap();

        assert!(output.contains("#define ADD"));
        assert!(output.contains("ADD(1)"));
    }

    #[test]
    fn finds_macro_operators_outside_literals() {
        assert!(has_token_paste("left ## right"));
        assert!(has_token_paste("left %:%: right"));
        assert!(has_stringification("#value"));
        assert!(has_stringification("%: value"));
        assert!(!has_token_paste(r###""##" '##' R"tag(##)tag""###));
        assert!(!has_stringification(r###""#value" R"tag(%:)tag""###));
    }
}

use std::collections::{BTreeSet, HashMap, HashSet};

use tree_sitter::Node;

mod declarator;
mod names;
mod preprocessor;

use crate::rewrite::TextEdit;
use crate::{ObfuscationError, Options};
use declarator::{
    EntityKind, analyze_declarator, declaration_point, declarator_fields,
    declarator_name_is_qualified, find_descendant, innermost_function_declarator,
};
use names::{NameGenerator, is_reserved_identifier, scan_identifiers};
use preprocessor::has_token_paste;

type ScopeId = usize;
type SymbolId = usize;
type ByteRange = (usize, usize);

pub(crate) fn rename_edits(
    source: &str,
    root: Node<'_>,
    options: &Options,
) -> Result<Vec<TextEdit>, ObfuscationError> {
    let mut analyzer = Analyzer::new(source, root, options);
    analyzer.collect_source_identifiers(root);
    analyzer.collect_preprocessor_names(root)?;
    analyzer.collect_linkage_names(root, false);
    analyzer.collect_declarations(root, analyzer.root_scope);
    analyzer.collect_ignored_parameter_names(root);
    analyzer.collect_references(root, analyzer.root_scope);
    Ok(analyzer.build_edits(options.seed))
}

struct Analyzer<'source> {
    source: &'source str,
    root_scope: ScopeId,
    scopes: Vec<Scope>,
    scope_by_node: HashMap<usize, ScopeId>,
    namespace_scopes: HashMap<(ScopeId, String), ScopeId>,
    symbols: Vec<Symbol>,
    declarations: HashMap<ByteRange, SymbolId>,
    ignored_ranges: HashSet<ByteRange>,
    force_preserve_ranges: HashSet<ByteRange>,
    preserve_names: HashSet<String>,
    unresolved_names: HashSet<String>,
    used_names: HashSet<String>,
}

impl<'source> Analyzer<'source> {
    fn new(source: &'source str, root: Node<'_>, options: &Options) -> Self {
        let root_scope = 0;
        let mut scope_by_node = HashMap::new();
        scope_by_node.insert(root.id(), root_scope);

        let mut preserve_names: HashSet<String> = options.preserve.iter().cloned().collect();
        preserve_names.insert("main".to_string());

        Self {
            source,
            root_scope,
            scopes: vec![Scope {
                parent: None,
                kind: ScopeKind::Root,
                start: root.start_byte(),
                allow_parent_lookup: true,
                symbols: HashMap::new(),
            }],
            scope_by_node,
            namespace_scopes: HashMap::new(),
            symbols: Vec::new(),
            declarations: HashMap::new(),
            ignored_ranges: HashSet::new(),
            force_preserve_ranges: HashSet::new(),
            preserve_names,
            unresolved_names: HashSet::new(),
            used_names: HashSet::new(),
        }
    }

    fn collect_linkage_names(&mut self, node: Node<'_>, inherited_linkage: bool) {
        let has_linkage = inherited_linkage || node.kind() == "linkage_specification";

        match node.kind() {
            "function_definition" => {
                if (has_linkage || self.has_storage_class(node, "extern"))
                    && let Some(declarator) = node.child_by_field_name("declarator")
                {
                    self.force_preserve_declarator(declarator);
                }
                self.collect_linkage_children(node, false);
                return;
            }
            "declaration" => {
                if has_linkage || self.has_storage_class(node, "extern") {
                    for declarator in declarator_fields(node) {
                        self.force_preserve_declarator(declarator);
                    }
                }
                self.collect_linkage_children(node, false);
                return;
            }
            _ => {}
        }

        self.collect_linkage_children(node, has_linkage);
    }

    fn collect_linkage_children(&mut self, node: Node<'_>, inherited_linkage: bool) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_linkage_names(child, inherited_linkage);
        }
    }

    fn force_preserve_declarator(&mut self, declarator: Node<'_>) {
        if let Some(binding) = find_descendant(declarator, "structured_binding_declarator") {
            let mut cursor = binding.walk();
            for name in binding.named_children(&mut cursor) {
                if name.kind() == "identifier" {
                    self.force_preserve_ranges.insert(node_range(name));
                }
            }
            return;
        }

        if let Some(info) = analyze_declarator(declarator) {
            self.force_preserve_ranges.insert(node_range(info.name));
        }
    }

    fn has_storage_class(&self, node: Node<'_>, expected: &str) -> bool {
        let mut cursor = node.walk();
        node.named_children(&mut cursor).any(|child| {
            child.kind() == "storage_class_specifier" && self.node_text(child) == Some(expected)
        })
    }

    fn collect_source_identifiers(&mut self, node: Node<'_>) {
        if is_identifier_kind(node.kind()) {
            if let Some(text) = self.node_text(node) {
                self.used_names.insert(text.to_string());
            }
        } else if node.kind() == "preproc_arg"
            && let Some(text) = self.node_text(node)
        {
            self.used_names.extend(scan_identifiers(text));
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_source_identifiers(child);
        }
    }

    fn collect_preprocessor_names(&mut self, node: Node<'_>) -> Result<(), ObfuscationError> {
        match node.kind() {
            "preproc_function_def" => {
                let name = node.child_by_field_name("name");
                if let Some(name) = name.and_then(|name| self.node_text(name)) {
                    self.preserve_names.insert(name.to_string());
                }

                let parameters: HashSet<String> = node
                    .child_by_field_name("parameters")
                    .map(|parameters| self.identifier_texts(parameters).into_iter().collect())
                    .unwrap_or_default();

                if let Some(value) = node.child_by_field_name("value") {
                    let text = self.node_text(value).unwrap_or_default();
                    reject_token_pasting(text)?;
                    for identifier in scan_identifiers(text) {
                        if !parameters.contains(&identifier) {
                            self.preserve_names.insert(identifier);
                        }
                    }
                }
                return Ok(());
            }
            "preproc_def" => {
                if let Some(name) = node
                    .child_by_field_name("name")
                    .and_then(|name| self.node_text(name))
                {
                    self.preserve_names.insert(name.to_string());
                }
                if let Some(value) = node.child_by_field_name("value") {
                    let text = self.node_text(value).unwrap_or_default();
                    reject_token_pasting(text)?;
                    self.preserve_names.extend(scan_identifiers(text));
                }
                return Ok(());
            }
            "preproc_include" | "preproc_call" => {
                if let Some(text) = self.node_text(node) {
                    reject_token_pasting(text)?;
                    self.preserve_names.extend(scan_identifiers(text));
                }
                return Ok(());
            }
            "preproc_if" | "preproc_elif" => {
                if let Some(condition) = node.child_by_field_name("condition")
                    && let Some(text) = self.node_text(condition)
                {
                    self.preserve_names.extend(scan_identifiers(text));
                }
            }
            "preproc_ifdef" | "preproc_elifdef" => {
                if let Some(name) = node
                    .child_by_field_name("name")
                    .and_then(|name| self.node_text(name))
                {
                    self.preserve_names.insert(name.to_string());
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_preprocessor_names(child)?;
        }
        Ok(())
    }

    fn collect_declarations(&mut self, node: Node<'_>, current_scope: ScopeId) {
        match node.kind() {
            "translation_unit" => {
                self.collect_children(node, current_scope);
            }
            "namespace_definition" => {
                let scope = self.add_namespace_scope(node, current_scope);
                self.collect_children(node, scope);
            }
            "class_specifier" | "struct_specifier" | "union_specifier" => {
                let scope = self.add_scope(node, current_scope, ScopeKind::Class, true);
                self.collect_children(node, scope);
            }
            "function_definition" => {
                let qualified = self.register_function_definition(node, current_scope);
                let function_scope =
                    self.add_scope(node, current_scope, ScopeKind::Function, !qualified);

                if let Some(declarator) = node.child_by_field_name("declarator") {
                    self.collect_function_parameters(declarator, function_scope);
                }

                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if node_field(node, child) != Some("declarator") {
                        self.collect_declarations(child, function_scope);
                    }
                }
            }
            "lambda_expression" => {
                self.collect_lambda(node, current_scope);
            }
            "catch_clause" => {
                let scope = self.add_scope(node, current_scope, ScopeKind::Catch, true);
                if let Some(parameters) = node.child_by_field_name("parameters") {
                    self.collect_parameter_list(parameters, scope);
                }
                if let Some(body) = node.child_by_field_name("body") {
                    self.collect_declarations(body, scope);
                }
            }
            "for_range_loop" => {
                let scope = self.add_scope(node, current_scope, ScopeKind::Control, true);
                let visible_from = node
                    .child_by_field_name("body")
                    .map_or(node.end_byte(), |body| body.start_byte());
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    self.register_declarator(
                        declarator,
                        scope,
                        visible_from,
                        DeclarationContext::Value,
                    );
                }
                self.collect_children(node, scope);
            }
            "compound_statement" => {
                let scope = self.add_scope(node, current_scope, ScopeKind::Block, true);
                self.collect_children(node, scope);
            }
            "for_statement" | "if_statement" | "switch_statement" | "while_statement"
            | "do_statement" => {
                let scope = self.add_scope(node, current_scope, ScopeKind::Control, true);
                self.collect_children(node, scope);
            }
            "declaration" => {
                self.register_node_declarators(node, current_scope, DeclarationContext::Value);
                self.collect_children(node, current_scope);
            }
            "field_declaration" => {
                self.register_node_declarators(node, current_scope, DeclarationContext::Member);
                self.collect_children(node, current_scope);
            }
            "friend_declaration" => {
                self.preserve_friend_names(node);
                self.collect_children(node, current_scope);
            }
            "enumerator" => {
                if let Some(name) = node.child_by_field_name("name").or_else(|| {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).next()
                }) {
                    self.declare(
                        name,
                        SymbolKind::Barrier,
                        current_scope,
                        self.scopes[current_scope].start,
                        true,
                    );
                }
                self.collect_children(node, current_scope);
            }
            _ if is_preprocessor_definition(node.kind()) => {}
            _ => self.collect_children(node, current_scope),
        }
    }

    fn collect_children(&mut self, node: Node<'_>, current_scope: ScopeId) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_declarations(child, current_scope);
        }
    }

    fn preserve_friend_names(&mut self, node: Node<'_>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "declaration" => {
                    for declarator in declarator_fields(child) {
                        self.preserve_declarator_name(declarator);
                    }
                }
                "function_definition" => {
                    if let Some(declarator) = child.child_by_field_name("declarator") {
                        self.preserve_declarator_name(declarator);
                    }
                }
                _ => {}
            }
        }
    }

    fn preserve_declarator_name(&mut self, declarator: Node<'_>) {
        if let Some(info) = analyze_declarator(declarator)
            && let Some(name) = self.node_text(info.name)
        {
            self.preserve_names.insert(name.to_string());
        }
    }

    fn collect_lambda(&mut self, node: Node<'_>, current_scope: ScopeId) {
        let lambda_scope = self.add_scope(node, current_scope, ScopeKind::Lambda, true);
        let body_start = node
            .child_by_field_name("body")
            .map_or(node.end_byte(), |body| body.start_byte());

        if let Some(captures) = node.child_by_field_name("captures") {
            let mut cursor = captures.walk();
            for capture in captures.named_children(&mut cursor) {
                if capture.kind() == "lambda_capture_initializer" {
                    if let Some(left) = capture.child_by_field_name("left") {
                        self.declare(left, SymbolKind::Variable, lambda_scope, body_start, false);
                    }
                    if let Some(right) = capture.child_by_field_name("right") {
                        self.collect_declarations(right, current_scope);
                    }
                }
            }
        }

        if let Some(declarator) = node.child_by_field_name("declarator")
            && let Some(parameters) = declarator.child_by_field_name("parameters")
        {
            self.collect_parameter_list(parameters, lambda_scope);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.collect_declarations(body, lambda_scope);
        }
    }

    fn register_function_definition(&mut self, node: Node<'_>, current_scope: ScopeId) -> bool {
        let Some(declarator) = node.child_by_field_name("declarator") else {
            return false;
        };
        let qualified = declarator_name_is_qualified(declarator);
        let info = analyze_declarator(declarator);

        if qualified {
            if let Some(info) = info {
                self.ignored_ranges.insert(node_range(info.name));
            }
            return true;
        }

        let Some(info) = info else {
            return false;
        };
        if self.scopes[current_scope].kind == ScopeKind::Class
            || info.name.kind() == "field_identifier"
        {
            self.declare(
                info.name,
                SymbolKind::Barrier,
                current_scope,
                self.scopes[current_scope].start,
                true,
            );
            return false;
        }

        if info.entity == EntityKind::Function {
            self.declare(
                info.name,
                SymbolKind::Function,
                current_scope,
                declarator.end_byte(),
                false,
            );
        } else {
            self.ignored_ranges.insert(node_range(info.name));
        }
        false
    }

    fn register_node_declarators(
        &mut self,
        node: Node<'_>,
        current_scope: ScopeId,
        context: DeclarationContext,
    ) {
        for declarator in declarator_fields(node) {
            let visible_from = declaration_point(declarator);
            self.register_declarator(declarator, current_scope, visible_from, context);
        }
    }

    fn register_declarator(
        &mut self,
        declarator: Node<'_>,
        current_scope: ScopeId,
        visible_from: usize,
        context: DeclarationContext,
    ) {
        if let Some(binding) = find_descendant(declarator, "structured_binding_declarator") {
            let mut cursor = binding.walk();
            for name in binding.named_children(&mut cursor) {
                if name.kind() == "identifier" {
                    self.declare(
                        name,
                        SymbolKind::Variable,
                        current_scope,
                        visible_from,
                        false,
                    );
                }
            }
            return;
        }

        let Some(info) = analyze_declarator(declarator) else {
            return;
        };

        if info.qualified {
            self.ignored_ranges.insert(node_range(info.name));
            return;
        }

        if context == DeclarationContext::Member
            || self.scopes[current_scope].kind == ScopeKind::Class
            || info.name.kind() == "field_identifier"
        {
            self.declare(
                info.name,
                SymbolKind::Barrier,
                current_scope,
                self.scopes[current_scope].start,
                true,
            );
            return;
        }

        let kind = match info.entity {
            EntityKind::Function => SymbolKind::Function,
            EntityKind::Variable | EntityKind::Unknown => SymbolKind::Variable,
        };

        self.declare(
            info.name,
            kind,
            current_scope,
            visible_from.max(info.name.end_byte()),
            false,
        );
    }

    fn collect_function_parameters(&mut self, declarator: Node<'_>, function_scope: ScopeId) {
        let function_declarator = analyze_declarator(declarator)
            .and_then(|info| info.function_declarator)
            .or_else(|| innermost_function_declarator(declarator));
        if let Some(function_declarator) = function_declarator
            && let Some(parameters) = function_declarator.child_by_field_name("parameters")
        {
            self.collect_parameter_list(parameters, function_scope);
        }
    }

    fn collect_parameter_list(&mut self, list: Node<'_>, scope: ScopeId) {
        let mut cursor = list.walk();
        for parameter in list.named_children(&mut cursor) {
            if !matches!(
                parameter.kind(),
                "parameter_declaration"
                    | "optional_parameter_declaration"
                    | "variadic_parameter_declaration"
            ) {
                continue;
            }
            if let Some(declarator) = parameter.child_by_field_name("declarator") {
                self.register_declarator(
                    declarator,
                    scope,
                    declarator.end_byte(),
                    DeclarationContext::Value,
                );
            }
        }
    }

    fn collect_ignored_parameter_names(&mut self, node: Node<'_>) {
        if matches!(
            node.kind(),
            "parameter_declaration"
                | "optional_parameter_declaration"
                | "variadic_parameter_declaration"
        ) && let Some(declarator) = node.child_by_field_name("declarator")
            && let Some(info) = analyze_declarator(declarator)
        {
            let range = node_range(info.name);
            if !self.declarations.contains_key(&range) {
                self.ignored_ranges.insert(range);
            }
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_ignored_parameter_names(child);
        }
    }

    fn collect_references(&mut self, node: Node<'_>, inherited_scope: ScopeId) {
        let current_scope = self
            .scope_by_node
            .get(&node.id())
            .copied()
            .unwrap_or(inherited_scope);

        match node.kind() {
            "preproc_def"
            | "preproc_function_def"
            | "preproc_include"
            | "preproc_call"
            | "preproc_arg"
            | "preproc_params"
            | "preproc_defined"
            | "preproc_directive" => return,
            "preproc_if" | "preproc_elif" | "preproc_ifdef" | "preproc_elifdef" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if !matches!(node_field(node, child), Some("condition" | "name")) {
                        self.collect_references(child, current_scope);
                    }
                }
                return;
            }
            "qualified_identifier" | "using_declaration" => {
                self.preserve_names.extend(self.identifier_texts(node));
                return;
            }
            "field_expression" => {
                if let Some(argument) = node.child_by_field_name("argument") {
                    self.collect_references(argument, current_scope);
                }
                if let Some(field) = node.child_by_field_name("field") {
                    self.preserve_names.extend(self.identifier_texts(field));
                }
                return;
            }
            "identifier" => {
                self.collect_identifier_reference(node, current_scope);
                return;
            }
            "field_identifier"
            | "type_identifier"
            | "namespace_identifier"
            | "statement_identifier"
            | "literal_suffix" => return,
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_references(child, current_scope);
        }
    }

    fn collect_identifier_reference(&mut self, node: Node<'_>, scope: ScopeId) {
        let range = node_range(node);
        if self.declarations.contains_key(&range) || self.ignored_ranges.contains(&range) {
            return;
        }
        let Some(name) = self.node_text(node).map(str::to_string) else {
            return;
        };

        match self.lookup(scope, &name, node.start_byte()) {
            LookupResult::Resolved(symbol) => {
                if self.symbols[symbol].kind != SymbolKind::Barrier {
                    self.symbols[symbol].occurrences.insert(range);
                }
            }
            LookupResult::Ambiguous | LookupResult::Missing => {
                self.unresolved_names.insert(name);
            }
        }
    }

    fn build_edits(&mut self, seed: u64) -> Vec<TextEdit> {
        self.preserve_names
            .extend(self.unresolved_names.iter().cloned());

        let mut candidates: Vec<SymbolId> = self
            .symbols
            .iter()
            .enumerate()
            .filter_map(|(id, symbol)| {
                (symbol.kind != SymbolKind::Barrier
                    && !symbol.force_preserve
                    && !self.preserve_names.contains(&symbol.name)
                    && !is_reserved_identifier(&symbol.name))
                .then_some(id)
            })
            .collect();

        candidates.sort_by(|left, right| {
            self.symbols[*right]
                .occurrences
                .len()
                .cmp(&self.symbols[*left].occurrences.len())
                .then_with(|| {
                    self.symbols[*left]
                        .first_declaration
                        .cmp(&self.symbols[*right].first_declaration)
                })
        });

        let mut generator = NameGenerator::new(seed, self.used_names.clone());
        let mut edits = Vec::new();
        for symbol_id in candidates {
            let new_name = generator.next_name();
            for &(start, end) in &self.symbols[symbol_id].occurrences {
                edits.push(TextEdit {
                    start,
                    end,
                    replacement: new_name.clone(),
                });
            }
        }
        edits
    }

    fn add_scope(
        &mut self,
        node: Node<'_>,
        parent: ScopeId,
        kind: ScopeKind,
        allow_parent_lookup: bool,
    ) -> ScopeId {
        if let Some(scope) = self.scope_by_node.get(&node.id()) {
            return *scope;
        }

        let id = self.scopes.len();
        self.scopes.push(Scope {
            parent: Some(parent),
            kind,
            start: node.start_byte(),
            allow_parent_lookup,
            symbols: HashMap::new(),
        });
        self.scope_by_node.insert(node.id(), id);
        id
    }

    fn add_namespace_scope(&mut self, node: Node<'_>, parent: ScopeId) -> ScopeId {
        let key = node
            .child_by_field_name("name")
            .and_then(|name| self.node_text(name))
            .unwrap_or("<anonymous>")
            .to_string();

        let scope = if let Some(scope) = self.namespace_scopes.get(&(parent, key.clone())) {
            *scope
        } else {
            let scope = self.scopes.len();
            self.scopes.push(Scope {
                parent: Some(parent),
                kind: ScopeKind::Namespace,
                start: node.start_byte(),
                allow_parent_lookup: true,
                symbols: HashMap::new(),
            });
            self.namespace_scopes.insert((parent, key), scope);
            scope
        };

        self.scope_by_node.insert(node.id(), scope);
        scope
    }

    fn declare(
        &mut self,
        name_node: Node<'_>,
        kind: SymbolKind,
        scope: ScopeId,
        visible_from: usize,
        force_preserve: bool,
    ) -> Option<SymbolId> {
        let name = self.node_text(name_node)?.to_string();
        let range = node_range(name_node);
        let force_preserve = force_preserve || self.force_preserve_ranges.contains(&range);

        let existing = self.scopes[scope].symbols.get(&name).and_then(|symbols| {
            symbols
                .iter()
                .copied()
                .find(|symbol| self.symbols[*symbol].kind == kind)
        });

        let symbol_id = if let Some(symbol_id) = existing {
            let symbol = &mut self.symbols[symbol_id];
            symbol.visible_from = symbol.visible_from.min(visible_from);
            symbol.first_declaration = symbol.first_declaration.min(name_node.start_byte());
            symbol.force_preserve |= force_preserve;
            symbol_id
        } else {
            let symbol_id = self.symbols.len();
            self.symbols.push(Symbol {
                name: name.clone(),
                kind,
                visible_from,
                first_declaration: name_node.start_byte(),
                force_preserve,
                occurrences: BTreeSet::new(),
            });
            self.scopes[scope]
                .symbols
                .entry(name)
                .or_default()
                .push(symbol_id);
            symbol_id
        };

        self.symbols[symbol_id].occurrences.insert(range);
        self.declarations.insert(range, symbol_id);
        Some(symbol_id)
    }

    fn lookup(&self, mut scope: ScopeId, name: &str, position: usize) -> LookupResult {
        loop {
            if let Some(symbols) = self.scopes[scope].symbols.get(name) {
                let visible: Vec<SymbolId> = symbols
                    .iter()
                    .copied()
                    .filter(|symbol| self.symbols[*symbol].visible_from <= position)
                    .collect();
                match visible.as_slice() {
                    [symbol] => return LookupResult::Resolved(*symbol),
                    [] => {}
                    _ => return LookupResult::Ambiguous,
                }
            }

            if !self.scopes[scope].allow_parent_lookup {
                return LookupResult::Missing;
            }
            let Some(parent) = self.scopes[scope].parent else {
                return LookupResult::Missing;
            };
            scope = parent;
        }
    }

    fn identifier_texts(&self, node: Node<'_>) -> Vec<String> {
        let mut output = Vec::new();
        self.collect_identifier_texts(node, &mut output);
        output
    }

    fn collect_identifier_texts(&self, node: Node<'_>, output: &mut Vec<String>) {
        if is_identifier_kind(node.kind())
            && let Some(text) = self.node_text(node)
        {
            output.push(text.to_string());
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_identifier_texts(child, output);
        }
    }

    fn node_text(&self, node: Node<'_>) -> Option<&'source str> {
        node.utf8_text(self.source.as_bytes()).ok()
    }
}

#[derive(Debug)]
struct Scope {
    parent: Option<ScopeId>,
    kind: ScopeKind,
    start: usize,
    allow_parent_lookup: bool,
    symbols: HashMap<String, Vec<SymbolId>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Root,
    Namespace,
    Class,
    Function,
    Lambda,
    Block,
    Control,
    Catch,
}

#[derive(Debug)]
struct Symbol {
    name: String,
    kind: SymbolKind,
    visible_from: usize,
    first_declaration: usize,
    force_preserve: bool,
    occurrences: BTreeSet<ByteRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolKind {
    Variable,
    Function,
    Barrier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeclarationContext {
    Value,
    Member,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookupResult {
    Resolved(SymbolId),
    Ambiguous,
    Missing,
}

fn node_field(parent: Node<'_>, child: Node<'_>) -> Option<&'static str> {
    for index in 0..parent.child_count() {
        if parent.child(index as u32) == Some(child) {
            return parent.field_name_for_child(index as u32);
        }
    }
    None
}

fn node_range(node: Node<'_>) -> ByteRange {
    (node.start_byte(), node.end_byte())
}

fn is_identifier_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "field_identifier"
            | "type_identifier"
            | "namespace_identifier"
            | "statement_identifier"
            | "literal_suffix"
    )
}

fn is_preprocessor_definition(kind: &str) -> bool {
    matches!(
        kind,
        "preproc_def" | "preproc_function_def" | "preproc_include" | "preproc_call"
    )
}

fn reject_token_pasting(text: &str) -> Result<(), ObfuscationError> {
    if has_token_paste(text) {
        Err(ObfuscationError::Unsupported(
            "token-pasting macros can synthesize identifiers".to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Options, obfuscate};

    #[test]
    fn renames_shadowed_locals_independently() {
        let output = obfuscate(
            "int main(){ int value=1; { int value=2; value++; } return value; }",
            &Options::default(),
        )
        .unwrap();

        assert!(!output.contains("value"));
    }

    #[test]
    fn preserves_macro_dependencies_but_not_macro_parameters() {
        let output = obfuscate(
            "#define CALL(x) helper(x)\nint helper(int x){return x;}\nint main(){return CALL(1);}",
            &Options::default(),
        )
        .unwrap();

        assert!(output.contains("helper"));
        assert!(!output.contains("int x"));
    }

    #[test]
    fn rejects_identifier_token_pasting() {
        let error = obfuscate(
            "#define JOIN(a,b) a ## b\nint main(){return 0;}",
            &Options::default(),
        )
        .unwrap_err();

        assert!(matches!(error, ObfuscationError::Unsupported(_)));
    }

    #[test]
    fn accepts_hash_pairs_inside_macro_literals() {
        let output = obfuscate(
            "#define HASH_TEXT \"##\"\nint main(){return HASH_TEXT[0];}",
            &Options::default(),
        )
        .unwrap();

        assert!(output.contains("\"##\""));
    }

    #[test]
    fn renames_parameters_when_function_names_are_operators() {
        let output = obfuscate(
            r#"
struct Counter {
    int value;
    int operator()(int deltaValue) const;
};
int Counter::operator()(int deltaValue) const {
    return value + deltaValue;
}
int main() {
    Counter counterValue{1};
    return counterValue(2);
}
"#,
            &Options::default(),
        )
        .unwrap();

        assert_eq!(output.matches("deltaValue").count(), 1);
        assert!(!output.contains("return value + deltaValue"));
        assert!(!output.contains("counterValue"));
        assert!(output.contains("operator()"));
    }

    #[test]
    fn qualified_member_bodies_do_not_bind_bare_members_to_globals() {
        let output = obfuscate(
            r#"
int memberValue = 7;
struct Item {
    int memberValue;
    int read() const;
};
int Item::read() const {
    return memberValue;
}
int main() {
    Item itemValue{1};
    return itemValue.read();
}
"#,
            &Options::default(),
        )
        .unwrap();

        assert!(output.contains("int memberValue = 7"));
        assert!(output.contains("return memberValue"));
        assert!(!output.contains("itemValue"));
    }

    #[test]
    fn preserves_namespace_functions_declared_as_friends() {
        let output = obfuscate(
            r#"
class Item {
    int value;
public:
    explicit Item(int inputValue) : value(inputValue) {}
    friend int inspectValue(const Item& itemValue);
};
int inspectValue(const Item& itemValue) {
    return itemValue.value;
}
int main() {
    return inspectValue(Item{7});
}
"#,
            &Options::default(),
        )
        .unwrap();

        assert!(output.contains("inspectValue"));
        assert!(!output.contains("inputValue"));
    }
}

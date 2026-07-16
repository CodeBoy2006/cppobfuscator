# cppobfuscator

English | [Chinese (中文)](README.zh.md)

`cppobfuscator` is a correctness-first obfuscator for one UTF-8 C++ translation unit, aimed at single-file competitive-programming code. It uses Tree-sitter C++ syntax nodes, lexical scopes, and byte-range edits instead of global text replacement.

> Use this tool only where contest and platform rules allow it. You are responsible for validating and submitting the generated source.

## Design

The transformation pipeline is intentionally narrow:

1. Parse the original source with `tree-sitter-cpp`.
2. Build lexical scopes, declaration points, overload groups, and identifier references.
3. Generate non-overlapping byte-range rename edits.
4. Reparse and require the same non-comment AST structure.
5. Remove comments and/or compact layout.
6. Reparse and validate the AST structure again.

Version `0.2` deliberately removes the old text-based macro expansion, function inlining, integer constant lifting, and unused-code deletion passes. Those transformations could change evaluation order, overload resolution, initialization side effects, or valid template code.

## Rename Policy

The tool renames symbols only when its lexical model can resolve them consistently:

- Local variables, structured bindings, lambda init-captures, and parameters.
- Ordinary free functions, including overload groups.
- File-scope and namespace variables when references are unambiguous.

The following are preserved conservatively:

- `main`, user-specified names, unresolved names, and reserved identifiers.
- Types, namespaces, fields, methods, operators, labels, and qualified names.
- `extern` and language-linkage entities.
- Namespace functions declared through `friend`.
- Macro names and non-parameter identifiers used in macro replacement text.

Generated names use lowercase letters and digits, never collide with an identifier already present in the source, and are deterministic for a given seed.

## Source Handling

- Macros are not expanded, and replacement text is not rewritten.
- Common function-like statement macros whose replacement starts with `for`, `if`, `while`, or `switch` are accepted conservatively.
- Multi-line macro continuations are retained.
- Raw strings, UTF-8 text, digit separators, and user-defined literal suffixes remain intact.
- Token-pasting macros using `##` or `%:%:` are rejected because they can synthesize identifiers that do not exist in the parsed AST.

Tree-sitter is a syntax parser, not a C++ compiler or preprocessor. Always compile and test the generated file with the same compiler flags used for submission.

## CLI

```text
cppobfuscator [OPTIONS]

-i, --input <PATH>       Read C++ source from a file (default: stdin)
-o, --output <PATH>      Write transformed source to a file (default: stdout)
    --seed <U64>         Rename seed; decimal or 0x-prefixed (default: 0xC0FFEE)
    --preserve <NAME>    Preserve an identifier; may be repeated
    --keep-comments      Keep comments
    --keep-layout        Keep original whitespace and line layout
-h, --help               Print help
-V, --version            Print version
```

File input and output:

```bash
cppobfuscator -i solution.cpp -o solution.obfuscated.cpp
```

Pipe through stdin/stdout:

```bash
cppobfuscator --seed 42 < solution.cpp > solution.obfuscated.cpp
```

Preserve an externally required function:

```bash
cppobfuscator --preserve solve -i solution.cpp -o solution.obfuscated.cpp
```

## Rust API

```rust
use cppobfuscator::{Options, obfuscate};

let output = obfuscate(source, &Options::default())?;
```

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
```

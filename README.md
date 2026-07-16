# cppobfuscator

English | [中文](README.zh.md)

`cppobfuscator` is a low-overhead C++ source obfuscator for single-file contest
programs. It combines Tree-sitter scope analysis, AST byte-range rewrites, and
compile-time lexical transformations. It does not expand headers or macros and
does not use global text replacement.

> Use it only when contest and platform rules permit source transformation.
> Always compile and test the generated file with the submission toolchain.

## Profiles

`0.3` provides three deterministic profiles:

| Profile | Transformations |
| --- | --- |
| `symbols` | Rename resolved variables, free functions, template parameters, and labels; compact layout and remove source comments as configured. |
| `balanced` | Default. Adds per-function short-name reuse, safe integer radix changes, ASCII string/character octal escapes, and C++ alternative operator tokens. |
| `maximum` | Adds visually ambiguous `i/l/o/0/1` names and validated separator comments between non-preprocessor tokens. |

The advanced profiles only change compile-time source representation. They do
not add runtime decoders, opaque branches, control-flow dispatchers, heap
allocations, or initialization work. A representative calculation kernel
produces byte-identical `g++ -O2` assembly before and after transformation.

## Pipeline

1. Parse the UTF-8 translation unit with `tree-sitter-cpp`.
2. Collect source identifiers, macro dependencies, linkage constraints, and
   C++ lookup hazards.
3. Build lexical scopes, declaration points, overload groups, template
   parameters, labels, and identifier references.
4. Apply non-overlapping symbol edits and reparse.
5. Rewrite safe literals and expression operator spellings and reparse.
6. Remove source comments and render compact layout while preserving every
   physical newline and macro continuation.
7. Reparse and require the normalized non-comment AST structure to match.

## Symbol obfuscation

The analyzer renames:

- Local, file, and namespace variables with unambiguous lexical references.
- Function parameters, structured bindings, and lambda init-captures.
- Ordinary free functions and same-scope overload groups.
- Type and non-type template parameters, including dependent qualified uses.
- Standard `goto` labels and their references.

Names remain unchanged when correctness cannot be established:

- `main`, explicit `--preserve` names, reserved or unresolved identifiers.
- Types, namespaces, fields, methods, operators, enum constants, and qualified
  member names.
- External linkage, language linkage, and friend-declared functions.
- Macro names and non-parameter identifiers used by macro replacement text.
- Cross-scope same-name functions and source functions called while a
  `using namespace` directive can extend the overload set.
- Bare outer-scope names in classes with base classes, where inherited member
  lookup requires compiler semantics.

Balanced and maximum profiles reuse the same local short names in independent
functions while reserving every generated translation-unit name. Maximum uses
an ambiguous lowercase alphabet; generated names never collide with identifiers
present in the source.

## Lexical obfuscation

- Integer literals within the signed 32-bit range may be rendered in binary,
  octal, or hexadecimal while preserving their suffix. Floating literals,
  large integers, and user-defined literals are left unchanged.
- Direct printable ASCII string content is encoded with fixed-width octal
  escapes. Existing escapes, raw strings, and non-ASCII text remain intact.
- Direct printable ASCII character literals use octal escapes.
- Expression operators may use `and`, `or`, `not`, `bitand`, `bitor`, `xor`,
  `compl`, `and_eq`, `or_eq`, `xor_eq`, and `not_eq`. Pointer/reference syntax
  and unary address-of remain symbolic.
- Maximum profile replaces eligible horizontal separators with `/**/` or
  `/*_*/`. It avoids preprocessor lines, alternative-token boundaries, and
  division boundaries.

Compaction preserves the exact number of physical newlines, so `__LINE__`
continues to observe the original line numbering. `--keep-layout` disables
compaction and separator-comment insertion.

Balanced and maximum output requires C++14 or later because binary integer
literals may be emitted.

## Macro boundaries

- Macro replacement text is never rewritten or expanded.
- Function-like statement macros beginning with `for`, `if`, `while`, or
  `switch` are accepted when Tree-sitter reports only their synthetic missing
  semicolon.
- Backslash-newline continuations are preserved.
- `##` and `%:%:` token-pasting macros are rejected.
- `#` and `%:` stringifying function macros are rejected because renaming an
  argument can change the resulting string.

Definitions from included headers are not visible. Header-provided
stringification, token pasting, unusual lowercase macros, compiler extensions,
and full ADL/type lookup remain outside the model.

## CLI

```text
cppobfuscator [OPTIONS]

-i, --input <PATH>       Read C++ source from a file (default: stdin)
-o, --output <PATH>      Write transformed source to a file (default: stdout)
    --seed <U64>         Rename seed; decimal or 0x-prefixed (default: 0xC0FFEE)
    --preserve <NAME>    Preserve an identifier; may be repeated
    --profile <NAME>     symbols, balanced, or maximum (default: balanced)
    --keep-comments      Keep source comments
    --keep-layout        Keep original whitespace and line layout
-h, --help               Print help
-V, --version            Print version
```

```bash
cppobfuscator --profile maximum -i solution.cpp -o solution.obfuscated.cpp
```

## Rust API

```rust
use cppobfuscator::{Options, Profile, obfuscate};

let options = Options {
    profile: Profile::Maximum,
    ..Options::default()
};
let output = obfuscate(source, &options)?;
```

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build --release
```

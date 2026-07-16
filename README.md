# cppobfuscator

English | [中文](README.zh.md)

`cppobfuscator` is a low-overhead C++ source obfuscator for single-file contest
programs. It combines Tree-sitter scope analysis, AST byte-range rewrites, and
compile-time lexical transformations. Before obfuscation it expands supported
source-local macros and simplifies AST-proven redundant code. It does not expand
included headers, invoke a compiler preprocessor, or use global text replacement.

> Use it only when contest and platform rules permit source transformation.
> Always compile and test the generated file with the submission toolchain.

## Profiles

`0.4.1` provides three deterministic profiles:

| Profile | Transformations |
| --- | --- |
| `symbols` | Rename resolved variables, free functions, template parameters, and labels; compact layout and remove source comments as configured. |
| `balanced` | Default. Adds per-function short-name reuse, safe integer radix changes, ASCII string/character octal escapes, and C++ alternative operator tokens. |
| `maximum` | Adds four-or-more-character ambiguous function names, validated arithmetic integers, deterministic inline lambda flow shards, and separator comments between non-preprocessor tokens. |

Balanced transformations only change compile-time source representation.
Maximum flow sharding creates local capture lambdas, but adds no runtime
decoder, opaque branch, state machine, heap allocation, or persistent state.
GCC and Clang fully inline representative shards at `-O2`; unoptimized builds
may retain local lambda calls, so generated code must be tested with the target
contest flags.

## Pipeline

1. Scan preprocessing directives and recursively expand eligible source-local
   object-like and function-like macros using definition-order-aware token rules.
2. Remove expanded and unused local macro definitions while preserving every
   physical newline.
3. Parse the normalized UTF-8 translation unit with `tree-sitter-cpp`.
4. Remove AST-proven empty statements, literal constant branches, false loops,
   and unreachable runtime tails, then reparse.
5. Collect source identifiers, remaining macro dependencies, linkage
   constraints, and C++ lookup hazards.
6. Build lexical scopes, declaration points, overload groups, template
   parameters, labels, and identifier references.
7. Apply non-overlapping symbol edits and reparse.
8. In maximum mode, split eligible expression-statement runs into validated
   immediately invoked lambda shards while preserving physical lines.
9. Replace eligible maximum-profile integer leaves with tracked arithmetic
   expression subtrees, verify each generated decoder, and reparse.
10. Establish the intentional post-constant AST as the validation baseline,
   then rewrite safe literal spellings and expression operators.
11. Remove source comments and render compact layout while preserving every
   physical newline and macro continuation.
12. Reparse and require the normalized non-comment AST structure to match the
   validated post-constant baseline.

## Pre-obfuscation normalization

- A macro is eligible for expansion only when it has one unconditional local
  definition, is not redefined or undefined, is not referenced by another
  preprocessing directive, and is defined after the last include.
- Object-like and fixed-arity function-like macros are recursively expanded
  using the definitions visible at each invocation. Strings, character
  literals, raw strings, comments, and preprocessing-number tokens are never
  scanned as macro identifiers.
- Invocation arguments follow preprocessing parenthesis rules. Wrong arity,
  recursion that leaves a macro token, variadic macros, multiline replacement
  tokens, and unsupported special operators cause the relevant definitions to
  remain in place.
- Unused local macros are removed even when they use unsupported `#` or `##`
  operators. A used stringifying or token-pasting macro remains rejected by the
  symbol phase because it can expose or synthesize identifier spellings.
- Simplification folds literal `if` conditions and `while(false)`, removes empty
  statements in blocks, and drops runtime statements after unconditional
  `return`, `co_return`, `break`, `continue`, or `goto`.
- Simplification does not cross labels, `case` entries, or preprocessing nodes.
  It does not perform type-driven dead declaration elimination or function
  inlining.

## Symbol obfuscation

The analyzer renames:

- Local, file, and namespace variables with unambiguous lexical references.
- Function parameters, structured bindings, and lambda init-captures.
- Ordinary free functions, function templates, and same-scope overload groups.
- Type and non-type template parameters, including dependent qualified uses.
- Standard `goto` labels and their references.

Names remain unchanged when correctness cannot be established:

- `main`, explicit `--preserve` names, reserved or unresolved identifiers.
- Types, namespaces, fields, methods, operators, enum constants, and qualified
  member names.
- Explicit `extern`, language-linkage, and friend-declared functions.
- Macro names and non-parameter identifiers used by macro replacement text.
- Functions that observe their own name through `__func__`,
  `__PRETTY_FUNCTION__`, compiler equivalents, or `std::source_location`.
- Cross-scope same-name functions, common standard-library overload hazards
  imported by `using namespace std`, and calls exposed by an unmodeled external
  namespace directive.
- Bare outer-scope names in classes with base classes, where inherited member
  lookup requires compiler semantics.

Balanced and maximum profiles reuse the same local short names in independent
functions while reserving every generated translation-unit name. Maximum puts
functions in a separate deterministic name domain with a minimum length of four
characters from the ambiguous `i/l/o/0/1` alphabet; generated names never
collide with identifiers present in the source.

## Function and flow obfuscation

- Source-local free functions are renamed consistently across definitions,
  forward declarations, recursion, calls, address-taking expressions, explicit
  template specializations, and explicit instantiations.
- Template function entities are registered in their enclosing namespace while
  template parameters remain in their lexical template scope. This lets called
  function templates be renamed without losing dependent-name lookup.
- Maximum partitions consecutive eligible expression statements into
  deterministic one-to-three-statement shards such as
  `([&](){ first(); second(); }());`. Lambdas are implicitly inline and each
  generated range is required to parse as one exact invocation node.
- Declarations and `return`, `break`, `continue`, `goto`, labels, and `case`
  entries are never moved into a shard. Existing lambda bodies are not rewritten.
- Entire functions are excluded when they are `constexpr`/`consteval`,
  variadic, coroutine-based, contain structured bindings or preprocessing
  directives, anonymous unions, GNU label addresses, use SEH or `register`, or
  depend on `alloca`, setjmp/longjmp, varargs, frame-address, return-address, or
  function-identity facilities. GNU asm, requires expressions, and statement
  expressions are excluded at the individual-statement level.
- At `-O2`, the tested GCC and Clang outputs contain no residual lambda call
  symbols. At `-O0`, compilers may materialize closure frames and calls; maximum
  is intended for optimized contest builds.

## Lexical obfuscation

- Integer literals within the signed 32-bit range may be rendered in binary,
  octal, or hexadecimal while preserving their suffix. Floating literals,
  large integers, and user-defined literals are left unchanged.
- Maximum additionally replaces eligible non-zero integers with one of three
  deterministic expression families: 32-bit affine multiplication with an odd
  modular inverse, rotated affine decoding, or masked bit splitting. Operations
  are performed in `unsigned long long` with explicit 32-bit masks, and the
  result is cast back to the literal's exact built-in type.
- Integer zero remains a literal because C++ gives zero-valued integer literals
  null-pointer semantics that equivalent constant expressions do not retain.
  C++23 size suffixes are also excluded from arithmetic encoding.
- Direct printable ASCII string content is encoded with fixed-width octal
  escapes. Existing escapes, raw strings, non-ASCII text, language-linkage
  strings, `static_assert` messages, attribute text, and GNU asm strings remain
  intact.
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
literals may be emitted. Maximum arithmetic encoding and flow shards can
substantially expand the source file. The shards create local closure
expressions but no heap allocation or persistent runtime state. A compiler or
dedicated constant folder can still reduce generated arithmetic expressions.

## Macro boundaries

- Included-header definitions are not visible and headers are never expanded.
- Conditional, redefined, undefined, variadic, include-sensitive, and
  preprocessing-directive-dependent macros remain untouched.
- Surviving function-like statement macros beginning with `for`, `if`, `while`,
  or `switch` retain the existing Tree-sitter missing-semicolon allowance.
- Used `##`/`%:%:` token-pasting and `#`/`%:` stringifying macros are rejected.
- Backslash continuations and all physical line counts remain stable.

Header-provided macros, full conditional preprocessing, compiler extensions,
and full ADL/type lookup remain outside the model. Maximum cannot inspect
function-identity or stack-frame effects hidden inside a header macro.

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

## End-to-end example

The repository includes an original macro-heavy contest source and the
deterministic maximum-profile result:

- [`examples/T708557.cpp`](examples/T708557.cpp)
- [`examples/T708557.obfuscated.cpp`](examples/T708557.obfuscated.cpp)

Regenerate the artifact directly from the original source:

```bash
cargo run --release -- \
  --profile maximum \
  --seed 0x708557 \
  -i examples/T708557.cpp \
  -o examples/T708557.obfuscated.cpp
```

No compiler preprocessing step or manually prepared intermediate source is
required. The input and output retain the same physical line count. A portable
header variant of this case compiled as C++14 with GCC and Clang and produced
byte-identical output for 401 deterministic random trees.

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

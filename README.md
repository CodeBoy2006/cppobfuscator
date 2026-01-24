# cppobfuscator

English (default) | [Chinese (中文)](README.zh.md)

## Overview

`cppobfuscator` is a single-file C++ obfuscator written in Rust. It can:

- Rename identifiers (optional simple 1-2 character mode).
- Minify whitespace.
- Inline simple functions.
- Obfuscate integer literals (constlift).
- Expand macros (internal expander; can be disabled).
- Strip unused macros, functions, and globals.
- Provide a wizard mode for interactive input.

## Usage

```bash
cppobfuscator [options]
```

If you are running from source:

```bash
cargo run -- [options]
```

### Default wizard behavior

If no arguments are provided, the tool enters wizard mode automatically.
It will prompt:

```
Enable simple-names? [y/N]:
```

Then paste your C++ code and end with the marker line (default: `END`).

## Options

- `-i, --input <path>`: Input C++ file (defaults to stdin).
- `-o, --output <path>`: Output file (defaults to stdout).
- `--seed <u64>`: Seed for deterministic renaming (default: `0xC0FFEE`).
- `--no-rename`: Disable identifier renaming.
- `--simple-names`: Rename identifiers to 1-2 character names (disables constlift).
- `--no-minify`: Preserve original whitespace/newlines.
- `--no-inline`: Disable inline substitution for simple functions.
- `--no-constlift`: Disable integer literal obfuscation.
- `--keep-comments`: Preserve comments (default strips).
- `--no-strip-unused-macros`: Keep unused `#define` macros (default strips).
- `--no-strip-unused-functions`: Keep unused function definitions (default strips).
- `--no-strip-unused-globals`: Keep unused global variables/objects (default strips).
- `--no-expand-macros`: Skip macro expansion before obfuscation.
- `--preserve <name>`: Preserve an identifier (repeatable).
- `--wizard`: Interactive mode (cannot be used with `--input`).
- `--wizard-end <marker>`: Marker line to finish wizard input (default: `END`).
- `-h, --help`: Show help.

Notes:
- `--simple-names` cannot be used with `--no-rename`.
- `--wizard-end` implies wizard mode.

## Examples

Obfuscate a file:

```bash
cppobfuscator -i test.cpp -o test_obf.cpp
```

Enable simple names:

```bash
cppobfuscator -i test.cpp -o test_obf.cpp --simple-names
```

Keep comments and formatting:

```bash
cppobfuscator -i test.cpp -o test_obf.cpp --keep-comments --no-minify
```

Use wizard mode:

```bash
cppobfuscator
```

Then follow the prompt and paste code, ending with `END`.

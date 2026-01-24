## [2026-01-23 23:15] C++ Obfuscator CLI
- **Changes:** Added Rust CLI, C++ tokenizer, obfuscation passes (rename, minify, inline, constlift), and updated plan file.
- **Status:** Completed
- **Next Steps:** Consider adding README usage examples and fixture tests if needed.
- **Context:** Obfuscation is heuristic for contest-style single-file C++; inline pass only handles simple return expressions.
## [2026-01-23 23:25] Wizard Mode
- **Changes:** Added wizard flags for interactive input and updated CLI to read pasted code until a marker.
- **Status:** Completed
- **Next Steps:** Consider README usage example for wizard flow.
- **Context:** Wizard prompts are printed to stderr to keep stdout clean for copy/paste.
## [2026-01-24 12:01] Tree-sitter Semantic Renaming
- **Changes:** Added tree-sitter C++ dependency, new `src/semantics.rs` declaration collector, and filtered renaming to declared identifiers to avoid touching standard library symbols.
- **Status:** Completed
- **Next Steps:** Expand declaration patterns if any user-defined identifiers are no longer being renamed.
- **Context:** Renaming now depends on parser-detected declarations; missing nodes will be left unchanged for safety.
## [2026-01-24 12:11] Strip Unused Macros
- **Changes:** Added unused-macro stripping pass for #define lines, plus CLI flag to disable it; wired into obfuscation flow.
- **Status:** Completed
- **Next Steps:** Expand preprocessor parsing if macro usage is missed in edge cases.
- **Context:** Macro removal is conservative and only drops definitions with no detected identifier uses.

## [2026-01-24 12:34] Provide g++ compile command
- **Changes:** None; provided compile command guidance.
- **Status:** Completed
- **Next Steps:** Run `g++ -std=c++17 test.cpp -o test` to verify locally.
- **Context:** No code changes made; local compile failed due to missing `bits/stdc++.h` with clang-based g++.

## [2026-01-24 12:38] Compile with g++-15
- **Changes:** None; compiled `test.cpp` with g++-15.
- **Status:** Completed
- **Next Steps:** Run `./test` if you want to execute the binary.
- **Context:** Compile used `g++-15 -std=c++17 test.cpp -o test` and succeeded.

## [2026-01-24 12:41] Test obfuscator with test.cpp
- **Changes:** None; generated obfuscated output at /tmp/test_obf.cpp.
- **Status:** Completed
- **Next Steps:** Fix minifier to preserve newline before preprocessor directives, then re-test.
- **Context:** `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` succeeded, but `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf` failed (stray `#` from `;#define` on line 3).

## [2026-01-24 12:46] Fix obfuscation compile regression
- **Changes:** Ensured minifier inserts a newline before preprocessor directives and limited declarator harvesting to avoid renaming initializer identifiers.
- **Status:** Completed
- **Next Steps:** Consider adding a regression test for preprocessor/minifier output.
- **Context:** `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf` now succeed.

## [2026-01-24 12:57] Strip unused functions
- **Changes:** Added unused-function stripping with a new config flag and identifier scanning for usage detection.
- **Status:** Completed
- **Next Steps:** Consider adding regression tests for function stripping edge cases.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

## [2026-01-24 13:05] Expand macros before obfuscation
- **Changes:** Added macro expansion using g++-15 with include markers and new `--no-expand-macros` flag.
- **Status:** Completed
- **Next Steps:** Consider documenting macro expansion behavior for header-defined macros.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

## [2026-01-24 13:16] Embed macro expansion
- **Changes:** Replaced g++-based macro expansion with an internal token-based expander and kept includes intact.
- **Status:** Completed
- **Next Steps:** Consider adding tests for variadic or token-paste macros that are intentionally skipped.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

## [2026-01-24 13:27] Strip unused globals
- **Changes:** Added unused global variable/object stripping with a new CLI flag and tree-sitter pass.
- **Status:** Completed
- **Next Steps:** Consider extending removal to handle multi-declarator statements if needed.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

## [2026-01-24 13:35] Remove unused macros with dependency scan
- **Changes:** Updated macro pruning to remove macros only referenced by other unused macros.
- **Status:** Completed
- **Next Steps:** Consider expanding the dependency scan to handle token-paste/stringify if needed.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`; `#define endl` no longer appears.

## [2026-01-24 13:41] Simple renaming mode
- **Changes:** Added `--simple-names` to rename identifiers to 1-2 character names with keyword avoidance.
- **Status:** Completed
- **Next Steps:** Consider extending the short-name generator if more than 2,756 identifiers are expected.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp --simple-names` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

## [2026-01-24 13:49] Improve constant obfuscation
- **Changes:** Replaced trivial XOR-cancel constant lifting with a bitwise NOT/XOR identity using two distinct keys.
- **Status:** Completed
- **Next Steps:** Consider adding unit tests for constlift patterns.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

## [2026-01-24 13:54] Disable constlift with simple-names
- **Changes:** Made `--simple-names` disable constlift by default and updated usage text.
- **Status:** Completed
- **Next Steps:** None.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp --simple-names`; constlift pattern not present.

## [2026-01-24 14:01] Default wizard mode with simple-names prompt
- **Changes:** Defaulted to wizard mode with no args and added an interactive simple-names prompt in wizard flow.
- **Status:** Completed
- **Next Steps:** None.
- **Context:** Verified wizard flow via stdin pipe and compiled output.

## [2026-01-24 14:06] Add bilingual README
- **Changes:** Added English README with link to Chinese README and documented options/usage.
- **Status:** Completed
- **Next Steps:** None.
- **Context:** New README.md and README.zh.md.

## [2026-01-24 14:21] Recursive constant obfuscation
- **Changes:** Replaced constlift with recursive arithmetic decomposition (add/sub/xor/shift + leaf variations).
- **Status:** Completed
- **Next Steps:** Consider adding tests for suffix parsing and large literals.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

## [2026-01-24 14:28] Increase const obfuscation variety
- **Changes:** Added per-occurrence seeding, more leaf/zero/one forms, and noisy wrappers in constlift; casted sizeof uses to avoid warnings.
- **Status:** Completed
- **Next Steps:** None.
- **Context:** Verified with `cargo run -- -i test.cpp -o /tmp/test_obf.cpp` and `g++-15 -std=c++17 /tmp/test_obf.cpp -o /tmp/test_obf`.

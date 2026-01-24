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

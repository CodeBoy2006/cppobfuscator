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

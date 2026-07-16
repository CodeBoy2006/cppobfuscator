## [2026-07-16 19:21] Tree-sitter AST Core Refactor
- **Changes:** Replaced the legacy lexer/text transformation pipeline with Tree-sitter C++ scope analysis, byte-range AST rewrites, structural reparse validation, conservative macro handling, a reduced CLI, public Rust API, integration tests, CI, and updated bilingual documentation. Removed macro expansion, function inlining, constant lifting, dead-code stripping, and other unsafe legacy passes.
- **Status:** Completed
- **Next Steps:** Review and tag the breaking `0.2.0` release when ready.
- **Context:** The tool targets one UTF-8 contest source file. Members, types, qualified or unresolved names, external linkage, friend-declared functions, and macro dependencies are preserved. Identifier token-pasting via `##` or `%:%:` is intentionally unsupported.

---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Use a Rust crate parser to collect declared identifiers and avoid renaming standard library symbols.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T11:56:25+0800
---
# Plan: Tree-sitter Semantic Renaming

🎯 Task Overview
The current token-based renamer replaces identifiers regardless of where they originate, which can mangle standard library calls. Introduce a C++ parser crate to collect declared identifiers and only rename those, keeping external/standard symbols intact.

📋 Execution Plan
1. Add tree-sitter C++ dependencies and a new semantics module to parse the input and extract declared identifiers.
2. Wire the declaration set into the renamer so only declared identifiers are renamed, preserving existing explicit preserve rules.
3. Run cargo checks and update status/commit workflow.

⚠️ Risks & Considerations
- Tree-sitter parses syntax, not full semantics; some declarations may be missed, reducing rename coverage.
- C++ grammar changes could require updates to declaration extraction rules.

📎 References
- `src/obfuscate.rs:1`
- `src/lexer.rs:1`

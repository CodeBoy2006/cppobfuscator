---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Add removal of unused free functions during obfuscation.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T04:54:59Z
---
# Plan: Strip Unused Functions

🎯 Task Overview
Add a conservative pass to remove unused free-function definitions from input before token-based obfuscation. Ensure the resulting obfuscated output still compiles with g++-15 using test.cpp.

📋 Execution Plan
1. Add a new config flag for stripping unused functions and wire it through CLI parsing/usage text.
2. Implement a tree-sitter based pass to find candidate free-function definitions and remove those not referenced outside their declarator.
3. Integrate the pass into the obfuscation pipeline before tokenization and renaming.
4. Verify by running obfuscation on test.cpp and compiling the output with g++-15.

⚠️ Risks & Considerations
- Heuristic may miss macro-only references; keep removal conservative to avoid false deletions.
- Ensure function name extraction skips parameters and class methods.

📎 References
- `src/obfuscate.rs:1`
- `src/semantics.rs:1`
- `src/config.rs:1`

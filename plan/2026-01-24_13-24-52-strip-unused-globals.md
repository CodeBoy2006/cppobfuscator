---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Strip unused global variables/objects.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T05:24:52Z
---
# Plan: Strip Unused Globals

🎯 Task Overview
Add a conservative pass to remove unused global variable/object definitions before obfuscation. Ensure the obfuscated output for test.cpp still compiles with g++-15 after the removal.

📋 Execution Plan
1. Add a config flag to toggle unused global stripping and wire it through CLI parsing and usage text.
2. Implement a tree-sitter pass to find top-level single-declarator globals and remove those not referenced elsewhere.
3. Integrate the pass into the obfuscation pipeline after macro expansion and unused-function stripping.
4. Verify by running obfuscation on test.cpp and compiling the output with g++-15.

⚠️ Risks & Considerations
- Declarations with multiple declarators are skipped to avoid partial rewrites.
- Side-effectful initializers will be removed if unused; this matches the request but may change behavior.

📎 References
- `src/obfuscate.rs:1`
- `src/semantics.rs:1`
- `src/config.rs:1`

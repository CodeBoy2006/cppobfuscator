---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Build a Rust CLI that obfuscates single-file contest-style C++ code with safe, deterministic transformations.
complexity: medium
planning_method: builtin
created_at: 2026-01-23T17:11:04+0800
---
# Plan: C++ Obfuscator CLI

🎯 Task Overview
Create a Rust command-line tool that reads a single-file C++ solution and applies safe obfuscation transformations (identifier renaming, whitespace minimization, simple inlining, and deterministic constant expressions) without changing behavior or degrading performance. The output should remain compilable and efficient, with configurable options.

📋 Execution Plan
1. Scaffold a Rust CLI with argument parsing, input/output handling, and a modular pipeline.
2. Implement a lightweight tokenizer for C++ (identifiers, keywords, literals, comments, preprocessor lines) to enable safe transformations.
3. Add transformation passes: identifier renaming with symbol table, whitespace/comment cleanup, and optional inline-eligible functions.
4. Add constant lifting to deterministic compile-time expressions and optional extra obfuscation passes.
5. Wire configuration flags, defaults, and documentation; add basic tests or fixtures.

⚠️ Risks & Considerations
- C++ parsing is complex; keep scope to contest-style single file and avoid macros or complex templates when transforming.
- Preserve semantics by skipping transformations in strings/comments/preprocessor blocks.

📎 References
- `plan/2026-01-23_17-10-50-cpp-obfuscator-cli.md`

✅ Progress
- Implemented CLI, tokenizer, and obfuscation passes with configurable flags.

---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Embed macro expansion logic in the obfuscator to avoid calling g++.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T05:13:16Z
---
# Plan: Embed Macro Expansion

🎯 Task Overview
Replace the external g++ preprocessor call with an internal macro-expansion step, so macros are expanded before obfuscation without spawning g++. Keep include directives intact and stay conservative to avoid incorrect expansions.

📋 Execution Plan
1. Add a preprocessing module that builds a macro table from #define lines and expands object-like/function-like macros in token streams.
2. Wire the internal expansion into the input flow and remove the g++ invocation.
3. Keep expansion conservative by skipping unsupported macros (stringification, token-paste, variadics).
4. Verify on test.cpp and compile the obfuscated output with g++-15.

⚠️ Risks & Considerations
- Macro stringification/paste and variadic macros are not expanded; leave those intact to avoid breaking code.
- Preprocessor conditionals are preserved, not evaluated.

📎 References
- `src/main.rs:1`
- `src/lexer.rs:1`
- `src/config.rs:1`

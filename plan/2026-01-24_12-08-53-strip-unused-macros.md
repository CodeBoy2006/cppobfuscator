---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Remove unused C++ macro definitions during obfuscation.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T12:09:01+0800
---
# Plan: Strip Unused Macros

🎯 Task Overview
The obfuscator currently preserves all macro definitions, even if they are never used. Add a preprocessor-aware pass that detects defined macros and removes those not referenced elsewhere, while remaining conservative about usage detection.

📋 Execution Plan
1. Add a macro analysis pass that collects `#define` names and scans for identifier usage in code and preprocessor lines.
2. Remove unused macro-definition preprocessor tokens and wire the new pass into the obfuscation pipeline with a CLI toggle.
3. Verify with cargo tests and update status/logging and git workflow.

⚠️ Risks & Considerations
- Preprocessor parsing is heuristic; avoid false removals by treating ambiguous identifiers as used.
- Function-like macro parameter parsing should not mark parameters as macro uses.

📎 References
- `src/obfuscate.rs:1`
- `src/config.rs:1`

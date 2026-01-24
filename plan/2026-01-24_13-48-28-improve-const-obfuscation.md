---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Improve numeric constant obfuscation to avoid trivial cancellation.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T05:48:28Z
---
# Plan: Improve Constant Obfuscation

🎯 Task Overview
Replace the constant lifting logic with recursive arithmetic decomposition to produce more varied expressions. Keep numeric literal values unchanged while reducing obvious repetition across literals.

📋 Execution Plan
1. Implement a recursive decomposition builder (add/sub/xor/shift) plus noisy wrappers to avoid trivial cancellations.
2. Add richer leaf generators for 0/1/short values and vary per occurrence using a deterministic counter.
3. Verify on test.cpp and compile the obfuscated output with g++-15.

⚠️ Risks & Considerations
- Keep expressions in safe ranges and avoid signed/unsigned promotion warnings.
- Ensure the new expressions preserve integer literal semantics with suffix parsing.

📎 References
- `src/obfuscate.rs:900`

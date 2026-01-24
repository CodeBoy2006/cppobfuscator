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
Replace the current XOR-cancel pattern for integer literals with a more complex but safe expression. Keep numeric literal values unchanged while making the replacement less obvious.

📋 Execution Plan
1. Update constlift logic to generate non-trivial expressions using bitwise NOT/XOR with multiple keys.
2. Ensure generated constants are non-zero and distinct for better obfuscation.
3. Verify on test.cpp and compile the obfuscated output with g++-15.

⚠️ Risks & Considerations
- Avoid arithmetic overflow by using bitwise-only identities.
- Ensure the new expression preserves integer literal semantics.

📎 References
- `src/obfuscate.rs:900`

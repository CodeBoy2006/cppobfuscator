---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Add simple renaming mode with single/double character identifiers.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T05:39:32Z
---
# Plan: Simple Renaming Mode

🎯 Task Overview
Add a simple naming mode that replaces identifiers with one-character names, falling back to two-character names when needed. Keep numeric constants untouched and preserve reserved keywords.

📋 Execution Plan
1. Add a CLI flag to enable simple renaming and wire it through config and obfuscation settings.
2. Implement a short-name generator that avoids reserved keywords and existing identifiers.
3. Route rename logic through the short-name generator when the flag is enabled.
4. Verify with test.cpp and compile the obfuscated output using g++-15.

⚠️ Risks & Considerations
- Must avoid generating keywords or preserved identifiers.
- Very large identifier sets may exceed two-character names; add a safe fallback.

📎 References
- `src/obfuscate.rs:1`
- `src/config.rs:1`
- `src/main.rs:1`

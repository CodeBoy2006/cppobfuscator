---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Expand macros before obfuscation without expanding system headers.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T05:04:57Z
---
# Plan: Expand Macros Before Obfuscation

🎯 Task Overview
Add a preprocessing step that expands macros before the obfuscation pipeline runs. Keep `#include` lines intact to avoid pulling in full system headers while still expanding file-local macros.

📋 Execution Plan
1. Add a config flag to toggle macro expansion and document it in CLI usage.
2. Implement a preprocessor call that strips `#include` lines into markers, expands macros, then restores the includes.
3. Wire the preprocessing step into the main input flow before tokenization.
4. Verify on test.cpp and compile the obfuscated output with g++-15.

⚠️ Risks & Considerations
- Macro expansion won’t include macros from headers since includes are preserved rather than expanded.
- Preprocessor output must retain markers; keep comments via `-CC` and use unique markers.

📎 References
- `src/main.rs:1`
- `src/config.rs:1`

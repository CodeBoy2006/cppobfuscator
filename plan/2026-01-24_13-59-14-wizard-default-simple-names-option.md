---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Default to wizard mode with a simple-names prompt.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T05:59:14Z
---
# Plan: Wizard Default + Simple Names Prompt

🎯 Task Overview
Make the tool enter wizard mode when no CLI args are provided, and add an interactive prompt in wizard mode to enable simple-names. Ensure simple-names disables constlift as before.

📋 Execution Plan
1. Update config parsing to set wizard when no args are supplied.
2. Refactor wizard input handling to share stdin and prompt for simple-names.
3. Wire the prompt result into the obfuscation config and keep constlift disabled when simple-names is enabled.
4. Verify with test.cpp using wizard mode.

⚠️ Risks & Considerations
- Prompt consumes one line from stdin; ensure it doesn't interfere with pasted code input.
- Keep behavior consistent when --simple-names is already passed.

📎 References
- `src/config.rs:1`
- `src/main.rs:1`

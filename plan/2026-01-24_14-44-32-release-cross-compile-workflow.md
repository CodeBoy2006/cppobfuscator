---
mode: plan
cwd: /Users/codeboy/cppobfuscator
task: Add GitHub Actions release workflow for cross-compiling targets.
complexity: medium
planning_method: builtin
created_at: 2026-01-24T06:44:32Z
---
# Plan: Release Cross-Compile Workflow

🎯 Task Overview
Add a GitHub Actions workflow to build and upload release binaries for Windows x86, macOS arm, and Linux x86_64. Trigger the workflow on release publication and attach compiled artifacts to the GitHub release.

📋 Execution Plan
1. Create a GitHub Actions workflow with a matrix for the three targets.
2. Build release binaries per target and package them with clear filenames.
3. Upload artifacts to the release using a release upload action.
4. Verify the workflow file and log the change.

⚠️ Risks & Considerations
- Windows 32-bit target may require extra tooling; use the MSVC target.
- macOS arm build requires the macOS 14 runner.

📎 References
- `.github/workflows/release.yml`

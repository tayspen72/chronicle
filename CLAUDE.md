# CLAUDE.md

## Build & Verify
- Build: `cargo build`
- Typecheck: `cargo check`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`

Run all four before opening a PR. Never suppress warnings to make a build pass.

## Workflow
- Start non-trivial tasks in Plan mode (Shift+Tab twice). Refine the plan before executing.
- Use `code-architect` before implementing anything that touches 3+ files.
- Use `code-simplifier` after every implementation.
- Use `verify-app` before every PR.

## Rules
<!-- This section grows over time. When Claude makes a mistake, add a rule here. -->
- [Add rules here as you encounter them]

## Off-Limits Without Explicit Instruction
- [Add protected paths/files specific to your project here]

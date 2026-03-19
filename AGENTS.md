# AGENTS.md

This file is loaded automatically by all agents. Keep it short and accurate.
Update it whenever an agent makes a mistake — commit the change so the whole team benefits.

---

## Project

- **Domain**: CLI tool / TUI application (Markdown-native planner and journal)
- **Languages**: Rust (2024 edition)
- **Build system**: Cargo
- **Test framework**: Built-in Rust testing (cargo test with tempfile for fixtures)

---

## Commands

Every agent must know these before touching anything.

```
build:     cargo build --release
typecheck: cargo check
test:      cargo test
lint:      cargo clippy
format:    cargo fmt
```

To run a **single test**:
```
cargo test <test_name>
```

Run them in that order. Never suppress a warning to make a step pass.

---

## Layout

```
/src        — production code
  /commands — CLI commands (init, new_task, jot, extract, task_template)
  /model    — domain models (Program, Project, Milestone, Task)
  /storage  — file I/O and persistence
  /tui      — terminal UI components
/tests      — integration tests (if any)
/docs       — documentation
/templates  — markdown templates for elements
```

---

## Code Style

### Imports
- Use absolute imports within the crate (`crate::module::Item`)
- Group std, external crates, and crate imports with blank lines between
- Order: std → external → crate

### Formatting
- Run `cargo fmt` before committing
- Use 4 spaces for indentation
- Keep lines under 100 characters when practical
- Use blank lines between function definitions

### Types and Derives
- Most structs use: `#[derive(Debug, Clone, Serialize, Deserialize, Default)]`
- Error types use: `#[derive(Error, Debug)]` with `thiserror`
- Add `#[must_use]` to functions that return important values

### Naming Conventions
- **Types**: PascalCase (`struct App`, `enum Mode`)
- **Functions/variables**: snake_case (`let config = ...`, `fn load_config()`)
- **Constants**: SCREAMING_SNAKE_CASE
- **Modules**: snake_case (`mod storage`, `mod tui`)
- **Files**: snake_case (`error.rs`, `mod.rs`)

### Error Handling
- Library code uses `crate::Result<T>` (wrapped with `thiserror`)
- Binary entry point uses `anyhow::Result`
- Convert errors at boundaries with `.map_err(|e| anyhow::anyhow!("{e}"))?`
- Use descriptive error messages: `#[error("Configuration error: {0}")]`

### Documentation
- Module-level: `//! Description` at top of file
- Public APIs: `/// Description` above items
- Keep docs concise; focus on "what" and "why", not "how"

### Testing
- Use `tempfile` crate for tests needing temp directories
- Integration tests go in `/tests` or can be in-module with `#[cfg(test)]`

---

## Conventions

- **Serialization**: Use `serde` with YAML for configs, TOML for user settings
- **Logging**: Use `tracing` with `tracing-subscriber` and `tracing-appender`
- **TUI**: Uses `ratatui` + `crossterm` for terminal UI
- **Date handling**: Use `chrono` with `DateTime<Utc>` for storage, `Local` for display
- **IDs**: Use `uuid` v4 for unique identifiers

---

## Off-limits

Do not touch these without an explicit instruction in the current session:

- Release build optimizations in Cargo.toml (`lto = true`, `opt-level = "z"`)
- Session files (`session-*.md`)

---

## Git workflow

The main agent handles all git operations. Branch names are auto-generated from the task/goal description.

### Branch Naming

```
<type>/<slug>
```

| Type | When |
|------|------|
| `feature/` | New functionality |
| `fix/` | Bug fixes |
| `refactor/` | Code restructuring without behavior change |
| `docs/` | Documentation only |

Examples:
- "implement planning wizard" → `feature/planning-wizard`
- "fix navigation scope bug" → `fix/navigation-scope`

### Workflow Steps

1. **Start**: Create branch from `develop` with generated name
2. **Work**: Implement changes (may call subagents)
3. **Simplify**: Run `@code-simplifier`
4. **Commit**: Commit with auto-generated conventional commit message
5. **Verify**: Run `@verify-app`
6. **Merge**: Merge branch to `develop`

### Rules

- **Never merge to master** — only `develop`. User handles develop → master merges manually.
- **Stop on merge conflicts** — ask user to resolve manually.
- **Conventional commits** — format: `<type>(<scope>): <description>`

---

## Agent workflow

These five subagents are available. Use them in this order:

| When | Call |
|---|---|
| Before implementing anything touching 3+ files | `@code-architect` |
| After implementation is complete | `@code-simplifier` |
| Before opening any PR | `@verify-app` |
| Before any deployment or release build | `@build-validator` |
| Something is broken and you don't know why | `@oncall-guide` |

`@code-architect` is read-only — it produces a written plan, it does not write code.
`@verify-app` and `@build-validator` run commands only — they do not edit files.
`@code-simplifier` edits existing files only — it does not create new ones.
`@oncall-guide` will ask before making any changes to production code.

---

## PR checklist

Before any PR is opened:
1. `@verify-app` reports all green
2. Commit message follows conventional commits
3. No build artifacts, `.env` files, or lockfile changes unless intentional
4. Docs updated if behavior changed

---

## Learned rules

_Append a rule here whenever an agent makes a mistake. One line, specific, actionable._

<!-- example: Never use enums — prefer string literal unions -->
<!-- example: Always run `cargo check` before `cargo test` — type errors mask test failures -->
- In normal mode, the main window is a preview/report/viewer of the current selection in the navigator. When the selected entry in the navigator changes, the main window must also update — it is simply a viewport into whatever is selected.

- When the user reports a bug, verify understanding of the expected vs actual behavior, then pass the issue to `@oncall-guide` for root-cause analysis and fix. Do not implement the fix directly unless the bug is trivial (one-liner) or the user explicitly asks for a direct fix.
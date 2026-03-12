# AGENTS.md

This file is loaded automatically by all agents. Keep it short and accurate.
Update it whenever an agent makes a mistake — commit the change so the whole team benefits.

---

## Project

- **Domain**: [your domain — e.g. embedded firmware, web app, CLI tool]
- **Languages**: [e.g. C/C++, TypeScript, Python]
- **Build system**: [e.g. CMake, bun, cargo]
- **Test framework**: [e.g. Unity/CMock, vitest, pytest]

---

## Commands

Every agent must know these before touching anything.

```
build:     <your build command>
typecheck: <your typecheck command>
test:      <your test command>
lint:      <your lint command>
format:    <your format command>
```

Run them in that order. Never suppress a warning to make a step pass.

---

## Layout

```
/src        — production code
/include    — public headers (if applicable)
/test       — tests
/docs       — documentation
/scripts    — helper scripts
```

---

## Conventions


---

## Off-limits

Do not touch these without an explicit instruction in the current session:


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
2. Commit message follows [your convention, e.g. conventional commits]
3. No build artifacts, `.env` files, or lockfile changes unless intentional
4. Docs updated if behavior changed

---

## Learned rules

_Append a rule here whenever an agent makes a mistake. One line, specific, actionable._

<!-- example: Never use enums — prefer string literal unions -->
<!-- example: Always run `bun run typecheck` before `bun run test` — type errors mask test failures -->

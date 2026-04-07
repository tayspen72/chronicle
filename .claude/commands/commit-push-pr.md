---
description: Commit staged changes, push, and open a PR
---

Current branch: `$(git branch --show-current)`

Changes to commit:
```
$(git diff --cached --stat)
$(git status --short)
```

Recent commits for context:
```
$(git log --oneline -5)
```

1. Write a commit message following conventional commits format (`type(scope): description`). Keep subject under 72 chars.
2. Stage all relevant changes. Do not stage build artifacts, .env files, or lockfile changes unless they are intentional.
3. Commit and push to origin.
4. Open a PR. Title should match the commit subject. Body should include:
   - What changed and why (not how — the diff shows how)
   - Checklist: build ✅, typecheck ✅, tests ✅, lint ✅
   - Any deployment notes or migration steps required

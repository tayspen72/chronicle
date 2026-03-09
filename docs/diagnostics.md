# Diagnostics

Chronicle uses `tracing` with optional file output.

## Default Log Path

- `~/.config/chronicle/logs/diagnostics.log`

## Enable Diagnostics

Config-based:

```toml
[diagnostics]
enabled = true
level = "debug"
```

Environment-based (override):

```bash
CHRONICLE_DIAGNOSTICS=1 CHRONICLE_DIAGNOSTICS_LEVEL=debug cargo run
```

Optional custom log destination:

```bash
CHRONICLE_DIAGNOSTICS_LOG=/tmp/chronicle.log cargo run
```

## Logged Signals

- diagnostics initialization path
- workspace startup snapshot
- tree navigation transitions (`Right`/`Left`)
- node open decisions and child discovery counts
- template parse and create-from-template events
- warnings/errors emitted via tracing

## Recommended Bug Report Data

For navigation bugs, collect:
1. workspace tree snippet (`find programs -maxdepth ... | sort`)
2. log tail after repro (`tail -n 120 ~/.config/chronicle/logs/diagnostics.log`)

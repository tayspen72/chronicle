# Configuration

Config path:
- `~/.config/chronicle/config.toml`

## Required Fields

- `workspace` (path): workspace root directory
- `editor` (string): external editor command
- `owner` (string): owner identity for created/assigned task flows

Chronicle validates these as non-empty when loading config.

## Optional Fields

- `workflow` (array of strings)
  - Default: `["New", "Active", "Blocked", "Testing", "Completed", "Cancelled"]`
- `importance` (array of strings)
  - Default: `["low", "medium", "high"]`
- `planning_duration` (string)
  - Allowed values: `weekly`, `biweekly`, `6weekly`
  - Default: `weekly`
- `theme` (string)
  - Default: `default_dark`
- `[notes].categories` (array of strings)
  - Default: `["Projects", "Areas", "Resources", "Archive"]`

## Diagnostics

Under `[diagnostics]`:
- `enabled` (bool, default `false`)
- `level` (`trace|debug|info|warn|error`, default `debug`)

## Removed/Unused Fields

- `navigator_width` and `[navigation_keys]` are no longer used.

## Minimal Example

```toml
workspace = "/home/user/chronicle/workspace"
editor = "hx"
owner = "Tay"
```

## Full Example

```toml
workspace = "/home/user/chronicle/workspace"
editor = "hx"
owner = "Tay"
workflow = ["New", "Active", "Blocked", "Testing", "Completed", "Cancelled"]
importance = ["low", "medium", "high"]
planning_duration = "weekly"
theme = "default_dark"

[notes]
categories = ["Projects", "Areas", "Resources", "Archive"]

[diagnostics]
enabled = true
level = "debug"
```

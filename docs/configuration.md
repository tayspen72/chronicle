# Configuration

Config path:
- `~/.config/chronicle/config.toml`

## Fields

- `workspace` (path): workspace root
- `editor` (string): external editor command
- `owner` (string): default owner value
- `workflow` (array): status workflow list (first value is default status)
- `navigator_width` (u16): target navigator width
- `planning_duration` (string): planning cadence label

## Navigation Keys

Under `[navigation_keys]`:
- `left`
- `right`
- `up`
- `down`

Defaults are `h/l/k/j` values, while arrow keys are also handled by the TUI input path.

## Diagnostics

Under `[diagnostics]`:
- `enabled` (bool, default `false`)
- `level` (`trace|debug|info|warn|error`, default `debug`)

Example:

```toml
workspace = "/home/user/chronicle/workspace"
editor = "hx"
owner = "Tay"
workflow = ["New", "Active", "Blocked", "Testing", "Completed", "Cancelled"]

[navigation_keys]
left = "h"
right = "l"
up = "k"
down = "j"

[diagnostics]
enabled = true
level = "debug"
```

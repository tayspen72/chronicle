# chronicle.rs

**chronicle.rs** is a Markdown‑native planner and journal with a terminal UI.  
Organize **programs → projects → milestones → tasks → subtasks**, capture daily **journals**, and **extract actionable items** (e.g., `/todo`) straight from your notes.

---

## ✨ Goals

- **Terminal UI first:** Navigate and manage from a clean TUI interface.
- **Plain‑text storage:** Tasks and journals are stored as `.md` files you can edit in any editor.
- **Predictable structure:** Hierarchy mapped to folders: `programs/`, `projects/`, `milestones/`, `tasks/`, `journal/`, `backlog/`.
- **Portable & scriptable:** Easy to parse for reports (Gantt, workload, velocity).

---

## 🎨 TUI Interface

### Layout

```
┌─────────────────────────────────────────────────────────────┐
│  Commands: /  (press '/' for command palette)                │
├────────────────┬────────────────────────────────────────────┤
│                │                                            │
│  PROGRAMS      │         Main Content Area                  │
│  Projects      │                                            │
│  Milestones    │   (Shows content based on selection)       │
│  Tasks         │                                            │
│  Journal       │                                            │
│  Backlog       │                                            │
│                │                                            │
├────────────────┴────────────────────────────────────────────┤
│  Status: Ready | Data: ~/chronicle                          │
└────────────────────────────────────────────────────────────┘
```

### Command Palette

Press `/` to open the command palette. Type to filter:

```
/p
 > Go to Programs
 > Go to Projects
 > Go to Milestones
```

### Navigation

| Key | Action |
|-----|--------|
| `/` | Open command palette |
| `Esc` | Close command palette |
| `↑/↓` | Navigate list |
| `Enter` | Select |

---

## 📁 Data Layout

```
~/chronicle/
├── programs/
├── projects/
├── milestones/
├── tasks/
├── journal/
│   └── YYYY-MM-DD.md
├── backlog/
└── templates/
    ├── task.md
    └── task_min.md
```

You can freely move/edit Markdown files; as long as task keywords remain (e.g., `#title`, `#status`, `#assigned-to`), chronicle can parse them.

---

## 🧩 Markdown Conventions

Chronicle looks for **hashtag‑style keys** at the start of lines:

```
#title: Implement journal extractor
#assignee: Taylor
#assigned-to: taylor
#status: todo|doing|done|blocked
#priority: low|med|high|urgent
#due: 2026-03-01
#tags: #task #backlog #program:platform #project:fw
```

### Journaling
- Use `/todo`, `/note`, `/learned` within daily entries.
- Extract pulls `/todo …` lines into `backlog/` as seed tasks.

---

## ⚙️ Configuration

On first run, chronicle creates a config file at:

```
~/.config/chronicle.rs/config.toml
```

Default config:

```toml
version = "1.0"
data_path = "~/chronicle"
```

---

## 🛠️ Build & Run

```bash
# Build
cargo build

# Run (starts TUI)
cargo run

# Install locally
cargo install --path .
```

---

## 🗺️ Roadmap

### MVP
- [x] Basic TUI layout (sidebar + content)
- [x] Command palette with filtering
- [ ] Navigation (up/down, enter to select)
- [ ] Load and display journal entries
- [ ] Load and display tasks
- [ ] Create new journal entry
- [ ] Create new task

### Future Features
- [ ] Theming system (stored in `~/.config/chronicle.rs/`)
  - Light/dark theme support
  - Terminal-aware colors (automatic detection)
- [ ] Search functionality
- [ ] Task creation/editing via TUI
- [ ] Parse hashtag metadata from `.md` (front‑matter or inline)
- [ ] Task indexer (scan `data/` → JSON index)
- [ ] Reports:
  - [ ] Gantt generation (from `#due` + inferred durations)
  - [ ] Workload per assignee, per week
- [ ] Configurable extractors (`/todo`, `/decision`, `/blocker`)
- [ ] Biweekly planning helper (select backlog → schedule)
- [ ] Optional YAML front‑matter mode (for stricter parsing)
- [ ] Watch mode (auto‑update index on file changes)

---

## 🧪 Testing

```bash
cargo test
```

---

## 📜 License

MIT

---

## 🙌 Why Markdown?

Markdown keeps your planning **portable, durable, and editor‑agnostic**.  
It's easy to version, diff, and script into visuals like **Gantt charts** or **workload dashboards**.

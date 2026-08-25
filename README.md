# Q-CTRL

A native Kanban board for [Scaffold](https://github.com/ksetlak/scaffold)-based projects.

Q-CTRL reads your backlog directly from Markdown files on disk, lets you drag cards between status columns and reorder them within a column, and writes changes back immediately — no server, no database, no sync.

![Q-CTRL screenshot placeholder](docs/screenshot.png)

---

## What it does

- Reads all tickets from `backlog/items/*.md` at launch
- Displays them on a 7-column Kanban board: **OPEN → READY → ON-HOLD → IN PROGRESS → AWAITING OPERATOR → CANCELED → DONE**
- Drag cards between columns to change status
- Drag cards within a column to reorder them
- Click any card to view and edit its fields (title, priority, type, milestone, body)
- Create new tickets directly from the board
- All changes write back to the Markdown files immediately — your AI agent sees them on next run

## Who it's for

Q-CTRL is an optional companion to [Scaffold](https://github.com/ksetlak/scaffold), an AI-assisted project template. If you're managing a Scaffold project backlog in plain Markdown and want a visual board on top of it, Q-CTRL is that board.

It does not replace your AI tool or your text editor. It's a thin visual layer over the same files your agent reads and writes.

---

## Installation

Download the latest binary for your platform from the [Releases](../../releases) page:

| Platform | File |
|----------|------|
| macOS (Apple Silicon / Intel) | `qctrl-macos` |
| Linux (x86_64) | `qctrl-linux` |
| Windows (x86_64) | `qctrl-windows.exe` |

Place the binary at the root of your Scaffold project and make it executable:

```bash
# macOS / Linux
chmod +x qctrl-macos
mv qctrl-macos qctrl-app
./qctrl-app
```

> **macOS note:** Gatekeeper will warn on first launch because the binary is unsigned. Right-click → Open → Open to bypass it once.

Q-CTRL must be launched from your project root — it looks for `backlog/items/` relative to its own location.

---

## Building from source

Requires [Rust](https://rustup.rs) and [just](https://github.com/casey/just).

```bash
git clone https://github.com/ksetlak/q-ctrl
cd q-ctrl
just build       # debug build → copies binary to repo root as qctrl-app
just release     # release build → same
just run         # build + launch immediately
```

The compiled binary lands at the repo root as `qctrl-app`. To use it with a Scaffold project, copy or symlink it to that project's root.

---

## Backlog format

Q-CTRL reads standard Scaffold Markdown tickets. Each file in `backlog/items/` must have YAML frontmatter with at least these fields:

```markdown
---
id: MY-001
title: Short imperative title
status: OPEN
priority: must
type: feature
order: 1.0
---

Body text here.
```

Valid `status` values: `OPEN`, `READY`, `ON-HOLD`, `IN_PROGRESS`, `AWAITING_OPERATOR`, `CANCELED`, `DONE`.

Valid `priority` values: `must`, `should`, `could`.

See [Scaffold's backlog schema](https://github.com/ksetlak/scaffold/blob/main/backlog/README.md) for the full spec.

---

## Project status

Early-stage and actively developed. The core board, drag-and-drop, and ticket editing all work. Known rough edges are tracked in the [backlog](backlog/items/).

---

## License

MIT — see [LICENSE](LICENSE).  
Copyright (c) 2026 Krzysztof Setlak.

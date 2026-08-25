# Scaffold App — Spec

Rust + Slint. Single binary. macOS + Linux primary.

---

## Specification

### Distribution

- Source code lives in `backlog/app/` within the Scaffold repo. Binary not committed.
- Releases published on GitHub.
- Users run the binary from within their project's `backlog/` folder. The app assumes the repo root is one level up (`../`).
- `backlog/app/` establishes a pattern: other mini-apps can live as siblings in `backlog/` in the future.

### Name

`backlog`. Binary invoked as `./backlog` from the `backlog/` folder.

### Scope

- Backlog only, for the foreseeable future.
- Full Kanban board: view, reorder (drag within column), move between columns (status change), edit ticket fields (priority, description, acceptance criteria, notes), create new tickets.
- No agent integration in this version.
- Single user, single project per instance.

### Kanban columns

All statuses rendered as columns on the main board:

`OPEN` · `READY` · `ON-HOLD` · `IN_PROGRESS` · `AWAITING_OPERATOR` · `CANCELED` · `DONE`

- Drag between columns changes `status` in the frontmatter. No rule enforcement — operator is trusted.
- The `DONE` drop zone is split into two surfaces: one sets `status: DONE`, the other sets `status: CANCELED`.
- Column visibility will be configurable in a future version.

### Archive view

Items in `backlog/archive/` shown in a separate view, not part of the main Kanban board.

### Ordering

`order` field (float) in frontmatter. Drag-and-drop within a column writes a new value using midpoint insertion. Rebalance when float precision runs out.

### Filesystem

App watches `backlog/items/` and `backlog/archive/` for changes and hot-reloads. Fall back to manual refresh if this proves problematic.

### UI style

Native backend (CoreGraphics on macOS, GTK on Linux). System title bar, native file dialogs, native fonts. Follows system dark/light mode by default, user-configurable.

### Configuration

User preferences (e.g. theme) stored at `~/.config/scaffold-app/config.toml`. Whenever a preference is written, the app shows a discreet, tasteful notification: "Configuration saved to `~/.config/scaffold-app/config.toml`" — so users stay aware of what's being written to their home directory and can find and clean it up later.

### New ticket creation

App creates new tickets by copying `_template.md`, assigning the next available ID, and opening the edit view.

---

## Open Questions

_None currently open._

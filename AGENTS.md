# Agent Guide

This is the canonical agent instruction file for this repository. All AI tools should read this file. If your tool uses a different entry point (`.cursorrules`, `.gemini/AGENTS.md`, etc.), point it here — see `SETUP.md` for per-tool instructions.

## Repository orientation

- `AGENTS.md` — this file; rules and working guidance for AI agents
- `README.md` — project overview, repository layout, developer setup
- `backlog/README.md` — backlog schema, statuses, agent workflow
- `backlog/items/` — one file per open backlog item; `backlog/archive/` for closed ones
- `docs/adrs/` — Architecture Decision Records (index: `docs/adrs/README.md`)
- `docs/prds/` — Product Requirements Documents (index: `docs/prds/README.md`)
- `.agents/` — role-specific agent definitions and skills

When exploring a documentation subfolder, read its `README.md` as an index before opening individual files.

## Working style

- Make small, reviewable changes. Avoid broad refactors unless they directly unblock the current task.
- When requirements are unclear, stop and ask the operator. Do not make assumptions on product or architecture decisions.
- Record all significant product and architecture decisions as PRDs or ADRs in `docs/`.
- Update `README.md` and relevant docs when adding commands, services, environment variables, or changing project structure.

## Backlog delivery

Use the `/pickup` skill (`.agents/skills/pickup/`) to work on backlog items. See `backlog/README.md` for the full workflow, ticket schema, and status rules.

## Dependency checks

Before adding any new dependency, verify it is appropriate for this project's constraints and document the decision if it represents an architectural shift.

## Key project rules

- **Language**: Rust. All application code is Rust. No exceptions.
- **UI framework**: Slint. All UI is defined in `.slint` files. No alternative UI frameworks.
- **App source lives in `qctrl/`** (standard Cargo workspace at the repo root). The compiled binary lands at the repo root as `qctrl-app`.
- **Binary is never committed**. It is built locally via `just build` / `just release`, or downloaded from GitHub Releases.
- **CWD convention**: the app always runs from the repo root. All file paths (`backlog/items/`, `backlog/archive/`, etc.) are resolved relative to the directory containing the `qctrl-app` binary.
- **No database**. All state is read from and written to Markdown files with YAML frontmatter in `backlog/items/` and `backlog/archive/`. Do not introduce SQLite or any other storage layer without an ADR.
- **User config** goes to `~/.config/qctrl/config.toml`. Never write config into the repo directory.
- **Project key**: `SB` (used for Q-CTRL's own development backlog). Ticket IDs: `SB-001`, `SB-002`, etc.
- Before adding any dependency, check that it compiles on both macOS and Linux. Prefer crates with no system library dependencies unless unavoidable.

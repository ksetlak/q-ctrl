# Setup

Follow this checklist after cloning Scaffold for a new project.

## 1. Reinitialise git

```bash
rm -rf .git
git init
git add .
git commit -m "chore: init from Scaffold"
```

## 2. Rename the project

- Rename the folder to your project name.
- Find and replace any remaining `scaffold` references in `README.md` and `AGENTS.md`.

## 3. Set the backlog project key

Open `backlog/_template.md` and `backlog/README.md` and replace `APP` with your project key abbreviation (e.g. `RC` for Rovicky-Cedar). Ticket IDs will follow the pattern `RC-001`, `RC-002`, etc.

## 4. Update `.gitignore`

Add these entries for things that should never be committed in a project built on Scaffold:

```
# Ephemeral agent run artifacts
backlog/_runs/

# AI tool config — generated locally per developer (see step 5)
.cursorrules
.gemini/
```

## 5. Configure your AI tool

`AGENTS.md` is the single source of truth for agent instructions. Point your tool at it:

| Tool | What to do |
|------|-----------|
| **Claude Code** | Nothing — it reads `AGENTS.md` automatically |
| **Cursor** | Already configured via `.cursorrules` → `@./AGENTS.md` |
| **Gemini CLI** | `mkdir -p .gemini && cp AGENTS.md .gemini/AGENTS.md` (re-copy when `AGENTS.md` changes) |
| **GitHub Copilot** | Nothing — recent versions read `AGENTS.md` automatically |
| **Codex / other** | Check your tool's docs for where it looks for system instructions; copy or symlink `AGENTS.md` there |
| **Custom harness (pi, etc.)** | Pass `AGENTS.md` as the system prompt or project context at session start |

## 6. Add project-specific rules to AGENTS.md

Open `AGENTS.md` and replace the placeholder `_No project-specific rules yet_` section with the rules that apply to your project. Typical entries:

- Naming conventions
- Forbidden libraries or patterns  
- Required file locations
- Testing requirements
- Language and framework constraints

## 7. Write your first ADR (optional but recommended)

If you have already made a significant architecture decision (language, framework, database), record it now before you forget the reasoning. Use the template at `docs/adrs/_template.md`.

## 8. Write your first backlog items

Copy `backlog/_template.md` to `backlog/items/APP-001.md` (replacing `APP` with your project key) for each item. See `backlog/README.md` for the full schema. Set `status: READY` on items you want agents to pick up.

## 9. Install Q-CTRL (optional)

Q-CTRL is a native Kanban board app for managing your Scaffold backlog visually. It reads directly from `backlog/items/` and writes status and order changes back to disk.

Download the latest binary for your platform from [Q-CTRL releases](https://github.com/your-handle/qctrl/releases) and place it at `qctrl/qctrl` (macOS/Linux) or `qctrl\qctrl.exe` (Windows). Run it from the repo root:

```bash
qctrl/qctrl
```

Q-CTRL is optional — the backlog works fine with plain Markdown and your AI tool alone.

## You're ready

Run `/pickup` in your AI tool to start the first backlog item.

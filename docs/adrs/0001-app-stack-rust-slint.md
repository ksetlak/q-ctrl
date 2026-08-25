# ADR-0001: App stack — Rust + Slint

**Status**: ACCEPTED  
**Date**: 2026-07-31  
**Deciders**: Krzysiek Setlak (@ksetlak)

---

## Context

The Scaffold framework needs a native GUI application to manage backlog items visually — a Kanban board that reads and writes Markdown files with YAML frontmatter. The app must:

- Run on macOS and Linux (primary targets).
- Ship as a single binary with no runtime dependencies.
- Be entirely AI-written, since the operator has no prior Rust or GUI experience.
- Parse and write Markdown frontmatter as its data layer.
- Feel native on both platforms (system title bar, file dialogs, dark/light mode).

The choice of language and UI framework is the foundational decision that all other implementation decisions follow from.

## Decision

**Rust** as the implementation language, **Slint** as the UI framework, using Slint's native backend.

## Alternatives Considered

| Alternative | Pros | Cons | Why Rejected |
|-------------|------|------|--------------|
| **Rust + Slint (chosen)** | Single binary, no runtime; cross-platform; native feel via OS renderer; strong frontmatter crates (`gray_matter`); large AI training corpus for Rust | New language and framework for the operator; Slint's Rust API is less documented than its C++ API | *Selected* |
| **Rust + egui** | Immediate-mode, simple mental model, very well AI-trained | Software renderer by default — does not feel native; less polished on macOS | Rejected — native feel is a stated requirement |
| **Go + Fyne** | Single binary, cross-platform, simpler than Rust | Smaller AI training corpus for Fyne; less control over native rendering; GC pauses | Rejected — Rust has stronger crate ecosystem for this use case |
| **Python + PyQt / Tkinter** | Fast to prototype, large AI corpus | Not a single binary without heavy bundling (PyInstaller etc.); dependency management fragile | Rejected — single binary is a hard requirement |
| **Electron / Tauri** | Web frontend skills transfer; rich UI ecosystem | Electron: ships a browser (heavy); Tauri: Rust backend but WebView frontend — loses native feel and adds complexity | Rejected — overkill, not native |
| **Swift + SwiftUI** | Best native macOS experience | macOS only; no Linux path | Rejected — Linux is a primary target |

## Consequences

### Positive

- Single binary: user downloads one file, runs it. No `npm install`, no Python venv, no system packages.
- Rust's ownership model and type system mean the AI is forced to be explicit about state — harder to write subtle data-corruption bugs against Markdown files.
- Slint's native backend delegates rendering to the OS: CoreGraphics on macOS, GTK on Linux. Dark/light mode, accessibility, and system fonts come for free.
- `gray_matter` and `serde_yaml` crates handle frontmatter parsing; well-maintained and cross-platform.
- `notify` crate provides filesystem watching on both platforms with a unified API.

### Negative

- Rust compile times are slow — iterating on UI layout is slower than a web stack.
- Slint's AI training corpus is smaller than React or Qt; the AI may need to reference Slint docs more frequently.
- No operator ability to debug or fix Rust compilation errors without AI assistance — the app is entirely AI-dependent for maintenance.

### Mitigations

- Keep the codebase as small and modular as possible. Fewer lines = fewer things to break.
- Pin dependency versions explicitly in `Cargo.lock` (committed) to avoid surprise breakage.
- If Slint proves limiting, `egui` is a viable fallback for a non-native-feel version; the data layer (frontmatter parsing) would be unchanged.

## Open Questions

- None. Decision is final.

## Related Decisions

- ADR-0002 (planned): frontmatter parsing library choice.
- ADR-0003 (planned): filesystem watching strategy.

---

## Changelog

| Date | Author | Change | Version |
|------|--------|--------|---------|
| 2026-07-31 | Krzysiek | Initial — ACCEPTED | 1.0 |

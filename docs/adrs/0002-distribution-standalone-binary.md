# ADR-0002: Distribute Q-CTRL as a standalone binary

**Status**: ACCEPTED  
**Date**: 2026-08-25  
**Deciders**: Krzysiek

---

## Context

Q-CTRL is a Rust/Slint desktop Kanban board that reads backlog items from a `backlog/items/` directory in a Scaffold-based project. It needs to be distributed to users who may be on macOS, Linux, or Windows.

Q-CTRL is designed as an optional extension to Scaffold — users who don't want a GUI backlog tool should never need to interact with it. This makes a lightweight, zero-dependency distribution format preferable over a tightly integrated installer.

The project is in early development with a single operator and no dedicated build infrastructure. Distribution complexity should match the current stage. The primary audience is developers comfortable with a terminal.

## Decision

Distribute Q-CTRL as a single standalone binary per platform (`qctrl-macos-aarch64`, `qctrl-linux-x86_64`, `qctrl-windows-x86_64.exe`), published to GitHub Releases on each tagged version.

This is chosen over bundled formats (`.app`, AppImage, `.msi`) because the current audience is developers who can handle a bare binary, the tooling cost of proper bundles is disproportionate for an early-stage tool, and a bare binary is strictly simpler to produce, test, and consume. The accepted trade-off is Gatekeeper/SmartScreen friction on first run on macOS and Windows, which technical users can bypass with a one-time right-click → Open.

Builds for all three platforms are automated via GitHub Actions on `git tag` push, producing no manual steps for the operator beyond tagging a release.

## Alternatives Considered

| Alternative | Pros | Cons | Why Rejected |
|-------------|------|------|--------------|
| Standalone binary (GitHub Releases) | Zero install friction for developers; simplest CI pipeline; no bundling toolchain required | Gatekeeper/SmartScreen warning on first run; less polished for non-technical users | *Selected* |
| macOS `.app` + `.dmg` / Linux AppImage | Native look; drag-to-install UX; no terminal needed | Significant packaging toolchain overhead (`cargo-bundle`, `appdmg`, AppImageTool); macOS notarization requires $99/yr Apple developer account | Disproportionate for current stage and audience |
| `cargo install` from crates.io | Familiar to Rust developers; handles updates | Requires Rust toolchain on user machine; not viable for non-Rust users; ties release cadence to crates.io | Too narrow audience; poor UX for non-Rust projects |
| System package managers (Homebrew, apt) | Best UX for end users; handles PATH automatically | Requires maintaining tap/formula/PPA; significant ongoing maintenance burden | Premature for current stage |

## Consequences

### Positive

- Release process is a single `git tag` push; GitHub Actions handles the rest.
- No packaging toolchain dependencies (`cargo-bundle`, `appdmg`, NSIS, etc.).
- Works identically on macOS, Linux, and Windows from the same CI configuration.
- Scaffold stays completely decoupled from Q-CTRL — `SETUP.md` links to the Releases page, nothing more.

### Negative

- macOS and Windows users see an OS security warning on first launch.
- No automatic updates — users must manually download new releases.
- Binary is not on PATH by default; users must place it themselves or launch from the project root.

### Mitigations

- Document the right-click → Open workaround prominently in the Q-CTRL README.
- GitHub Releases provides a changelog and download page that makes manual updates straightforward.
- Q-CTRL will use `std::env::current_dir()` as the project root (see Open Questions), so launching from the project root is the natural usage pattern — no PATH entry required for normal operation.

## Open Questions

- **Resolved:** the project root is now the working directory Q-CTRL is started from (`project_root()` in `main.rs`), not the folder the binary sits in. Outside a folder with `backlog/items/`, it prints a message and exits.
- **Still open:** a folder-picker fallback for when Q-CTRL is launched outside a project, for example double-clicked from Finder. Until it exists, Finder launches exit with the message above. A separate ADR or backlog item should capture this.

## Related Decisions

- ADR-0001: App Stack — Rust + Slint — establishes the build toolchain that produces the binary being distributed here.

---

## Changelog

| Date | Author | Change | Version |
|------|--------|--------|---------|
| 2026-08-25 | Krzysiek | Initial draft | 1.0 |

# Q-CTRL build recipes
# Requires: https://github.com/casey/just
#
# Usage:
#   just        → debug build + copy binary to repo root
#   just build  → same as above
#   just release → release build + copy binary to repo root
#   just run    → debug build, copy, then launch from repo root

repo_root := justfile_directory()
bin_name  := "qctrl"
src_dir   := repo_root / "qctrl"
dest      := repo_root / "qctrl-app"

# Default: debug build
default: build

# Debug build — fast iteration
build:
    cd {{src_dir}} && cargo build
    cp {{src_dir}}/target/debug/{{bin_name}} {{dest}}
    @echo "Binary copied to: {{dest}}"

# Release build — optimised, for distribution
release:
    cd {{src_dir}} && cargo build --release
    cp {{src_dir}}/target/release/{{bin_name}} {{dest}}
    @echo "Binary copied to: {{dest}}"

# Build (debug) and launch immediately from repo root, tee logs to qctrl-debug.log
run: build
    {{dest}} 2>&1 | tee qctrl-debug.log

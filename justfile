# mcps — Rust workspace of MCP servers
#
# Run `just --list` for the available recipes.

# All workspace member crate names (one per directory under the workspace root).
crates := "emacs-mcp gcal-mcp notify-mcp pdf-mcp smhi-mcp"

# Default: show available recipes.
default:
    @just --list

# ── Build ─────────────────────────────────────────────────────────────────────

# Debug build of the whole workspace.
build:
    cargo build --workspace

# Release build of the whole workspace.
build-release:
    cargo build --workspace --release

# Build a single crate in release mode.
build-one crate:
    cargo build -p {{crate}} --release

# ── Lint / format / test ──────────────────────────────────────────────────────

# Run clippy across the workspace with warnings as errors.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Check formatting (does not modify files).
fmt-check:
    cargo fmt --all -- --check

# Apply rustfmt to the workspace.
fmt:
    cargo fmt --all

# Run all offline tests (ignored tests are skipped — see `test-all`).
test:
    cargo test --workspace

# Run every test, including those marked `#[ignore]` (needs Emacs, mako, network).
test-all:
    cargo test --workspace -- --include-ignored

# Run tests for a single crate.
test-one crate:
    cargo test -p {{crate}}

# ── Install / uninstall ───────────────────────────────────────────────────────

# Install every MCP server binary to ~/.cargo/bin.
install-all:
    #!/usr/bin/env sh
    set -eu
    for c in {{crates}}; do
        echo "Installing $c..."
        cargo install --path "$c" --locked
    done

# Install a single MCP server binary to ~/.cargo/bin.
install crate:
    cargo install --path {{crate}} --locked

# Uninstall every MCP server binary from ~/.cargo/bin.
uninstall-all:
    #!/usr/bin/env sh
    set -eu
    for c in {{crates}}; do
        echo "Uninstalling $c..."
        cargo uninstall "$c" || true
    done

# Uninstall a single MCP server binary.
uninstall crate:
    cargo uninstall {{crate}}

# ── Claude Code MCP registration ──────────────────────────────────────────────
#
# Pairs of `short-name=binary-name`. The short name is what Claude Code uses to
# refer to the server; the binary must be on PATH (run `just install-all` first).
mcp_servers := "emacs=emacs-mcp gcal=gcal-mcp notify=notify-mcp pdf=pdf-mcp smhi=smhi-mcp"

# Register every MCP server with Claude Code at user scope (runs install-all first).
claude-register-all: install-all
    #!/usr/bin/env sh
    set -eu
    for pair in {{mcp_servers}}; do
        name="${pair%%=*}"
        bin="${pair##*=}"
        echo "Registering $name -> $bin..."
        claude mcp add --transport stdio --scope user "$name" -- "$bin"
    done

# Register one MCP server (e.g. `just claude-register emacs`); installs `<name>-mcp` first.
claude-register name: (install (name + "-mcp"))
    claude mcp add --transport stdio --scope user {{name}} -- {{name}}-mcp

# Unregister every MCP server from Claude Code's user scope.
claude-unregister-all:
    #!/usr/bin/env sh
    set -eu
    for pair in {{mcp_servers}}; do
        name="${pair%%=*}"
        echo "Unregistering $name..."
        claude mcp remove --scope user "$name" || true
    done

# Unregister a single MCP server (e.g. `just claude-unregister emacs`).
claude-unregister name:
    claude mcp remove --scope user {{name}}

# ── Housekeeping ──────────────────────────────────────────────────────────────

# `cargo clean` — remove the target/ directory.
clean:
    cargo clean

# Full sanity check: fmt-check + clippy + test.
ci: fmt-check clippy test

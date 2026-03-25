# AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) when
working with code in this repository.

## Overview

A Rust workspace with three independent MCP (Model Context Protocol)
servers:
- **emacs-mcp** — Interact with a running Emacs instance via
  `emacsclient`
- **notify-mcp** — Send Linux desktop notifications via D-Bus
  (notify-rust)
- **smhi-mcp** — Fetch Swedish weather forecasts from the SMHI public
  API

## Commands

```bash
# Build all crates
cargo build --release

# Build a single crate
cargo build -p emacs-mcp --release

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p smhi-mcp

# Lint
cargo clippy

# Format
cargo fmt
```

## Architecture

Each server follows the same three-layer pattern:

1. **`main.rs`** — Tokio runtime + tracing setup, stdio transport
2. **`X_server.rs`** — Implements `ServerHandler` from the `rmcp` SDK;
   defines MCP tools using `#[tool_router]` / `#[tool]` macros;
   returns `String` results
3. **`X_client.rs`** — Encapsulates the actual work (subprocess, HTTP,
   D-Bus)

Tool parameters are structs deriving `serde::Deserialize` and
`schemars::JsonSchema`; parameter injection uses `Parameters(p):
Parameters<ParamStruct>`.

Errors use `thiserror` for enum definitions and `anyhow` for
propagation. Tool handler methods swallow errors by returning
formatted error strings rather than propagating them through MCP.

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `rmcp` | Official Rust MCP SDK (GitHub source) |
| `tokio` | Async runtime |
| `serde` / `serde_json` | Serialization |
| `schemars` | JSON Schema for tool parameters |
| `thiserror` / `anyhow` | Error handling |
| `tracing` | Logging |
| `reqwest` | HTTP (smhi-mcp only) |
| `notify-rust` | D-Bus notifications (notify-mcp only) |
| `chrono` | Date/time parsing (smhi-mcp only) |

All workspace-level deps are declared in the root `Cargo.toml` and
inherited by member crates.

## Notes

- Rust edition 2024 throughout
- `emacs-mcp` supports multiple Emacs instances via socket names
  (`--socket-name`)
- `smhi-mcp` defaults coordinates to Gothenburg; supports 1–240 hour
  forecast windows
- Integration tests for smhi-mcp live in `smhi-mcp/tests/forecast.rs`
  and avoid real network requests

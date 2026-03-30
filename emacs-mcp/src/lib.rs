//! # emacs-mcp
//!
//! An MCP (Model Context Protocol) server that exposes a running Emacs instance
//! as a set of tools. It communicates with Emacs by spawning `emacsclient`
//! subprocesses, so Emacs must have `(server-start)` active.
//!
//! ## Modules
//!
//! - [`emacs_client`] — Low-level interface to `emacsclient`.
//! - [`emacs_server`] — MCP server handler and tool definitions.
//! - [`error`] — Error types for the crate.

pub mod emacs_client;
pub mod emacs_server;
pub mod error;

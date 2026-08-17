//! # pdf-mcp
//!
//! An MCP (Model Context Protocol) server that opens PDFs in
//! [sioyek](https://github.com/ahrm/sioyek) at a specific page or text
//! location. It spawns sioyek as a detached subprocess so the MCP request
//! returns immediately and the reader keeps running independently.
//!
//! ## Modules
//!
//! - [`pdf_client`] — Low-level interface to `sioyek`.
//! - [`pdf_server`] — MCP server handler and tool definitions.
//! - [`error`] — Error types for the crate.

pub mod error;
pub mod pdf_client;
pub mod pdf_server;

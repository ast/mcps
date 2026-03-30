//! Entry point for the emacs-mcp server.
//!
//! Initialises tracing, creates an [`EmacsServer`], and serves it over
//! stdio using the MCP transport protocol.

use anyhow::Result;
use emacs_mcp::emacs_server::EmacsServer;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for the MCP stdio transport.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting emacs-mcp server");

    let service = EmacsServer::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

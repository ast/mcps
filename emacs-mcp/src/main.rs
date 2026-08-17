//! Entry point for the emacs-mcp server.
//!
//! Initialises tracing, creates an [`EmacsServer`], and serves it over
//! stdio using the MCP transport protocol.
//!
//! Set `EMACS_SOCKET_NAME` to target a specific Emacs server socket when
//! multiple Emacs instances are running.

use anyhow::Result;
use emacs_mcp::emacs_client::EmacsClient;
use emacs_mcp::emacs_server::EmacsServer;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let client = match std::env::var("EMACS_SOCKET_NAME") {
        Ok(name) if !name.is_empty() => {
            tracing::info!("Using Emacs socket name: {name}");
            EmacsClient::with_socket(name)
        }
        _ => EmacsClient::new(),
    };

    tracing::info!("Starting emacs-mcp server");

    let service = EmacsServer::with_client(client).serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

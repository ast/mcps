//! Entry point for the pdf-mcp server.
//!
//! Initialises tracing, creates a [`PdfServer`], and serves it over stdio
//! using the MCP transport protocol.

use anyhow::Result;
use pdf_mcp::pdf_server::PdfServer;
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

    tracing::info!("Starting pdf-mcp server");

    let service = PdfServer::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

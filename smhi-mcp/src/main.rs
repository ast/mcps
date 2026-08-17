use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use smhi_mcp::smhi_server::SmhiServer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for the MCP stdio transport.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting smhi-mcp server");

    let service = SmhiServer::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

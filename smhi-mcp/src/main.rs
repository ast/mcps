use anyhow::Result;
use smhi_mcp::smhi_server::SmhiServer;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for the MCP stdio transport.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting smhi-mcp server");

    let service = SmhiServer::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

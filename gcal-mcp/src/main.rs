use anyhow::Result;
use gcal_mcp::gcal_server::GcalServer;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for the MCP stdio transport.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting gcal-mcp server");

    let service = GcalServer::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

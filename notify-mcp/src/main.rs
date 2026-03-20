use anyhow::Result;
use notify_mcp::notify_server::NotifyServer;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for the MCP stdio transport.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting notify-mcp server");

    let service = NotifyServer::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

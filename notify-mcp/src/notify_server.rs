use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;

use crate::notify_client::{NotifyClient, Urgency};

// ── Tool parameter structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotifyParams {
    #[schemars(description = "The notification title / summary")]
    pub summary: String,

    #[schemars(description = "Optional longer description shown in the notification body")]
    pub body: Option<String>,

    #[schemars(description = "Urgency level: \"low\", \"normal\", or \"critical\". Defaults to normal.")]
    pub urgency: Option<String>,
}

// ── NotifyServer ──────────────────────────────────────────────────────────────

/// MCP server that exposes desktop notification functionality as tools.
///
/// It delegates notification delivery to [`NotifyClient`], which uses the
/// `notify-rust` crate to communicate with the running notification daemon via D-Bus.
#[derive(Debug, Clone)]
pub struct NotifyServer {
    client: NotifyClient,
    tool_router: ToolRouter<NotifyServer>,
}

#[tool_router]
impl NotifyServer {
    pub fn new() -> Self {
        Self {
            client: NotifyClient::new(),
            tool_router: Self::tool_router(),
        }
    }

    /// Send a desktop notification.
    #[tool(description = "Send a desktop notification with a summary and optional body and urgency level")]
    fn notify(
        &self,
        Parameters(NotifyParams {
            summary,
            body,
            urgency,
        }): Parameters<NotifyParams>,
    ) -> String {
        let urgency_level = urgency.as_deref().and_then(|u| match u {
            "low" => Some(Urgency::Low),
            "normal" => Some(Urgency::Normal),
            "critical" => Some(Urgency::Critical),
            _ => None,
        });

        match self
            .client
            .notify(&summary, body.as_deref(), urgency_level)
        {
            Ok(()) => format!("Notification sent: {summary}"),
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for NotifyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "Send desktop notifications via D-Bus. \
                 Requires a running notification daemon such as Mako or dunst."
                    .to_owned(),
            )
    }
}

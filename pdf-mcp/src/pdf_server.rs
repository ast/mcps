use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::pdf_client::SioyekClient;

// ── Tool parameter structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OpenPdfParams {
    #[schemars(description = "Absolute path to the PDF file to open")]
    pub path: String,
    #[schemars(description = "1-indexed page number to jump to")]
    pub page: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OpenPdfAtTextParams {
    #[schemars(description = "Absolute path to the PDF file to open")]
    pub path: String,
    #[schemars(
        description = "Text snippet to locate; sioyek places a visual mark on the line containing this text"
    )]
    pub text: String,
    #[schemars(description = "Optional 1-indexed page to restrict the text search to")]
    pub page: Option<u32>,
}

// ── PdfServer ────────────────────────────────────────────────────────────────

/// MCP server that opens PDFs in sioyek.
#[derive(Debug, Clone)]
pub struct PdfServer {
    sioyek: SioyekClient,
    #[allow(dead_code)]
    tool_router: ToolRouter<PdfServer>,
}

impl Default for PdfServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl PdfServer {
    pub fn new() -> Self {
        Self {
            sioyek: SioyekClient::new(),
            tool_router: Self::tool_router(),
        }
    }

    /// Open a PDF at a specific page.
    #[tool(
        description = "Open a PDF in sioyek at the given page (1-indexed). The path must be absolute. Reuses the existing sioyek window if one is open. Returns immediately — the reader keeps running independently."
    )]
    async fn open_pdf(
        &self,
        Parameters(OpenPdfParams { path, page }): Parameters<OpenPdfParams>,
    ) -> String {
        match self.sioyek.open(&path, page).await {
            Ok(()) => format!("Opened {path} at page {page}"),
            Err(e) => format!("Error: {e}"),
        }
    }

    /// Open a PDF and jump to a text location.
    #[tool(
        description = "Open a PDF in sioyek and place a visual mark on the line containing the given text. Optionally restrict the search to one page (1-indexed). Useful for visually confirming a snippet that ripgrep-all found in a PDF."
    )]
    async fn open_pdf_at_text(
        &self,
        Parameters(OpenPdfAtTextParams { path, text, page }): Parameters<OpenPdfAtTextParams>,
    ) -> String {
        match self.sioyek.open_at_text(&path, &text, page).await {
            Ok(()) => match page {
                Some(p) => format!("Opened {path} at text {text:?} on page {p}"),
                None => format!("Opened {path} at text {text:?}"),
            },
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for PdfServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Open PDFs in the sioyek reader at a specific page or text location. \
             Useful for visually verifying details that ripgrep-all (rga) has \
             located in PDF documentation. The reader must be installed and on PATH. \
             All paths must be absolute."
                .to_owned(),
        )
    }
}

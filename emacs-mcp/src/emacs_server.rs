use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::emacs_client::{EmacsClient, elisp_string};

// ── Tool parameter structs ────────────────────────────────────────────────────

/// Parameters for the `eval_expression` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EvalParams {
    /// The Emacs Lisp expression to evaluate.
    #[schemars(description = "The Emacs Lisp expression to evaluate")]
    pub expression: String,
}

/// Parameters for the `get_buffer_content` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBufferParams {
    /// Name of the buffer to read. When `None`, the current buffer is used.
    #[schemars(description = "Name of the Emacs buffer. Omit to use the current buffer.")]
    pub buffer_name: Option<String>,
}

/// Parameters for the `open_file` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OpenFileParams {
    /// Absolute or relative path to the file to open.
    #[schemars(description = "Absolute or relative path of the file to open in Emacs")]
    pub path: String,
}

// ── EmacsServer ──────────────────────────────────────────────────────────────

/// MCP server that exposes Emacs functionality as tools.
///
/// It delegates all Emacs interaction to [`EmacsClient`], which spawns
/// `emacsclient` subprocesses to communicate with a running Emacs instance.
#[derive(Debug, Clone)]
pub struct EmacsServer {
    emacs: EmacsClient,
    #[allow(dead_code)]
    tool_router: ToolRouter<EmacsServer>,
}

impl Default for EmacsServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl EmacsServer {
    /// Create a new server with a default [`EmacsClient`] and pre-built tool router.
    pub fn new() -> Self {
        Self::with_client(EmacsClient::new())
    }

    /// Create a new server backed by a specific [`EmacsClient`] (e.g., one with
    /// a custom `--socket-name`).
    pub fn with_client(emacs: EmacsClient) -> Self {
        Self {
            emacs,
            tool_router: Self::tool_router(),
        }
    }

    /// Evaluate an Emacs Lisp expression in the running Emacs instance.
    #[tool(description = "Evaluate an Emacs Lisp expression and return its result")]
    async fn eval_expression(
        &self,
        Parameters(EvalParams { expression }): Parameters<EvalParams>,
    ) -> String {
        match self.emacs.eval(&expression).await {
            Ok(result) => result,
            Err(e) => format!("Error: {e}"),
        }
    }

    /// List all open buffers.
    #[tool(description = "List all open buffers in Emacs")]
    async fn list_buffers(&self) -> String {
        match self
            .emacs
            .eval("(mapcar #'buffer-name (buffer-list))")
            .await
        {
            Ok(result) => result,
            Err(e) => format!("Error: {e}"),
        }
    }

    /// Get the text content of an Emacs buffer.
    #[tool(description = "Get the full text content of an Emacs buffer")]
    async fn get_buffer_content(
        &self,
        Parameters(GetBufferParams { buffer_name }): Parameters<GetBufferParams>,
    ) -> String {
        let expr = match buffer_name {
            Some(name) => format!(
                "(with-current-buffer {} (buffer-string))",
                elisp_string(&name)
            ),
            None => "(buffer-string)".to_owned(),
        };
        match self.emacs.eval(&expr).await {
            Ok(result) => result,
            Err(e) => format!("Error: {e}"),
        }
    }

    /// Open a file in Emacs.
    #[tool(description = "Open a file in the running Emacs instance")]
    async fn open_file(
        &self,
        Parameters(OpenFileParams { path }): Parameters<OpenFileParams>,
    ) -> String {
        let expr = format!("(find-file {})", elisp_string(&path));
        match self.emacs.eval(&expr).await {
            Ok(_) => format!("Opened {path}"),
            Err(e) => format!("Error: {e}"),
        }
    }

    /// Get the current cursor position and buffer file path.
    #[tool(description = "Get the current cursor position and buffer file path in Emacs")]
    async fn get_cursor_position(&self) -> String {
        let expr = "(list (buffer-file-name) (point) (line-number-at-pos) (current-column))";
        match self.emacs.eval(expr).await {
            Ok(result) => result,
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for EmacsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Interact with a running Emacs instance. \
                 Emacs must have `(server-start)` active for tools to work."
                .to_owned(),
        )
    }
}

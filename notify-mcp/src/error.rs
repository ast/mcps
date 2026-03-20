use thiserror::Error;

/// Errors produced by the notify-mcp server.
#[derive(Debug, Error)]
pub enum Error {
    #[error("notification error: {0}")]
    Notify(#[from] notify_rust::error::Error),
}

use thiserror::Error;

/// Errors produced by the smhi-mcp server.
#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid coordinate: {0}")]
    InvalidCoordinate(String),
}

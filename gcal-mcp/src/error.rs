use thiserror::Error;

/// Errors produced by the gcal-mcp server.
#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("iCal parse error: {0}")]
    Parse(String),
    #[error("no calendars configured — add entries to ~/.config/gcal-mcp/config.toml")]
    NoCalendars,
}

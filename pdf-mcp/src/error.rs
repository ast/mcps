use thiserror::Error;

/// Errors produced by the pdf-mcp server.
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to spawn sioyek (is it installed and on PATH?): {0}")]
    SioyekSpawn(std::io::Error),

    #[error("PDF file not found: {path}")]
    FileNotFound { path: String },

    #[error("path must be absolute: {path}")]
    PathNotAbsolute { path: String },

    #[error("invalid page number: {page} (must be >= 1)")]
    InvalidPage { page: u32 },
}

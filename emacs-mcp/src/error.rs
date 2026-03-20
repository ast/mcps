use thiserror::Error;

/// Errors produced by the emacs-mcp server.
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("emacsclient exited with code {code}\nstderr: {stderr}")]
    EmacsExit { code: i32, stderr: String },

    #[error("emacsclient output was not valid UTF-8: {0}")]
    Encoding(#[from] std::string::FromUtf8Error),
}

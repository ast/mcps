use thiserror::Error;

/// Errors produced by the emacs-mcp server.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to spawn or communicate with the `emacsclient` process.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// `emacsclient` exited with a non-zero status code.
    #[error("emacsclient exited with code {code}\nstderr: {stderr}")]
    EmacsExit {
        /// The process exit code (`-1` if the OS did not provide one).
        code: i32,
        /// Captured stderr output from `emacsclient`.
        stderr: String,
    },

    /// `emacsclient` produced stdout that is not valid UTF-8.
    #[error("emacsclient output was not valid UTF-8: {0}")]
    Encoding(#[from] std::string::FromUtf8Error),
}

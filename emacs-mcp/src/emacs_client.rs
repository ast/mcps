use crate::error::Error;

/// Communicates with a running Emacs instance via `emacsclient`.
///
/// By default, `emacsclient` connects to the server started by `(server-start)`
/// in Emacs. Pass a custom socket name when multiple Emacs servers are running.
#[derive(Debug, Clone)]
pub struct EmacsClient {
    socket_name: Option<String>,
}

impl EmacsClient {
    pub fn new() -> Self {
        Self { socket_name: None }
    }

    pub fn with_socket(socket_name: impl Into<String>) -> Self {
        Self {
            socket_name: Some(socket_name.into()),
        }
    }

    /// Evaluate an Emacs Lisp expression and return its printed representation.
    pub async fn eval(&self, expression: &str) -> Result<String, Error> {
        let mut cmd = tokio::process::Command::new("emacsclient");
        cmd.arg("--eval").arg(expression);

        if let Some(name) = &self.socket_name {
            cmd.arg("--socket-name").arg(name);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(Error::EmacsExit { code, stderr });
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    }
}

impl Default for EmacsClient {
    fn default() -> Self {
        Self::new()
    }
}

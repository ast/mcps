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
    /// Create a new client that connects to the default Emacs server socket.
    pub fn new() -> Self {
        Self { socket_name: None }
    }

    /// Create a new client that connects to a specific Emacs server socket.
    ///
    /// This is useful when multiple Emacs instances are running, each with its
    /// own server started via `(server-start)` with a distinct socket name.
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

/// Format a Rust string as an Emacs Lisp string literal, escaping `\` and `"`.
///
/// Use this whenever interpolating untrusted input into elisp source, so that
/// a stray quote in a buffer name or path cannot escape the string and inject
/// arbitrary code.
pub fn elisp_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' | '"' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elisp_string_escapes_quote_and_backslash() {
        assert_eq!(elisp_string("hello"), r#""hello""#);
        assert_eq!(elisp_string(r#"a"b"#), r#""a\"b""#);
        assert_eq!(elisp_string(r"a\b"), r#""a\\b""#);
        assert_eq!(elisp_string(r#""#), r#""""#);
    }
}

use std::path::Path;
use std::process::Stdio;

use crate::error::Error;

/// Drives [sioyek](https://github.com/ahrm/sioyek) via subprocess.
///
/// All operations spawn sioyek detached and return immediately. The child's
/// stdio is redirected to `/dev/null` because the MCP itself uses stdout for
/// its JSON-RPC transport — any output from sioyek (Qt warnings, etc.) would
/// otherwise corrupt the protocol.
#[derive(Debug, Clone, Default)]
pub struct SioyekClient;

impl SioyekClient {
    pub fn new() -> Self {
        Self
    }

    /// Open `path` at `page` (1-indexed). Reuses the running sioyek window
    /// if one exists.
    pub async fn open(&self, path: &str, page: u32) -> Result<(), Error> {
        validate_page(page)?;
        validate_path(path).await?;
        spawn_detached(&build_open_args(path, page))
    }

    /// Open `path` and place a visual mark on the line containing `text`.
    /// Optionally scope the search to a single 1-indexed `page`.
    pub async fn open_at_text(
        &self,
        path: &str,
        text: &str,
        page: Option<u32>,
    ) -> Result<(), Error> {
        if let Some(p) = page {
            validate_page(p)?;
        }
        validate_path(path).await?;
        spawn_detached(&build_open_at_text_args(path, text, page))
    }
}

// ── Validation ────────────────────────────────────────────────────────────────

fn validate_page(page: u32) -> Result<(), Error> {
    if page < 1 {
        return Err(Error::InvalidPage { page });
    }
    Ok(())
}

async fn validate_path(path: &str) -> Result<(), Error> {
    if !Path::new(path).is_absolute() {
        return Err(Error::PathNotAbsolute {
            path: path.to_owned(),
        });
    }
    match tokio::fs::metadata(path).await {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::FileNotFound {
            path: path.to_owned(),
        }),
        Err(e) => Err(Error::Io(e)),
    }
}

// ── Spawning ──────────────────────────────────────────────────────────────────

fn spawn_detached(args: &[String]) -> Result<(), Error> {
    let _child = tokio::process::Command::new("sioyek")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(Error::SioyekSpawn)?;
    // Dropping `_child` does NOT kill sioyek on Unix — the GUI keeps running
    // after this function returns.
    Ok(())
}

// ── Argv builders (pure, unit-testable) ───────────────────────────────────────

pub fn build_open_args(path: &str, page: u32) -> Vec<String> {
    vec![
        "--reuse-window".to_string(),
        "--page".to_string(),
        page.to_string(),
        path.to_string(),
    ]
}

pub fn build_open_at_text_args(path: &str, text: &str, page: Option<u32>) -> Vec<String> {
    let mut args = vec![
        "--reuse-window".to_string(),
        "--focus-text".to_string(),
        text.to_string(),
    ];
    if let Some(p) = page {
        args.push("--focus-text-page".to_string());
        args.push(p.to_string());
    }
    args.push(path.to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_open_args_layout() {
        assert_eq!(
            build_open_args("/p.pdf", 42),
            vec!["--reuse-window", "--page", "42", "/p.pdf"]
        );
    }

    #[test]
    fn build_open_at_text_args_without_page() {
        assert_eq!(
            build_open_at_text_args("/p.pdf", "Theorem 3.1", None),
            vec!["--reuse-window", "--focus-text", "Theorem 3.1", "/p.pdf"]
        );
    }

    #[test]
    fn build_open_at_text_args_with_page() {
        assert_eq!(
            build_open_at_text_args("/p.pdf", "Theorem 3.1", Some(42)),
            vec![
                "--reuse-window",
                "--focus-text",
                "Theorem 3.1",
                "--focus-text-page",
                "42",
                "/p.pdf"
            ]
        );
    }

    #[test]
    fn validate_page_rejects_zero() {
        assert!(matches!(
            validate_page(0),
            Err(Error::InvalidPage { page: 0 })
        ));
    }

    #[test]
    fn validate_page_accepts_one_and_above() {
        assert!(validate_page(1).is_ok());
        assert!(validate_page(1_000_000).is_ok());
    }

    #[tokio::test]
    async fn validate_path_rejects_relative() {
        let err = validate_path("relative.pdf").await.unwrap_err();
        assert!(matches!(err, Error::PathNotAbsolute { .. }));
    }

    #[tokio::test]
    async fn validate_path_rejects_missing_file() {
        let err = validate_path("/nonexistent/path/foo.pdf")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::FileNotFound { .. }));
    }
}

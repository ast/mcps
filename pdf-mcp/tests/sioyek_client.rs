use std::time::Duration;

use pdf_mcp::pdf_client::SioyekClient;

/// `open` rejects relative paths without ever invoking sioyek.
#[tokio::test]
async fn open_rejects_relative_path() {
    let client = SioyekClient::new();
    let result = client.open("relative.pdf", 1).await;
    assert!(result.is_err(), "expected error for relative path");
}

/// `open` rejects a missing file before spawning sioyek.
#[tokio::test]
async fn open_rejects_missing_file() {
    let client = SioyekClient::new();
    let result = client.open("/nonexistent/path/foo.pdf", 1).await;
    assert!(result.is_err(), "expected error for missing file");
}

/// `open` rejects `page == 0` (sioyek uses 1-indexed pages).
#[tokio::test]
async fn open_rejects_zero_page() {
    let client = SioyekClient::new();
    let result = client.open("/etc/hostname", 0).await;
    assert!(result.is_err(), "expected error for page 0");
}

// ── Live sioyek tests (need DISPLAY + a real PDF) ─────────────────────────────

/// Spawning sioyek must return immediately, not block on the reader.
///
/// We write a tiny PDF fixture, call `open`, and assert that the future
/// resolves within 2 seconds. A regression to `.wait()` would hang here.
#[tokio::test]
#[ignore = "requires sioyek + DISPLAY"]
async fn open_returns_immediately() {
    let fixture = write_pdf_fixture();
    let client = SioyekClient::new();

    let result = tokio::time::timeout(
        Duration::from_secs(2),
        client.open(fixture.to_str().unwrap(), 1),
    )
    .await;

    assert!(
        result.is_ok(),
        "open() did not return within 2s — accidentally blocking on the child?"
    );
    assert!(result.unwrap().is_ok(), "open() should succeed");
}

#[tokio::test]
#[ignore = "requires sioyek + DISPLAY"]
async fn open_at_text_returns_immediately() {
    let fixture = write_pdf_fixture();
    let client = SioyekClient::new();

    let result = tokio::time::timeout(
        Duration::from_secs(2),
        client.open_at_text(fixture.to_str().unwrap(), "anything", None),
    )
    .await
    .expect("did not return within 2s");
    assert!(result.is_ok());
}

/// Minimal valid PDF (single blank page) written to a temp path.
fn write_pdf_fixture() -> std::path::PathBuf {
    let path = std::env::temp_dir().join("pdf-mcp-fixture.pdf");
    // Smallest legal PDF I could find — enough for sioyek to open.
    let bytes: &[u8] = b"%PDF-1.4\n\
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj\n\
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj\n\
xref\n\
0 4\n\
0000000000 65535 f \n\
0000000009 00000 n \n\
0000000053 00000 n \n\
0000000098 00000 n \n\
trailer<</Size 4/Root 1 0 R>>\n\
startxref\n\
156\n\
%%EOF";
    std::fs::write(&path, bytes).expect("write fixture PDF");
    path
}

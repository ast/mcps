use emacs_mcp::emacs_client::EmacsClient;

/// Evaluates a simple arithmetic expression in a live Emacs instance.
///
/// Run with: cargo test -- --nocapture
/// Requires Emacs to be running with `(server-start)` active.
#[tokio::test]
#[ignore = "requires a running Emacs server (server-start)"]
async fn test_eval_arithmetic() {
    let client = EmacsClient::new();
    let result = client.eval("(+ 1 2)").await;
    assert!(
        result.is_ok(),
        "Failed to eval expression: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), "3");
}

#[tokio::test]
#[ignore = "requires a running Emacs server (server-start)"]
async fn test_eval_string() {
    let client = EmacsClient::new();
    let result = client.eval("(concat \"hello\" \" \" \"emacs\")").await;
    assert!(
        result.is_ok(),
        "Failed to eval string expression: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), "\"hello emacs\"");
}

use notify_mcp::notify_client::{NotifyClient, Urgency};

/// Sends a real desktop notification to verify the notification system works.
///
/// Run with: cargo test -- --nocapture
/// You should see a desktop notification appear when this test runs.
#[test]
#[ignore = "requires a running notification daemon (mako/dunst)"]
fn test_send_notification() {
    let client = NotifyClient::new();
    let result = client.notify(
        "notify-mcp test",
        Some("Desktop notification integration test passed."),
        Some(Urgency::Normal),
    );
    assert!(
        result.is_ok(),
        "Failed to send notification: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "requires a running notification daemon (mako/dunst)"]
fn test_send_notification_no_body() {
    let client = NotifyClient::new();
    let result = client.notify("notify-mcp: no body", None, None);
    assert!(
        result.is_ok(),
        "Failed to send notification without body: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "requires a running notification daemon (mako/dunst)"]
fn test_send_notification_critical() {
    let client = NotifyClient::new();
    let result = client.notify(
        "notify-mcp: critical",
        Some("This is a critical urgency notification."),
        Some(Urgency::Critical),
    );
    assert!(
        result.is_ok(),
        "Failed to send critical notification: {:?}",
        result.err()
    );
}

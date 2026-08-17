use gcal_mcp::gcal_client::{CalendarConfig, ConfigOutcome, GcalClient};
use gcal_mcp::gcal_server::format_events;

/// Load calendars from the real config file, skipping the test if it is absent.
macro_rules! load_calendars {
    () => {{
        match CalendarConfig::load() {
            Ok(ConfigOutcome::Loaded(cals)) if !cals.is_empty() => cals,
            Ok(ConfigOutcome::Loaded(_)) => {
                eprintln!("SKIP: config file exists but contains no calendars");
                return;
            }
            Ok(ConfigOutcome::Missing) => {
                eprintln!("SKIP: no config file at ~/.config/gcal-mcp/config.toml");
                return;
            }
            Err(e) => {
                eprintln!("SKIP: could not load config file: {e}");
                return;
            }
        }
    }};
}

// ── Config loading ────────────────────────────────────────────────────────────

#[test]
#[ignore = "requires user config file with calendar URLs"]
fn config_loads_at_least_one_calendar() {
    let calendars = load_calendars!();
    assert!(!calendars.is_empty());
    for cal in &calendars {
        assert!(!cal.name.is_empty(), "calendar name should not be empty");
        assert!(
            cal.url.starts_with("https://"),
            "calendar URL should be HTTPS: {}",
            cal.url
        );
    }
}

// ── Live network tests ────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "hits the live Google Calendar iCal feed"]
async fn fetch_returns_ok() {
    let calendars = load_calendars!();
    let client = GcalClient::new();

    let result = client.list_upcoming(&calendars, 30).await;
    assert!(result.is_ok(), "list_upcoming failed: {:?}", result.err());
}

#[tokio::test]
#[ignore = "hits the live Google Calendar iCal feed"]
async fn fetched_events_have_non_empty_summaries() {
    let calendars = load_calendars!();
    let client = GcalClient::new();

    let events = client
        .list_upcoming(&calendars, 30)
        .await
        .expect("list_upcoming should succeed");

    for event in &events {
        assert!(
            !event.summary.is_empty(),
            "event summary should not be empty"
        );
        assert!(
            !event.calendar.is_empty(),
            "event calendar label should not be empty"
        );
    }
}

#[tokio::test]
#[ignore = "hits the live Google Calendar iCal feed"]
async fn fetched_events_are_sorted_by_start_time() {
    let calendars = load_calendars!();
    let client = GcalClient::new();

    let events = client
        .list_upcoming(&calendars, 30)
        .await
        .expect("list_upcoming should succeed");

    for window in events.windows(2) {
        assert!(
            window[0].start <= window[1].start,
            "events should be sorted: {} > {}",
            window[0].start,
            window[1].start
        );
    }
}

#[tokio::test]
#[ignore = "hits the live Google Calendar iCal feed"]
async fn fetched_events_are_within_requested_window() {
    let calendars = load_calendars!();
    let client = GcalClient::new();
    let days = 14u32;

    let now = chrono::Utc::now();
    let cutoff = now + chrono::Duration::days(days as i64);

    let events = client
        .list_upcoming(&calendars, days)
        .await
        .expect("list_upcoming should succeed");

    for event in &events {
        assert!(
            event.start >= now,
            "event should not be in the past: {} — {}",
            event.summary,
            event.start
        );
        assert!(
            event.start <= cutoff,
            "event should be within {days}-day window: {} — {}",
            event.summary,
            event.start
        );
    }
}

// ── Formatting integration ────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "hits the live Google Calendar iCal feed"]
async fn format_output_is_non_empty_string() {
    let calendars = load_calendars!();
    let client = GcalClient::new();

    let events = client
        .list_upcoming(&calendars, 7)
        .await
        .expect("list_upcoming should succeed");

    let output = format_events(&events, 7);
    assert!(!output.is_empty(), "formatted output should not be empty");
    println!("\n--- gcal-mcp output (next 7 days) ---\n{output}\n---");
}

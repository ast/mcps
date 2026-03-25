use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;

use crate::gcal_client::{CalEvent, CalendarConfig, GcalClient};

// ── Tool parameter structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListEventsParams {
    #[schemars(description = "Number of days ahead to include (default: 7, max: 90)")]
    pub days: Option<u32>,
}

// ── GcalServer ────────────────────────────────────────────────────────────────

/// MCP server that exposes Google Calendar events via private iCal feed URLs.
#[derive(Debug, Clone)]
pub struct GcalServer {
    client: GcalClient,
    calendars: Vec<CalendarConfig>,
    tool_router: ToolRouter<GcalServer>,
}

#[tool_router]
impl GcalServer {
    pub fn new() -> Self {
        let calendars = CalendarConfig::from_config_file().unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {e:#}");
            Vec::new()
        });
        Self {
            client: GcalClient::new(),
            calendars,
            tool_router: Self::tool_router(),
        }
    }

    /// List upcoming calendar events.
    #[tool(description = "List upcoming Google Calendar events from all configured calendars. Returns event titles, times, and locations for the next N days. Configure calendars via the GCAL_ICAL_URLS environment variable.")]
    async fn list_events(
        &self,
        Parameters(ListEventsParams { days }): Parameters<ListEventsParams>,
    ) -> String {
        let days = days.unwrap_or(7).min(90);

        match self.client.list_upcoming(&self.calendars, days).await {
            Ok(events) => format_events(&events, days),
            Err(e) => format!("Error fetching calendar events: {e}"),
        }
    }
}

pub fn format_events(events: &[CalEvent], days: u32) -> String {
    if events.is_empty() {
        return format!("No upcoming events in the next {days} day(s).");
    }

    let mut lines = Vec::new();
    let mut current_date = None;

    for event in events {
        let date = event.start.date_naive();

        // Print a date header when the day changes.
        if current_date != Some(date) {
            current_date = Some(date);
            lines.push(format!("\n{}", date.format("%A, %B %-d")));
        }

        let time = if event.all_day {
            "All day".to_string()
        } else {
            event.start.format("%H:%M UTC").to_string()
        };

        let mut line = format!("  {time}  {}", event.summary);

        if let Some(loc) = &event.location {
            line.push_str(&format!("  [{loc}]"));
        }

        if events.iter().filter(|e| e.calendar == event.calendar).count() > 0
            && events.iter().map(|e| &e.calendar).collect::<std::collections::HashSet<_>>().len() > 1
        {
            line.push_str(&format!("  ({})", event.calendar));
        }

        lines.push(line);
    }

    lines.join("\n").trim_start_matches('\n').to_string()
}

#[tool_handler]
impl ServerHandler for GcalServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "Provides read-only access to Google Calendar events via private iCal feed URLs. \
                 Use list_events to see upcoming events. Configure calendar feeds by setting \
                 GCAL_ICAL_URLS to a comma-separated list of iCal URLs, optionally prefixed \
                 with a name (e.g. \"Work=https://...,Personal=https://...\")."
                    .to_owned(),
            )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::gcal_client::CalEvent;

    use super::*;

    fn make_event(calendar: &str, summary: &str, hour: u32, all_day: bool) -> CalEvent {
        CalEvent {
            calendar: calendar.to_string(),
            summary: summary.to_string(),
            start: Utc.with_ymd_and_hms(2024, 6, 3, hour, 0, 0).unwrap(),
            end: None,
            location: None,
            description: None,
            all_day,
        }
    }

    #[test]
    fn format_no_events() {
        let output = format_events(&[], 7);
        assert!(output.contains("No upcoming events"));
    }

    #[test]
    fn format_single_event() {
        let events = vec![make_event("Work", "Team standup", 9, false)];
        let output = format_events(&events, 7);
        assert!(output.contains("Team standup"));
        assert!(output.contains("09:00 UTC"));
        assert!(output.contains("Monday"));
    }

    #[test]
    fn format_all_day_event() {
        let events = vec![make_event("Personal", "Birthday", 0, true)];
        let output = format_events(&events, 7);
        assert!(output.contains("All day"));
        assert!(output.contains("Birthday"));
    }

    #[test]
    fn format_multiple_calendars_shows_name() {
        let events = vec![
            make_event("Work", "Standup", 9, false),
            make_event("Personal", "Gym", 7, false),
        ];
        let output = format_events(&events, 7);
        assert!(output.contains("(Work)") || output.contains("(Personal)"));
    }
}

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use icalendar::{Calendar, CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, EventLike};
use serde::Deserialize;

use crate::error::Error;

// ── Calendar configuration ────────────────────────────────────────────────────

/// Deserialised form of `~/.config/gcal-mcp/config.toml`.
///
/// ```toml
/// [[calendars]]
/// name = "Work"
/// url  = "https://calendar.google.com/calendar/ical/.../basic.ics"
///
/// [[calendars]]
/// name = "Personal"
/// url  = "https://calendar.google.com/calendar/ical/.../basic.ics"
/// ```
#[derive(Debug, Deserialize)]
struct ConfigFile {
    calendars: Vec<CalendarConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CalendarConfig {
    pub name: String,
    pub url: String,
}

impl CalendarConfig {
    /// Load calendars from `$XDG_CONFIG_HOME/gcal-mcp/config.toml`
    /// (falls back to `~/.config/gcal-mcp/config.toml`).
    pub fn from_config_file() -> Result<Vec<Self>> {
        let config_path = dirs::config_dir()
            .context("could not determine config directory")?
            .join("gcal-mcp")
            .join("config.toml");

        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;

        let cfg: ConfigFile = toml::from_str(&contents)
            .with_context(|| format!("parsing {}", config_path.display()))?;

        Ok(cfg.calendars)
    }
}

// ── Event model ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CalEvent {
    pub calendar: String,
    pub summary: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub all_day: bool,
}

// ── Client ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GcalClient {
    http: reqwest::Client,
}

impl GcalClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Fetch and parse a single iCal feed URL, returning raw events.
    async fn fetch_calendar(&self, config: &CalendarConfig) -> Result<Vec<CalEvent>> {
        let body = self
            .http
            .get(&config.url)
            .send()
            .await
            .map_err(Error::Http)
            .context("fetching iCal feed")?
            .error_for_status()
            .map_err(Error::Http)
            .context("iCal feed returned error status")?
            .text()
            .await
            .map_err(Error::Http)
            .context("reading iCal body")?;

        let calendar = body
            .parse::<Calendar>()
            .map_err(|e| Error::Parse(e.to_string()))
            .context("parsing iCal data")?;

        let mut events = Vec::new();
        for component in &calendar.components {
            if let CalendarComponent::Event(event) = component {
                let summary = event
                    .get_summary()
                    .unwrap_or("(no title)")
                    .to_string();

                let Some(start_raw) = event.get_start() else {
                    continue;
                };

                let (start, all_day) = date_perhaps_time_to_utc(start_raw);
                let end = event.get_end().map(|t| date_perhaps_time_to_utc(t).0);

                events.push(CalEvent {
                    calendar: config.name.clone(),
                    summary,
                    start,
                    end,
                    location: event.get_location().map(str::to_string),
                    description: event.get_description().map(str::to_string),
                    all_day,
                });
            }
        }

        Ok(events)
    }

    /// Fetch all configured calendars and return events within the next `days` days.
    pub async fn list_upcoming(
        &self,
        calendars: &[CalendarConfig],
        days: u32,
    ) -> Result<Vec<CalEvent>> {
        if calendars.is_empty() {
            return Err(Error::NoCalendars.into());
        }

        let now = Utc::now();
        let cutoff = now + chrono::Duration::days(days as i64);

        let mut all_events = Vec::new();
        for cal in calendars {
            match self.fetch_calendar(cal).await {
                Ok(mut events) => {
                    events.retain(|e| e.start >= now && e.start <= cutoff);
                    all_events.extend(events);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch calendar '{}': {e:#}", cal.name);
                }
            }
        }

        all_events.sort_by_key(|e| e.start);
        Ok(all_events)
    }
}

// ── Date/time helpers ─────────────────────────────────────────────────────────

fn date_perhaps_time_to_utc(dt: DatePerhapsTime) -> (DateTime<Utc>, bool) {
    match dt {
        DatePerhapsTime::DateTime(cal_dt) => (cal_datetime_to_utc(cal_dt), false),
        DatePerhapsTime::Date(date) => (naive_date_to_utc(date), true),
    }
}

fn cal_datetime_to_utc(dt: CalendarDateTime) -> DateTime<Utc> {
    match dt {
        CalendarDateTime::Utc(utc) => utc,
        // Treat floating/zoned datetimes as UTC (best-effort for v1).
        CalendarDateTime::Floating(ndt) => Utc.from_utc_datetime(&ndt),
        CalendarDateTime::WithTimezone { date_time, .. } => Utc.from_utc_datetime(&date_time),
    }
}

fn naive_date_to_utc(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .map(|ndt| Utc.from_utc_datetime(&ndt))
        .unwrap_or_else(Utc::now)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn config_file_parses_toml() {
        let toml = r#"
            [[calendars]]
            name = "Work"
            url  = "https://example.com/work.ics"

            [[calendars]]
            name = "Personal"
            url  = "https://example.com/personal.ics"
        "#;
        let cfg: ConfigFile = toml::from_str(toml).unwrap();
        assert_eq!(cfg.calendars.len(), 2);
        assert_eq!(cfg.calendars[0].name, "Work");
        assert_eq!(cfg.calendars[1].url, "https://example.com/personal.ics");
    }

    #[test]
    fn naive_date_converts_to_midnight_utc() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let dt = naive_date_to_utc(date);
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap());
    }
}

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use icalendar::{
    Calendar, CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, EventLike,
};
use rrule::{RRuleSet, Tz as RRuleTz};
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

/// Outcome of attempting to load the config file.
pub enum ConfigOutcome {
    /// File loaded successfully (may still be empty).
    Loaded(Vec<CalendarConfig>),
    /// File does not exist — treat as "no calendars configured."
    Missing,
}

impl CalendarConfig {
    /// Load calendars from `$XDG_CONFIG_HOME/gcal-mcp/config.toml`
    /// (falls back to `~/.config/gcal-mcp/config.toml`).
    ///
    /// Distinguishes a missing file (expected on first run) from an invalid one.
    pub fn load() -> Result<ConfigOutcome> {
        let config_path = dirs::config_dir()
            .context("could not determine config directory")?
            .join("gcal-mcp")
            .join("config.toml");

        let contents = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ConfigOutcome::Missing);
            }
            Err(e) => {
                return Err(e).with_context(|| format!("reading {}", config_path.display()));
            }
        };

        let cfg: ConfigFile = toml::from_str(&contents)
            .with_context(|| format!("parsing {}", config_path.display()))?;

        Ok(ConfigOutcome::Loaded(cfg.calendars))
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
    pub all_day: bool,
}

// ── Client ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GcalClient {
    http: reqwest::Client,
}

impl Default for GcalClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GcalClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent(concat!("gcal-mcp/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("reqwest client builds with default TLS backend"),
        }
    }

    /// Fetch and parse a single iCal feed URL, returning events that overlap
    /// the `[now, cutoff)` window. Recurring events (RRULE) are expanded.
    async fn fetch_calendar(
        &self,
        config: &CalendarConfig,
        now: DateTime<Utc>,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<CalEvent>> {
        let body = self
            .http
            .get(&config.url)
            .send()
            .await
            .context("fetching iCal feed")?
            .error_for_status()
            .context("iCal feed returned error status")?
            .text()
            .await
            .context("reading iCal body")?;

        let calendar = body
            .parse::<Calendar>()
            .map_err(Error::Parse)
            .context("parsing iCal data")?;

        let mut events = Vec::new();
        for component in &calendar.components {
            let CalendarComponent::Event(event) = component else {
                continue;
            };
            let summary = event.get_summary().unwrap_or("(no title)").to_string();
            let location = event.get_location().map(str::to_string);

            let Some(start_raw) = event.get_start() else {
                continue;
            };
            let end_raw = event.get_end();

            let (start_utc, all_day) = date_perhaps_time_to_utc(&start_raw);
            let end_utc = end_raw.as_ref().map(|t| date_perhaps_time_to_utc(t).0);
            let duration = end_utc.map(|e| e - start_utc);

            let mk_event = |start: DateTime<Utc>| CalEvent {
                calendar: config.name.clone(),
                summary: summary.clone(),
                start,
                end: duration.map(|d| start + d),
                location: location.clone(),
                all_day,
            };

            if let Some(rrule_text) = event.property_value("RRULE") {
                let instances = expand_recurring(&start_raw, rrule_text, now, cutoff);
                events.extend(instances.into_iter().map(mk_event));
            } else if start_utc >= now && start_utc <= cutoff {
                events.push(mk_event(start_utc));
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
            match self.fetch_calendar(cal, now, cutoff).await {
                Ok(events) => all_events.extend(events),
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

fn date_perhaps_time_to_utc(dt: &DatePerhapsTime) -> (DateTime<Utc>, bool) {
    match dt {
        DatePerhapsTime::DateTime(cal_dt) => (cal_datetime_to_utc(cal_dt), false),
        DatePerhapsTime::Date(date) => (naive_date_to_utc(*date), true),
    }
}

fn cal_datetime_to_utc(dt: &CalendarDateTime) -> DateTime<Utc> {
    match dt {
        CalendarDateTime::Utc(utc) => *utc,
        // Floating time: spec says "interpret in the local zone of the observer."
        // Best-effort fallback to UTC for v1.
        CalendarDateTime::Floating(ndt) => Utc.from_utc_datetime(ndt),
        CalendarDateTime::WithTimezone { date_time, tzid } => match tzid.parse::<chrono_tz::Tz>() {
            Ok(tz) => tz
                .from_local_datetime(date_time)
                .earliest()
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|| Utc.from_utc_datetime(date_time)),
            Err(_) => Utc.from_utc_datetime(date_time),
        },
    }
}

fn naive_date_to_utc(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .map(|ndt| Utc.from_utc_datetime(&ndt))
        .unwrap_or_else(Utc::now)
}

// ── RRULE expansion ───────────────────────────────────────────────────────────

/// Build an iCalendar-style block (`DTSTART:...\nRRULE:...`) and feed it to the
/// `rrule` crate, returning instances inside `[now, cutoff)`.
///
/// Falls back to an empty list if the RRULE is malformed or unsupported.
fn expand_recurring(
    start: &DatePerhapsTime,
    rrule_text: &str,
    now: DateTime<Utc>,
    cutoff: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let dtstart_line = dtstart_ical_line(start);
    let block = format!("{dtstart_line}\nRRULE:{rrule_text}");

    let set: RRuleSet = match block.parse() {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("Could not parse RRULE block: {e} ({block:?})");
            return Vec::new();
        }
    };

    let set = set
        .after(now.with_timezone(&RRuleTz::UTC))
        .before(cutoff.with_timezone(&RRuleTz::UTC));

    set.all(500)
        .dates
        .into_iter()
        .map(|d| d.with_timezone(&Utc))
        .collect()
}

fn dtstart_ical_line(dt: &DatePerhapsTime) -> String {
    match dt {
        DatePerhapsTime::DateTime(CalendarDateTime::Utc(utc)) => {
            format!("DTSTART:{}", utc.format("%Y%m%dT%H%M%SZ"))
        }
        DatePerhapsTime::DateTime(CalendarDateTime::Floating(ndt)) => {
            format!("DTSTART:{}Z", ndt.format("%Y%m%dT%H%M%S"))
        }
        DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { date_time, tzid }) => {
            format!("DTSTART;TZID={tzid}:{}", date_time.format("%Y%m%dT%H%M%S"))
        }
        DatePerhapsTime::Date(date) => format!("DTSTART;VALUE=DATE:{}", date.format("%Y%m%d")),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn cal_datetime_with_timezone_converts_to_utc() {
        // 10:00 in Stockholm in summer (CEST, +02:00) is 08:00 UTC.
        let ndt = chrono::NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let cdt = CalendarDateTime::WithTimezone {
            date_time: ndt,
            tzid: "Europe/Stockholm".to_string(),
        };
        let utc = cal_datetime_to_utc(&cdt);
        assert_eq!(utc, Utc.with_ymd_and_hms(2024, 6, 15, 8, 0, 0).unwrap());
    }

    #[test]
    fn cal_datetime_utc_passes_through() {
        let original = Utc.with_ymd_and_hms(2024, 6, 15, 8, 0, 0).unwrap();
        let cdt = CalendarDateTime::Utc(original);
        assert_eq!(cal_datetime_to_utc(&cdt), original);
    }

    #[test]
    fn dtstart_line_utc() {
        let cdt = CalendarDateTime::Utc(Utc.with_ymd_and_hms(2024, 6, 15, 8, 0, 0).unwrap());
        let line = dtstart_ical_line(&DatePerhapsTime::DateTime(cdt));
        assert_eq!(line, "DTSTART:20240615T080000Z");
    }

    #[test]
    fn dtstart_line_with_timezone() {
        let ndt = chrono::NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let cdt = CalendarDateTime::WithTimezone {
            date_time: ndt,
            tzid: "Europe/Stockholm".to_string(),
        };
        let line = dtstart_ical_line(&DatePerhapsTime::DateTime(cdt));
        assert_eq!(line, "DTSTART;TZID=Europe/Stockholm:20240615T100000");
    }

    #[test]
    fn expand_weekly_rrule_returns_multiple_instances() {
        let start_utc = Utc.with_ymd_and_hms(2024, 6, 3, 9, 0, 0).unwrap();
        let cdt = CalendarDateTime::Utc(start_utc);
        let dt = DatePerhapsTime::DateTime(cdt);

        let now = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let cutoff = Utc.with_ymd_and_hms(2024, 7, 1, 0, 0, 0).unwrap();

        let instances = expand_recurring(&dt, "FREQ=WEEKLY;COUNT=4", now, cutoff);
        assert_eq!(instances.len(), 4);
        assert_eq!(instances[0], start_utc);
        assert_eq!(
            instances[1],
            Utc.with_ymd_and_hms(2024, 6, 10, 9, 0, 0).unwrap()
        );
    }

    #[test]
    fn expand_invalid_rrule_returns_empty() {
        let cdt = CalendarDateTime::Utc(Utc.with_ymd_and_hms(2024, 6, 3, 9, 0, 0).unwrap());
        let dt = DatePerhapsTime::DateTime(cdt);

        let now = Utc::now();
        let cutoff = now + chrono::Duration::days(30);
        assert!(expand_recurring(&dt, "NONSENSE=YES", now, cutoff).is_empty());
    }
}

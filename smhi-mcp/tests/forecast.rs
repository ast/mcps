use chrono::Utc;
use smhi_mcp::smhi_client::{Parameter, TimeSeries, weather_symbol, wind_direction_name};
use smhi_mcp::smhi_server::format_forecast;

fn make_ts(valid_time: chrono::DateTime<Utc>, params: &[(&str, f32)]) -> TimeSeries {
    TimeSeries {
        valid_time,
        parameters: params
            .iter()
            .map(|(name, val)| Parameter {
                name: name.to_string(),
                values: vec![*val],
            })
            .collect(),
    }
}

// ── smhi_client helpers ───────────────────────────────────────────────────────

#[test]
fn wind_direction_cardinal_points() {
    assert_eq!(wind_direction_name(0.0), "N");
    assert_eq!(wind_direction_name(45.0), "NE");
    assert_eq!(wind_direction_name(90.0), "E");
    assert_eq!(wind_direction_name(135.0), "SE");
    assert_eq!(wind_direction_name(180.0), "S");
    assert_eq!(wind_direction_name(225.0), "SW");
    assert_eq!(wind_direction_name(270.0), "W");
    assert_eq!(wind_direction_name(315.0), "NW");
}

#[test]
fn wind_direction_boundary_wraps() {
    // 360° should wrap back to N
    assert_eq!(wind_direction_name(360.0), "N");
    // 337.5° is the boundary between NW and N — rounds to N
    assert_eq!(wind_direction_name(338.0), "N");
}

#[test]
fn weather_symbol_all_known_codes() {
    let expected = [
        (1, "Clear sky"),
        (7, "Fog"),
        (11, "Thunderstorm"),
        (18, "Light rain"),
        (19, "Moderate rain"),
        (20, "Heavy rain"),
        (25, "Light snowfall"),
        (27, "Heavy snowfall"),
    ];
    for (code, desc) in expected {
        assert_eq!(weather_symbol(code as f32), desc, "code {code}");
    }
}

#[test]
fn weather_symbol_unknown_code() {
    assert_eq!(weather_symbol(0.0), "Unknown");
    assert_eq!(weather_symbol(99.0), "Unknown");
}

#[test]
fn param_extraction_present() {
    let ts = make_ts(
        Utc::now(),
        &[("t", 12.5), ("ws", 4.2), ("r", 70.0)],
    );
    assert_eq!(ts.param("t"), Some(12.5));
    assert_eq!(ts.param("ws"), Some(4.2));
    assert_eq!(ts.param("r"), Some(70.0));
}

#[test]
fn param_extraction_missing() {
    let ts = make_ts(Utc::now(), &[("t", 5.0)]);
    assert_eq!(ts.param("wd"), None);
    assert_eq!(ts.param("pmean"), None);
}

// ── format_forecast ───────────────────────────────────────────────────────────

#[test]
fn format_forecast_contains_all_fields() {
    let ts = make_ts(
        "2024-06-15T10:00:00Z".parse().unwrap(),
        &[
            ("t", 18.0),
            ("ws", 5.5),
            ("wd", 270.0),
            ("r", 65.0),
            ("pmean", 0.0),
            ("Wsymb2", 2.0),
        ],
    );
    let out = format_forecast(&[ts], 24);
    assert!(out.contains("18.0°C"), "temperature: {out}");
    assert!(out.contains("5.5 m/s"), "wind speed: {out}");
    assert!(out.contains("W"), "wind direction: {out}");
    assert!(out.contains("65%"), "humidity: {out}");
    assert!(out.contains("Nearly clear sky"), "symbol: {out}");
}

#[test]
fn format_forecast_shows_precipitation() {
    let ts = make_ts(
        "2024-11-01T14:00:00Z".parse().unwrap(),
        &[
            ("t", 4.0),
            ("ws", 9.0),
            ("wd", 180.0),
            ("r", 90.0),
            ("pmean", 2.5),
            ("Wsymb2", 20.0),
        ],
    );
    let out = format_forecast(&[ts], 24);
    assert!(out.contains("2.5 mm/h"), "precipitation: {out}");
    assert!(out.contains("Heavy rain"), "symbol: {out}");
}

#[test]
fn format_forecast_hides_zero_precipitation() {
    let ts = make_ts(
        "2024-06-01T08:00:00Z".parse().unwrap(),
        &[
            ("t", 20.0),
            ("ws", 2.0),
            ("wd", 90.0),
            ("r", 45.0),
            ("pmean", 0.0),
            ("Wsymb2", 1.0),
        ],
    );
    let out = format_forecast(&[ts], 24);
    assert!(!out.contains("mm/h"), "should not show zero precip: {out}");
}

#[test]
fn format_forecast_respects_hours_limit() {
    let entries: Vec<TimeSeries> = (0..10)
        .map(|i| {
            make_ts(
                chrono::DateTime::parse_from_rfc3339(&format!("2024-06-01T{i:02}:00:00Z"))
                    .unwrap()
                    .into(),
                &[("t", i as f32), ("ws", 1.0), ("wd", 0.0), ("r", 50.0)],
            )
        })
        .collect();

    let out3 = format_forecast(&entries, 3);
    assert_eq!(out3.lines().count(), 3, "should have 3 lines: {out3}");

    let out10 = format_forecast(&entries, 10);
    assert_eq!(out10.lines().count(), 10, "should have 10 lines: {out10}");
}

#[test]
fn format_forecast_empty_input() {
    let out = format_forecast(&[], 24);
    assert_eq!(out, "No forecast data available.");
}

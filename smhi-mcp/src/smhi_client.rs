use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::Error;

const BASE_URL: &str = "https://opendata-download-metfcst.smhi.se";
const USER_AGENT: &str = concat!("smhi-mcp/", env!("CARGO_PKG_VERSION"));
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

// ── API response types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SmhiForecast {
    #[serde(rename = "timeSeries")]
    pub time_series: Vec<TimeSeries>,
}

#[derive(Debug, Deserialize)]
pub struct TimeSeries {
    #[serde(rename = "validTime")]
    pub valid_time: DateTime<Utc>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub values: Vec<f32>,
}

impl TimeSeries {
    /// Extract the first value of a named parameter, if present.
    pub fn param(&self, name: &str) -> Option<f32> {
        self.parameters
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.values.first())
            .copied()
    }
}

// ── Client ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SmhiClient {
    http: reqwest::Client,
}

impl Default for SmhiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SmhiClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(HTTP_TIMEOUT)
            .build()
            .expect("reqwest client builds with default TLS backend");
        Self { http }
    }

    pub async fn get_forecast(&self, lat: f32, lon: f32) -> Result<SmhiForecast> {
        validate_coords(lat, lon)?;

        let url = format!(
            "{BASE_URL}/api/category/pmp3g/version/2/geotype/point/lon/{lon}/lat/{lat}/data.json"
        );

        let forecast = self
            .http
            .get(&url)
            .send()
            .await
            .context("sending SMHI request")?
            .error_for_status()
            .context("SMHI API returned error status")?
            .json::<SmhiForecast>()
            .await
            .context("deserializing SMHI response")?;

        Ok(forecast)
    }
}

fn validate_coords(lat: f32, lon: f32) -> Result<(), Error> {
    if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(Error::InvalidCoordinate(format!(
            "latitude {lat} is out of range (expected -90..=90)"
        )));
    }
    if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
        return Err(Error::InvalidCoordinate(format!(
            "longitude {lon} is out of range (expected -180..=180)"
        )));
    }
    Ok(())
}

// ── Wind direction helper ─────────────────────────────────────────────────────

pub fn wind_direction_name(degrees: f32) -> &'static str {
    if !degrees.is_finite() {
        return "?";
    }
    let normalized = degrees.rem_euclid(360.0);
    let index = ((normalized + 22.5) / 45.0) as u32 % 8;
    match index {
        0 => "N",
        1 => "NE",
        2 => "E",
        3 => "SE",
        4 => "S",
        5 => "SW",
        6 => "W",
        7 => "NW",
        _ => "?",
    }
}

/// Map Wsymb2 code (1–27) to a short description.
pub fn weather_symbol(code: f32) -> &'static str {
    match code as u32 {
        1 => "Clear sky",
        2 => "Nearly clear sky",
        3 => "Variable cloudiness",
        4 => "Halfclear sky",
        5 => "Cloudy sky",
        6 => "Overcast",
        7 => "Fog",
        8 => "Light rain showers",
        9 => "Moderate rain showers",
        10 => "Heavy rain showers",
        11 => "Thunderstorm",
        12 => "Light sleet showers",
        13 => "Moderate sleet showers",
        14 => "Heavy sleet showers",
        15 => "Light snow showers",
        16 => "Moderate snow showers",
        17 => "Heavy snow showers",
        18 => "Light rain",
        19 => "Moderate rain",
        20 => "Heavy rain",
        21 => "Thunder",
        22 => "Light sleet",
        23 => "Moderate sleet",
        24 => "Heavy sleet",
        25 => "Light snowfall",
        26 => "Moderate snowfall",
        27 => "Heavy snowfall",
        _ => "Unknown",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ts(params: &[(&str, f32)]) -> TimeSeries {
        TimeSeries {
            valid_time: Utc::now(),
            parameters: params
                .iter()
                .map(|(name, val)| Parameter {
                    name: name.to_string(),
                    values: vec![*val],
                })
                .collect(),
        }
    }

    #[test]
    fn param_extraction() {
        let ts = make_ts(&[("t", 5.0), ("ws", 8.3), ("r", 82.0)]);
        assert_eq!(ts.param("t"), Some(5.0));
        assert_eq!(ts.param("ws"), Some(8.3));
        assert_eq!(ts.param("missing"), None);
    }

    #[test]
    fn wind_direction_cardinals() {
        assert_eq!(wind_direction_name(0.0), "N");
        assert_eq!(wind_direction_name(90.0), "E");
        assert_eq!(wind_direction_name(180.0), "S");
        assert_eq!(wind_direction_name(270.0), "W");
        assert_eq!(wind_direction_name(315.0), "NW");
    }

    #[test]
    fn wind_direction_handles_out_of_range() {
        assert_eq!(wind_direction_name(360.0), "N");
        assert_eq!(wind_direction_name(720.0), "N");
        assert_eq!(wind_direction_name(-45.0), "NW");
        assert_eq!(wind_direction_name(f32::NAN), "?");
    }

    #[test]
    fn weather_symbol_mapping() {
        assert_eq!(weather_symbol(1.0), "Clear sky");
        assert_eq!(weather_symbol(18.0), "Light rain");
        assert_eq!(weather_symbol(27.0), "Heavy snowfall");
    }

    #[test]
    fn coord_validation_accepts_valid() {
        assert!(validate_coords(57.7089, 11.9746).is_ok());
        assert!(validate_coords(-90.0, -180.0).is_ok());
        assert!(validate_coords(90.0, 180.0).is_ok());
    }

    #[test]
    fn coord_validation_rejects_invalid() {
        assert!(validate_coords(91.0, 0.0).is_err());
        assert!(validate_coords(0.0, 181.0).is_err());
        assert!(validate_coords(f32::NAN, 0.0).is_err());
        assert!(validate_coords(0.0, f32::INFINITY).is_err());
    }
}

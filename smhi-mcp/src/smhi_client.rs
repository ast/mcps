use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::Error;

const BASE_URL: &str = "https://opendata-download-metfcst.smhi.se";

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

impl SmhiClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn get_forecast(&self, lat: f32, lon: f32) -> Result<SmhiForecast> {
        let url = format!(
            "{BASE_URL}/api/category/pmp3g/version/2/geotype/point/lon/{lon}/lat/{lat}/data.json"
        );

        let forecast = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(Error::Http)
            .context("sending SMHI request")?
            .error_for_status()
            .map_err(Error::Http)
            .context("SMHI API returned error status")?
            .json::<SmhiForecast>()
            .await
            .map_err(Error::Http)
            .context("deserializing SMHI response")?;

        Ok(forecast)
    }
}

// ── Wind direction helper ─────────────────────────────────────────────────────

pub fn wind_direction_name(degrees: f32) -> &'static str {
    match ((degrees + 22.5) as u32 / 45) % 8 {
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
    fn wind_direction() {
        assert_eq!(wind_direction_name(0.0), "N");
        assert_eq!(wind_direction_name(90.0), "E");
        assert_eq!(wind_direction_name(180.0), "S");
        assert_eq!(wind_direction_name(270.0), "W");
        assert_eq!(wind_direction_name(315.0), "NW");
    }

    #[test]
    fn weather_symbol_mapping() {
        assert_eq!(weather_symbol(1.0), "Clear sky");
        assert_eq!(weather_symbol(18.0), "Light rain");
        assert_eq!(weather_symbol(27.0), "Heavy snowfall");
    }
}

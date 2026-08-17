use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::smhi_client::{SmhiClient, weather_symbol, wind_direction_name};

// ── Tool parameter structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ForecastParams {
    #[schemars(description = "Latitude in decimal degrees (default: 57.7089 for Gothenburg)")]
    pub lat: Option<f32>,
    #[schemars(description = "Longitude in decimal degrees (default: 11.9746 for Gothenburg)")]
    pub lon: Option<f32>,
    #[schemars(description = "Number of hours ahead to include (default: 24, max: 240)")]
    pub hours: Option<u32>,
}

// ── SmhiServer ────────────────────────────────────────────────────────────────

/// MCP server that exposes SMHI weather forecast data as tools.
#[derive(Debug, Clone)]
pub struct SmhiServer {
    client: SmhiClient,
    #[allow(dead_code)]
    tool_router: ToolRouter<SmhiServer>,
}

impl Default for SmhiServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl SmhiServer {
    pub fn new() -> Self {
        Self {
            client: SmhiClient::new(),
            tool_router: Self::tool_router(),
        }
    }

    /// Get a weather forecast for a location.
    #[tool(
        description = "Get an hourly weather forecast from SMHI for a given latitude/longitude. Returns temperature, wind, precipitation, and humidity — useful for deciding how to dress."
    )]
    async fn get_forecast(
        &self,
        Parameters(ForecastParams { lat, lon, hours }): Parameters<ForecastParams>,
    ) -> String {
        let lat = lat.unwrap_or(57.7089);
        let lon = lon.unwrap_or(11.9746_f32);
        let hours = hours.unwrap_or(24).min(240) as usize;

        match self.client.get_forecast(lat, lon).await {
            Ok(forecast) => format_forecast(&forecast.time_series, hours),
            Err(e) => format!("Error fetching forecast: {e}"),
        }
    }
}

pub fn format_forecast(time_series: &[crate::smhi_client::TimeSeries], hours: usize) -> String {
    let mut lines = Vec::new();

    for ts in time_series.iter().take(hours) {
        let when = ts.valid_time.format("%a %d %b %H:%M UTC");

        let temp = ts
            .param("t")
            .map_or("?".to_string(), |v| format!("{v:.1}°C"));
        let wind_speed = ts
            .param("ws")
            .map_or("?".to_string(), |v| format!("{v:.1} m/s"));
        let wind_dir = ts
            .param("wd")
            .map_or("?".to_string(), |v| wind_direction_name(v).to_string());
        let humidity = ts
            .param("r")
            .map_or("?".to_string(), |v| format!("{v:.0}%"));
        let precip = ts.param("pmean");
        let symbol = ts.param("Wsymb2").map(weather_symbol).unwrap_or("?");

        let precip_str = match precip {
            Some(p) if p > 0.0 => format!(", rain {p:.1} mm/h"),
            _ => String::new(),
        };

        lines.push(format!(
            "{when}: {temp}, wind {wind_speed} {wind_dir}{precip_str}, humidity {humidity} — {symbol}"
        ));
    }

    if lines.is_empty() {
        "No forecast data available.".to_string()
    } else {
        lines.join("\n")
    }
}

#[tool_handler]
impl ServerHandler for SmhiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Provides hourly weather forecasts from SMHI (Swedish Meteorological and \
                 Hydrological Institute). Use get_forecast with a latitude and longitude \
                 to get temperature, wind, precipitation, and humidity data."
                .to_owned(),
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::smhi_client::{Parameter, TimeSeries};

    use super::*;

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

    #[test]
    fn format_forecast_basic() {
        let ts = make_ts(
            "2024-03-19T12:00:00Z".parse().unwrap(),
            &[
                ("t", 5.0),
                ("ws", 8.3),
                ("wd", 315.0),
                ("r", 82.0),
                ("pmean", 1.2),
                ("Wsymb2", 19.0),
            ],
        );
        let output = format_forecast(&[ts], 24);
        assert!(output.contains("5.0°C"));
        assert!(output.contains("8.3 m/s"));
        assert!(output.contains("NW"));
        assert!(output.contains("82%"));
        assert!(output.contains("1.2 mm/h"));
        assert!(output.contains("Moderate rain"));
    }

    #[test]
    fn format_forecast_no_precip() {
        let ts = make_ts(
            "2024-03-19T08:00:00Z".parse().unwrap(),
            &[
                ("t", 10.0),
                ("ws", 3.0),
                ("wd", 90.0),
                ("r", 60.0),
                ("pmean", 0.0),
                ("Wsymb2", 1.0),
            ],
        );
        let output = format_forecast(&[ts], 24);
        assert!(!output.contains("mm/h"));
        assert!(output.contains("Clear sky"));
    }
}

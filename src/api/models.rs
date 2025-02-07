use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stats {
    pub pageviews: MetricValue,
    pub visitors: MetricValue,
    pub visits: MetricValue,
    pub bounces: MetricValue,
    #[serde(rename = "totaltime")]
    pub total_time: MetricValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricValue {
    pub value: f64,
    pub prev: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metric {
    pub x: String,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthResponse {
    pub token: String,
}

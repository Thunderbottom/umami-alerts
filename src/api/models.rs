use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stats {
    pub pageviews: f64,
    pub visitors: f64,
    pub visits: f64,
    pub bounces: f64,
    pub totaltime: f64,
    pub comparison: StatsComparison,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatsComparison {
    pub pageviews: f64,
    pub visitors: f64,
    pub visits: f64,
    pub bounces: f64,
    pub totaltime: f64,
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

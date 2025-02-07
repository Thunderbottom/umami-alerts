use crate::api::models::{Metric, MetricValue, Stats};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReportData {
    pub website_name: String,
    pub date: String,
    pub stats: Stats,
    pub bounce_rate: MetricValue,
    pub time_spent: String,
    pub pages: Vec<Metric>,
    pub countries: Vec<Metric>,
    pub browsers: Vec<Metric>,
    pub devices: Vec<Metric>,
    pub referrers: Vec<Metric>,
}

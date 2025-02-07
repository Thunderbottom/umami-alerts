use reqwest::{Client, StatusCode};
use std::time::Duration;
use tracing::{debug, error, instrument};

use super::models::{AuthResponse, Metric, Stats};
use crate::error::{AppError, Result};

const API_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct UmamiClient {
    client: Client,
    base_url: String,
}

impl UmamiClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .map_err(|e| AppError::api(format!("Failed to create HTTP client: {}", e)))?;

        // Ensure base_url doesn't end with a slash
        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self { client, base_url })
    }

    #[instrument(skip(self, password))]
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<String> {
        debug!("Authenticating with Umami API");

        let response = self
            .client
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await
            .map_err(|e| AppError::api(format!("Authentication request failed: {}", e)))?;

        match response.status() {
            StatusCode::OK => {
                let auth = response.json::<AuthResponse>().await.map_err(|e| {
                    AppError::api(format!("Failed to parse authentication response: {}", e))
                })?;
                Ok(auth.token)
            }
            status => {
                let error = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                error!("Authentication failed with status {}: {}", status, error);
                Err(AppError::api(format!(
                    "Authentication failed ({}): {}",
                    status, error
                )))
            }
        }
    }

    #[instrument(skip(self, token))]
    pub async fn get_stats(
        &self,
        token: &str,
        website_id: &str,
        start_at: i64,
        end_at: i64,
    ) -> Result<Stats> {
        debug!("Fetching stats for website {}", website_id);

        let response = self
            .client
            .get(format!(
                "{}/api/websites/{}/stats",
                self.base_url, website_id
            ))
            .query(&[
                ("startAt", start_at.to_string()),
                ("endAt", end_at.to_string()),
            ])
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::api(format!("Failed to fetch stats: {}", e)))?;

        self.handle_response(response).await
    }

    #[instrument(skip(self, token))]
    pub async fn get_metrics(
        &self,
        token: &str,
        website_id: &str,
        metric_type: &str,
        start_at: i64,
        end_at: i64,
        limit: u32,
    ) -> Result<Vec<Metric>> {
        debug!(
            "Fetching {} metrics for website {} (limit: {})",
            metric_type, website_id, limit
        );

        let response = self
            .client
            .get(format!(
                "{}/api/websites/{}/metrics",
                self.base_url, website_id
            ))
            .query(&[
                ("type", metric_type.to_string()),
                ("startAt", start_at.to_string()),
                ("endAt", end_at.to_string()),
                ("limit", limit.to_string()),
            ])
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::api(format!("Failed to fetch metrics: {}", e)))?;

        let mut metrics: Vec<Metric> = self.handle_response(response).await?;

        if metric_type == "country" {
            for metric in &mut metrics {
                metric.x = crate::config::get_country_name(&metric.x);
            }
        }

        Ok(metrics)
    }

    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        match response.status() {
            StatusCode::OK => response
                .json::<T>()
                .await
                .map_err(|e| AppError::api(format!("Failed to parse API response: {}", e))),
            StatusCode::UNAUTHORIZED => {
                error!("API authentication failed");
                Err(AppError::api("Authentication token expired or invalid"))
            }
            StatusCode::NOT_FOUND => {
                error!("API endpoint or resource not found");
                Err(AppError::api("Resource not found"))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                error!("API rate limit exceeded");
                Err(AppError::api("Rate limit exceeded"))
            }
            status => {
                let error = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                error!("API request failed with status {}: {}", status, error);
                Err(AppError::api(format!(
                    "API request failed ({}): {}",
                    status, error
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    #[tokio::test]
    async fn test_authentication_success() {
        let mut server = Server::new_async().await;
        let client = UmamiClient::new(server.url()).unwrap();

        let _mock = server
            .mock("POST", "/api/auth/login")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"token": "test-token"}"#)
            .create_async()
            .await;

        let result = client.authenticate("test", "password").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-token");
    }

    #[tokio::test]
    async fn test_authentication_failure() {
        let mut server = Server::new_async().await;
        let client = UmamiClient::new(server.url()).unwrap();

        let _mock = server
            .mock("POST", "/api/auth/login")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid credentials"}"#)
            .create_async()
            .await;

        let result = client.authenticate("test", "wrong").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Api(_)));
    }

    #[tokio::test]
    async fn test_get_stats_success() {
        let mut server = Server::new_async().await;
        let client = UmamiClient::new(server.url()).unwrap();

        let stats = json!({
            "pageviews": { "value": 100, "prev": 90 },
            "visitors": { "value": 50, "prev": 45 },
            "visits": { "value": 75, "prev": 70 },
            "bounces": { "value": 20, "prev": 25 },
            "totaltime": { "value": 3600, "prev": 3300 }
        });

        let _mock = server
            .mock("GET", "/api/websites/test-id/stats")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(stats.to_string())
            .create_async()
            .await;

        let result = client.get_stats("token", "test-id", 0, 1000).await;
        assert!(result.is_ok());
    }
}

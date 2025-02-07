use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use url::Url;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub smtp: SmtpConfig,
    pub websites: HashMap<String, WebsiteConfig>,
    #[serde(default = "default_max_concurrent_jobs")]
    pub max_concurrent_jobs: usize,
}

fn default_max_concurrent_jobs() -> usize {
    4
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    #[serde(default = "default_tls")]
    pub tls: bool,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub skip_verify: bool,
}

fn default_timeout() -> u64 {
    30
}

fn default_tls() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebsiteConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub recipients: Vec<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub disabled: bool,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

impl Config {
    /// Load configuration from a TOML file
    pub async fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config = toml::from_str(&content)?;
        tracing::info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Get all enabled websites
    pub fn enabled_websites(&self) -> impl Iterator<Item = (&String, &WebsiteConfig)> {
        self.websites.iter().filter(|(_, config)| !config.disabled)
    }
}

impl SmtpConfig {
    /// Validate SMTP configuration
    pub fn validate(&self) -> Result<()> {
        if self.host.is_empty() {
            return Err(AppError::Config("SMTP host cannot be empty".to_string()));
        }

        if self.port == 0 {
            return Err(AppError::Config("Invalid SMTP port".to_string()));
        }

        if self.username.is_empty() {
            return Err(AppError::Config(
                "SMTP username cannot be empty".to_string(),
            ));
        }

        if self.password.is_empty() {
            return Err(AppError::Config(
                "SMTP password cannot be empty".to_string(),
            ));
        }

        if !self.from.contains('@') {
            return Err(AppError::Config("Invalid SMTP from address".to_string()));
        }

        Ok(())
    }
}

impl WebsiteConfig {
    /// Validate website configuration
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(AppError::Config("Website ID cannot be empty".to_string()));
        }

        if self.name.is_empty() {
            return Err(AppError::Config("Website name cannot be empty".to_string()));
        }

        // Validate base URL
        Url::parse(&self.base_url)
            .map_err(|e| AppError::Config(format!("Invalid base URL {}: {}", self.base_url, e)))?;

        if self.username.is_empty() {
            return Err(AppError::Config("Username cannot be empty".to_string()));
        }

        if self.password.is_empty() {
            return Err(AppError::Config("Password cannot be empty".to_string()));
        }

        if self.recipients.is_empty() {
            return Err(AppError::Config(
                "At least one recipient is required".to_string(),
            ));
        }

        // Validate email addresses
        for recipient in &self.recipients {
            if !recipient.contains('@') {
                return Err(AppError::Config(format!(
                    "Invalid email address: {}",
                    recipient
                )));
            }
        }

        // Validate timezone
        self.timezone
            .parse::<chrono_tz::Tz>()
            .map_err(|e| AppError::Config(format!("Invalid timezone {}: {}", self.timezone, e)))?;

        Ok(())
    }
}

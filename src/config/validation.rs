use super::models::Config;
use crate::config::models::SmtpConfig;
use crate::error::{AppError, Result};
use crate::WebsiteConfig;
use tracing::warn;

pub fn validate_config(config: &Config) -> Result<()> {
    // Validate SMTP configuration
    config.smtp.validate()?;

    // Validate website configurations
    if config.websites.is_empty() {
        return Err(AppError::Config("No websites configured".to_string()));
    }

    let mut has_enabled_websites = false;
    for (name, website) in &config.websites {
        if website.disabled {
            warn!("Website {} is disabled", name);
            continue;
        }
        has_enabled_websites = true;
        website.validate()?;
    }

    if !has_enabled_websites {
        return Err(AppError::Config("No enabled websites found".to_string()));
    }

    // Validate max concurrent jobs
    if config.max_concurrent_jobs == 0 {
        return Err(AppError::Config(
            "max_concurrent_jobs must be greater than 0".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        Config {
            smtp: SmtpConfig {
                host: "smtp.example.com".to_string(),
                port: 587,
                username: "test".to_string(),
                password: "password".to_string(),
                from: "test@example.com".to_string(),
                tls: true,
                timeout_seconds: 30,
                skip_verify: false,
            },
            websites: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    WebsiteConfig {
                        id: "test-id".to_string(),
                        name: "Test Site".to_string(),
                        base_url: "https://analytics.example.com".to_string(),
                        username: "test".to_string(),
                        password: "password".to_string(),
                        recipients: vec!["admin@example.com".to_string()],
                        timezone: "UTC".to_string(),
                        disabled: false,
                    },
                );
                map
            },
            max_concurrent_jobs: 4,
            // debug: false,
        }
    }

    #[test]
    fn test_valid_config() {
        let config = create_test_config();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_smtp_validation() {
        let mut config = create_test_config();
        config.smtp.host = "".to_string();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_website_validation() {
        let mut config = create_test_config();
        config.websites.get_mut("test").unwrap().base_url = "invalid-url".to_string();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_timezone_validation() {
        let mut config = create_test_config();
        config.websites.get_mut("test").unwrap().timezone = "Invalid/Timezone".to_string();
        assert!(validate_config(&config).is_err());
    }
}

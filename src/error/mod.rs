use std::error::Error as StdError;
use std::fmt;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Config(String),
    Api(String),
    Template(String),
    Smtp(String),
    Json(serde_json::Error),
    Toml(toml::de::Error),
    Request(reqwest::Error),
    Email(String),
    Handlebars(handlebars::RenderError),
    Url(url::ParseError),
    Task(String),
}

impl StdError for AppError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            // Self::TimeZone(e) => Some(e),  // chrono_tz::ParseError implements StdError
            Self::Json(e) => Some(e),
            Self::Toml(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Handlebars(e) => Some(e),
            Self::Url(e) => Some(e),
            // String-based variants don't have a source
            Self::Config(_)
            | Self::Api(_)
            | Self::Template(_)
            | Self::Smtp(_)
            | Self::Email(_)
            | Self::Task(_) => None,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Config(msg) => write!(f, "Configuration error: {msg}"),
            Self::Api(msg) => write!(f, "API error: {msg}"),
            Self::Template(msg) => write!(f, "Template error: {msg}"),
            Self::Smtp(msg) => write!(f, "SMTP error: {msg}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::Toml(e) => write!(f, "TOML error: {e}"),
            Self::Request(e) => write!(f, "Request error: {e}"),
            Self::Email(msg) => write!(f, "Email error: {msg}"),
            Self::Handlebars(e) => write!(f, "Handlebars error: {e}"),
            Self::Url(e) => write!(f, "URL parsing error: {e}"),
            Self::Task(msg) => write!(f, "Task error: {msg}"),
        }
    }
}

// Implement conversions from other error types
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::Request(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        Self::Toml(err)
    }
}

impl From<url::ParseError> for AppError {
    fn from(err: url::ParseError) -> Self {
        Self::Url(err)
    }
}

impl From<handlebars::RenderError> for AppError {
    fn from(err: handlebars::RenderError) -> Self {
        Self::Handlebars(err)
    }
}

impl From<handlebars::TemplateError> for AppError {
    fn from(err: handlebars::TemplateError) -> Self {
        Self::Template(err.to_string())
    }
}

impl From<lettre::error::Error> for AppError {
    fn from(err: lettre::error::Error) -> Self {
        Self::Smtp(err.to_string())
    }
}

impl From<lettre::transport::smtp::Error> for AppError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        Self::Smtp(err.to_string())
    }
}

impl From<lettre::address::AddressError> for AppError {
    fn from(err: lettre::address::AddressError) -> Self {
        Self::Email(err.to_string())
    }
}

impl From<&str> for AppError {
    fn from(msg: &str) -> Self {
        Self::Task(msg.to_string())
    }
}

impl From<String> for AppError {
    fn from(msg: String) -> Self {
        Self::Task(msg)
    }
}

// Helper methods for error classification
impl AppError {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_network_error(&self) -> bool {
        match self {
            Self::Request(e) => e.is_connect() || e.is_timeout(),
            Self::Api(msg) => msg.contains("network"),
            Self::Smtp(msg) => msg.contains("connection"),
            _ => false,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_auth_error(&self) -> bool {
        match self {
            Self::Api(msg) => msg.contains("unauthorized") || msg.contains("forbidden"),
            Self::Smtp(msg) => msg.contains("authentication"),
            _ => false,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_retryable(&self) -> bool {
        self.is_network_error()
            || match self {
                Self::Api(msg) => msg.contains("rate limit") || msg.contains("timeout"),
                Self::Smtp(msg) => msg.contains("try again"),
                _ => false,
            }
    }

    // Helper constructors
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn config<T: ToString>(msg: T) -> Self {
        Self::Config(msg.to_string())
    }

    pub fn api<T: ToString>(msg: T) -> Self {
        Self::Api(msg.to_string())
    }

    pub fn task<T: ToString>(msg: T) -> Self {
        Self::Task(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));

        let str_err: AppError = "test error".into();
        assert!(matches!(str_err, AppError::Task(_)));

        let string_err: AppError = String::from("test error").into();
        assert!(matches!(string_err, AppError::Task(_)));
    }

    #[test]
    fn test_error_classification() {
        let network_err = AppError::Api("network timeout".to_string());
        assert!(network_err.is_network_error());
        assert!(!network_err.is_auth_error());

        let auth_err = AppError::Api("unauthorized access".to_string());
        assert!(!auth_err.is_network_error());
        assert!(auth_err.is_auth_error());

        let retry_err = AppError::Api("rate limit exceeded".to_string());
        assert!(retry_err.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = AppError::config("invalid config");
        assert_eq!(err.to_string(), "Configuration error: invalid config");

        let err = AppError::api("API timeout");
        assert_eq!(err.to_string(), "API error: API timeout");
    }
}

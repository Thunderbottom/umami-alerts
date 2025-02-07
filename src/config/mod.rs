use std::collections::HashMap;
use std::sync::OnceLock;

pub mod models;
pub mod validation;

pub use models::{Config, WebsiteConfig};

static COUNTRY_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
static COUNTRY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/countries.json"
));

pub async fn load_country_map() -> Result<(), crate::error::AppError> {
    let map: HashMap<String, String> = serde_json::from_str(COUNTRY_JSON)?;

    // Initialize the static map
    COUNTRY_MAP.set(map).map_err(|_| {
        crate::error::AppError::Config("Failed to initialize country map".to_string())
    })?;

    Ok(())
}

pub fn get_country_name(code: &str) -> String {
    COUNTRY_MAP
        .get()
        .and_then(|map| map.get(code))
        .map(|name| name.to_string())
        .unwrap_or_else(|| code.to_string())
}

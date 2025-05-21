use clap::Parser;
use futures::stream::{self, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, Level};

mod api;
mod config;
mod error;
mod report;

use crate::{
    api::UmamiClient,
    config::{validation::validate_config, Config, WebsiteConfig},
    error::{AppError, Result},
    report::generator::ReportGenerator,
};

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    report_generator: Arc<ReportGenerator>,
    semaphore: Arc<Semaphore>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Check if config file exists
    if !args.config.exists() {
        return Err(format!("Config file not found: {}", args.config.display()).into());
    }
    // Load configuration
    let config = Config::load(&args.config).await?;
    validate_config(&config)
        .map_err(|e| AppError::api(format!("Config validation failed: {e}")))?;
    let max_concurrent_jobs = config.app.max_concurrent_jobs;

    let log_level = if config.app.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .init();

    info!("Starting umami-alerts");
    debug!("Debug mode enabled");
    debug!("Report type: {:?}", config.app.report_type);

    config::load_country_map().await?;
    info!("Loaded country mappings");

    // Initialize template engine
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.register_template_string(
        "email",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/email.html")),
    )?;
    handlebars.register_helper("formatNumber", Box::new(report::helpers::format_number));
    handlebars.register_helper("percentage", Box::new(report::helpers::percentage));
    handlebars.register_helper("formatFloat", Box::new(report::helpers::format_float));
    handlebars.set_strict_mode(false);
    handlebars::handlebars_helper!(sub: |x:f64, y:f64| x - y);
    handlebars.register_helper("sub", Box::new(sub));
    let handlebars = Arc::new(handlebars);

    // Create application state
    let state = AppState {
        config: Arc::new(config),
        report_generator: Arc::new(ReportGenerator::new(handlebars)),
        semaphore: Arc::new(Semaphore::new(max_concurrent_jobs)),
    };

    // Process all enabled websites concurrently
    let results = stream::iter(state.config.enabled_websites())
        .map(|(name, website)| {
            let state = state.clone();
            async move {
                let _permit = state.semaphore.acquire().await.unwrap();
                match process_website(&state, name, website).await {
                    Ok(_) => Ok(name.to_string()),
                    Err(e) => Err((name.to_string(), e)),
                }
            }
        })
        .buffer_unordered(max_concurrent_jobs)
        .collect::<Vec<_>>()
        .await;

    // Report results
    let (successes, failures): (Vec<_>, Vec<_>) = results.into_iter().partition(|r| r.is_ok());

    info!(
        "Processing complete. {} succeeded, {} failed",
        successes.len(),
        failures.len()
    );

    if !failures.is_empty() {
        let failed_sites: Vec<_> = failures
            .iter()
            .map(|r| r.as_ref().unwrap_err().0.clone())
            .collect();
        error!("Failed websites: {}", failed_sites.join(", "));
        return Err(AppError::task(format!(
            "Failed to process {} websites",
            failures.len()
        )));
    }

    Ok(())
}

async fn process_website(state: &AppState, site_name: &str, website: &WebsiteConfig) -> Result<()> {
    info!("Processing website: {}", site_name);

    // Create API client
    let client = UmamiClient::new(website.base_url.clone())?;

    // Authenticate
    let token = client
        .authenticate(&website.username, &website.password)
        .await?;

    // Generate and send report
    state
        .report_generator
        .generate_and_send(
            &client,
            &state.config.app.dry_run,
            website,
            &state.config.app.report_type,
            &state.config.smtp,
            &token,
        )
        .await?;

    Ok(())
}

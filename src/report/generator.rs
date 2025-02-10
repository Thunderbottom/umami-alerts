use chrono::{DateTime, Datelike, TimeZone, Utc};
use handlebars::Handlebars;
use lettre::{
    message::{header, Message, MultiPart},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use std::sync::Arc;
use tracing::{debug, error, info};

use super::{helpers, models::ReportData};
use crate::{
    api::client::UmamiClient,
    api::models::MetricValue,
    config::models::{ReportType, SmtpConfig, WebsiteConfig},
    error::{AppError, Result},
};

#[derive(Debug)]
struct TimeRange {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ReportGenerator {
    template: Arc<Handlebars<'static>>,
}

impl ReportGenerator {
    pub fn new(template: Arc<Handlebars<'static>>) -> Self {
        Self { template }
    }

    pub async fn generate_and_send(
        &self,
        client: &UmamiClient,
        dry_run: &bool,
        website: &WebsiteConfig,
        report_type: &ReportType,
        smtp_config: &SmtpConfig,
        token: &str,
    ) -> Result<()> {
        info!("Generating report for website: {}", website.name);

        let time_range = self.calculate_time_range(&website.timezone, report_type)?;
        let report_data = self
            .fetch_report_data(client, website, token, time_range, report_type)
            .await?;
        let html = self.render_report(&report_data)?;

        if *dry_run {
            info!("Dry run enabled, will not send an email");
        } else {
            self.send_email(
                smtp_config,
                &website.recipients,
                &format!(
                    "{} Analytics Report - {} - {}",
                    report_type.to_string(),
                    website.name,
                    report_data.date
                ),
                &html,
            )
            .await?;
        }

        info!("Successfully sent report for website: {}", website.name);
        Ok(())
    }

    fn calculate_time_range(&self, timezone: &str, report_type: &ReportType) -> Result<TimeRange> {
        let tz: chrono_tz::Tz = timezone.parse().map_err(|e| {
            error!("Invalid timezone {}: {}", timezone, e);
            AppError::Config(format!("Invalid timezone: {}", e))
        })?;

        let now = Utc::now().with_timezone(&tz);
        let (start, end);

        if report_type == &ReportType::Daily {
            let yesterday = now - chrono::Duration::days(1);
            debug!(
                "Calculating daily report for {}",
                yesterday.format("%Y-%m-%d")
            );

            start = tz
                .with_ymd_and_hms(
                    yesterday.year(),
                    yesterday.month(),
                    yesterday.day(),
                    0,
                    0,
                    0,
                )
                .unwrap()
                .with_timezone(&Utc);

            end = start + chrono::Duration::days(1) - chrono::Duration::seconds(1);
        } else {
            // Generate report for time ending yesterday
            let yesterday = now - chrono::Duration::days(1);
            debug!(
                "Calculating weekly report ending: {}",
                yesterday.format("%Y-%m-%d")
            );

            // Set end time to yesterday 23:59:59
            end = tz
                .with_ymd_and_hms(
                    yesterday.year(),
                    yesterday.month(),
                    yesterday.day(),
                    23,
                    59,
                    59,
                )
                .unwrap()
                .with_timezone(&Utc);

            // Start time is 7 days before end time (previous Sunday 00:00:00)
            start = end - chrono::Duration::days(7) + chrono::Duration::seconds(1);
        }

        debug!("Time range: {} to {}", start, end);
        Ok(TimeRange { start, end })
    }

    async fn fetch_report_data(
        &self,
        client: &UmamiClient,
        website: &WebsiteConfig,
        token: &str,
        time_range: TimeRange,
        report_type: &ReportType,
    ) -> Result<ReportData> {
        debug!(
            "Fetching metrics for time range: {} to {}",
            time_range.start, time_range.end
        );

        let start_at = time_range.start.timestamp_millis();
        let end_at = time_range.end.timestamp_millis();

        let stats = client
            .get_stats(token, &website.id, start_at, end_at)
            .await?;

        let bounce_rate = MetricValue {
            value: if stats.visits.value > 0.0 {
                (stats.bounces.value / stats.visits.value * 100.0).min(100.0)
            } else {
                0.0
            },
            prev: if stats.visits.prev > 0.0 {
                (stats.bounces.prev / stats.visits.prev * 100.0).min(100.0)
            } else {
                0.0
            },
        };

        let time_spent = helpers::format_time_spent(stats.total_time.value, stats.visits.value);

        let pages = client
            .get_metrics(token, &website.id, "url", start_at, end_at, 10)
            .await?;

        let countries = client
            .get_metrics(token, &website.id, "country", start_at, end_at, 10)
            .await?;

        let browsers = client
            .get_metrics(token, &website.id, "browser", start_at, end_at, 5)
            .await?;

        let devices = client
            .get_metrics(token, &website.id, "device", start_at, end_at, 5)
            .await?;

        let referrers = client
            .get_metrics(token, &website.id, "referrer", start_at, end_at, 5)
            .await?;

        Ok(ReportData {
            website_name: website.name.clone(),
            date: time_range.end.format("%B %d, %Y").to_string(),
            report_type: report_type.to_string(),
            stats,
            bounce_rate,
            time_spent,
            pages,
            countries,
            browsers,
            devices,
            referrers,
        })
    }

    fn render_report(&self, data: &ReportData) -> Result<String> {
        debug!("Rendering report template");

        self.template.render("email", &data).map_err(|e| {
            error!("Failed to render template: {}", e);
            AppError::Template(format!("Failed to render report: {}", e))
        })
    }

    async fn send_email(
        &self,
        config: &SmtpConfig,
        recipients: &[String],
        subject: &str,
        html_content: &str,
    ) -> Result<()> {
        debug!("Sending email to {} recipients", recipients.len());

        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let tls_parameters = if config.tls {
            Tls::Required(TlsParameters::new(config.host.clone())?)
        } else {
            Tls::None
        };

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)?
            .credentials(creds)
            .port(config.port)
            .tls(tls_parameters)
            .build();

        for recipient in recipients {
            let email = Message::builder()
                .from(config.from.parse()?)
                .to(recipient.parse()?)
                .subject(subject)
                .multipart(
                    MultiPart::alternative().singlepart(
                        lettre::message::SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(html_content.to_string()),
                    ),
                )?;

            match mailer.send(email).await {
                Ok(_) => debug!("Email sent successfully to {}", recipient),
                Err(e) => {
                    error!("Failed to send email to {}: {}", recipient, e);
                    return Err(AppError::Smtp(format!(
                        "Failed to send email to {}: {}",
                        recipient, e
                    )));
                }
            }
        }

        Ok(())
    }
}

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
    label: String,
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
                    report_type, website.name, report_data.date
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
            AppError::Config(format!("Invalid timezone: {e}"))
        })?;

        let now = Utc::now().with_timezone(&tz);
        let yesterday = now - chrono::Duration::days(1);

        let (start, end, label) = match report_type {
            ReportType::Daily => {
                debug!(
                    "Calculating daily report for {}",
                    yesterday.format("%Y-%m-%d")
                );
                let start_local = tz
                    .with_ymd_and_hms(
                        yesterday.year(),
                        yesterday.month(),
                        yesterday.day(),
                        0,
                        0,
                        0,
                    )
                    .unwrap();
                let end_local =
                    start_local + chrono::Duration::days(1) - chrono::Duration::seconds(1);
                let label = yesterday.format("%B %d, %Y").to_string();
                (
                    start_local.with_timezone(&Utc),
                    end_local.with_timezone(&Utc),
                    label,
                )
            }
            ReportType::Weekly => {
                debug!(
                    "Calculating weekly report ending: {}",
                    yesterday.format("%Y-%m-%d")
                );
                let end_local = tz
                    .with_ymd_and_hms(
                        yesterday.year(),
                        yesterday.month(),
                        yesterday.day(),
                        23,
                        59,
                        59,
                    )
                    .unwrap();
                let start_local =
                    end_local - chrono::Duration::days(7) + chrono::Duration::seconds(1);
                let label = format!(
                    "{} \u{2013} {}",
                    start_local.format("%B %d"),
                    end_local.format("%B %d, %Y")
                );
                (
                    start_local.with_timezone(&Utc),
                    end_local.with_timezone(&Utc),
                    label,
                )
            }
            ReportType::Monthly => {
                // Previous calendar month: first day 00:00:00 through last day 23:59:59
                let first_of_this_month = tz
                    .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
                    .unwrap();
                let last_of_prev_month = first_of_this_month - chrono::Duration::seconds(1);
                let first_of_prev_month = tz
                    .with_ymd_and_hms(
                        last_of_prev_month.year(),
                        last_of_prev_month.month(),
                        1,
                        0,
                        0,
                        0,
                    )
                    .unwrap();
                let label = last_of_prev_month.format("%B %Y").to_string();
                debug!("Calculating monthly report for {}", label);
                (
                    first_of_prev_month.with_timezone(&Utc),
                    last_of_prev_month.with_timezone(&Utc),
                    label,
                )
            }
        };

        debug!("Time range: {} to {}", start, end);
        Ok(TimeRange { start, end, label })
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

        let (stats, pages, countries, browsers, devices, referrers) = tokio::try_join!(
            client.get_stats(token, &website.id, start_at, end_at),
            client.get_metrics(token, &website.id, "path", start_at, end_at, 10),
            client.get_metrics(token, &website.id, "country", start_at, end_at, 10),
            client.get_metrics(token, &website.id, "browser", start_at, end_at, 5),
            client.get_metrics(token, &website.id, "device", start_at, end_at, 5),
            client.get_metrics(token, &website.id, "referrer", start_at, end_at, 5),
        )?;

        let bounce_rate = MetricValue {
            value: if stats.visits > 0.0 {
                (stats.bounces / stats.visits * 100.0).min(100.0)
            } else {
                0.0
            },
            prev: if stats.comparison.visits > 0.0 {
                (stats.comparison.bounces / stats.comparison.visits * 100.0).min(100.0)
            } else {
                0.0
            },
        };

        let time_spent = helpers::format_time_spent(stats.totaltime, stats.visits);

        Ok(ReportData {
            website_name: website.name.clone(),
            date: time_range.label,
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
            AppError::Template(format!("Failed to render report: {e}"))
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
            let tls_params = if config.skip_tls_verify {
                TlsParameters::builder(config.host.clone())
                    .dangerous_accept_invalid_certs(true)
                    .build()?
            } else {
                TlsParameters::new(config.host.clone())?
            };
            Tls::Required(tls_params)
        } else {
            Tls::None
        };

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)?
            .credentials(creds)
            .port(config.port)
            .tls(tls_parameters)
            .timeout(Some(std::time::Duration::from_secs(config.timeout_seconds)))
            .build();

        let from_address: lettre::message::Mailbox = config.from.parse()?;

        for recipient in recipients {
            let email = Message::builder()
                .from(from_address.clone())
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
                        "Failed to send email to {recipient}: {e}"
                    )));
                }
            }
        }

        Ok(())
    }
}

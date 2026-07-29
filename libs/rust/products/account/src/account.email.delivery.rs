use nvbes_core::config::AppConfig;
use std::sync::Arc;

use crate::{AccountError, AccountResult};

pub fn build_email_sender(config: &AppConfig) -> AccountResult<Arc<dyn nvbes_email::EmailSender>> {
    match config.email_provider.as_str() {
        "smtp" => {
            let host = config.smtp_host.clone().ok_or_else(|| {
                AccountError::internal(
                    "email_smtp_host_missing",
                    "NVBES_SMTP_HOST is required when NVBES_EMAIL_PROVIDER=smtp",
                )
            })?;
            if config.environment != "development" && config.email_from_email.is_none() {
                return Err(AccountError::internal(
                    "email_from_missing",
                    "NVBES_EMAIL_FROM_EMAIL is required outside development when SMTP email is enabled",
                ));
            }
            tracing::info!(
                "Email sender: SMTP (host={host}, port={})",
                config.smtp_port
            );
            Ok(Arc::new(nvbes_email::SmtpEmailSender::new(
                nvbes_email::SmtpEmailConfig {
                    host,
                    port: config.smtp_port,
                    username: config.smtp_username.clone(),
                    password: config.smtp_password.clone(),
                    starttls: config.smtp_starttls,
                },
            )?))
        }
        "mock" => {
            if config.environment != "development" {
                return Err(AccountError::internal(
                    "email_mock_forbidden",
                    "Mock email sender is forbidden outside development. Configure NVBES_EMAIL_PROVIDER=smtp.",
                ));
            }
            tracing::info!("Email sender: Mock (development mode)");
            Ok(Arc::new(nvbes_email::MockEmailSender::new()))
        }
        "test-capture" => {
            if config.environment != "development" {
                return Err(AccountError::internal(
                    "email_test_capture_forbidden",
                    "Test email capture is forbidden outside development.",
                ));
            }
            let directory = std::env::var("NVBES_EMAIL_TEST_CAPTURE_DIR").map_err(|_| {
                AccountError::internal(
                    "email_test_capture_missing",
                    "NVBES_EMAIL_TEST_CAPTURE_DIR is required for the test capture provider.",
                )
            })?;
            tracing::info!("Email sender: isolated test capture");
            Ok(Arc::new(nvbes_email::TestCaptureEmailSender::new(
                directory,
            )?))
        }
        provider => Err(AccountError::internal(
            "email_provider_unsupported",
            format!("Unsupported NVBES_EMAIL_PROVIDER={provider}"),
        )),
    }
}

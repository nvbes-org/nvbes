use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::{EmailError, EmailMessage, EmailSender, SendResult};

const JOB_ID_HEADER: &str = "x-nvbes-email-job-id";
const BUSINESS_TYPE_HEADER: &str = "x-nvbes-email-business-type";

pub struct TestCaptureEmailSender {
    directory: PathBuf,
}

impl TestCaptureEmailSender {
    pub fn new(directory: impl AsRef<Path>) -> Result<Self, EmailError> {
        let directory = directory.as_ref();
        if !directory.is_absolute() {
            return Err(config_error());
        }
        std::fs::create_dir_all(directory).map_err(|_| config_error())?;
        let metadata = std::fs::symlink_metadata(directory).map_err(|_| config_error())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(config_error());
        }
        restrict_directory_permissions(directory)?;
        Ok(Self {
            directory: directory.to_path_buf(),
        })
    }
}

#[async_trait]
impl EmailSender for TestCaptureEmailSender {
    async fn send_message(&self, message: &EmailMessage) -> Result<SendResult, EmailError> {
        let job_id = safe_job_id(required_header(message, JOB_ID_HEADER)?)?;
        let business_type = required_header(message, BUSINESS_TYPE_HEADER)?;
        let provider_email_id = format!("<account-job-{job_id}@notify.nvbes.eu>");
        let capture = Capture {
            business_type,
            html_body: message.html_body.as_deref(),
            job_id,
            provider_email_id: &provider_email_id,
            subject: &message.subject,
            text_body: message.text_body.as_deref(),
            to: message
                .to
                .iter()
                .map(|address| address.email.as_str())
                .collect(),
        };
        let serialized = serde_json::to_vec(&capture)?;
        let final_path = self.directory.join(format!("capture-{job_id}.json"));
        if final_path.exists() {
            return Ok(SendResult { provider_email_id });
        }

        let temporary_path = self
            .directory
            .join(format!(".capture-{job_id}-{}.tmp", std::process::id()));
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary_path)
            .await
            .map_err(|_| config_error())?;
        file.write_all(&serialized)
            .await
            .map_err(|_| config_error())?;
        file.sync_all().await.map_err(|_| config_error())?;
        drop(file);

        match tokio::fs::rename(&temporary_path, &final_path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                tokio::fs::remove_file(&temporary_path)
                    .await
                    .map_err(|_| config_error())?;
            }
            Err(_) => return Err(config_error()),
        }
        Ok(SendResult { provider_email_id })
    }
}

#[derive(Serialize)]
struct Capture<'a> {
    job_id: &'a str,
    business_type: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    text_body: Option<&'a str>,
    html_body: Option<&'a str>,
    provider_email_id: &'a str,
}

fn required_header<'a>(message: &'a EmailMessage, expected: &str) -> Result<&'a str, EmailError> {
    message
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(expected))
        .map(|(_, value)| value.as_str())
        .ok_or_else(config_error)
}

fn safe_job_id(value: &str) -> Result<&str, EmailError> {
    let valid = value.len() == 36
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                8 | 13 | 18 | 23 => character == '-',
                _ => character.is_ascii_hexdigit(),
            });
    if valid {
        Ok(value)
    } else {
        Err(config_error())
    }
}

fn config_error() -> EmailError {
    EmailError::Config("Test email capture is not safely configured".to_string())
}

#[cfg(unix)]
fn restrict_directory_permissions(directory: &Path) -> Result<(), EmailError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
        .map_err(|_| config_error())
}

#[cfg(not(unix))]
fn restrict_directory_permissions(_directory: &Path) -> Result<(), EmailError> {
    Ok(())
}

#[cfg(test)]
#[path = "test_capture.tests.rs"]
mod tests;

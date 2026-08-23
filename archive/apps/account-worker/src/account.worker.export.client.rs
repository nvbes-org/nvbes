use std::{fmt, time::Duration};

use nvbes_product_account::export_event::AccountExportFragmentV1;
use reqwest::StatusCode;

use crate::types::{ClaimedExport, ExportParticipant};

#[derive(Clone)]
pub struct ExportClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
    token: String,
    participant: ExportParticipant,
    max_bytes: usize,
}

#[derive(Debug)]
pub struct ExportError {
    retryable: bool,
    code: &'static str,
}

impl ExportClient {
    pub fn new(
        base_url: &str,
        token: String,
        participant: ExportParticipant,
        max_bytes: usize,
    ) -> anyhow::Result<Self> {
        let mut endpoint = reqwest::Url::parse(base_url)?;
        endpoint.set_path("/internal/v1/account-exports");
        Ok(Self {
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(30))
                .user_agent("nvbes-account-worker/1")
                .build()?,
            endpoint,
            token,
            participant,
            max_bytes,
        })
    }

    pub async fn collect(&self, export: &ClaimedExport) -> Result<serde_json::Value, ExportError> {
        let mut response = self
            .http
            .post(self.endpoint.clone())
            .bearer_auth(&self.token)
            .json(&export.payload)
            .send()
            .await
            .map_err(|_| ExportError::transient("account_export_participant_unreachable"))?;
        if !response.status().is_success() {
            return Err(ExportError::from_status(response.status()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.max_bytes as u64)
        {
            return Err(ExportError::permanent("account_export_fragment_too_large"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ExportError::transient("account_export_fragment_interrupted"))?
        {
            if body.len().saturating_add(chunk.len()) > self.max_bytes {
                return Err(ExportError::permanent("account_export_fragment_too_large"));
            }
            body.extend_from_slice(&chunk);
        }
        let fragment: AccountExportFragmentV1 = serde_json::from_slice(&body)
            .map_err(|_| ExportError::permanent("account_export_fragment_invalid"))?;
        fragment
            .validate_for(self.participant.as_str(), export.principal_id)
            .map_err(|_| ExportError::permanent("account_export_fragment_invalid"))?;
        serde_json::to_value(fragment)
            .map_err(|_| ExportError::permanent("account_export_fragment_invalid"))
    }
}

impl ExportError {
    fn transient(code: &'static str) -> Self {
        Self {
            retryable: true,
            code,
        }
    }

    fn permanent(code: &'static str) -> Self {
        Self {
            retryable: false,
            code,
        }
    }

    pub fn local() -> Self {
        Self::transient("account_export_local_fragment_failed")
    }

    fn from_status(status: StatusCode) -> Self {
        if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            return Self::transient("account_export_participant_unavailable");
        }
        if status == StatusCode::UNAUTHORIZED {
            return Self::permanent("account_export_participant_unauthorized");
        }
        Self::permanent("account_export_participant_rejected")
    }

    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }

    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for ExportError {}

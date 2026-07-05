use reqwest::StatusCode;
use serde::Deserialize;

use crate::http::error::AppError;

use super::otp_types::{
    OtpChallengeStatus, OtpCheckRequest, OtpCheckResult, OtpStartRequest, OtpStartResult,
};

pub struct TwilioVerifyOtpProvider {
    account_sid: String,
    auth_token: String,
    service_sid: String,
    api_base_url: String,
}

impl TwilioVerifyOtpProvider {
    pub fn new(
        account_sid: String,
        auth_token: String,
        service_sid: String,
        api_base_url: String,
    ) -> Self {
        Self {
            account_sid,
            auth_token,
            service_sid,
            api_base_url,
        }
    }

    async fn post_form<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        fields: Vec<(String, String)>,
    ) -> Result<T, AppError> {
        let url = format!("{}{}", self.api_base_url.trim_end_matches('/'), path);
        let client = nvbes_core::security::pinned_http_client();
        let request = client
            .post(url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(form_encode(fields));

        let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
            .send()
            .await
            .map_err(|error| {
                AppError::internal(
                    "otp_provider_request_failed",
                    format!("OTP provider request failed: {error}").as_str(),
                )
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::internal(
                "otp_provider_response_failed",
                format!("OTP provider response failed: {error}").as_str(),
            )
        })?;

        if !status.is_success() {
            return Err(twilio_error(status, &body));
        }

        serde_json::from_str(&body).map_err(|error| {
            AppError::internal(
                "otp_provider_response_invalid",
                format!("OTP provider response is not valid JSON: {error}").as_str(),
            )
        })
    }
}

#[async_trait::async_trait]
impl super::otp_provider::OtpProvider for TwilioVerifyOtpProvider {
    async fn start_sms_challenge(
        &self,
        request: &OtpStartRequest,
    ) -> Result<OtpStartResult, AppError> {
        let mut fields = vec![
            ("To".to_string(), request.recipient_phone_e164.clone()),
            ("Channel".to_string(), "sms".to_string()),
        ];
        if let Some(locale) = &request.locale {
            fields.push(("Locale".to_string(), locale.clone()));
        }

        let response: TwilioVerificationResponse = self
            .post_form(
                &format!("/v2/Services/{}/Verifications", self.service_sid),
                fields,
            )
            .await?;

        Ok(OtpStartResult {
            provider_challenge_id: response.sid,
            status: OtpChallengeStatus::from_provider_status(&response.status, None),
        })
    }

    async fn check_sms_challenge(
        &self,
        request: &OtpCheckRequest,
    ) -> Result<OtpCheckResult, AppError> {
        let response: TwilioVerificationCheckResponse = self
            .post_form(
                &format!("/v2/Services/{}/VerificationCheck", self.service_sid),
                vec![
                    ("To".to_string(), request.recipient_phone_e164.clone()),
                    ("Code".to_string(), request.code.clone()),
                ],
            )
            .await?;

        Ok(OtpCheckResult {
            provider_challenge_id: response.sid,
            status: OtpChallengeStatus::from_provider_status(&response.status, response.valid),
        })
    }
}

#[derive(Debug, Deserialize)]
struct TwilioVerificationResponse {
    sid: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct TwilioVerificationCheckResponse {
    sid: Option<String>,
    status: String,
    valid: Option<bool>,
}

fn twilio_error(status: StatusCode, body: &str) -> AppError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("OTP provider returned HTTP {status}."));

    match status {
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            AppError::bad_request("otp_provider_request_rejected", &message)
        }
        StatusCode::TOO_MANY_REQUESTS => {
            AppError::too_many_requests("otp_provider_rate_limited", &message, None, None)
        }
        _ => AppError::conflict("otp_provider_rejected", &message),
    }
}

fn form_encode(fields: Vec<(String, String)>) -> String {
    fields
        .into_iter()
        .map(|(key, value)| format!("{}={}", percent_encode(&key), percent_encode(&value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

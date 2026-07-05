use async_trait::async_trait;
use tracing::info;

use crate::http::error::AppError;

use super::otp_provider::OtpProvider;
use super::otp_types::{
    OtpChallengeStatus, OtpCheckRequest, OtpCheckResult, OtpStartRequest, OtpStartResult,
};

pub struct MockOtpProvider;

impl MockOtpProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OtpProvider for MockOtpProvider {
    async fn start_sms_challenge(
        &self,
        request: &OtpStartRequest,
    ) -> Result<OtpStartResult, AppError> {
        info!(
            phone = %request.recipient_phone_e164,
            "MockOtp: SMS challenge started (not delivered)"
        );

        Ok(OtpStartResult {
            provider_challenge_id: format!("mock-sms-{}", request.recipient_phone_e164),
            status: OtpChallengeStatus::Pending,
        })
    }

    async fn check_sms_challenge(
        &self,
        request: &OtpCheckRequest,
    ) -> Result<OtpCheckResult, AppError> {
        Ok(OtpCheckResult {
            provider_challenge_id: Some(format!("mock-sms-{}", request.recipient_phone_e164)),
            status: if request.code == "000000" {
                OtpChallengeStatus::Approved
            } else {
                OtpChallengeStatus::Failed
            },
        })
    }
}

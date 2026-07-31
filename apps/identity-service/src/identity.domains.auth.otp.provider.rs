use async_trait::async_trait;

use crate::http::error::AppError;

use super::otp_types::{OtpCheckRequest, OtpCheckResult, OtpStartRequest, OtpStartResult};

#[async_trait]
pub trait OtpProvider: Send + Sync {
    async fn start_sms_challenge(
        &self,
        request: &OtpStartRequest,
    ) -> Result<OtpStartResult, AppError>;

    async fn check_sms_challenge(
        &self,
        request: &OtpCheckRequest,
    ) -> Result<OtpCheckResult, AppError>;
}

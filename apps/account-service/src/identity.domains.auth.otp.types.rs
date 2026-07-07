#[derive(Debug, Clone)]
pub struct OtpStartRequest {
    pub recipient_phone_e164: String,
    pub locale: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OtpStartResult {
    pub provider_challenge_id: String,
    pub status: OtpChallengeStatus,
}

#[derive(Debug, Clone)]
pub struct OtpCheckRequest {
    pub recipient_phone_e164: String,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct OtpCheckResult {
    pub provider_challenge_id: Option<String>,
    pub status: OtpChallengeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtpChallengeStatus {
    Pending,
    Approved,
    Canceled,
    Failed,
}

impl OtpChallengeStatus {
    pub fn from_provider_status(status: &str, valid: Option<bool>) -> Self {
        if valid == Some(true) || status == "approved" {
            return Self::Approved;
        }

        match status {
            "pending" => Self::Pending,
            "canceled" => Self::Canceled,
            _ => Self::Failed,
        }
    }
}

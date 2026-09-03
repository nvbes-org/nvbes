use axum::{
    Json,
    http::{HeaderMap, StatusCode, header},
};
use serde_json::json;

pub const DEFAULT_OPERATOR_ROLE: &str = "platform_owner";
pub const MFA_STEP_UP_HEADER: &str = "x-nvbes-mfa-step-up";

#[derive(Debug, Clone)]
pub struct OperatorSession {
    pub operator_id: String,
    pub role: String,
    pub has_mfa_step_up: bool,
}

pub struct OperatorAuthPolicy {
    expected_token: String,
}

impl OperatorAuthPolicy {
    pub fn new(expected_token: impl Into<String>) -> Self {
        Self {
            expected_token: expected_token.into(),
        }
    }

    pub fn authenticate_headers(&self, headers: &HeaderMap) -> Result<OperatorSession, StatusCode> {
        let auth_header = headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let token = &auth_header[7..];
        if token != self.expected_token {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let has_mfa_step_up = headers
            .get(MFA_STEP_UP_HEADER)
            .and_then(|val| val.to_str().ok())
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Ok(OperatorSession {
            operator_id: "solo_operator".to_string(),
            role: DEFAULT_OPERATOR_ROLE.to_string(),
            has_mfa_step_up,
        })
    }
}

pub async fn require_operator_auth(
    auth_policy: &OperatorAuthPolicy,
    headers: &HeaderMap,
) -> Result<OperatorSession, (StatusCode, Json<serde_json::Value>)> {
    auth_policy.authenticate_headers(headers).map_err(|status| {
        (
            status,
            Json(json!({
                "error": "unauthorized",
                "message": "Platform Operations cockpit requires authenticated solo operator access"
            })),
        )
    })
}

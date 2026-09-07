use axum::{
    Json,
    http::{HeaderMap, StatusCode, header},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Deserialize;
use serde_json::json;

pub const DEFAULT_OPERATOR_ROLE: &str = "platform_owner";
pub const MFA_STEP_UP_HEADER: &str = "x-nvbes-mfa-step-up";

#[derive(Debug, Clone)]
pub struct OperatorSession {
    pub operator_id: String,
    pub role: String,
    pub has_mfa_step_up: bool,
}

#[derive(Clone, Copy)]
pub enum Permission {
    ReadCases,
    WriteCases,
    ReadAudit,
    WriteCosts,
}

impl OperatorSession {
    pub fn permits(&self, _permission: Permission) -> bool {
        // Future roles must get an explicit per-capability grant here.
        self.role == DEFAULT_OPERATOR_ROLE
    }
}

#[derive(Clone, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    amr: Vec<String>,
    auth_time: i64,
}

pub struct OperatorAuthPolicy {
    key: DecodingKey,
    validation: Validation,
}

impl OperatorAuthPolicy {
    pub fn from_rsa_pem(
        pem: &[u8],
        issuer: &str,
        audience: &str,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);
        validation.set_required_spec_claims(&["exp", "sub", "iss", "aud", "nbf"]);
        validation.validate_nbf = true;
        validation.leeway = 0;
        Ok(Self {
            key: DecodingKey::from_rsa_pem(pem)?,
            validation,
        })
    }

    #[cfg(test)]
    pub(crate) fn test_policy() -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["platform-operations"]);
        validation.set_issuer(&["test-identity"]);
        validation.leeway = 0;
        Self {
            key: DecodingKey::from_secret(b"test-signing-key-for-platform-operations"),
            validation,
        }
    }

    pub fn authenticate_headers(&self, headers: &HeaderMap) -> Result<OperatorSession, StatusCode> {
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .and_then(|val| val.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let claims = decode::<Claims>(token, &self.key, &self.validation)
            .map_err(|_| StatusCode::UNAUTHORIZED)?
            .claims;
        if claims.sub.trim().is_empty()
            || claims.sub.len() > 128
            || claims.role != DEFAULT_OPERATOR_ROLE
        {
            return Err(StatusCode::FORBIDDEN);
        }
        let age = chrono::Utc::now().timestamp() - claims.auth_time;
        let mfa = claims
            .amr
            .iter()
            .any(|method| method == "mfa" || method == "totp" || method == "webauthn");
        if !mfa {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(OperatorSession {
            operator_id: claims.sub,
            role: claims.role,
            has_mfa_step_up: (0..=300).contains(&age),
        })
    }
}

pub async fn require_operator_auth(
    policy: &OperatorAuthPolicy,
    headers: &HeaderMap,
) -> Result<OperatorSession, (StatusCode, Json<serde_json::Value>)> {
    policy.authenticate_headers(headers).map_err(|status| {
        (
            status,
            Json(json!({"error": "operator_authentication_required"})),
        )
    })
}

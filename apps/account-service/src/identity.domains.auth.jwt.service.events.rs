use chrono::Utc;
use uuid::Uuid;

use super::types::{JwtService, LogoutTokenClaims, SecurityEventTokenClaims};
use crate::http::error::AppError;

impl JwtService {
    pub async fn generate_logout_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        audience: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        self.encode_typed_claims(
            &LogoutTokenClaims {
                iss: self.issuer.clone(),
                sub: user_id.to_string(),
                aud: audience.to_string(),
                iat: now.timestamp(),
                exp: (now + chrono::Duration::minutes(2)).timestamp(),
                jti: Uuid::new_v4().to_string(),
                sid: session_id.to_string(),
                events: serde_json::json!({
                    "http://schemas.openid.net/event/backchannel-logout": {}
                }),
            },
            Some("logout+jwt"),
        )
        .await
    }

    pub async fn generate_caep_session_revoked_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        audience: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        self.encode_typed_claims(
            &SecurityEventTokenClaims {
                iss: self.issuer.clone(),
                aud: audience.to_string(),
                iat: now.timestamp(),
                jti: Uuid::new_v4().to_string(),
                sub_id: Some(serde_json::json!({
                    "format": "complex",
                    "user": {
                        "format": "iss_sub",
                        "iss": self.issuer.clone(),
                        "sub": user_id,
                    },
                    "session": {
                        "format": "opaque",
                        "id": session_id,
                    }
                })),
                events: serde_json::json!({
                    "https://schemas.openid.net/secevent/caep/event-type/session-revoked": {
                        "event_timestamp": now.timestamp(),
                    }
                }),
            },
            Some("secevent+jwt"),
        )
        .await
    }

    pub async fn generate_risc_credential_compromise_token(
        &self,
        user_id: Uuid,
        audience: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        self.encode_typed_claims(
            &SecurityEventTokenClaims {
                iss: self.issuer.clone(),
                aud: audience.to_string(),
                iat: now.timestamp(),
                jti: Uuid::new_v4().to_string(),
                sub_id: None,
                events: serde_json::json!({
                    "https://schemas.openid.net/secevent/risc/event-type/credential-compromise": {
                        "subject": {
                            "format": "iss_sub",
                            "iss": self.issuer.clone(),
                            "sub": user_id,
                        },
                        "credential_type": "refresh_token",
                        "event_timestamp": now.timestamp(),
                    }
                }),
            },
            Some("secevent+jwt"),
        )
        .await
    }
}

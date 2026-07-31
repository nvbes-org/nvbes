use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;

use crate::http::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OAuthSecurityProfile {
    Standard,
    HighAssurance,
}

impl OAuthSecurityProfile {
    pub fn parse(value: Option<&str>) -> Result<Self, AppError> {
        match value.unwrap_or("standard") {
            "standard" => Ok(Self::Standard),
            "high_assurance" => Ok(Self::HighAssurance),
            _ => Err(AppError::bad_request(
                "invalid_security_profile",
                "security_profile must be standard or high_assurance.",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::HighAssurance => "high_assurance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OAuthSenderConstraint {
    Dpop,
    Mtls,
}

impl OAuthSenderConstraint {
    pub fn parse(value: Option<&str>) -> Result<Option<Self>, AppError> {
        value
            .map(|value| match value {
                "dpop" => Ok(Self::Dpop),
                "mtls" => Ok(Self::Mtls),
                _ => Err(AppError::bad_request(
                    "invalid_sender_constraint",
                    "sender_constraint must be dpop or mtls.",
                )),
            })
            .transpose()
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dpop => "dpop",
            Self::Mtls => "mtls",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OAuthClientSecurity {
    pub tenant_id: uuid::Uuid,
    pub client_type: String,
    pub profile: OAuthSecurityProfile,
    pub request_object_signing_jwks: Option<serde_json::Value>,
    pub sender_constraint: Option<OAuthSenderConstraint>,
    pub tls_client_certificate_sha256: Option<String>,
}

pub async fn load_client_security(
    db: &PgPool,
    client_id: &str,
) -> Result<OAuthClientSecurity, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          tenant_id,
          client_type::text AS client_type,
          security_profile::text AS security_profile,
          request_object_signing_jwks,
          sender_constraint::text AS sender_constraint,
          tls_client_certificate_sha256
        FROM oauth_clients
        WHERE client_id = $1
          AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::unauthorized("invalid_client", "The OAuth client is invalid."))?;

    let tenant_id = row.get("tenant_id");
    let request_object_signing_jwks = if row
        .get::<Option<serde_json::Value>, _>("request_object_signing_jwks")
        .is_some()
    {
        Some(
            crate::domains::oauth::clients::keys::active_jwks(
                db,
                tenant_id,
                client_id,
                crate::domains::oauth::clients::keys::OAuthClientKeyPurpose::RequestObject,
            )
            .await?,
        )
    } else {
        None
    };

    Ok(OAuthClientSecurity {
        tenant_id,
        client_type: row.get("client_type"),
        profile: OAuthSecurityProfile::parse(Some(
            row.get::<String, _>("security_profile").as_str(),
        ))?,
        request_object_signing_jwks,
        sender_constraint: OAuthSenderConstraint::parse(
            row.get::<Option<String>, _>("sender_constraint").as_deref(),
        )?,
        tls_client_certificate_sha256: row.get("tls_client_certificate_sha256"),
    })
}

pub fn validate_high_assurance_configuration(
    profile: OAuthSecurityProfile,
    client_type: &str,
    client_assertion_required: bool,
    client_assertion_jwk: Option<&serde_json::Value>,
    request_object_jwks: Option<&serde_json::Value>,
    sender_constraint: Option<OAuthSenderConstraint>,
    tls_thumbprint: Option<&str>,
) -> Result<(), AppError> {
    if profile == OAuthSecurityProfile::Standard {
        return Ok(());
    }

    if client_type == "public" {
        return Err(configuration_error(
            "High-assurance OAuth clients must be confidential.",
        ));
    }
    if !client_assertion_required
        || client_assertion_jwk.is_none_or(|jwk| {
            !crate::domains::oauth::client_assertion::is_high_assurance_client_assertion_jwk(jwk)
        })
    {
        return Err(configuration_error(
            "High-assurance OAuth clients require private_key_jwt with PS256 or ES256.",
        ));
    }
    let request_object_jwks = request_object_jwks.ok_or_else(|| {
        configuration_error("High-assurance OAuth clients require request-object signing JWKS.")
    })?;
    crate::domains::oauth::jar::validate_high_assurance_jwks(request_object_jwks)?;
    let sender_constraint = sender_constraint
        .ok_or_else(|| configuration_error("High-assurance OAuth clients require DPoP or mTLS."))?;
    if sender_constraint == OAuthSenderConstraint::Mtls {
        validate_certificate_thumbprint(tls_thumbprint)?;
    }

    Ok(())
}

pub fn validate_certificate_thumbprint(value: Option<&str>) -> Result<(), AppError> {
    let value = value.unwrap_or_default();
    let valid = value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(configuration_error(
            "tls_client_certificate_sha256 must be an unpadded base64url SHA-256 thumbprint.",
        ))
    }
}

fn configuration_error(message: &'static str) -> AppError {
    AppError::bad_request("invalid_high_assurance_configuration", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtls_thumbprints_use_unpadded_sha256_base64url() {
        assert!(validate_certificate_thumbprint(Some(&"a".repeat(43))).is_ok());
        assert!(validate_certificate_thumbprint(Some("not-a-thumbprint")).is_err());
    }
}

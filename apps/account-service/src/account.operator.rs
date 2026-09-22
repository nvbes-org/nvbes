use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, header},
    routing::get,
};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{app::AccountState, audit, error::AccountError};

const OPERATOR_AUDIENCE: &str = "platform-operations";
const OPERATOR_ROLE: &str = "platform_owner";

#[derive(Debug, Deserialize)]
struct OperatorClaims {
    sub: String,
    token_type: String,
    role: String,
    amr: Vec<String>,
    auth_time: i64,
}

#[derive(Debug)]
struct OperatorPrincipal {
    id: Uuid,
}

#[derive(Debug, Serialize)]
struct OperatorProfileResponse {
    principal_id: Uuid,
    username: Option<String>,
    region: Option<String>,
    lifecycle_status: String,
    created_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
    redacted: bool,
}

pub fn router(state: AccountState) -> Router {
    Router::new()
        .route(
            "/api/v1/operator/profiles/{principal_id}",
            get(get_operator_profile),
        )
        .with_state(state)
}

async fn get_operator_profile(
    State(state): State<AccountState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
) -> Result<Json<OperatorProfileResponse>, AccountError> {
    let operator = require_operator(&state, &headers)?;
    let row: Option<(
        Option<String>,
        Option<String>,
        String,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
    )> = sqlx::query_as(
        "SELECT username, region, lifecycle_status, created_at, closed_at
         FROM account_profiles WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_optional(&state.db)
    .await?;
    let Some((username, region, lifecycle_status, created_at, closed_at)) = row else {
        return Err(AccountError::NotFound);
    };

    let mut tx = state.db.begin().await?;
    audit::record(
        &mut tx,
        audit::AuditInput {
            principal_id,
            actor_principal_id: operator.id,
            event_type: "account.operator.profile_read",
            resource_type: "profile",
            resource_id: Some(principal_id),
            correlation_id: audit::correlation_id(&headers),
            details: serde_json::json!({"redacted": true}),
        },
    )
    .await?;
    tx.commit().await?;

    Ok(Json(OperatorProfileResponse {
        principal_id,
        username,
        region,
        lifecycle_status,
        created_at,
        closed_at,
        redacted: true,
    }))
}

fn require_operator(
    state: &AccountState,
    headers: &HeaderMap,
) -> Result<OperatorPrincipal, AccountError> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AccountError::Unauthorized)?;
    let header = decode_header(token).map_err(|_| AccountError::Unauthorized)?;
    if header.alg != Algorithm::RS256
        || header.kid.as_deref() != Some(&state.config.token_key_id)
        || header.typ.as_deref() != Some("operator+jwt")
    {
        return Err(AccountError::Unauthorized);
    }
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[&state.config.token_issuer]);
    validation.set_audience(&[OPERATOR_AUDIENCE]);
    validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub", "nbf"]);
    validation.validate_nbf = true;
    validation.leeway = 0;
    let key = DecodingKey::from_rsa_pem(state.config.token_public_key_pem.as_bytes())
        .map_err(|_| AccountError::Unauthorized)?;
    let claims = decode::<OperatorClaims>(token, &key, &validation)
        .map_err(|_| AccountError::Unauthorized)?
        .claims;
    let mfa = claims
        .amr
        .iter()
        .any(|method| matches!(method.as_str(), "mfa" | "totp" | "webauthn"));
    if claims.token_type != "operator" || claims.role != OPERATOR_ROLE || !mfa {
        return Err(AccountError::Forbidden);
    }
    let age = chrono::Utc::now().timestamp() - claims.auth_time;
    if !(0..=900).contains(&age) {
        return Err(AccountError::Forbidden);
    }
    Ok(OperatorPrincipal {
        id: Uuid::parse_str(&claims.sub).map_err(|_| AccountError::Unauthorized)?,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn operator_audience_is_platform_operations() {
        assert_eq!(super::OPERATOR_AUDIENCE, "platform-operations");
        assert_eq!(super::OPERATOR_ROLE, "platform_owner");
    }
}

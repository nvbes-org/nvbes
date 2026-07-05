use axum::{Extension, Json, extract::State};
use chrono::{DateTime, TimeZone, Utc};
use nvbes_core::auth::token_hash;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::DeveloperPermission,
        service,
        types::{DebugDeveloperTokenInput, DebugDeveloperTokenResponse, DeveloperTokenClaimsView},
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenAccessDecision {
    active: bool,
    access_decision: &'static str,
}

fn token_access_decision(
    now: i64,
    token_tenant_id: Option<&str>,
    expected_tenant_id: Uuid,
    not_before: i64,
    expires_at: i64,
) -> TokenAccessDecision {
    let active = now >= not_before && now < expires_at;
    let expected_tenant_id = expected_tenant_id.to_string();
    let access_decision = if token_tenant_id != Some(expected_tenant_id.as_str()) {
        "tenant_mismatch"
    } else if !active {
        "expired"
    } else {
        "allowed"
    };

    TokenAccessDecision {
        active,
        access_decision,
    }
}

pub async fn debug_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<DebugDeveloperTokenInput>,
) -> Result<Json<DebugDeveloperTokenResponse>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::ConsoleTokensInspect)
            .await?;
    let token = input.access_token.trim();
    if token.is_empty() {
        return Err(AppError::bad_request(
            "access_token_required",
            "An access token is required.",
        ));
    }

    let token_hash_prefix = token_hash(token).chars().take(16).collect::<String>();
    let decoded = state.jwt.decode_token_ignore_expiry(token);
    let (active, access_decision, claims) = match decoded {
        Ok(claims) => {
            let decision = token_access_decision(
                Utc::now().timestamp(),
                claims.tenant_id.as_deref(),
                tenant_id,
                claims.nbf,
                claims.exp,
            );

            (
                decision.active,
                decision.access_decision.to_string(),
                Some(claims_to_view(claims)?),
            )
        }
        Err(_) => (false, "invalid".to_string(), None),
    };

    sqlx::query(
        r#"
        INSERT INTO developer_token_debug_sessions (
          tenant_id,
          actor_principal_id,
          token_hash_prefix,
          active,
          access_decision
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(tenant_id)
    .bind(auth.user_id)
    .bind(&token_hash_prefix)
    .bind(active)
    .bind(&access_decision)
    .execute(&state.db)
    .await?;

    Ok(Json(DebugDeveloperTokenResponse {
        active,
        access_decision,
        claims,
        token_hash_prefix,
    }))
}

fn claims_to_view(
    claims: crate::domains::auth::jwt::TokenClaims,
) -> Result<DeveloperTokenClaimsView, AppError> {
    Ok(DeveloperTokenClaimsView {
        subject: claims.sub,
        tenant_id: claims.tenant_id,
        workspace_id: claims.workspace_id,
        client_id: claims.client_id,
        scopes: claims
            .scope
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        audience: claims.aud,
        issuer: claims.iss,
        expires_at: timestamp_to_datetime(claims.exp)?,
        issued_at: timestamp_to_datetime(claims.iat)?,
        not_before: timestamp_to_datetime(claims.nbf)?,
        token_type: claims.token_type,
        amr: claims.amr,
        acr: claims.acr,
    })
}

fn timestamp_to_datetime(timestamp: i64) -> Result<DateTime<Utc>, AppError> {
    Utc.timestamp_opt(timestamp, 0).single().ok_or_else(|| {
        AppError::bad_request(
            "invalid_token_timestamp",
            "Token timestamp is out of range.",
        )
    })
}

#[cfg(test)]
#[path = "identity.domains.developer.routes.tokens.tests.rs"]
mod tests;

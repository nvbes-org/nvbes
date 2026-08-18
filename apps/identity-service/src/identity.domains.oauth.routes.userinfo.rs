use crate::domains::auth::sessions;
use crate::{app::AppState, http::error::AppError};
use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use nvbes_core::http::error::ErrorEnvelope;
use std::time::Duration;

pub fn router() -> Router<AppState> {
    Router::new().route("/userinfo", get(userinfo))
}

#[utoipa::path(
    get,
    path = "/oauth/userinfo",
    tag = "oauth",
    responses(
        (status = 200, description = "User info claims"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn userinfo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let auth = sessions::authenticate_bearer(&state.db, &state.redis, &state.jwt, &headers).await?;
    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "oauth_userinfo",
        &format!("session:{}", auth.session_id),
        120,
        120,
        Duration::from_secs(60),
    )
    .await?;
    let assurance = crate::domains::oauth::assurance::resolve_assurance_context(
        &state.db,
        &state.redis,
        auth.user_id,
        Some(auth.session_id),
        auth.client_id.as_deref(),
        auth.tenant_id,
        auth.organization_id,
        auth.workspace_id,
    )
    .await?;
    if !assurance.sufficient {
        return Err(AppError::forbidden(
            "assurance_level_insufficient",
            "The current session assurance level is insufficient for this context.",
        ));
    }

    let mut claims = serde_json::Map::new();
    claims.insert("sub".to_string(), serde_json::json!(auth.user_id));
    if has_scope(&auth.scope, "email") {
        claims.insert("email".to_string(), serde_json::json!(auth.user_email));
        claims.insert(
            "email_verified".to_string(),
            serde_json::json!(auth.email_verified_at.is_some()),
        );
    }
    if has_scope(&auth.scope, "profile")
        && let Some(profile) =
            crate::domains::auth::oidc_profile_projection::fetch_claims(&state.db, auth.user_id)
                .await?
    {
        claims.insert("name".to_string(), serde_json::json!(profile.display_name));
        insert_optional(&mut claims, "given_name", profile.given_name);
        insert_optional(&mut claims, "family_name", profile.family_name);
        insert_optional(
            &mut claims,
            "preferred_username",
            profile.preferred_username,
        );
        insert_optional(
            &mut claims,
            "birthdate",
            profile.birthdate.map(|date| date.to_string()),
        );
        claims.insert(
            "updated_at".to_string(),
            serde_json::json!(profile.projected_at.timestamp()),
        );
    }
    Ok(Json(serde_json::Value::Object(claims)))
}

fn has_scope(granted: &str, required: &str) -> bool {
    granted.split_whitespace().any(|scope| scope == required)
}

fn insert_optional(
    claims: &mut serde_json::Map<String, serde_json::Value>,
    name: &'static str,
    value: Option<String>,
) {
    if let Some(value) = value {
        claims.insert(name.to_string(), serde_json::Value::String(value));
    }
}

#[cfg(test)]
mod tests {
    use super::has_scope;

    #[test]
    fn userinfo_claim_scopes_are_exact_tokens() {
        assert!(has_scope("openid email profile", "email"));
        assert!(!has_scope("openid account:email", "email"));
        assert!(!has_scope("openid profile:read", "profile"));
    }
}

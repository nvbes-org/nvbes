use crate::{app::AppState, http::error::AppError};
use axum::extract::Query;
use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use sqlx::Row;
use std::time::Duration;

#[path = "identity.domains.oauth.routes.authorize.params.rs"]
mod params;

use params::build_params_from_map;

pub fn router() -> Router<AppState> {
    Router::new().route("/authorize", get(authorize))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub request_uri: Option<String>,
    #[serde(default)]
    pub authuser: Option<String>,
}

struct AuthorizationSubject {
    auth: crate::domains::auth::types::AuthContext,
    workspace_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
}

#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tag = "oauth",
    params(
        ("response_type" = String, Query, description = "Response type"),
        ("client_id" = String, Query, description = "OAuth client ID"),
        ("request_uri" = Option<String>, Query, description = "One-time PAR request_uri (RFC 9126)"),
        ("authuser" = Option<String>, Query, description = "Auth user index"),
    ),
    responses(
        (status = 200, description = "Authorization code issued"),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Client not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(request): Query<AuthorizeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if request.response_type != "code" {
        return Err(AppError::bad_request(
            "invalid_response_type",
            "Only 'code' is supported",
        ));
    }

    let request_uri = request.request_uri.as_ref().ok_or_else(|| {
        AppError::bad_request(
            "pushed_authorization_request_required",
            "Pushed authorization requests (PAR) are required. Use POST /oauth/par first and pass the request_uri.",
        )
    })?;

    if !crate::domains::oauth::jar::is_par_urn(request_uri) {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            "Only PAR request URIs are accepted.",
        ));
    }

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "oauth_authorize",
        &request.client_id,
        30,
        20,
        Duration::from_secs(60),
    )
    .await?;

    let client_row = sqlx::query(
        r#"
        SELECT client_secret_hash, client_type::text AS client_type, redirect_uris, revoked_at
        FROM oauth_clients
        WHERE client_id = $1
        LIMIT 1
        "#,
    )
    .bind(&request.client_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("client_not_found", "The OAuth client was not found."))?;

    if client_row
        .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
        .is_some()
    {
        return Err(AppError::not_found(
            "client_not_found",
            "The OAuth client was not found.",
        ));
    }

    let redirect_uris: Vec<String> = client_row.get("redirect_uris");
    let client_type: String = client_row.get("client_type");

    let params =
        super::par::resolve_pushed_parameters(&state.redis, request_uri, &request.client_id)
            .await?;
    let resolved = build_params_from_map(&params)?;

    if resolved.redirect_uri.is_empty() {
        return Err(AppError::bad_request(
            "missing_redirect_uri",
            "The redirect_uri parameter is required.",
        ));
    }

    crate::domains::oauth::validation::validate_pkce_for_authorize(
        &client_type,
        resolved.code_challenge.as_deref(),
        resolved.code_challenge_method.as_deref(),
    )?;
    crate::domains::oauth::validation::validate_redirect_uri_allowed(
        &redirect_uris,
        &resolved.redirect_uri,
    )?;

    let subject = match authenticate_authorization_subject(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        request.authuser.as_deref(),
    )
    .await
    {
        Ok(subject) => subject,
        Err(err) if err.status == axum::http::StatusCode::UNAUTHORIZED => {
            let hosted = crate::domains::oauth::hosted_service::create_hosted_authorization_state(
                &state.redis,
                crate::domains::oauth::hosted_service::StartHostedAuthorizationInput {
                    client_id: request.client_id.clone(),
                    redirect_uri: resolved.redirect_uri.clone(),
                    scope: resolved.scope.clone(),
                    state: resolved.state.clone(),
                    nonce: resolved.nonce.clone(),
                    request_uri: Some(request_uri.clone()),
                    code_challenge: resolved.code_challenge.clone(),
                    code_challenge_method: resolved.code_challenge_method.clone(),
                },
            )
            .await?;

            return Ok(Json(serde_json::json!({
                "kind": "login_required",
                "login_url": crate::domains::oauth::hosted_service::build_hosted_login_url(
                    &state.config.web_base_url,
                    &hosted.state_id,
                ),
                "state_id": hosted.state_id,
            })));
        }
        Err(err) => return Err(err),
    };

    let code = crate::domains::oauth::flows::create_authorization_code(
        &state.db,
        &state.redis,
        subject.auth.user_id,
        subject.auth.session_id,
        crate::domains::oauth::service::CreateAuthorizationCodeInput {
            client_id: request.client_id.clone(),
            user_id: subject.auth.user_id,
            session_id: Some(subject.auth.session_id),
            workspace_id: Some(subject.workspace_id),
            tenant_id: Some(subject.tenant_id),
            organization_id: subject.auth.organization_id,
            scope: resolved.scope.unwrap_or_default(),
            redirect_uri: resolved.redirect_uri.clone(),
            nonce: resolved.nonce,
            audience: resolved.audience,
            resource_indicators: resolved.resource.unwrap_or_default(),
            authorization_details: resolved.authorization_details,
            code_challenge: resolved.code_challenge,
            code_challenge_method: resolved.code_challenge_method,
            consent_action: resolved.consent_action,
            dpop_jkt: resolved.dpop_jkt,
        },
    )
    .await?;

    super::par::mark_par_used(&state.redis, request_uri).await?;

    Ok(Json(serde_json::json!({
        "code": code.code,
        "redirect_uri": resolved.redirect_uri,
        "state": resolved.state,
        "expires_at": code.expires_at,
    })))
}

async fn authenticate_authorization_subject(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &crate::domains::auth::jwt::JwtService,
    headers: &HeaderMap,
    authuser: Option<&str>,
) -> Result<AuthorizationSubject, AppError> {
    let authuser = authuser.unwrap_or("0");
    let auth = if let Some(token) = crate::http::request::authorization_bearer_token(headers)? {
        crate::domains::auth::sessions::authenticate(db, redis, jwt, &token).await?
    } else {
        let cookie = crate::http::request::browser_session_token_with_authuser(headers, authuser)?;
        crate::domains::auth::sessions::authenticate_browser_session(db, redis, &cookie, headers)
            .await?
    };
    let workspace_id = auth.workspace_id.ok_or_else(|| {
        AppError::forbidden(
            "workspace_context_required",
            "Switch to a workspace before starting an authorization flow.",
        )
    })?;
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before starting an authorization flow.",
        )
    })?;

    Ok(AuthorizationSubject {
        auth,
        workspace_id,
        tenant_id,
    })
}

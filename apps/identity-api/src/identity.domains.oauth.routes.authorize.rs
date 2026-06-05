use crate::{app::AppState, http::error::AppError};
use axum::extract::Query;
use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use sqlx::Row;

pub fn router() -> Router<AppState> {
    Router::new().route("/authorize", get(authorize))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub audience: Option<String>,
    pub resource: Option<Vec<String>>,
    pub authorization_details: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub consent_action: Option<String>,
    pub request_uri: Option<String>,
    pub request: Option<String>,
    pub client_secret: Option<String>,
    #[serde(default)]
    pub authuser: Option<String>,
}

#[derive(Default)]
struct ResolvedParams {
    redirect_uri: String,
    scope: Option<String>,
    state: Option<String>,
    audience: Option<String>,
    resource: Option<Vec<String>>,
    authorization_details: crate::domains::oauth::rar::AuthorizationDetails,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    consent_action: Option<String>,
}

#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tag = "oauth",
    params(
        ("response_type" = String, Query, description = "Response type"),
        ("client_id" = String, Query, description = "OAuth client ID"),
        ("redirect_uri" = Option<String>, Query, description = "Redirect URI"),
        ("scope" = Option<String>, Query, description = "Requested scopes"),
        ("state" = Option<String>, Query, description = "State parameter"),
        ("audience" = Option<String>, Query, description = "Audience"),
        ("code_challenge" = Option<String>, Query, description = "PKCE code challenge"),
        ("code_challenge_method" = Option<String>, Query, description = "PKCE code challenge method"),
        ("request_uri" = Option<String>, Query, description = "PAR request_uri (RFC 9126) or JAR request_uri (RFC 9101)"),
        ("request" = Option<String>, Query, description = "JAR request object JWT (RFC 9101)"),
        ("client_secret" = Option<String>, Query, description = "Client secret (required for JAR)"),
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

    super::enforce_public_oauth_rate_limit_db(
        &state.redis,
        &headers,
        "oauth_authorize",
        &request.client_id,
        30,
        20,
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

    let authuser = request.authuser.as_deref().unwrap_or("0");
    let token = crate::http::request::bearer_token_with_authuser(&headers, authuser)?;
    let auth =
        crate::domains::auth::sessions::authenticate(&state.db, &state.redis, &state.jwt, &token)
            .await?;
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

    let code = crate::domains::oauth::flows::create_authorization_code(
        &state,
        auth.user_id,
        auth.session_id,
        crate::domains::oauth::service::CreateAuthorizationCodeInput {
            client_id: request.client_id.clone(),
            user_id: auth.user_id,
            session_id: Some(auth.session_id),
            workspace_id: Some(workspace_id),
            tenant_id: Some(tenant_id),
            organization_id: auth.organization_id,
            scope: resolved.scope.unwrap_or_default(),
            redirect_uri: resolved.redirect_uri.clone(),
            audience: resolved.audience,
            resource_indicators: resolved.resource.unwrap_or_default(),
            authorization_details: resolved.authorization_details,
            code_challenge: resolved.code_challenge,
            code_challenge_method: resolved.code_challenge_method,
            consent_action: resolved.consent_action,
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

fn build_params_from_map(
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<ResolvedParams, AppError> {
    let redirect_uri = params
        .get("redirect_uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::bad_request(
                "invalid_request",
                "The pushed authorization request is missing redirect_uri.",
            )
        })?
        .to_string();

    Ok(ResolvedParams {
        redirect_uri,
        scope: params
            .get("scope")
            .and_then(|v| v.as_str())
            .map(String::from),
        state: params
            .get("state")
            .and_then(|v| v.as_str())
            .map(String::from),
        audience: params
            .get("audience")
            .and_then(|v| v.as_str())
            .map(String::from),
        resource: params.get("resource").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        }),
        authorization_details: crate::domains::oauth::rar::parse_authorization_details_value(
            params.get("authorization_details"),
        )?,
        code_challenge: params
            .get("code_challenge")
            .and_then(|v| v.as_str())
            .map(String::from),
        code_challenge_method: params
            .get("code_challenge_method")
            .and_then(|v| v.as_str())
            .map(String::from),
        consent_action: params
            .get("consent_action")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

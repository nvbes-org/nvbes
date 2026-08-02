use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{app::AppState, auth::AuthenticatedPrincipal, error::AppError};

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    limit: Option<u16>,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SessionsPage {
    pub sessions: Vec<SessionView>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SessionView {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub workspace_region: Option<String>,
    pub created_at: String,
    pub last_seen_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub ip: Option<String>,
    pub geo_country_code: Option<String>,
    pub user_agent: Option<String>,
    pub client: Option<SessionClient>,
    pub device_id: Option<Uuid>,
    pub device_trust_level: Option<String>,
    pub device_trust_score: Option<i32>,
    pub risk_score: Option<f64>,
    pub risk_decision: Option<String>,
    pub risk_confirmed_at: Option<String>,
    pub current: bool,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SessionClient {
    pub browser: Option<String>,
    pub browser_version: Option<f64>,
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub device: Option<String>,
    pub device_type: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SessionMutationResult {
    success: bool,
}

#[utoipa::path(
    get,
    path = "/api/v1/security/sessions",
    tag = "security",
    params(
        ("limit" = Option<u16>, Query, description = "Maximum sessions to return"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor"),
    ),
    responses((status = 200, body = SessionsPage)),
    security(("identityOAuth2" = ["account:session:read"]))
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<SessionsPage>, AppError> {
    let mut url = identity_url(&state, "/auth/sessions")?;
    {
        let mut params = url.query_pairs_mut();
        if let Some(limit) = query.limit {
            params.append_pair("limit", &limit.to_string());
        }
        if let Some(cursor) = query.cursor.as_deref() {
            params.append_pair("cursor", cursor);
        }
    }
    let response = state
        .identity_http
        .get(url)
        .bearer_auth(auth.access_token)
        .send()
        .await
        .map_err(identity_unavailable)?;
    Ok(Json(identity_json(response, "account:session:read").await?))
}

#[utoipa::path(
    delete,
    path = "/api/v1/security/sessions/{sessionId}",
    tag = "security",
    params(("sessionId" = Uuid, Path, description = "Identity session ID")),
    responses((status = 200, body = SessionMutationResult)),
    security(("identityOAuth2" = ["account:session:write"]))
)]
pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionMutationResult>, AppError> {
    let url = identity_url(&state, &format!("/auth/sessions/{session_id}"))?;
    let response = state
        .identity_http
        .delete(url)
        .bearer_auth(auth.access_token)
        .send()
        .await
        .map_err(identity_unavailable)?;
    ensure_identity_success(response, "account:session:write").await?;
    Ok(Json(SessionMutationResult { success: true }))
}

fn identity_url(state: &AppState, path: &str) -> Result<url::Url, AppError> {
    let base = url::Url::parse(&state.config.identity_service_base_url).map_err(|error| {
        tracing::error!(%error, "validated Identity base URL became invalid");
        AppError::internal("identity_url_invalid", "Identity configuration is invalid.")
    })?;
    base.join(path).map_err(|error| {
        tracing::error!(%error, "Identity session URL could not be built");
        AppError::internal("identity_url_invalid", "Identity configuration is invalid.")
    })
}

async fn identity_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    required_scope: &'static str,
) -> Result<T, AppError> {
    let response = ensure_identity_success(response, required_scope).await?;
    response.json().await.map_err(|error| {
        tracing::error!(%error, "Identity returned an invalid session response");
        AppError::service_unavailable(
            "identity_contract_invalid",
            "Identity returned an invalid response.",
        )
    })
}

async fn ensure_identity_success(
    response: reqwest::Response,
    required_scope: &'static str,
) -> Result<reqwest::Response, AppError> {
    match response.status() {
        status if status.is_success() => Ok(response),
        reqwest::StatusCode::UNAUTHORIZED => Err(AppError::unauthorized(
            "invalid_token",
            "Identity rejected the access token.",
        )),
        reqwest::StatusCode::FORBIDDEN => Err(AppError::insufficient_scope(required_scope)),
        reqwest::StatusCode::NOT_FOUND => Err(AppError::not_found(
            "session_not_found",
            "The session does not exist.",
        )),
        status => {
            tracing::error!(%status, "Identity session operation failed");
            Err(AppError::service_unavailable(
                "identity_sessions_unavailable",
                "Session management is temporarily unavailable.",
            ))
        }
    }
}

fn identity_unavailable(error: reqwest::Error) -> AppError {
    tracing::error!(%error, "Identity session request failed");
    AppError::service_unavailable(
        "identity_sessions_unavailable",
        "Session management is temporarily unavailable.",
    )
}

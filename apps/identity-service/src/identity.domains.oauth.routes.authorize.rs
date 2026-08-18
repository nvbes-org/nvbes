use crate::{app::AppState, http::error::AppError};
use axum::extract::Query;
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use sqlx::Row;
use std::time::Duration;

#[path = "identity.domains.oauth.routes.authorize.params.rs"]
mod params;
#[path = "identity.domains.oauth.routes.authorize.response.rs"]
mod response;
#[cfg(test)]
#[path = "identity.domains.oauth.routes.authorize.tests.rs"]
mod tests;

use params::build_params_from_map;
use response::{oauth_error_redirect, oauth_success_redirect};

pub fn router() -> Router<AppState> {
    Router::new().route("/authorize", get(authorize))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorizeRequest {
    pub response_type: Option<String>,
    pub client_id: String,
    pub request_uri: Option<String>,
    #[serde(default)]
    pub authuser: Option<String>,
}

#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tag = "oauth",
    params(
        ("response_type" = Option<String>, Query, description = "Legacy response type; resolved from the pushed authorization request"),
        ("client_id" = String, Query, description = "OAuth client ID"),
        ("request_uri" = Option<String>, Query, description = "One-time PAR request_uri (RFC 9126)"),
        ("authuser" = Option<String>, Query, description = "Auth user index"),
    ),
    responses(
        (status = 303, description = "Redirect to Identity login or the registered client callback"),
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
) -> Result<Response, AppError> {
    if request
        .response_type
        .as_deref()
        .is_some_and(|response_type| response_type != "code")
    {
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

    crate::domains::oauth::validation::validate_redirect_uri_allowed(
        &redirect_uris,
        &resolved.redirect_uri,
    )?;
    if let Err(error) = validate_resolved_authorization_target(&client_type, &request, &resolved) {
        return oauth_error_redirect(&resolved.redirect_uri, resolved.state.as_deref(), &error);
    }
    if let Err(error) =
        super::par::consume_pushed_parameters(&state.redis, request_uri, &request.client_id).await
    {
        return oauth_error_redirect(&resolved.redirect_uri, resolved.state.as_deref(), &error);
    }

    let subject =
        match crate::domains::oauth::authorization_subject::authenticate_authorization_subject(
            &state.db,
            &state.redis,
            &state.jwt,
            &headers,
            request.authuser.as_deref(),
            &request.client_id,
        )
        .await
        {
            Ok(subject) => subject,
            Err(error) if error.status == StatusCode::UNAUTHORIZED => {
                return redirect_to_hosted_login(&state, &request, &resolved, request_uri).await;
            }
            Err(error) => {
                return oauth_error_redirect(
                    &resolved.redirect_uri,
                    resolved.state.as_deref(),
                    &error,
                );
            }
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
            workspace_id: subject.workspace_id,
            tenant_id: subject.tenant_id,
            organization_id: subject.auth.organization_id,
            scope: resolved.scope.clone().unwrap_or_default(),
            redirect_uri: resolved.redirect_uri.clone(),
            nonce: resolved.nonce.clone(),
            audience: resolved.audience.clone(),
            resource_indicators: resolved.resource.clone().unwrap_or_default(),
            authorization_details: resolved.authorization_details.clone(),
            code_challenge: resolved.code_challenge.clone(),
            code_challenge_method: resolved.code_challenge_method.clone(),
            consent_action: None,
            dpop_jkt: resolved.dpop_jkt.clone(),
        },
    )
    .await;

    match code {
        Ok(code) => oauth_success_redirect(
            &resolved.redirect_uri,
            &code.code,
            resolved.state.as_deref(),
        ),
        Err(error) if requires_hosted_consent(&error) => {
            redirect_to_hosted_login(&state, &request, &resolved, request_uri).await
        }
        Err(error) => {
            oauth_error_redirect(&resolved.redirect_uri, resolved.state.as_deref(), &error)
        }
    }
}

fn validate_resolved_authorization_target(
    client_type: &str,
    request: &AuthorizeRequest,
    resolved: &params::ResolvedParams,
) -> Result<(), AppError> {
    crate::domains::oauth::validation::validate_pkce_for_authorize(
        client_type,
        resolved.code_challenge.as_deref(),
        resolved.code_challenge_method.as_deref(),
    )?;
    let audience = crate::domains::oauth::validation::resolve_access_token_audience(
        resolved.audience.as_deref(),
        resolved.resource.as_deref().unwrap_or_default(),
    )?;
    crate::domains::oauth::system_clients::validate_system_client_audience(
        &request.client_id,
        &audience,
    )
}

async fn redirect_to_hosted_login(
    state: &AppState,
    request: &AuthorizeRequest,
    resolved: &params::ResolvedParams,
    request_uri: &str,
) -> Result<Response, AppError> {
    let hosted = crate::domains::oauth::hosted_service::create_hosted_authorization_state(
        &state.redis,
        crate::domains::oauth::hosted_service::StartHostedAuthorizationInput {
            client_id: request.client_id.clone(),
            redirect_uri: resolved.redirect_uri.clone(),
            scope: resolved.scope.clone(),
            state: resolved.state.clone(),
            nonce: resolved.nonce.clone(),
            audience: resolved.audience.clone(),
            resource_indicators: resolved.resource.clone().unwrap_or_default(),
            authorization_details: resolved.authorization_details.clone(),
            request_uri: Some(request_uri.to_string()),
            code_challenge: resolved.code_challenge.clone(),
            code_challenge_method: resolved.code_challenge_method.clone(),
            dpop_jkt: resolved.dpop_jkt.clone(),
        },
    )
    .await;
    let hosted = match hosted {
        Ok(hosted) => hosted,
        Err(error) => {
            return oauth_error_redirect(&resolved.redirect_uri, resolved.state.as_deref(), &error);
        }
    };
    let login_url = crate::domains::oauth::hosted_service::build_hosted_login_url(
        &state.config.web_base_url,
        &hosted.state_id,
    );
    Ok(Redirect::to(&login_url).into_response())
}

fn requires_hosted_consent(error: &AppError) -> bool {
    matches!(
        error.code.as_str(),
        "consent_required" | "admin_consent_required"
    )
}

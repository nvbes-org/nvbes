use crate::{app::AppState, http::error::AppError};
use axum::{Extension, Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_redis::par as par_store;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

#[path = "identity.domains.oauth.routes.par.request.rs"]
mod request;
#[path = "identity.domains.oauth.routes.par.store.rs"]
mod store;
#[cfg(test)]
#[path = "identity.domains.oauth.routes.par.tests.rs"]
mod tests;

pub fn router() -> Router<AppState> {
    Router::new().route("/par", post(par))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParRequest {
    response_type: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
    scope: Option<String>,
    state: Option<String>,
    nonce: Option<String>,
    audience: Option<String>,
    resource: Option<Vec<String>>,
    authorization_details: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    consent_action: Option<String>,
    client_assertion_type: Option<String>,
    client_assertion: Option<String>,
    request: Option<String>,
}

async fn par(
    State(state): State<AppState>,
    headers: HeaderMap,
    dpop: Option<Extension<crate::http::middleware::dpop::DpopContext>>,
    axum::extract::Form(request): axum::extract::Form<ParRequest>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), AppError> {
    let mut client_auth = par_client_auth(
        &headers,
        request.client_id.as_deref(),
        request.client_secret.as_deref(),
        request.client_assertion_type.as_deref(),
        request.client_assertion.as_deref(),
    )?;
    let par_endpoint = format!(
        "{}/oauth/par",
        state.config.api_base_url.trim_end_matches('/')
    );
    client_auth.client_assertion_verified =
        crate::domains::oauth::client_assertion::verify_private_key_jwt(
            &state.db,
            &client_auth,
            &par_endpoint,
            state.config.api_base_url.trim_end_matches('/'),
        )
        .await?;

    let client_security =
        crate::domains::oauth::profiles::load_client_security(&state.db, &client_auth.client_id)
            .await?;
    let (request, dpop_jkt) = resolve_profile_request(
        &state,
        request,
        &client_auth,
        &client_security,
        dpop.as_ref().map(|Extension(context)| context),
    )
    .await?;
    let response_type = request.response_type.as_deref().unwrap_or("code");
    if response_type != "code" {
        return Err(AppError::bad_request(
            "invalid_response_type",
            "Only 'code' is supported",
        ));
    }

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "oauth_par",
        &client_auth.client_id,
        30,
        20,
        Duration::from_secs(60),
    )
    .await?;

    request::validate_par_request(&request)?;
    validate_registered_authorization_parameters(
        &state.db,
        &client_auth.client_id,
        &client_security.client_type,
        &request,
    )
    .await?;

    let mut parameters = serde_json::Map::new();
    parameters.insert(
        "response_type".to_string(),
        serde_json::Value::String(response_type.to_string()),
    );
    parameters.insert(
        "client_id".to_string(),
        serde_json::Value::String(client_auth.client_id.clone()),
    );
    parameters.insert(
        "redirect_uri".to_string(),
        serde_json::Value::String(request.redirect_uri.clone().unwrap_or_default()),
    );
    if let Some(ref scope) = request.scope {
        parameters.insert(
            "scope".to_string(),
            serde_json::Value::String(scope.clone()),
        );
    }
    if let Some(ref state) = request.state {
        parameters.insert(
            "state".to_string(),
            serde_json::Value::String(state.clone()),
        );
    }
    if let Some(ref nonce) = request.nonce {
        parameters.insert(
            "nonce".to_string(),
            serde_json::Value::String(nonce.clone()),
        );
    }
    if let Some(ref audience) = request.audience {
        parameters.insert(
            "audience".to_string(),
            serde_json::Value::String(audience.clone()),
        );
    }
    if let Some(ref resource) = request.resource
        && !resource.is_empty()
    {
        parameters.insert("resource".to_string(), serde_json::json!(resource));
    }
    let authorization_details = crate::domains::oauth::rar::parse_authorization_details(
        request.authorization_details.as_deref(),
    )?;
    if !authorization_details.is_empty() {
        parameters.insert(
            "authorization_details".to_string(),
            serde_json::Value::Array(authorization_details),
        );
    }
    if let Some(ref code_challenge) = request.code_challenge {
        parameters.insert(
            "code_challenge".to_string(),
            serde_json::Value::String(code_challenge.clone()),
        );
    }
    if let Some(ref code_challenge_method) = request.code_challenge_method {
        parameters.insert(
            "code_challenge_method".to_string(),
            serde_json::Value::String(code_challenge_method.clone()),
        );
    }
    if let Some(ref consent_action) = request.consent_action {
        parameters.insert(
            "consent_action".to_string(),
            serde_json::Value::String(consent_action.clone()),
        );
    }
    if let Some(dpop_jkt) = dpop_jkt {
        parameters.insert("dpop_jkt".to_string(), serde_json::Value::String(dpop_jkt));
    }

    let request_uri = format!(
        "urn:ietf:params:oauth:request_uri:gxpar_{}",
        Uuid::new_v4().simple()
    );
    let expires_in: i64 = 90;
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);

    par_store::store_pushed_authorization_request(
        &state.redis,
        &par_store::CachedPushedAuthorizationRequest {
            request_uri: request_uri.clone(),
            client_id: client_auth.client_id.clone(),
            parameters,
            expires_at,
            used_at: None,
        },
    )
    .await
    .map_err(|err| {
        AppError::internal("pushed_authorization_request_store_failed", err.to_string())
    })?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(serde_json::json!({
            "request_uri": request_uri,
            "expires_in": expires_in,
        })),
    ))
}

async fn resolve_profile_request(
    state: &AppState,
    request: ParRequest,
    client_auth: &crate::domains::oauth::service::ClientAuthentication,
    security: &crate::domains::oauth::profiles::OAuthClientSecurity,
    dpop: Option<&crate::http::middleware::dpop::DpopContext>,
) -> Result<(ParRequest, Option<String>), AppError> {
    use crate::domains::oauth::profiles::{OAuthSecurityProfile, OAuthSenderConstraint};

    if security.profile == OAuthSecurityProfile::Standard {
        if request.request.is_some() {
            return Err(AppError::bad_request(
                "request_object_not_configured",
                "Signed request objects are only accepted for high-assurance clients.",
            ));
        }
        return Ok((request, None));
    }
    if !client_auth.client_assertion_verified {
        return Err(AppError::unauthorized(
            "invalid_client",
            "High-assurance clients must authenticate PAR with private_key_jwt.",
        ));
    }
    if has_unsigned_authorization_parameters(&request) {
        return Err(AppError::bad_request(
            "invalid_request",
            "High-assurance authorization parameters must be contained only in the signed request object.",
        ));
    }
    let request_jwt = request.request.as_deref().ok_or_else(|| {
        AppError::bad_request(
            "request_object_required",
            "High-assurance clients must submit a signed request object to PAR.",
        )
    })?;
    let jwks = security
        .request_object_signing_jwks
        .as_ref()
        .ok_or_else(|| AppError::unauthorized("invalid_client", "Request-object JWKS missing."))?;
    let issuer = state.config.api_base_url.trim_end_matches('/');
    let claims = crate::domains::oauth::jar::validate_high_assurance_request_object(
        request_jwt,
        jwks,
        &client_auth.client_id,
        issuer,
    )?;
    crate::domains::oauth::jar::record_request_object_jti(
        &state.db,
        security.tenant_id,
        &client_auth.client_id,
        &claims,
    )
    .await?;

    let dpop_jkt = match security.sender_constraint {
        Some(OAuthSenderConstraint::Dpop) => Some(
            dpop.ok_or_else(|| {
                AppError::unauthorized(
                    "dpop_required",
                    "High-assurance DPoP clients must bind PAR to a DPoP key.",
                )
            })?
            .jkt
            .clone(),
        ),
        _ => None,
    };

    Ok((
        ParRequest {
            response_type: claims.response_type,
            client_id: Some(client_auth.client_id.clone()),
            client_secret: None,
            redirect_uri: claims.redirect_uri,
            scope: claims.scope,
            state: claims.state,
            nonce: claims.nonce,
            audience: claims.audience,
            resource: claims.resource,
            authorization_details: claims.authorization_details.map(|value| value.to_string()),
            code_challenge: claims.code_challenge,
            code_challenge_method: claims.code_challenge_method,
            consent_action: claims.consent_action,
            client_assertion_type: None,
            client_assertion: None,
            request: None,
        },
        dpop_jkt,
    ))
}

fn has_unsigned_authorization_parameters(request: &ParRequest) -> bool {
    request.response_type.is_some()
        || request.redirect_uri.is_some()
        || request.scope.is_some()
        || request.state.is_some()
        || request.nonce.is_some()
        || request.audience.is_some()
        || request.resource.is_some()
        || request.authorization_details.is_some()
        || request.code_challenge.is_some()
        || request.code_challenge_method.is_some()
        || request.consent_action.is_some()
}

async fn validate_registered_authorization_parameters(
    db: &sqlx::PgPool,
    client_id: &str,
    client_type: &str,
    request: &ParRequest,
) -> Result<(), AppError> {
    let redirect_uris = sqlx::query_scalar::<_, Vec<String>>(
        "SELECT redirect_uris FROM oauth_clients WHERE client_id = $1 AND revoked_at IS NULL",
    )
    .bind(client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::unauthorized("invalid_client", "The OAuth client is invalid."))?;
    let redirect_uri = request.redirect_uri.as_deref().unwrap_or_default();
    crate::domains::oauth::validation::validate_redirect_uri_allowed(&redirect_uris, redirect_uri)?;
    crate::domains::oauth::validation::validate_pkce_for_authorize(
        client_type,
        request.code_challenge.as_deref(),
        request.code_challenge_method.as_deref(),
    )
}

pub(crate) use store::resolve_pushed_parameters;

pub(crate) use store::mark_par_used;

fn par_client_auth(
    headers: &HeaderMap,
    body_client_id: Option<&str>,
    body_client_secret: Option<&str>,
    body_client_assertion_type: Option<&str>,
    body_client_assertion: Option<&str>,
) -> Result<crate::domains::oauth::service::ClientAuthentication, AppError> {
    super::token::token_client_auth(
        headers,
        body_client_id,
        body_client_secret,
        body_client_assertion_type,
        body_client_assertion,
    )
}

use crate::{app::AppState, http::error::AppError};
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_redis::par as par_store;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/par", post(par))
}

#[derive(Debug, Deserialize)]
struct ParRequest {
    response_type: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
    scope: Option<String>,
    state: Option<String>,
    audience: Option<String>,
    resource: Option<Vec<String>>,
    authorization_details: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    consent_action: Option<String>,
    client_assertion_type: Option<String>,
    client_assertion: Option<String>,
}

async fn par(
    State(state): State<AppState>,
    headers: HeaderMap,
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
        )
        .await?;

    let response_type = request.response_type.as_deref().unwrap_or("code");
    if response_type != "code" {
        return Err(AppError::bad_request(
            "invalid_response_type",
            "Only 'code' is supported",
        ));
    }

    super::enforce_public_oauth_rate_limit_db(
        &state.redis,
        &headers,
        "oauth_par",
        &client_auth.client_id,
        30,
        20,
    )
    .await?;

    validate_par_request(&request)?;

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
    if let Some(ref audience) = request.audience {
        parameters.insert(
            "audience".to_string(),
            serde_json::Value::String(audience.clone()),
        );
    }
    if let Some(ref resource) = request.resource {
        if !resource.is_empty() {
            parameters.insert("resource".to_string(), serde_json::json!(resource));
        }
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
        AppError::internal(
            "pushed_authorization_request_store_failed",
            &err.to_string(),
        )
    })?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(serde_json::json!({
            "request_uri": request_uri,
            "expires_in": expires_in,
        })),
    ))
}

pub(crate) async fn resolve_pushed_parameters(
    redis: &nvbes_redis::RedisPool,
    request_uri: &str,
    client_id: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let record = par_store::get_pushed_authorization_request(redis, request_uri)
        .await
        .map_err(|err| {
            AppError::internal("pushed_authorization_request_read_failed", &err.to_string())
        })?
        .ok_or_else(|| {
            AppError::bad_request("invalid_request_uri", "The request_uri is invalid.")
        })?;

    if record.client_id != client_id {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            "The request_uri does not belong to this client.",
        ));
    }

    if record.expires_at < chrono::Utc::now() || record.used_at.is_some() {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            "The request_uri has expired or has already been used.",
        ));
    }

    Ok(record.parameters)
}

pub(crate) async fn mark_par_used(
    redis: &nvbes_redis::RedisPool,
    request_uri: &str,
) -> Result<(), AppError> {
    par_store::mark_pushed_authorization_request_used(redis, request_uri)
        .await
        .map_err(|err| {
            AppError::internal("pushed_authorization_request_mark_failed", &err.to_string())
        })?;
    Ok(())
}

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

fn validate_par_request(request: &ParRequest) -> Result<(), AppError> {
    let redirect_uri = request
        .redirect_uri
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();

    if redirect_uri.is_empty() {
        return Err(AppError::bad_request(
            "missing_redirect_uri",
            "The redirect_uri parameter is required for PAR.",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_redis_pool() -> nvbes_redis::RedisPool {
        crate::test_support::test_redis_pool().await
    }

    #[tokio::test]
    async fn stores_and_resolves_pushed_authorization_request_via_redis() {
        let redis = test_redis_pool().await;
        let request_uri = format!(
            "urn:ietf:params:oauth:request_uri:gxpar_{}",
            Uuid::new_v4().simple()
        );
        let client_id = format!("client-{}", Uuid::new_v4());

        par_store::store_pushed_authorization_request(
            &redis,
            &par_store::CachedPushedAuthorizationRequest {
                request_uri: request_uri.clone(),
                client_id: client_id.clone(),
                parameters: serde_json::json!({
                    "redirect_uri": "https://client.example.com/cb",
                    "scope": "openid profile",
                    "state": "state-123",
                })
                .as_object()
                .cloned()
                .expect("parameters should be an object"),
                expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
                used_at: None,
            },
        )
        .await
        .expect("par should store");

        let params = resolve_pushed_parameters(&redis, &request_uri, &client_id)
            .await
            .expect("par should resolve");
        assert_eq!(
            params.get("redirect_uri").and_then(|value| value.as_str()),
            Some("https://client.example.com/cb")
        );

        mark_par_used(&redis, &request_uri)
            .await
            .expect("par should mark used");

        let error = resolve_pushed_parameters(&redis, &request_uri, &client_id)
            .await
            .expect_err("used request_uri should be rejected");
        assert_eq!(error.code, "invalid_request_uri");
    }
}

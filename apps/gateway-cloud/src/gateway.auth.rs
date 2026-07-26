use axum::http::{HeaderMap, StatusCode, header};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::{GatewayRequestContext, GatewayState};

const REQUEST_ID: &str = "x-request-id";
const CORRELATION_ID: &str = "x-correlation-id";

#[derive(Debug, Deserialize)]
struct IdentityIntrospectionResponse {
    active: bool,
    principal_type: Option<String>,
    sub: Option<String>,
    tenant_id: Option<Uuid>,
    network_valid: Option<bool>,
}

pub async fn request_context(
    state: &GatewayState,
    headers: &HeaderMap,
) -> Result<GatewayRequestContext, StatusCode> {
    let request_id =
        optional_header(headers, REQUEST_ID).unwrap_or_else(|| Uuid::new_v4().to_string());
    let correlation_id =
        optional_header(headers, CORRELATION_ID).unwrap_or_else(|| request_id.clone());
    let identity = introspect_identity_token(state, headers).await?;
    let actor_principal_id = identity
        .sub
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_string();
    let tenant_id = identity.tenant_id.ok_or(StatusCode::FORBIDDEN)?.to_string();

    Ok(GatewayRequestContext {
        request_id,
        correlation_id,
        actor_principal_id,
        tenant_id,
    })
}

async fn introspect_identity_token(
    state: &GatewayState,
    headers: &HeaderMap,
) -> Result<IdentityIntrospectionResponse, StatusCode> {
    let token = bearer_token(headers)?;
    let mut outgoing = HeaderMap::new();
    nvbes_observability::propagate_headers_trace_context(headers, &mut outgoing);
    let response = reqwest::Client::new()
        .post(format!(
            "{}/oauth/introspect",
            state.identity_base_url.trim_end_matches('/')
        ))
        .headers(outgoing)
        .basic_auth(
            &state.identity_client_id,
            Some(&state.identity_client_secret),
        )
        .json(&serde_json::json!({
            "token": token,
            "token_type_hint": "access_token",
        }))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    let identity = response
        .json::<IdentityIntrospectionResponse>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    validate_identity(identity)
}

fn validate_identity(
    identity: IdentityIntrospectionResponse,
) -> Result<IdentityIntrospectionResponse, StatusCode> {
    if !identity.active {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if identity.network_valid == Some(false) {
        return Err(StatusCode::FORBIDDEN);
    }
    if identity.principal_type.as_deref() != Some("user") {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(identity)
}

fn bearer_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(StatusCode::UNAUTHORIZED)
}

fn optional_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

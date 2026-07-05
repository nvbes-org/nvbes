use axum::http::{HeaderMap, StatusCode};
use uuid::Uuid;

use crate::state::GatewayRequestContext;

const REQUEST_ID: &str = "x-request-id";
const CORRELATION_ID: &str = "x-correlation-id";
const ACTOR_PRINCIPAL_ID: &str = "x-nvbes-actor-principal-id";
const TENANT_ID: &str = "x-nvbes-tenant-id";

pub fn request_context(headers: &HeaderMap) -> Result<GatewayRequestContext, StatusCode> {
    let request_id = optional_header(headers, REQUEST_ID).unwrap_or_else(|| Uuid::new_v4().to_string());
    let correlation_id = optional_header(headers, CORRELATION_ID).unwrap_or_else(|| request_id.clone());
    let actor_principal_id = required_uuid_header(headers, ACTOR_PRINCIPAL_ID)?;
    let tenant_id = required_uuid_header(headers, TENANT_ID)?;

    Ok(GatewayRequestContext {
        request_id,
        correlation_id,
        actor_principal_id,
        tenant_id,
    })
}

fn required_uuid_header(headers: &HeaderMap, name: &'static str) -> Result<String, StatusCode> {
    let value = optional_header(headers, name).ok_or(StatusCode::UNAUTHORIZED)?;
    Uuid::parse_str(&value).map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(value)
}

fn optional_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

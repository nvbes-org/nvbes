use axum::http::{HeaderMap, StatusCode, header};
use uuid::Uuid;

use crate::pb::nvbes::identity::internal::v1::IntrospectAccessTokenResponse;
use crate::state::{GatewayRequestContext, GatewayState};

const REQUEST_ID: &str = "x-request-id";
const CORRELATION_ID: &str = "x-correlation-id";
const CLOUD_AUDIENCE: &str = "nvbes-cloud-service";

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
    let tenant_id = identity
        .tenant_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or(StatusCode::FORBIDDEN)?
        .to_string();

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
) -> Result<IntrospectAccessTokenResponse, StatusCode> {
    let token = bearer_token(headers)?;
    let identity = state.identity.introspect(token, headers).await?;
    validate_identity(identity)
}

fn validate_identity(
    identity: IntrospectAccessTokenResponse,
) -> Result<IntrospectAccessTokenResponse, StatusCode> {
    if !identity.active {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if identity.audience.as_deref() != Some(CLOUD_AUDIENCE) {
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

#[cfg(test)]
mod tests {
    use super::{CLOUD_AUDIENCE, IntrospectAccessTokenResponse, validate_identity};
    use axum::http::StatusCode;

    #[test]
    fn gateway_accepts_only_cloud_audience() {
        let valid = IntrospectAccessTokenResponse {
            active: true,
            audience: Some(CLOUD_AUDIENCE.to_string()),
            principal_type: Some("user".to_string()),
            ..Default::default()
        };
        assert!(validate_identity(valid).is_ok());

        let foreign = IntrospectAccessTokenResponse {
            active: true,
            audience: Some("nvbes-account-service".to_string()),
            principal_type: Some("user".to_string()),
            ..Default::default()
        };
        assert_eq!(validate_identity(foreign), Err(StatusCode::UNAUTHORIZED));
    }
}

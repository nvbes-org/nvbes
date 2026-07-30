use std::{sync::OnceLock, time::Duration};

use base64::Engine;
use tonic::{
    Code, Request,
    transport::{Channel, Endpoint},
};

use crate::{
    grpc::pb::nvbes::identity::internal::v1::{
        IntrospectAccessTokenRequest, IntrospectAccessTokenResponse,
        identity_internal_service_client::IdentityInternalServiceClient,
    },
    http::error::AppError,
};

static IDENTITY_CHANNEL: OnceLock<Channel> = OnceLock::new();
const INTROSPECTION_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn introspect(
    headers: &axum::http::HeaderMap,
    access_token: &str,
) -> Result<IntrospectAccessTokenResponse, AppError> {
    let client_id = required_env("NVBES_BILLING_IDENTITY_CLIENT_ID")?;
    let client_secret = required_env("NVBES_BILLING_IDENTITY_CLIENT_SECRET")?;
    let client_ip = nvbes_core::http::client_ip::client_ip(headers).unwrap_or_default();
    let mut request = Request::new(IntrospectAccessTokenRequest {
        access_token: access_token.to_string(),
        client_ip,
    });
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
    request.metadata_mut().insert(
        "authorization",
        format!("Basic {encoded}").parse().map_err(|_| {
            AppError::internal(
                "identity_grpc_credential_invalid",
                "Identity gRPC credential contains invalid metadata.",
            )
        })?,
    );
    request.set_timeout(INTROSPECTION_TIMEOUT);
    IdentityInternalServiceClient::new(identity_channel()?)
        .introspect_access_token(request)
        .await
        .map_err(grpc_error)
        .map(tonic::Response::into_inner)
}

fn identity_channel() -> Result<Channel, AppError> {
    if let Some(channel) = IDENTITY_CHANNEL.get() {
        return Ok(channel.clone());
    }
    let endpoint = std::env::var("NVBES_IDENTITY_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4010".to_string());
    let channel = Endpoint::from_shared(endpoint)
        .map_err(|error| AppError::internal("identity_grpc_endpoint_invalid", error.to_string()))?
        .connect_lazy();
    let _ = IDENTITY_CHANNEL.set(channel.clone());
    Ok(IDENTITY_CHANNEL.get().cloned().unwrap_or(channel))
}

fn required_env(name: &'static str) -> Result<String, AppError> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::internal(
                "identity_grpc_configuration_missing",
                format!("{name} is required to introspect Identity tokens."),
            )
        })
}

fn grpc_error(error: tonic::Status) -> AppError {
    match error.code() {
        Code::Unauthenticated => AppError::unauthorized("invalid_token", error.message()),
        Code::PermissionDenied => {
            AppError::forbidden("identity_introspection_forbidden", error.message())
        }
        _ => AppError::internal("identity_introspection_failed", error.to_string()),
    }
}

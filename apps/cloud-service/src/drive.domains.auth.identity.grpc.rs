use std::time::Duration;

use base64::Engine;
use tonic::{
    Code, Request,
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
};
use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::identity::internal::v1::{
        IntrospectAccessTokenRequest, IntrospectAccessTokenResponse,
        identity_internal_service_client::IdentityInternalServiceClient,
    },
    http::error::AppError,
};

use super::types::IdentityIntrospectionResponse;

const ACCOUNT_GRPC_ENDPOINT_ENV: &str = "NVBES_ACCOUNT_GRPC_ENDPOINT";
const ACCOUNT_CLIENT_ID_ENV: &str = "NVBES_ACCOUNT_SERVICE_CLIENT_ID";
const ACCOUNT_CLIENT_SECRET_ENV: &str = "NVBES_ACCOUNT_SERVICE_CLIENT_SECRET";
const INTROSPECTION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(super) struct IdentityGrpcClient {
    client: IdentityInternalServiceClient<Channel>,
    authorization: MetadataValue<tonic::metadata::Ascii>,
}

impl IdentityGrpcClient {
    pub(super) fn from_env() -> Result<Self, AppError> {
        let endpoint = std::env::var(ACCOUNT_GRPC_ENDPOINT_ENV)
            .unwrap_or_else(|_| "http://127.0.0.1:4010".to_string());
        let client_id = required_env(ACCOUNT_CLIENT_ID_ENV)?;
        let client_secret = required_env(ACCOUNT_CLIENT_SECRET_ENV)?;
        let channel = Endpoint::from_shared(endpoint)
            .map_err(|error| {
                AppError::internal("identity_grpc_endpoint_invalid", error.to_string())
            })?
            .connect_lazy();
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));
        let authorization = format!("Basic {encoded}").parse().map_err(|_| {
            AppError::internal(
                "identity_grpc_credential_invalid",
                "Identity gRPC credential contains invalid metadata.",
            )
        })?;
        Ok(Self {
            client: IdentityInternalServiceClient::new(channel),
            authorization,
        })
    }

    pub(super) async fn introspect(
        &self,
        access_token: &str,
        headers: Option<&axum::http::HeaderMap>,
    ) -> Result<IdentityIntrospectionResponse, AppError> {
        let client_ip = headers
            .and_then(nvbes_core::http::client_ip::client_ip)
            .unwrap_or_default();
        let mut request = Request::new(IntrospectAccessTokenRequest {
            access_token: access_token.to_string(),
            client_ip,
        });
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request.set_timeout(INTROSPECTION_TIMEOUT);
        let response = self
            .client
            .clone()
            .introspect_access_token(request)
            .await
            .map_err(grpc_error)?
            .into_inner();
        convert_response(response)
    }
}

fn convert_response(
    value: IntrospectAccessTokenResponse,
) -> Result<IdentityIntrospectionResponse, AppError> {
    Ok(IdentityIntrospectionResponse {
        active: value.active,
        scope: value.scope,
        client_id: value.client_id,
        principal_type: value.principal_type,
        token_type: value.token_type,
        sub: value.sub,
        role: value.role,
        tenant_id: optional_uuid(value.tenant_id, "tenant_id")?,
        organization_id: optional_uuid(value.organization_id, "organization_id")?,
        workspace_id: optional_uuid(value.workspace_id, "workspace_id")?,
        username: value.username,
        email: value.email,
        email_verified: value.email_verified,
        name: value.display_name,
        acr: value.acr,
        amr: value.amr,
        auth_time: value.auth_time,
        jti: value.jti,
        sid: value.sid,
        exp: value.exp,
        iat: value.iat,
        nbf: value.nbf,
        act: optional_json(value.act_json, "act")?,
        actor_principal_type: value.actor_principal_type,
        actor_role: value.actor_role,
        actor_workspace_id: optional_uuid(value.actor_workspace_id, "actor_workspace_id")?,
        actor_organization_id: optional_uuid(value.actor_organization_id, "actor_organization_id")?,
        actor_tenant_id: optional_uuid(value.actor_tenant_id, "actor_tenant_id")?,
        network_valid: value.network_valid,
    })
}

fn optional_uuid(value: Option<String>, field: &str) -> Result<Option<Uuid>, AppError> {
    value
        .map(|value| {
            Uuid::parse_str(&value).map_err(|_| {
                AppError::internal(
                    "identity_grpc_response_invalid",
                    format!("Identity gRPC response contains an invalid {field}."),
                )
            })
        })
        .transpose()
}

fn optional_json(
    value: Option<String>,
    field: &str,
) -> Result<Option<serde_json::Value>, AppError> {
    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|_| {
                AppError::internal(
                    "identity_grpc_response_invalid",
                    format!("Identity gRPC response contains invalid {field} JSON."),
                )
            })
        })
        .transpose()
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

#[cfg(test)]
mod tests {
    use super::{IntrospectAccessTokenResponse, convert_response};

    #[test]
    fn rejects_invalid_uuid_in_response() {
        let response = IntrospectAccessTokenResponse {
            tenant_id: Some("not-a-uuid".to_string()),
            ..Default::default()
        };

        assert!(convert_response(response).is_err());
    }
}
